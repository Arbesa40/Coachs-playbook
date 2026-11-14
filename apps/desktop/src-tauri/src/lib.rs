
mod media;
mod share;
mod sync_server;

use coach_core::{export, stats, sync, templates, Error, Result, Store};
use media::{Job, MediaQueue, SharedStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use sync_server::{SyncCtx, SyncServer};
use tauri::{Manager, State};

pub struct AppState {
    data_dir: PathBuf,
    device_id: String,
    device_name: String,
    store: SharedStore,
    media: MediaQueue,
    ffmpeg: Option<PathBuf>,
    sync: Mutex<Option<SyncServer>>,
}

#[derive(Serialize, Deserialize)]
struct DeviceFile {
    device_id: String,
    device_name: String,
}

impl AppState {
    fn teams_dir(&self) -> PathBuf {
        self.data_dir.join("teams")
    }
    fn team_path(&self, team_id: &str) -> PathBuf {
        self.teams_dir().join(format!("{team_id}.sqlite"))
    }
    fn with<T>(&self, f: impl FnOnce(&mut Store) -> Result<T>) -> Result<T> {
        let mut g = self.store.lock().unwrap();
        let s = g.as_mut().ok_or_else(|| Error::Other("No team is open".into()))?;
        f(s)
    }
    fn db_path(&self) -> Result<PathBuf> {
        self.with(|s| Ok(s.path().to_path_buf()))
    }
    fn ffmpeg(&self) -> Result<&Path> {
        self.ffmpeg
            .as_deref()
            .ok_or_else(|| Error::Other("FFmpeg was not found. Run setup.ps1 or reinstall the app.".into()))
    }
}

fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

fn load_device(data_dir: &Path) -> DeviceFile {
    let f = data_dir.join("device.json");
    if let Some(d) = std::fs::read(&f).ok().and_then(|b| serde_json::from_slice::<DeviceFile>(&b).ok()) {
        return d;
    }
    let d = DeviceFile {
        device_id: coach_core::new_id(),
        device_name: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Coach PC".into()),
    };
    let _ = std::fs::create_dir_all(data_dir);
    let _ = std::fs::write(&f, serde_json::to_vec_pretty(&d).unwrap());
    d
}

// ---------------- app & teams ----------------

#[tauri::command]
fn app_info(state: State<'_, AppState>) -> Value {
    json!({
        "device_id": state.device_id,
        "device_name": state.device_name,
        "data_dir": state.data_dir,
        "ffmpeg": state.ffmpeg,
        "version": env!("CARGO_PKG_VERSION"),
    })
}

#[tauri::command]
fn list_templates() -> Vec<Value> {
    templates::builtin()
}

#[tauri::command]
async fn list_teams(state: State<'_, AppState>) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    let dir = state.teams_dir();
    std::fs::create_dir_all(&dir)?;
    for entry in std::fs::read_dir(&dir)? {
        let p = entry?.path();
        if p.extension().and_then(|e| e.to_str()) != Some("sqlite") {
            continue;
        }
        let Some(team_id) = p.file_stem().map(|s| s.to_string_lossy().to_string()) else { continue };
        let s = Store::open(&p, &state.device_id)?;
        if let Some(mut t) = s.get("team", &team_id)? {
            t["players"] = json!(s.list("player")?.len());
            out.push(t);
        }
    }
    Ok(out)
}

#[tauri::command]
async fn create_team(state: State<'_, AppState>, name: String, sport: String) -> Result<Value> {
    let team_id = coach_core::new_id();
    let mut s = Store::open(state.team_path(&team_id), &state.device_id)?;
    s.set_meta("team_id", &team_id)?;
    let team = s.put("team", Some(&team_id), &obj(json!({"name": name, "sport": sport})))?;
    if let Some(t) = templates::get(&sport) {
        let mut panel = obj(t["panel"].clone());
        panel.insert("default".into(), json!(true));
        s.put("panel", None, &panel)?;
    }
    *state.store.lock().unwrap() = Some(s);
    Ok(team)
}

#[tauri::command]
async fn open_team(state: State<'_, AppState>, team_id: String) -> Result<Value> {
    *state.sync.lock().unwrap() = None;
    let s = Store::open(state.team_path(&team_id), &state.device_id)?;
    let team = s.get("team", &team_id)?.ok_or_else(|| Error::Other("Team not found".into()))?;
    *state.store.lock().unwrap() = Some(s);
    Ok(team)
}

/// The database is moved to `deleted-teams\`, not destroyed, so a mistake can be undone.
#[tauri::command]
async fn delete_team(state: State<'_, AppState>, team_id: String) -> Result<()> {
    if team_id.is_empty() || team_id.contains(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')) {
        return Err(Error::Other("Invalid team id".into()));
    }
    let src = state.team_path(&team_id);
    if !src.exists() {
        return Err(Error::Other("Team not found".into()));
    }
    // Close it first if it is the open team (Windows can't move an open file).
    *state.sync.lock().unwrap() = None;
    {
        let mut g = state.store.lock().unwrap();
        if g.as_ref().map(|s| s.path() == src.as_path()).unwrap_or(false) {
            *g = None;
        }
    }
    let bin = state.data_dir.join("deleted-teams");
    std::fs::create_dir_all(&bin)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    for suffix in ["", "-wal", "-shm"] {
        let from = PathBuf::from(format!("{}{suffix}", src.display()));
        if from.exists() {
            let to = bin.join(format!("{team_id}-{stamp}.sqlite{suffix}"));
            std::fs::rename(&from, &to)?;
        }
    }
    Ok(())
}

// ---------------- generic records ----------------

#[tauri::command]
async fn rec_list(state: State<'_, AppState>, kind: String) -> Result<Vec<Value>> {
    state.with(|s| s.list(&kind))
}

#[tauri::command]
async fn rec_list_by(state: State<'_, AppState>, kind: String, field: String, value: Value) -> Result<Vec<Value>> {
    state.with(|s| s.list_by(&kind, &field, &value))
}

#[tauri::command]
async fn rec_get(state: State<'_, AppState>, kind: String, id: String) -> Result<Option<Value>> {
    state.with(|s| s.get(&kind, &id))
}

#[tauri::command]
async fn rec_put(state: State<'_, AppState>, kind: String, id: Option<String>, fields: Map<String, Value>) -> Result<Value> {
    state.with(|s| s.put(&kind, id.as_deref(), &fields))
}

#[tauri::command]
async fn rec_delete(state: State<'_, AppState>, kind: String, id: String) -> Result<()> {
    state.with(|s| s.delete(&kind, &id))
}

// ---------------- video ----------------

#[tauri::command]
async fn import_videos(state: State<'_, AppState>, paths: Vec<String>, match_id: Option<String>) -> Result<Vec<Value>> {
    let db = state.db_path()?;
    let mut out = Vec::new();
    for p in paths {
        let path = PathBuf::from(&p);
        let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let v = state.with(|s| {
            s.put(
                "video",
                None,
                &obj(json!({"name": name, "source": "local", "path": p, "match_id": match_id, "proxy_status": "queued"})),
            )
        })?;
        state.media.push(Job { video_id: v["id"].as_str().unwrap_or_default().into(), src: path, db_path: db.clone() });
        out.push(v);
    }
    Ok(out)
}

/// Accepts watch, short, embed and youtu.be links, or a bare 11-character id.
fn youtube_id(url: &str) -> Option<String> {
    let u = url.trim();
    let valid = |s: &str| s.len() == 11 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valid(u) {
        return Some(u.into());
    }
    for marker in ["v=", "youtu.be/", "/embed/", "/shorts/", "/live/"] {
        if let Some(i) = u.find(marker) {
            let id: String = u[i + marker.len()..].chars().take(11).collect();
            if valid(&id) {
                return Some(id);
            }
        }
    }
    None
}

#[tauri::command]
async fn add_youtube(state: State<'_, AppState>, url: String, name: Option<String>, match_id: Option<String>) -> Result<Value> {
    let id = youtube_id(&url).ok_or_else(|| Error::Other("That doesn't look like a YouTube link".into()))?;
    state.with(|s| {
        s.put(
            "video",
            None,
            &obj(json!({
                "name": name.unwrap_or_else(|| format!("YouTube {id}")),
                "source": "youtube",
                "youtube_id": id,
                "match_id": match_id,
                "proxy_status": "stream",
            })),
        )
    })
}

#[tauri::command]
fn media_status(state: State<'_, AppState>) -> media::QueueStatus {
    state.media.status()
}

#[tauri::command]
fn media_pause(state: State<'_, AppState>, paused: bool) {
    state.media.set_paused(paused);
}

#[tauri::command]
async fn media_retry(state: State<'_, AppState>, video_id: String) -> Result<()> {
    let db = state.db_path()?;
    let v = state
        .with(|s| s.get("video", &video_id))?
        .ok_or_else(|| Error::Other("video not found".into()))?;
    let src = PathBuf::from(v["path"].as_str().unwrap_or_default());
    state.with(|s| s.put("video", Some(&video_id), &obj(json!({"proxy_status": "queued"}))))?;
    state.media.push(Job { video_id, src, db_path: db });
    Ok(())
}

fn source_file(v: &Value) -> Result<PathBuf> {
    let orig = PathBuf::from(v["path"].as_str().unwrap_or_default());
    if orig.exists() {
        return Ok(orig);
    }
    let proxy = PathBuf::from(v["proxy_path"].as_str().unwrap_or_default());
    if proxy.exists() {
        return Ok(proxy);
    }
    Err(Error::Other(format!("The video file was moved or deleted: {}", orig.display())))
}

/// Export a local clip to MP4. `overlay_png` is a base64 PNG (data URL ok) of the drawings, rendered at the video's size.
#[tauri::command]
async fn export_clip(
    state: State<'_, AppState>,
    video_id: String,
    start: f64,
    end: f64,
    out: String,
    overlay_png: Option<String>,
) -> Result<String> {
    let ffmpeg = state.ffmpeg()?.to_path_buf();
    let v = state
        .with(|s| s.get("video", &video_id))?
        .ok_or_else(|| Error::Other("video not found".into()))?;
    if v["source"] == "youtube" {
        return Err(Error::Other("YouTube clips can't be downloaded; share them as links instead.".into()));
    }
    let src = source_file(&v)?;
    let tmp_png = match overlay_png {
        Some(b64) => {
            let p = std::env::temp_dir().join(format!("cpb-overlay-{}.png", coach_core::new_id()));
            std::fs::write(&p, share::decode_data_url(&b64)?)?;
            Some(p)
        }
        None => None,
    };
    let r = media::export_clip(&ffmpeg, &src, start, end, Path::new(&out), tmp_png.as_deref());
    if let Some(p) = tmp_png {
        let _ = std::fs::remove_file(p);
    }
    r.map_err(Error::Other)?;
    Ok(out)
}

#[tauri::command]
async fn share_package(state: State<'_, AppState>, playlist_id: String, out: String) -> Result<String> {
    let (team, playlist, clips, videos) = state.with(|s| {
        let team_id = s.meta("team_id")?.unwrap_or_default();
        let team = s.get("team", &team_id)?.unwrap_or(json!({}));
        let playlist = s
            .get("playlist", &playlist_id)?
            .ok_or_else(|| Error::Other("playlist not found".into()))?;
        let clips = s.list("clip")?;
        let videos = s.list("video")?;
        Ok((team, playlist, clips, videos))
    })?;
    share::build_package(state.ffmpeg.as_deref(), &team, &playlist, &clips, &videos, Path::new(&out))?;
    Ok(out)
}

// ---------------- stats ----------------

#[tauri::command]
async fn tag_stats(state: State<'_, AppState>, video_id: Option<String>, match_id: Option<String>) -> Result<Value> {
    state.with(|s| {
        let videos: Vec<String> = match (&video_id, &match_id) {
            (Some(v), _) => vec![v.clone()],
            (None, Some(m)) => s
                .list_by("video", "match_id", &json!(m))?
                .iter()
                .filter_map(|v| v["id"].as_str().map(String::from))
                .collect(),
            _ => s.list("video")?.iter().filter_map(|v| v["id"].as_str().map(String::from)).collect(),
        };
        let tags: Vec<Value> = s
            .list("tag")?
            .into_iter()
            .filter(|t| t["video_id"].as_str().map_or(false, |v| videos.iter().any(|x| x == v)))
            .collect();
        Ok(stats::from_tags(&tags, &s.list("player")?))
    })
}

#[tauri::command]
async fn minutes_played(state: State<'_, AppState>, match_id: Option<String>) -> Result<Value> {
    state.with(|s| {
        let events = match &match_id {
            Some(m) => s.list_by("match_event", "match_id", &json!(m))?,
            None => s.list("match_event")?,
        };
        if match_id.is_some() {
            return Ok(json!(stats::minutes_played(&events)));
        }
        // Season totals: sum per match.
        let mut total: std::collections::BTreeMap<String, f64> = Default::default();
        let mut by_match: std::collections::BTreeMap<String, Vec<Value>> = Default::default();
        for e in events {
            by_match.entry(e["match_id"].as_str().unwrap_or("").to_string()).or_default().push(e);
        }
        for evs in by_match.values() {
            for (p, m) in stats::minutes_played(evs) {
                *total.entry(p).or_default() += m;
            }
        }
        Ok(json!(total))
    })
}

// ---------------- import / export / backup ----------------

#[tauri::command]
async fn export_csv(state: State<'_, AppState>, kind: String, path: String) -> Result<usize> {
    let recs = state.with(|s| s.list(&kind))?;
    std::fs::write(&path, export::to_csv(&recs)?)?;
    Ok(recs.len())
}

#[tauri::command]
async fn import_csv(state: State<'_, AppState>, kind: String, path: String) -> Result<usize> {
    let text = std::fs::read_to_string(&path)?;
    let rows = export::from_csv(text.trim_start_matches('\u{feff}'))?;
    state.with(|s| {
        for mut r in rows.iter().cloned() {
            let id = r.remove("id").and_then(|v| v.as_str().map(String::from));
            s.put(&kind, id.as_deref(), &r)?;
        }
        Ok(rows.len())
    })
}

#[tauri::command]
async fn export_xml(state: State<'_, AppState>, video_id: String, path: String) -> Result<usize> {
    let (tags, players, panels) = state.with(|s| {
        Ok((s.list_by("tag", "video_id", &json!(video_id))?, s.list("player")?, s.list("panel")?))
    })?;
    let mut colors: Vec<(String, String)> = Vec::new();
    for p in &panels {
        for b in p["buttons"].as_array().into_iter().flatten() {
            if let (Some(c), Some(col)) = (b["code"].as_str(), b["color"].as_str()) {
                if !colors.iter().any(|(x, _)| x == c) {
                    colors.push((c.into(), col.into()));
                }
            }
        }
    }
    let name = |id: &str| {
        players
            .iter()
            .find(|p| p["id"] == id)
            .map(|p| format!("#{} {}", p["number"].as_i64().map(|n| n.to_string()).unwrap_or_default(), p["name"].as_str().unwrap_or("")))
            .unwrap_or_else(|| id.to_string())
    };
    std::fs::write(&path, export::tags_to_xml(&tags, &colors, name))?;
    Ok(tags.len())
}

#[tauri::command]
async fn backup(state: State<'_, AppState>, path: String) -> Result<String> {
    state.with(|s| s.backup_to(Path::new(&path)))?;
    Ok(path)
}

#[tauri::command]
async fn restore(state: State<'_, AppState>, path: String) -> Result<Value> {
    let team_id = {
        let s = Store::open(&path, &state.device_id)?;
        s.meta("team_id")?.ok_or_else(|| Error::Other("Not a Coach's Playbook backup".into()))?
    };
    *state.sync.lock().unwrap() = None;
    *state.store.lock().unwrap() = None;
    let dest = state.team_path(&team_id);
    for ext in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{ext}", dest.display()));
    }
    std::fs::create_dir_all(state.teams_dir())?;
    std::fs::copy(&path, &dest)?;
    open_team(state, team_id).await
}

#[tauri::command]
async fn export_sync_file(state: State<'_, AppState>, path: String) -> Result<usize> {
    state.with(|s| sync::export_sync_file(s, Path::new(&path)))
}

#[tauri::command]
async fn import_sync_file(state: State<'_, AppState>, path: String) -> Result<Value> {
    let r = state.with(|s| sync::import_sync_file(s, Path::new(&path)))?;
    Ok(json!(r))
}

// ---------------- LAN sync ----------------

#[tauri::command]
async fn sync_open(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<Value> {
    let (team_id, team_name, db_path) = state.with(|s| {
        let id = s.meta("team_id")?.unwrap_or_default();
        let name = s.get("team", &id)?.and_then(|t| t["name"].as_str().map(String::from)).unwrap_or_default();
        Ok((id, name, s.path().to_path_buf()))
    })?;
    let server = SyncServer::start(SyncCtx {
        app,
        media: state.media.clone(),
        db_path,
        inbox: state.data_dir.join("inbox"),
        device_id: state.device_id.clone(),
        device_name: state.device_name.clone(),
        team_id,
        team_name,
    })
    .map_err(Error::Other)?;
    let v = json!({"qr_svg": server.qr_svg, "uri": server.uri, "hosts": server.info.hosts, "port": server.info.port});
    *state.sync.lock().unwrap() = Some(server);
    Ok(v)
}

#[tauri::command]
fn sync_close(state: State<'_, AppState>) {
    *state.sync.lock().unwrap() = None;
}

#[tauri::command]
async fn sync_peers(state: State<'_, AppState>) -> Result<Vec<Value>> {
    state.with(|s| {
        Ok(s.peers()?
            .into_iter()
            .map(|p| json!({"device_id": p.device_id, "name": p.name, "last_sync_hlc": p.last_sync_hlc}))
            .collect())
    })
}

#[tauri::command]
async fn conflicts(state: State<'_, AppState>) -> Result<Value> {
    state.with(|s| Ok(json!(s.conflicts()?)))
}

#[tauri::command]
async fn resolve_conflict(state: State<'_, AppState>, id: i64, keep_local: bool) -> Result<()> {
    state.with(|s| s.resolve_conflict(id, keep_local))
}

// ---------------- misc ----------------

#[tauri::command]
fn reveal(path: String) -> Result<()> {
    tauri_plugin_opener::reveal_item_in_dir(&path).map_err(|e| Error::Other(e.to_string()))
}

#[tauri::command]
fn open_external(url: String) -> Result<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(Error::Other("only web links can be opened".into()));
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| Error::Other(e.to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(data_dir.join("teams"))?;
            let dev = load_device(&data_dir);
            let ffmpeg = media::find_ffmpeg(app.path().resource_dir().ok());
            let store: SharedStore = Arc::new(Mutex::new(None));
            let mediaq = MediaQueue::new();
            mediaq.start(app.handle().clone(), store.clone(), dev.device_id.clone(), ffmpeg.clone(), data_dir.join("proxies"));
            app.manage(AppState {
                data_dir,
                device_id: dev.device_id,
                device_name: dev.device_name,
                store,
                media: mediaq,
                ffmpeg,
                sync: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_templates,
            list_teams,
            create_team,
            open_team,
            delete_team,
            rec_list,
            rec_list_by,
            rec_get,
            rec_put,
            rec_delete,
            import_videos,
            add_youtube,
            media_status,
            media_pause,
            media_retry,
            export_clip,
            share_package,
            tag_stats,
            minutes_played,
            export_csv,
            import_csv,
            export_xml,
            backup,
            restore,
            export_sync_file,
            import_sync_file,
            sync_open,
            sync_close,
            sync_peers,
            conflicts,
            resolve_conflict,
            reveal,
            open_external,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Coach's Playbook");
}

#[cfg(test)]
mod tests {
    #[test]
    fn youtube_ids() {
        use super::youtube_id;
        assert_eq!(youtube_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=10").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(youtube_id("https://youtu.be/dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(youtube_id("dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
        assert!(youtube_id("https://example.com").is_none());
    }
}
