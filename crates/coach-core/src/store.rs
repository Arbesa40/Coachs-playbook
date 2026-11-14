//! One SQLite file per team; per-field HLC last-writer-wins, tombstone deletes, and a change log for sync.

use crate::hlc::{self, Clock};
use crate::{is_append_only, is_bookkeeping, new_id, Error, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const DELETED_FIELD: &str = "_deleted";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Change {
    pub kind: String,
    pub record_id: String,
    pub field: String,
    pub value: Value,
    pub hlc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Peer {
    pub device_id: String,
    pub name: String,
    /// Base64 shared key from pairing.
    pub key: String,
    /// Highest change seq we have pulled from this peer.
    pub last_seq_received: i64,
    /// Our own clock at the end of the last successful sync with this peer.
    pub last_sync_hlc: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Conflict {
    pub id: i64,
    pub kind: String,
    pub record_id: String,
    pub field: String,
    pub local_value: Value,
    pub remote_value: Value,
    pub winner: String,
    pub remote_device: String,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct ApplyReport {
    pub received: usize,
    pub applied: usize,
    pub conflicts: usize,
}

const MIGRATIONS: &[&str] = &[
    // v1
    r#"
    CREATE TABLE records (
        kind TEXT NOT NULL,
        id TEXT NOT NULL,
        data TEXT NOT NULL DEFAULT '{}',
        deleted INTEGER NOT NULL DEFAULT 0,
        hlc TEXT NOT NULL,
        created_hlc TEXT NOT NULL,
        PRIMARY KEY (kind, id)
    );
    CREATE INDEX records_by_kind ON records(kind, deleted, created_hlc);

    CREATE TABLE field_clocks (
        kind TEXT NOT NULL,
        id TEXT NOT NULL,
        field TEXT NOT NULL,
        hlc TEXT NOT NULL,
        PRIMARY KEY (kind, id, field)
    );

    CREATE TABLE changes (
        seq INTEGER PRIMARY KEY AUTOINCREMENT,
        kind TEXT NOT NULL,
        record_id TEXT NOT NULL,
        field TEXT NOT NULL,
        value TEXT NOT NULL,
        hlc TEXT NOT NULL,
        device_id TEXT NOT NULL
    );
    CREATE UNIQUE INDEX changes_unique ON changes(kind, record_id, field, hlc);

    CREATE TABLE peers (
        device_id TEXT PRIMARY KEY,
        name TEXT NOT NULL DEFAULT '',
        key TEXT NOT NULL DEFAULT '',
        last_seq_received INTEGER NOT NULL DEFAULT 0,
        last_sync_hlc TEXT NOT NULL DEFAULT ''
    );

    CREATE TABLE conflicts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        kind TEXT NOT NULL,
        record_id TEXT NOT NULL,
        field TEXT NOT NULL,
        local_value TEXT NOT NULL,
        remote_value TEXT NOT NULL,
        winner TEXT NOT NULL,
        remote_device TEXT NOT NULL,
        hlc TEXT NOT NULL,
        resolved INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
    "#,
];

pub struct Store {
    conn: Connection,
    clock: Clock,
    path: PathBuf,
}

impl Store {
    pub fn open(path: impl AsRef<Path>, device_id: &str) -> Result<Store> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(&path)?;
        Self::init(conn, path, device_id)
    }

    pub fn open_in_memory(device_id: &str) -> Result<Store> {
        Self::init(Connection::open_in_memory()?, PathBuf::new(), device_id)
    }

    fn init(conn: Connection, path: PathBuf, device_id: &str) -> Result<Store> {
        let _mode: String =
            conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        // The UI, the media queue and a sync session may each hold a connection.
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        migrate(&conn)?;
        let clock = Clock::new(device_id);
        let max: Option<String> =
            conn.query_row("SELECT MAX(hlc) FROM changes", [], |r| r.get(0))?;
        if let Some(m) = max {
            clock.observe(&m);
        }
        Ok(Store { conn, clock, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn device_id(&self) -> &str {
        self.clock.device()
    }

    // ---------- meta ----------

    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key=?1", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ---------- writes ----------

    /// Create or update a record. Only the given fields are touched.
    pub fn put(&mut self, kind: &str, id: Option<&str>, fields: &Map<String, Value>) -> Result<Value> {
        let id = id.map(str::to_string).unwrap_or_else(new_id);
        let tx = self.conn.transaction()?;
        let mut wrote = false;
        for (field, value) in fields {
            if field == "id" {
                continue;
            }
            // Catch up with newer clocks another connection wrote, or this edit could lose to them.
            observe_field_clock(&tx, &self.clock, kind, &id, field)?;
            let c = Change {
                kind: kind.into(),
                record_id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                hlc: self.clock.now(),
            };
            apply(&tx, &c, None)?;
            wrote = true;
        }
        if !wrote {
            // Make sure an empty record still exists and syncs.
            observe_field_clock(&tx, &self.clock, kind, &id, DELETED_FIELD)?;
            let c = Change {
                kind: kind.into(),
                record_id: id.clone(),
                field: DELETED_FIELD.into(),
                value: Value::Bool(false),
                hlc: self.clock.now(),
            };
            apply(&tx, &c, None)?;
        }
        tx.commit()?;
        self.get(kind, &id)?
            .ok_or_else(|| Error::Other(format!("{kind} {id} missing after write")))
    }

    pub fn delete(&mut self, kind: &str, id: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        observe_field_clock(&tx, &self.clock, kind, id, DELETED_FIELD)?;
        let c = Change {
            kind: kind.into(),
            record_id: id.into(),
            field: DELETED_FIELD.into(),
            value: Value::Bool(true),
            hlc: self.clock.now(),
        };
        apply(&tx, &c, None)?;
        tx.commit()?;
        Ok(())
    }

    // ---------- reads ----------

    pub fn get(&self, kind: &str, id: &str) -> Result<Option<Value>> {
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT id, data FROM records WHERE kind=?1 AND id=?2 AND deleted=0",
                params![kind, id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map(|(id, data)| to_value(id, &data)).transpose()
    }

    /// All live records of a kind, oldest first.
    pub fn list(&self, kind: &str) -> Result<Vec<Value>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, data FROM records WHERE kind=?1 AND deleted=0 ORDER BY created_hlc",
        )?;
        let rows = stmt.query_map([kind], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (id, data) = row?;
            out.push(to_value(id, &data)?);
        }
        Ok(out)
    }

    /// Records of a kind whose `field` equals `value`. Team data is small, so we filter in Rust.
    pub fn list_by(&self, kind: &str, field: &str, value: &Value) -> Result<Vec<Value>> {
        Ok(self
            .list(kind)?
            .into_iter()
            .filter(|r| r.get(field) == Some(value))
            .collect())
    }

    // ---------- sync ----------

    pub fn max_seq(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COALESCE(MAX(seq),0) FROM changes", [], |r| r.get(0))?)
    }

    /// Changes after `seq`, at most `limit`, and the last seq included.
    pub fn changes_since(&self, seq: i64, limit: usize) -> Result<(Vec<Change>, i64)> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT seq, kind, record_id, field, value, hlc FROM changes WHERE seq > ?1 ORDER BY seq LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![seq, limit as i64], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })?;
        let mut out = Vec::new();
        let mut last = seq;
        for row in rows {
            let (s, kind, record_id, field, value, hlc) = row?;
            last = s;
            out.push(Change { kind, record_id, field, value: serde_json::from_str(&value)?, hlc });
        }
        Ok((out, last))
    }

    /// Merge changes received from another device.
    pub fn apply_remote(&mut self, remote_device: &str, changes: &[Change]) -> Result<ApplyReport> {
        let last_sync = self
            .peer(remote_device)?
            .map(|p| p.last_sync_hlc)
            .unwrap_or_default();
        let mut report = ApplyReport { received: changes.len(), ..Default::default() };
        let tx = self.conn.transaction()?;
        for c in changes {
            self.clock.observe(&c.hlc);
            let (applied, conflict) = apply(&tx, c, Some(&last_sync))?;
            report.applied += applied as usize;
            report.conflicts += conflict as usize;
        }
        tx.commit()?;
        Ok(report)
    }

    pub fn peer(&self, device_id: &str) -> Result<Option<Peer>> {
        Ok(self
            .conn
            .query_row(
                "SELECT device_id, name, key, last_seq_received, last_sync_hlc FROM peers WHERE device_id=?1",
                [device_id],
                |r| {
                    Ok(Peer {
                        device_id: r.get(0)?,
                        name: r.get(1)?,
                        key: r.get(2)?,
                        last_seq_received: r.get(3)?,
                        last_sync_hlc: r.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn peers(&self) -> Result<Vec<Peer>> {
        let mut stmt = self
            .conn
            .prepare("SELECT device_id, name, key, last_seq_received, last_sync_hlc FROM peers")?;
        let rows = stmt.query_map([], |r| {
            Ok(Peer {
                device_id: r.get(0)?,
                name: r.get(1)?,
                key: r.get(2)?,
                last_seq_received: r.get(3)?,
                last_sync_hlc: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<Peer>>>()?)
    }

    pub fn save_peer(&self, p: &Peer) -> Result<()> {
        self.conn.execute(
            "INSERT INTO peers(device_id,name,key,last_seq_received,last_sync_hlc) VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(device_id) DO UPDATE SET name=excluded.name, key=excluded.key,
               last_seq_received=excluded.last_seq_received, last_sync_hlc=excluded.last_sync_hlc",
            params![p.device_id, p.name, p.key, p.last_seq_received, p.last_sync_hlc],
        )?;
        Ok(())
    }

    /// Record a finished sync: where we got to in their log, and our clock now.
    pub fn finish_sync(&self, remote_device: &str, last_seq_received: i64) -> Result<()> {
        let now = self.clock.now();
        self.conn.execute(
            "UPDATE peers SET last_seq_received=MAX(last_seq_received, ?2), last_sync_hlc=?3 WHERE device_id=?1",
            params![remote_device, last_seq_received, now],
        )?;
        Ok(())
    }

    // ---------- conflicts ----------

    pub fn conflicts(&self) -> Result<Vec<Conflict>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, record_id, field, local_value, remote_value, winner, remote_device
             FROM conflicts WHERE resolved=0 ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, kind, record_id, field, lv, rv, winner, remote_device) = row?;
            out.push(Conflict {
                id,
                kind,
                record_id,
                field,
                local_value: serde_json::from_str(&lv)?,
                remote_value: serde_json::from_str(&rv)?,
                winner,
                remote_device,
            });
        }
        Ok(out)
    }

    /// Settle a conflict by writing the chosen value as a fresh edit.
    pub fn resolve_conflict(&mut self, conflict_id: i64, keep_local: bool) -> Result<()> {
        let c = self
            .conflicts()?
            .into_iter()
            .find(|c| c.id == conflict_id)
            .ok_or_else(|| Error::Other("conflict not found".into()))?;
        let value = if keep_local { c.local_value } else { c.remote_value };
        let mut fields = Map::new();
        fields.insert(c.field.clone(), value);
        self.put(&c.kind, Some(&c.record_id), &fields)?;
        self.conn
            .execute("UPDATE conflicts SET resolved=1 WHERE id=?1", [conflict_id])?;
        Ok(())
    }

    // ---------- backup ----------

    /// Write a consistent copy of the whole database to one file.
    pub fn backup_to(&self, dest: &Path) -> Result<()> {
        if dest.exists() {
            std::fs::remove_file(dest)?;
        }
        self.conn
            .execute("VACUUM INTO ?1", [dest.to_string_lossy().to_string()])?;
        Ok(())
    }
}

fn migrate(conn: &Connection) -> Result<()> {
    let v: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let target = i as i64 + 1;
        if v < target {
            conn.execute_batch(sql)?;
            conn.pragma_update(None, "user_version", target)?;
        }
    }
    Ok(())
}

/// Fold a field's stored clock into `clock` so the next local write is newer.
fn observe_field_clock(conn: &Connection, clock: &Clock, kind: &str, id: &str, field: &str) -> Result<()> {
    let seen: Option<String> = conn
        .query_row(
            "SELECT hlc FROM field_clocks WHERE kind=?1 AND id=?2 AND field=?3",
            params![kind, id, field],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(h) = seen {
        clock.observe(&h);
    }
    Ok(())
}

fn to_value(id: String, data: &str) -> Result<Value> {
    let mut m: Map<String, Value> = serde_json::from_str(data)?;
    m.insert("id".into(), Value::String(id));
    Ok(Value::Object(m))
}

/// Apply one change; returns (field updated, conflict recorded).
fn apply(conn: &Connection, c: &Change, remote_last_sync: Option<&str>) -> Result<(bool, bool)> {
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO changes(kind,record_id,field,value,hlc,device_id) VALUES(?1,?2,?3,?4,?5,?6)",
        params![c.kind, c.record_id, c.field, c.value.to_string(), c.hlc, hlc::device_of(&c.hlc)],
    )?;
    if inserted == 0 {
        return Ok((false, false)); // already have it
    }

    let current: Option<String> = conn
        .query_row(
            "SELECT hlc FROM field_clocks WHERE kind=?1 AND id=?2 AND field=?3",
            params![c.kind, c.record_id, c.field],
            |r| r.get(0),
        )
        .optional()?;

    let row: Option<(String, bool)> = conn
        .query_row(
            "SELECT data, deleted FROM records WHERE kind=?1 AND id=?2",
            params![c.kind, c.record_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let exists = row.is_some();
    let (data_s, mut deleted) = row.unwrap_or_else(|| ("{}".into(), false));
    let mut data: Map<String, Value> = serde_json::from_str(&data_s)?;

    let local_value = if c.field == DELETED_FIELD {
        Value::Bool(deleted)
    } else {
        data.get(&c.field).cloned().unwrap_or(Value::Null)
    };

    let wins = current.as_deref().map_or(true, |cur| c.hlc.as_str() > cur);

    let mut conflict = false;
    if let (Some(last_sync), Some(cur)) = (remote_last_sync, current.as_deref()) {
        let concurrent = cur > last_sync && hlc::device_of(cur) != hlc::device_of(&c.hlc);
        if concurrent && !is_append_only(&c.kind) && !is_bookkeeping(&c.field) && local_value != c.value {
            conflict = true;
            conn.execute(
                "INSERT INTO conflicts(kind,record_id,field,local_value,remote_value,winner,remote_device,hlc)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    c.kind,
                    c.record_id,
                    c.field,
                    local_value.to_string(),
                    c.value.to_string(),
                    if wins { "remote" } else { "local" },
                    hlc::device_of(&c.hlc),
                    c.hlc
                ],
            )?;
        }
    }

    if !wins {
        return Ok((false, conflict));
    }

    if c.field == DELETED_FIELD {
        deleted = c.value.as_bool().unwrap_or(false);
    } else {
        data.insert(c.field.clone(), c.value.clone());
    }
    let data_s = serde_json::to_string(&data)?;
    if exists {
        conn.execute(
            "UPDATE records SET data=?3, deleted=?4, hlc=MAX(hlc, ?5) WHERE kind=?1 AND id=?2",
            params![c.kind, c.record_id, data_s, deleted, c.hlc],
        )?;
    } else {
        conn.execute(
            "INSERT INTO records(kind,id,data,deleted,hlc,created_hlc) VALUES(?1,?2,?3,?4,?5,?5)",
            params![c.kind, c.record_id, data_s, deleted, c.hlc],
        )?;
    }
    conn.execute(
        "INSERT INTO field_clocks(kind,id,field,hlc) VALUES(?1,?2,?3,?4)
         ON CONFLICT(kind,id,field) DO UPDATE SET hlc=excluded.hlc",
        params![c.kind, c.record_id, c.field, c.hlc],
    )?;
    Ok((true, conflict))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fields(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn sync(a: &mut Store, b: &mut Store) {
        let (ca, sa) = a.changes_since(0, 10_000).unwrap();
        let (cb, sb) = b.changes_since(0, 10_000).unwrap();
        let (ida, idb) = (a.device_id().to_string(), b.device_id().to_string());
        for (s, other) in [(&*a, &idb), (&*b, &ida)] {
            if s.peer(other).unwrap().is_none() {
                s.save_peer(&Peer { device_id: other.clone(), ..Default::default() }).unwrap();
            }
        }
        b.apply_remote(&ida, &ca).unwrap();
        a.apply_remote(&idb, &cb).unwrap();
        a.finish_sync(&idb, sb).unwrap();
        b.finish_sync(&ida, sa).unwrap();
    }

    #[test]
    fn put_get_list_delete() {
        let mut s = Store::open_in_memory("dev-a").unwrap();
        let p = s.put("player", None, &fields(json!({"name": "Ann", "number": 7}))).unwrap();
        let id = p["id"].as_str().unwrap().to_string();
        assert_eq!(s.get("player", &id).unwrap().unwrap()["number"], 7);
        s.put("player", Some(&id), &fields(json!({"number": 9}))).unwrap();
        let got = s.get("player", &id).unwrap().unwrap();
        assert_eq!(got["number"], 9);
        assert_eq!(got["name"], "Ann");
        assert_eq!(s.list_by("player", "number", &json!(9)).unwrap().len(), 1);
        s.delete("player", &id).unwrap();
        assert!(s.get("player", &id).unwrap().is_none());
        assert!(s.list("player").unwrap().is_empty());
    }

    #[test]
    fn two_devices_converge_per_field() {
        let mut a = Store::open_in_memory("dev-a").unwrap();
        let mut b = Store::open_in_memory("dev-b").unwrap();
        let p = a.put("player", None, &fields(json!({"name": "Ann", "position": "GK"}))).unwrap();
        let id = p["id"].as_str().unwrap().to_string();
        sync(&mut a, &mut b);
        assert_eq!(b.get("player", &id).unwrap().unwrap()["name"], "Ann");

        // Different fields edited on each side: both survive.
        a.put("player", Some(&id), &fields(json!({"name": "Anne"}))).unwrap();
        b.put("player", Some(&id), &fields(json!({"position": "CB"}))).unwrap();
        sync(&mut a, &mut b);
        for s in [&a, &b] {
            let r = s.get("player", &id).unwrap().unwrap();
            assert_eq!(r["name"], "Anne");
            assert_eq!(r["position"], "CB");
        }
        assert!(a.conflicts().unwrap().is_empty());
    }

    #[test]
    fn same_field_conflict_is_flagged_and_converges() {
        let mut a = Store::open_in_memory("dev-a").unwrap();
        let mut b = Store::open_in_memory("dev-b").unwrap();
        let p = a.put("player", None, &fields(json!({"position": "GK"}))).unwrap();
        let id = p["id"].as_str().unwrap().to_string();
        sync(&mut a, &mut b);
        a.put("player", Some(&id), &fields(json!({"position": "LB"}))).unwrap();
        b.put("player", Some(&id), &fields(json!({"position": "RB"}))).unwrap();
        sync(&mut a, &mut b);
        let va = a.get("player", &id).unwrap().unwrap()["position"].clone();
        let vb = b.get("player", &id).unwrap().unwrap()["position"].clone();
        assert_eq!(va, vb, "devices must converge");
        assert_eq!(a.conflicts().unwrap().len() + b.conflicts().unwrap().len(), 2);
    }

    #[test]
    fn append_only_never_conflicts_and_deletes_sync() {
        let mut a = Store::open_in_memory("dev-a").unwrap();
        let mut b = Store::open_in_memory("dev-b").unwrap();
        a.put("attendance", None, &fields(json!({"player_id": "p1", "present": true}))).unwrap();
        b.put("attendance", None, &fields(json!({"player_id": "p2", "present": true}))).unwrap();
        let note = a.put("note", None, &fields(json!({"text": "hi"}))).unwrap();
        sync(&mut a, &mut b);
        assert_eq!(a.list("attendance").unwrap().len(), 2);
        assert_eq!(b.list("attendance").unwrap().len(), 2);
        a.delete("note", note["id"].as_str().unwrap()).unwrap();
        sync(&mut a, &mut b);
        assert!(b.list("note").unwrap().is_empty());
        assert!(a.conflicts().unwrap().is_empty() && b.conflicts().unwrap().is_empty());
    }

    #[test]
    fn reapplying_changes_is_idempotent() {
        let mut a = Store::open_in_memory("dev-a").unwrap();
        let mut b = Store::open_in_memory("dev-b").unwrap();
        a.put("tag", None, &fields(json!({"code": "Shot"}))).unwrap();
        let (c, _) = a.changes_since(0, 100).unwrap();
        let r1 = b.apply_remote("dev-a", &c).unwrap();
        let r2 = b.apply_remote("dev-a", &c).unwrap();
        assert_eq!(r1.applied, 1);
        assert_eq!(r2.applied, 0);
        assert_eq!(b.list("tag").unwrap().len(), 1);
    }

    #[test]
    fn local_edit_wins_after_another_connection_applied_a_future_change() {
        // The PC UI and its sync server hold separate connections (and clocks).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("team.sqlite");
        let mut ui = Store::open(&path, "pc").unwrap();
        ui.put("tactic", Some("t1"), &fields(json!({"name": "Old"}))).unwrap();
        let mut syncer = Store::open(&path, "pc").unwrap();
        // A phone whose clock runs an hour ahead renames it.
        let ahead = crate::hlc::Timestamp { millis: 4_000_000_000_000, counter: 0 }.encode("phone");
        let change = Change { kind: "tactic".into(), record_id: "t1".into(), field: "name".into(), value: json!("Phone"), hlc: ahead };
        syncer.apply_remote("phone", &[change]).unwrap();
        assert_eq!(ui.get("tactic", "t1").unwrap().unwrap()["name"], "Phone");
        // Editing on the PC afterwards must take effect, not be silently ignored.
        ui.put("tactic", Some("t1"), &fields(json!({"name": "PC"}))).unwrap();
        assert_eq!(ui.get("tactic", "t1").unwrap().unwrap()["name"], "PC");
        ui.delete("tactic", "t1").unwrap();
        assert!(ui.get("tactic", "t1").unwrap().is_none());
    }

    #[test]
    fn reopen_from_disk_and_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("team.sqlite");
        {
            let mut s = Store::open(&path, "dev-a").unwrap();
            s.put("player", Some("p1"), &fields(json!({"name": "Ann"}))).unwrap();
            s.backup_to(&dir.path().join("backup.sqlite")).unwrap();
        }
        let s = Store::open(dir.path().join("backup.sqlite"), "dev-a").unwrap();
        assert_eq!(s.get("player", "p1").unwrap().unwrap()["name"], "Ann");
    }
}
