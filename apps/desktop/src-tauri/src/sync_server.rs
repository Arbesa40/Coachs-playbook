//! The desktop's LAN sync listener. It only runs while the Sync screen is open.

use crate::media::{Job, MediaQueue};
use coach_core::crypto::{self, SyncKey};
use coach_core::pairing::{PairingInfo, DEFAULT_PORT};
use coach_core::sync;
use coach_core::Store;
use serde_json::{json, Map, Value};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub struct SyncCtx {
    pub app: AppHandle,
    pub media: MediaQueue,
    pub db_path: PathBuf,
    pub inbox: PathBuf,
    pub device_id: String,
    pub device_name: String,
    pub team_id: String,
    pub team_name: String,
}

pub struct SyncServer {
    stop: Arc<AtomicBool>,
    pub info: PairingInfo,
    pub uri: String,
    pub qr_svg: String,
}

impl Drop for SyncServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Private LAN IPv4 addresses (Wi-Fi, Ethernet, phone hotspot), best first.
pub fn lan_addresses() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Ok(list) = local_ip_address::list_afinet_netifas() {
        for (_name, ip) in list {
            if let IpAddr::V4(v4) = ip {
                if v4.is_private() && !v4.is_loopback() {
                    out.push(v4.to_string());
                }
            }
        }
    }
    if let Ok(IpAddr::V4(v4)) = local_ip_address::local_ip() {
        let s = v4.to_string();
        out.retain(|x| x != &s);
        out.insert(0, s);
    }
    out
}

impl SyncServer {
    pub fn start(ctx: SyncCtx) -> Result<SyncServer, String> {
        let listener = TcpListener::bind(("0.0.0.0", DEFAULT_PORT))
            .or_else(|_| TcpListener::bind(("0.0.0.0", 0)))
            .map_err(|e| format!("could not open sync port: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;

        let hosts = lan_addresses();
        if hosts.is_empty() {
            return Err("This PC is not on a network. Join the same Wi-Fi as the phone, or the phone's hotspot.".into());
        }
        let key: SyncKey = crypto::generate_key();
        let info = PairingInfo {
            v: 1,
            hosts,
            port,
            key: crypto::key_to_string(&key),
            device_id: ctx.device_id.clone(),
            name: ctx.device_name.clone(),
            team_id: ctx.team_id.clone(),
            team_name: ctx.team_name.clone(),
        };
        let uri = info.to_uri();
        let qr_svg = qrcode::QrCode::new(uri.as_bytes())
            .map_err(|e| e.to_string())?
            .render::<qrcode::render::svg::Color>()
            .min_dimensions(260, 260)
            .build();

        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        std::thread::Builder::new()
            .name("sync-server".into())
            .spawn(move || {
                while !stop2.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((sock, addr)) => {
                            let _ = ctx.app.emit("sync-event", json!({"state": "connected", "from": addr.to_string()}));
                            match handle(&ctx, sock, key) {
                                Ok(report) => {
                                    let _ = ctx.app.emit("sync-event", json!({"state": "done", "report": report}));
                                }
                                Err(e) => {
                                    let _ = ctx.app.emit("sync-event", json!({"state": "error", "message": e}));
                                }
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(200));
                        }
                        Err(_) => std::thread::sleep(Duration::from_millis(500)),
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(SyncServer { stop, info, uri, qr_svg })
    }
}

fn handle(ctx: &SyncCtx, sock: TcpStream, pairing_key: SyncKey) -> Result<sync::SessionReport, String> {
    sock.set_nonblocking(false).map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(Duration::from_secs(60))).ok();
    sock.set_nodelay(true).ok();
    // Own connection to the team database so the UI never waits on a transfer.
    let mut store = Store::open(&ctx.db_path, &ctx.device_id).map_err(|e| e.to_string())?;
    let (mut ch, _dev) = sync::accept(&store, sock, Some(pairing_key)).map_err(|e| e.to_string())?;
    let report = sync::run_session(&mut store, &mut ch, false, &ctx.device_name, &[], &ctx.inbox)
        .map_err(|e| e.to_string())?;

    // Videos recorded on the phone become library videos and get proxies.
    for f in &report.files_received {
        if f.kind == "memo" {
            // Voice memo: attach the audio file to its note.
            if let Some(note_id) = f.meta["note_id"].as_str() {
                let mut fields = Map::new();
                fields.insert("audio_path".into(), json!(f.path.to_string_lossy()));
                let _ = store.put("note", Some(note_id), &fields);
            }
            continue;
        }
        if f.kind != "video" {
            continue;
        }
        let mut fields: Map<String, Value> = f.meta.as_object().cloned().unwrap_or_default();
        fields.insert("source".into(), json!("local"));
        fields.insert("path".into(), json!(f.path.to_string_lossy()));
        fields.entry("name").or_insert_with(|| {
            json!(f.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
        });
        fields.insert("proxy_status".into(), json!("queued"));
        fields.insert("from_phone".into(), json!(true));
        if let Ok(v) = store.put("video", None, &fields) {
            if let Some(id) = v["id"].as_str() {
                ctx.media.push(Job { video_id: id.to_string(), src: f.path.clone(), db_path: ctx.db_path.clone() });
            }
        }
    }
    Ok(report)
}
