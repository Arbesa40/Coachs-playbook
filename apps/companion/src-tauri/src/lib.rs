
use coach_core::pairing::PairingInfo;
use coach_core::sync::{self, OutgoingFile};
use coach_core::{crypto, Error, Result, Store};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Manager, State};

#[derive(Serialize, Deserialize, Clone)]
struct Device {
    device_id: String,
    device_name: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct Outbox {
    files: Vec<OutgoingFile>,
}

pub struct AppState {
    data_dir: PathBuf,
    device: Device,
    store: Mutex<Option<Store>>,
    pairing: Mutex<Option<PairingInfo>>,
    outbox: Mutex<Outbox>,
}

impl AppState {
    fn with<T>(&self, f: impl FnOnce(&mut Store) -> Result<T>) -> Result<T> {
        let mut g = self.store.lock().unwrap();
        let s = g.as_mut().ok_or_else(|| Error::Other("Pair with the PC first (Sync tab).".into()))?;
        f(s)
    }
    fn save_json<T: Serialize>(&self, name: &str, v: &T) -> Result<()> {
        std::fs::write(self.data_dir.join(name), serde_json::to_vec_pretty(v)?)?;
        Ok(())
    }
    fn media_dir(&self) -> PathBuf {
        self.data_dir.join("media")
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(p: PathBuf) -> Option<T> {
    std::fs::read(p).ok().and_then(|b| serde_json::from_slice(&b).ok())
}

// ---------------- records ----------------

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

#[tauri::command]
fn list_templates() -> Vec<Value> {
    coach_core::templates::builtin()
}

#[tauri::command]
fn app_info(state: State<'_, AppState>) -> Value {
    let p = state.pairing.lock().unwrap().clone();
    json!({
        "device_id": state.device.device_id,
        "device_name": state.device.device_name,
        "paired": p.is_some(),
        "pc_name": p.as_ref().map(|p| p.name.clone()),
        "team_id": p.as_ref().map(|p| p.team_id.clone()),
        "team_name": p.as_ref().map(|p| p.team_name.clone()),
        "outbox": state.outbox.lock().unwrap().files.len(),
    })
}

#[tauri::command]
fn set_device_name(state: State<'_, AppState>, name: String) -> Result<()> {
    let mut d = state.device.clone();
    d.device_name = name;
    state.save_json("device.json", &d)
}

// ---------------- pairing & sync ----------------

fn connect(p: &PairingInfo) -> Result<TcpStream> {
    let mut last = String::from("no address");
    for h in &p.hosts {
        let addr: SocketAddr = match format!("{h}:{}", p.port).parse() {
            Ok(a) => a,
            Err(_) => continue,
        };
        match TcpStream::connect_timeout(&addr, Duration::from_secs(4)) {
            Ok(s) => {
                s.set_read_timeout(Some(Duration::from_secs(60))).ok();
                s.set_nodelay(true).ok();
                return Ok(s);
            }
            Err(e) => last = e.to_string(),
        }
    }
    Err(Error::Other(format!(
        "Can't reach {}. Open Sync on the PC and check both are on the same Wi-Fi or hotspot ({last}).",
        p.name
    )))
}

fn do_sync(state: &AppState, pairing: bool) -> Result<Value> {
    let p = state
        .pairing
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| Error::Other("Not paired yet".into()))?;
    let key = crypto::key_from_string(&p.key)?;
    let sock = connect(&p)?;
    let outgoing = state.outbox.lock().unwrap().files.clone();
    let inbox = state.data_dir.join("inbox");
    let report = state.with(|s| {
        let mut ch = sync::connect(sock, s.device_id(), key, pairing)?;
        sync::run_session(s, &mut ch, true, &state.device.device_name, &outgoing, &inbox)
    })?;
    // Sent: clear them from the outbox.
    let mut ob = state.outbox.lock().unwrap();
    ob.files.retain(|f| !outgoing.iter().any(|o| o.path == f.path));
    state.save_json("outbox.json", &*ob)?;
    Ok(json!(report))
}

#[tauri::command]
async fn pair(state: State<'_, AppState>, code: String) -> Result<Value> {
    let info = PairingInfo::from_uri(&code)?;
    let path = state.data_dir.join("teams").join(format!("{}.sqlite", info.team_id));
    let store = Store::open(&path, &state.device.device_id)?;
    store.set_meta("team_id", &info.team_id)?;
    *state.store.lock().unwrap() = Some(store);
    *state.pairing.lock().unwrap() = Some(info.clone());
    state.save_json("pairing.json", &info)?;
    match do_sync(&state, true) {
        Ok(r) => Ok(r),
        Err(e) => {
            // Keep the pairing so the next "Sync" can retry, but report the problem.
            Err(e)
        }
    }
}

#[tauri::command]
async fn sync_now(state: State<'_, AppState>) -> Result<Value> {
    // A phone that has never completed a sync still needs the pairing key path.
    let first = state.with(|s| Ok(s.peers()?.is_empty()))?;
    do_sync(&state, first)
}

#[tauri::command]
async fn conflict_count(state: State<'_, AppState>) -> Result<usize> {
    state.with(|s| Ok(s.conflicts()?.len()))
}

// ---------------- media (recorded videos, voice memos) ----------------

/// Start receiving a file from the WebView; returns a file id.
#[tauri::command]
fn media_begin(state: State<'_, AppState>, name: String) -> Result<String> {
    std::fs::create_dir_all(state.media_dir())?;
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_string()).unwrap_or_else(|| "bin".into());
    let ext: String = ext.chars().filter(|c| c.is_ascii_alphanumeric()).take(5).collect();
    let id = format!("{}.{ext}", coach_core::new_id());
    std::fs::File::create(state.media_dir().join(&id))?;
    Ok(id)
}

/// Append a raw chunk (request body) to a file started with `media_begin`.
#[tauri::command]
fn media_chunk(state: State<'_, AppState>, request: tauri::ipc::Request<'_>) -> Result<()> {
    let id = request
        .headers()
        .get("x-file-id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Error::Other("missing file id".into()))?
        .to_string();
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(Error::Other("bad file id".into()));
    }
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(Error::Other("expected binary data".into()));
    };
    let mut f = std::fs::OpenOptions::new().append(true).open(state.media_dir().join(&id))?;
    f.write_all(bytes)?;
    Ok(())
}

/// Queue a finished file to go to the PC at the next sync.
#[tauri::command]
fn media_finish(state: State<'_, AppState>, id: String, kind: String, meta: Value) -> Result<String> {
    let path = state.media_dir().join(&id);
    if !path.exists() {
        return Err(Error::Other("file not found".into()));
    }
    let mut ob = state.outbox.lock().unwrap();
    ob.files.push(OutgoingFile { path: path.clone(), kind, meta });
    state.save_json("outbox.json", &*ob)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn outbox(state: State<'_, AppState>) -> Vec<Value> {
    state
        .outbox
        .lock()
        .unwrap()
        .files
        .iter()
        .map(|f| {
            let size = std::fs::metadata(&f.path).map(|m| m.len()).unwrap_or(0);
            json!({"path": f.path, "kind": f.kind, "meta": f.meta, "size": size})
        })
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_barcode_scanner::init());
    builder
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(data_dir.join("teams"))?;
            let device: Device = read_json(data_dir.join("device.json")).unwrap_or_else(|| {
                let d = Device { device_id: coach_core::new_id(), device_name: "Coach's phone".into() };
                let _ = std::fs::write(data_dir.join("device.json"), serde_json::to_vec_pretty(&d).unwrap());
                d
            });
            let pairing: Option<PairingInfo> = read_json(data_dir.join("pairing.json"));
            // Reopen the team database (crash recovery: everything is saved as it's entered).
            let store = match &pairing {
                Some(p) => Some(Store::open(data_dir.join("teams").join(format!("{}.sqlite", p.team_id)), &device.device_id)?),
                None => None,
            };
            let outbox: Outbox = read_json(data_dir.join("outbox.json")).unwrap_or_default();
            app.manage(AppState {
                data_dir,
                device,
                store: Mutex::new(store),
                pairing: Mutex::new(pairing),
                outbox: Mutex::new(outbox),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            rec_list,
            rec_list_by,
            rec_get,
            rec_put,
            rec_delete,
            list_templates,
            app_info,
            set_device_name,
            pair,
            sync_now,
            conflict_count,
            media_begin,
            media_chunk,
            media_finish,
            outbox,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the companion app");
}
