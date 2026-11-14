//! Encrypted device-to-device sync with strict turn-taking: hello, pull both ways, files, bye.

use crate::crypto::{self, SyncKey};
use crate::store::{ApplyReport, Change, Peer, Store};
use crate::{Error, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

pub const PROTOCOL: u32 = 1;
const BATCH: usize = 500;
const CHUNK: usize = 512 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct Intro {
    device_id: String,
    pairing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub device_id: String,
    pub name: String,
    pub team_id: String,
    pub protocol: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
enum Msg {
    Hello(Hello),
    Pull { since: i64 },
    Changes { changes: Vec<Change>, last_seq: i64, more: bool },
    File { name: String, size: u64, kind: String, meta: Value },
    Chunk { data: String },
    FileEnd,
    NoMoreFiles,
    Bye,
    Error { message: String },
}

pub struct Channel<S: Read + Write> {
    s: S,
    key: SyncKey,
}

impl<S: Read + Write> Channel<S> {
    fn send(&mut self, m: &Msg) -> Result<()> {
        let pt = serde_json::to_vec(m)?;
        crypto::write_frame(&mut self.s, &crypto::seal(&self.key, &pt)?)
    }

    fn recv(&mut self) -> Result<Msg> {
        let f = crypto::read_frame(&mut self.s)?;
        let pt = crypto::open(&self.key, &f)?;
        match serde_json::from_slice(&pt)? {
            Msg::Error { message } => Err(Error::Protocol(format!("peer: {message}"))),
            m => Ok(m),
        }
    }
}

/// A file to hand to the other device (e.g. a video recorded on the phone).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingFile {
    pub path: PathBuf,
    pub kind: String,
    pub meta: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivedFile {
    pub path: PathBuf,
    pub kind: String,
    pub meta: Value,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct SessionReport {
    pub peer_device: String,
    pub peer_name: String,
    pub pulled: ApplyReport,
    pub sent_changes: usize,
    pub files_received: Vec<ReceivedFile>,
    pub files_sent: usize,
    pub conflicts_total: usize,
}

/// Phone side: open the channel. `pairing` is true right after scanning a QR code.
pub fn connect<S: Read + Write>(mut s: S, my_device: &str, key: SyncKey, pairing: bool) -> Result<Channel<S>> {
    let intro = Intro { device_id: my_device.into(), pairing };
    crypto::write_frame(&mut s, &serde_json::to_vec(&intro)?)?;
    Ok(Channel { s, key })
}

/// Desktop side: use the paired phone's key, or the one-time key from the QR code on screen.
pub fn accept<S: Read + Write>(
    store: &Store,
    mut s: S,
    pending_pairing_key: Option<SyncKey>,
) -> Result<(Channel<S>, String)> {
    let intro: Intro = serde_json::from_slice(&crypto::read_frame(&mut s)?)?;
    let key = if intro.pairing {
        pending_pairing_key.ok_or_else(|| Error::Protocol("pairing is not open on this PC".into()))?
    } else {
        let peer = store
            .peer(&intro.device_id)?
            .filter(|p| !p.key.is_empty())
            .ok_or_else(|| Error::Protocol("unknown device; pair it first".into()))?;
        crypto::key_from_string(&peer.key)?
    };
    Ok((Channel { s, key }, intro.device_id))
}

pub fn run_session<S: Read + Write>(
    store: &mut Store,
    ch: &mut Channel<S>,
    initiator: bool,
    my_name: &str,
    outgoing: &[OutgoingFile],
    inbox: &Path,
) -> Result<SessionReport> {
    let team_id = store.meta("team_id")?.unwrap_or_default();
    let me = Hello {
        device_id: store.device_id().to_string(),
        name: my_name.to_string(),
        team_id: team_id.clone(),
        protocol: PROTOCOL,
    };

    // 1. hello
    let them = if initiator {
        ch.send(&Msg::Hello(me))?;
        expect_hello(ch)?
    } else {
        let h = expect_hello(ch)?;
        if h.team_id != team_id {
            let _ = ch.send(&Msg::Error { message: "this phone is synced to a different team".into() });
            return Err(Error::Protocol("team mismatch".into()));
        }
        ch.send(&Msg::Hello(me))?;
        h
    };
    if them.team_id != team_id {
        return Err(Error::Protocol("team mismatch".into()));
    }

    // Remember the peer and its key (first sync after pairing stores it).
    let mut peer = store.peer(&them.device_id)?.unwrap_or_default();
    peer.device_id = them.device_id.clone();
    peer.name = them.name.clone();
    peer.key = crypto::key_to_string(&ch.key);
    store.save_peer(&peer)?;

    let mut report = SessionReport {
        peer_device: them.device_id.clone(),
        peer_name: them.name.clone(),
        ..Default::default()
    };

    // 2 + 3. exchange changes
    let last_seq;
    if initiator {
        let (r, s) = pull(store, ch, &peer)?;
        report.pulled = r;
        last_seq = s;
        report.sent_changes = serve(store, ch)?;
    } else {
        report.sent_changes = serve(store, ch)?;
        let (r, s) = pull(store, ch, &peer)?;
        report.pulled = r;
        last_seq = s;
    }

    // 4. files
    std::fs::create_dir_all(inbox)?;
    if initiator {
        report.files_sent = send_files(ch, outgoing)?;
        report.files_received = recv_files(ch, inbox)?;
    } else {
        report.files_received = recv_files(ch, inbox)?;
        report.files_sent = send_files(ch, outgoing)?;
    }

    // 5. done
    store.finish_sync(&them.device_id, last_seq)?;
    if initiator {
        ch.send(&Msg::Bye)?;
        let _ = ch.recv();
    } else {
        let _ = ch.recv();
        ch.send(&Msg::Bye)?;
    }
    report.conflicts_total = store.conflicts()?.len();
    Ok(report)
}

fn expect_hello<S: Read + Write>(ch: &mut Channel<S>) -> Result<Hello> {
    match ch.recv()? {
        Msg::Hello(h) if h.protocol == PROTOCOL => Ok(h),
        Msg::Hello(h) => Err(Error::Protocol(format!(
            "app versions differ (protocol {} vs {PROTOCOL}); update both apps",
            h.protocol
        ))),
        other => Err(Error::Protocol(format!("expected hello, got {other:?}"))),
    }
}

fn pull<S: Read + Write>(store: &mut Store, ch: &mut Channel<S>, peer: &Peer) -> Result<(ApplyReport, i64)> {
    ch.send(&Msg::Pull { since: peer.last_seq_received })?;
    let mut total = ApplyReport::default();
    let mut last: i64;
    loop {
        match ch.recv()? {
            Msg::Changes { changes, last_seq, more } => {
                let r = store.apply_remote(&peer.device_id, &changes)?;
                total.received += r.received;
                total.applied += r.applied;
                total.conflicts += r.conflicts;
                last = last_seq;
                if !more {
                    break;
                }
            }
            other => return Err(Error::Protocol(format!("expected changes, got {other:?}"))),
        }
    }
    Ok((total, last))
}

fn serve<S: Read + Write>(store: &Store, ch: &mut Channel<S>) -> Result<usize> {
    let mut since = match ch.recv()? {
        Msg::Pull { since } => since,
        other => return Err(Error::Protocol(format!("expected pull, got {other:?}"))),
    };
    let mut sent = 0;
    loop {
        let (changes, last_seq) = store.changes_since(since, BATCH)?;
        let more = changes.len() == BATCH;
        sent += changes.len();
        ch.send(&Msg::Changes { changes, last_seq, more })?;
        since = last_seq;
        if !more {
            return Ok(sent);
        }
    }
}

fn send_files<S: Read + Write>(ch: &mut Channel<S>, files: &[OutgoingFile]) -> Result<usize> {
    let mut buf = vec![0u8; CHUNK];
    for f in files {
        let name = f
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".into());
        let size = std::fs::metadata(&f.path)?.len();
        ch.send(&Msg::File { name, size, kind: f.kind.clone(), meta: f.meta.clone() })?;
        let mut r = BufReader::new(File::open(&f.path)?);
        loop {
            let n = r.read(&mut buf)?;
            if n == 0 {
                break;
            }
            ch.send(&Msg::Chunk { data: B64.encode(&buf[..n]) })?;
        }
        ch.send(&Msg::FileEnd)?;
    }
    ch.send(&Msg::NoMoreFiles)?;
    Ok(files.len())
}

fn recv_files<S: Read + Write>(ch: &mut Channel<S>, inbox: &Path) -> Result<Vec<ReceivedFile>> {
    let mut out = Vec::new();
    loop {
        match ch.recv()? {
            Msg::NoMoreFiles => return Ok(out),
            Msg::File { name, kind, meta, .. } => {
                let path = unique_path(inbox, &safe_name(&name));
                let mut w = BufWriter::new(File::create(&path)?);
                loop {
                    match ch.recv()? {
                        Msg::Chunk { data } => {
                            let bytes = B64
                                .decode(data)
                                .map_err(|_| Error::Protocol("bad file chunk".into()))?;
                            w.write_all(&bytes)?;
                        }
                        Msg::FileEnd => break,
                        other => return Err(Error::Protocol(format!("expected chunk, got {other:?}"))),
                    }
                }
                w.flush()?;
                out.push(ReceivedFile { path, kind, meta });
            }
            other => return Err(Error::Protocol(format!("expected file, got {other:?}"))),
        }
    }
}

fn safe_name(name: &str) -> String {
    let n: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || "._- ".contains(c) { c } else { '_' })
        .collect();
    if n.trim_matches('.').is_empty() { "file".into() } else { n }
}

fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    if !p.exists() {
        return p;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (name.to_string(), String::new()),
    };
    (1..)
        .map(|i| dir.join(format!("{stem} ({i}){ext}")))
        .find(|p| !p.exists())
        .unwrap()
}

// ---------- .sync file fallback ----------

#[derive(Serialize, Deserialize)]
struct SyncFile {
    format: String,
    protocol: u32,
    device_id: String,
    team_id: String,
    changes: Vec<Change>,
}

/// Write every change this device knows to a `.sync` file (plain JSON).
pub fn export_sync_file(store: &Store, dest: &Path) -> Result<usize> {
    let (changes, _) = store.changes_since(0, usize::MAX >> 1)?;
    let n = changes.len();
    let f = SyncFile {
        format: "coachs-playbook-sync".into(),
        protocol: PROTOCOL,
        device_id: store.device_id().into(),
        team_id: store.meta("team_id")?.unwrap_or_default(),
        changes,
    };
    serde_json::to_writer(BufWriter::new(File::create(dest)?), &f)?;
    Ok(n)
}

pub fn import_sync_file(store: &mut Store, src: &Path) -> Result<ApplyReport> {
    let f: SyncFile = serde_json::from_reader(BufReader::new(File::open(src)?))?;
    if f.format != "coachs-playbook-sync" {
        return Err(Error::Other("not a .sync file".into()));
    }
    let team = store.meta("team_id")?.unwrap_or_default();
    if f.team_id != team {
        return Err(Error::Other("this .sync file belongs to a different team".into()));
    }
    if store.peer(&f.device_id)?.is_none() {
        store.save_peer(&Peer { device_id: f.device_id.clone(), ..Default::default() })?;
    }
    store.apply_remote(&f.device_id, &f.changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::net::{TcpListener, TcpStream};

    fn team_store(dev: &str) -> Store {
        let s = Store::open_in_memory(dev).unwrap();
        s.set_meta("team_id", "team-1").unwrap();
        s
    }

    #[test]
    fn lan_session_syncs_both_ways_with_file() {
        let dir = tempfile::tempdir().unwrap();
        let video = dir.path().join("clip.mp4");
        std::fs::write(&video, vec![7u8; 1_300_000]).unwrap();
        let inbox = dir.path().join("inbox");

        let key = crypto::generate_key();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let server = std::thread::spawn(move || {
            let mut pc = team_store("pc");
            pc.put("player", Some("p1"), json!({"name": "Ann"}).as_object().unwrap()).unwrap();
            let (sock, _) = listener.accept().unwrap();
            let (mut ch, _dev) = accept(&pc, sock, Some(key)).unwrap();
            let r = run_session(&mut pc, &mut ch, false, "PC", &[], &inbox).unwrap();
            (pc.list("attendance").unwrap().len(), r)
        });

        let mut phone = team_store("phone");
        phone
            .put("attendance", None, json!({"player_id": "p1", "present": true}).as_object().unwrap())
            .unwrap();
        let sock = TcpStream::connect(addr).unwrap();
        let mut ch = connect(sock, "phone", key, true).unwrap();
        let out = [OutgoingFile { path: video, kind: "video".into(), meta: json!({"match_id": "m1"}) }];
        let r = run_session(&mut phone, &mut ch, true, "Phone", &out, &dir.path().join("phone-inbox")).unwrap();

        let (pc_att, pc_report) = server.join().unwrap();
        assert_eq!(pc_att, 1);
        assert_eq!(phone.get("player", "p1").unwrap().unwrap()["name"], "Ann");
        assert_eq!(r.files_sent, 1);
        assert_eq!(pc_report.files_received.len(), 1);
        assert_eq!(std::fs::metadata(&pc_report.files_received[0].path).unwrap().len(), 1_300_000);
        // Peer key stored on both sides for next time.
        assert!(!phone.peer("pc").unwrap().unwrap().key.is_empty());
    }

    #[test]
    fn sync_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut a = team_store("a");
        let mut b = team_store("b");
        a.put("note", None, json!({"text": "hello"}).as_object().unwrap()).unwrap();
        let f = dir.path().join("x.sync");
        export_sync_file(&a, &f).unwrap();
        let r = import_sync_file(&mut b, &f).unwrap();
        assert_eq!(r.applied, 1);
        assert_eq!(b.list("note").unwrap().len(), 1);
    }
}
