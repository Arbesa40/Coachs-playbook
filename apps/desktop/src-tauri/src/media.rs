//! FFmpeg runs as a separate program with fewer threads so tagging stays responsive.

use coach_core::Store;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use tauri::{AppHandle, Emitter};

pub type SharedStore = Arc<Mutex<Option<Store>>>;

/// A child process with no console window on Windows.
pub fn command(program: &Path) -> Command {
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

pub fn find_ffmpeg(resource_dir: Option<PathBuf>) -> Option<PathBuf> {
    let exe = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    if let Some(r) = resource_dir {
        for p in [r.join("resources").join("ffmpeg").join(exe), r.join("ffmpeg").join(exe)] {
            if p.exists() {
                return Some(p);
            }
        }
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join(exe)).find(|p| p.exists())
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Probe {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub vcodec: String,
}

pub fn probe(ffmpeg: &Path, file: &Path) -> Result<Probe, String> {
    let out = command(ffmpeg)
        .args(["-hide_banner", "-i"])
        .arg(file)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run ffmpeg: {e}"))?;
    let p = parse_probe(&String::from_utf8_lossy(&out.stderr));
    if p.duration <= 0.0 && p.width == 0 {
        return Err("not a readable video file".into());
    }
    Ok(p)
}

fn parse_probe(s: &str) -> Probe {
    let mut p = Probe::default();
    if let Some(i) = s.find("Duration: ") {
        let t = &s[i + 10..];
        let t = t.split(',').next().unwrap_or("");
        let parts: Vec<f64> = t.trim().split(':').filter_map(|x| x.parse().ok()).collect();
        if parts.len() == 3 {
            p.duration = parts[0] * 3600.0 + parts[1] * 60.0 + parts[2];
        }
    }
    if let Some(line) = s.lines().find(|l| l.contains("Video: ")) {
        let after = &line[line.find("Video: ").unwrap() + 7..];
        p.vcodec = after
            .split(|c: char| c == ' ' || c == ',')
            .next()
            .unwrap_or("")
            .to_string();
        let tokens: Vec<&str> = after.split(|c: char| c == ',' || c == ' ').filter(|t| !t.is_empty()).collect();
        for (i, t) in tokens.iter().enumerate() {
            if let Some((w, h)) = t.split_once('x') {
                if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                    if w >= 16 && h >= 16 && p.width == 0 {
                        p.width = w;
                        p.height = h;
                    }
                }
            }
            if *t == "fps" && i > 0 {
                p.fps = tokens[i - 1].parse().unwrap_or(0.0);
            }
        }
    }
    p
}

#[derive(Debug, Clone, Serialize)]
pub struct Job {
    pub video_id: String,
    pub src: PathBuf,
    /// The team database the video belongs to.
    pub db_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct QueueStatus {
    pub paused: bool,
    pub pending: usize,
    pub current: Option<String>,
    pub progress: f64,
}

struct Inner {
    jobs: Mutex<VecDeque<Job>>,
    cv: Condvar,
    paused: AtomicBool,
    status: Mutex<QueueStatus>,
}

#[derive(Clone)]
pub struct MediaQueue {
    inner: Arc<Inner>,
}

impl MediaQueue {
    pub fn new() -> Self {
        MediaQueue {
            inner: Arc::new(Inner {
                jobs: Mutex::new(VecDeque::new()),
                cv: Condvar::new(),
                paused: AtomicBool::new(false),
                status: Mutex::new(QueueStatus::default()),
            }),
        }
    }

    pub fn push(&self, job: Job) {
        self.inner.jobs.lock().unwrap().push_back(job);
        self.inner.cv.notify_all();
    }

    pub fn set_paused(&self, paused: bool) {
        self.inner.paused.store(paused, Ordering::SeqCst);
        self.inner.cv.notify_all();
    }

    pub fn status(&self) -> QueueStatus {
        let mut s = self.inner.status.lock().unwrap().clone();
        s.paused = self.inner.paused.load(Ordering::SeqCst);
        s.pending = self.inner.jobs.lock().unwrap().len();
        s
    }

    pub fn start(&self, app: AppHandle, store: SharedStore, device_id: String, ffmpeg: Option<PathBuf>, proxy_dir: PathBuf) {
        let inner = self.inner.clone();
        std::thread::Builder::new()
            .name("media-queue".into())
            .spawn(move || loop {
                let job = {
                    let mut q = inner.jobs.lock().unwrap();
                    loop {
                        if !inner.paused.load(Ordering::SeqCst) {
                            if let Some(j) = q.pop_front() {
                                break j;
                            }
                        }
                        q = inner.cv.wait(q).unwrap();
                    }
                };
                {
                    let mut st = inner.status.lock().unwrap();
                    st.current = Some(job.video_id.clone());
                    st.progress = 0.0;
                }
                let requeue = process(&app, &inner, &store, &device_id, ffmpeg.as_deref(), &proxy_dir, &job);
                if requeue {
                    inner.jobs.lock().unwrap().push_front(job);
                }
                {
                    let mut st = inner.status.lock().unwrap();
                    st.current = None;
                    st.progress = 0.0;
                }
            })
            .expect("spawn media queue");
    }
}

/// Run `f` on the team database at `db_path`, reusing the open store when it is that team.
pub fn with_team_store<T>(
    shared: &SharedStore,
    db_path: &Path,
    device_id: &str,
    f: impl FnOnce(&mut Store) -> coach_core::Result<T>,
) -> coach_core::Result<T> {
    {
        let mut guard = shared.lock().unwrap();
        if let Some(s) = guard.as_mut() {
            if s.path() == db_path {
                return f(s);
            }
        }
    }
    let mut s = Store::open(db_path, device_id)?;
    f(&mut s)
}

fn update(store: &SharedStore, job: &Job, device_id: &str, fields: Value) {
    let _ = with_team_store(store, &job.db_path, device_id, |s| {
        s.put("video", Some(&job.video_id), fields.as_object().unwrap())
    });
}

/// Returns true if the job should go back on the queue (paused mid-encode).
fn process(
    app: &AppHandle,
    inner: &Inner,
    store: &SharedStore,
    device_id: &str,
    ffmpeg: Option<&Path>,
    proxy_dir: &Path,
    job: &Job,
) -> bool {
    let notify = |extra: Value| {
        let _ = app.emit("media-changed", json!({ "video_id": job.video_id, "extra": extra }));
    };
    let Some(ffmpeg) = ffmpeg else {
        update(store, job, device_id, json!({"proxy_status": "no-ffmpeg"}));
        notify(Value::Null);
        return false;
    };
    let _ = std::fs::create_dir_all(proxy_dir);

    let info = match probe(ffmpeg, &job.src) {
        Ok(p) => p,
        Err(e) => {
            update(store, job, device_id, json!({"proxy_status": "failed", "proxy_error": e}));
            notify(Value::Null);
            return false;
        }
    };
    update(
        store,
        job,
        device_id,
        json!({"duration": info.duration, "width": info.width, "height": info.height, "fps": info.fps, "vcodec": info.vcodec, "proxy_status": "working"}),
    );

    // Thumbnail (cheap, one frame).
    let thumb = proxy_dir.join(format!("{}.jpg", job.video_id));
    let seek = (info.duration / 2.0).min(5.0).max(0.0);
    let ok = command(ffmpeg)
        .args(["-hide_banner", "-y", "-ss", &format!("{seek:.2}"), "-i"])
        .arg(&job.src)
        .args(["-frames:v", "1", "-vf", "scale=320:-2", "-q:v", "5"])
        .arg(&thumb)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        update(store, job, device_id, json!({"thumb": thumb.to_string_lossy()}));
    }
    notify(Value::Null);

    // Already light enough to play smoothly: use the original as the proxy.
    let ext = job
        .src
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if info.vcodec == "h264" && info.height > 0 && info.height <= 720 && ["mp4", "m4v", "mov"].contains(&ext.as_str()) {
        update(store, job, device_id, json!({"proxy_status": "original", "proxy_path": job.src.to_string_lossy()}));
        notify(Value::Null);
        return false;
    }

    let out = proxy_dir.join(format!("{}.mp4", job.video_id));
    let tmp = proxy_dir.join(format!("{}.part.mp4", job.video_id));
    let threads = std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(1))
        .unwrap_or(2)
        .to_string();
    let child = command(ffmpeg)
        .args(["-hide_banner", "-y", "-i"])
        .arg(&job.src)
        .args([
            "-map", "0:v:0", "-map", "0:a:0?",
            "-vf", "scale=-2:'min(720,ih)'",
            "-c:v", "libx264", "-preset", "veryfast", "-crf", "26",
            "-g", "25", "-keyint_min", "25", "-pix_fmt", "yuv420p",
            "-c:a", "aac", "-b:a", "96k", "-ac", "2",
            "-movflags", "+faststart",
            "-threads", &threads,
            "-progress", "pipe:1", "-nostats",
        ])
        .arg(&tmp)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            update(store, job, device_id, json!({"proxy_status": "failed", "proxy_error": e.to_string()}));
            notify(Value::Null);
            return false;
        }
    };

    let mut last_emit = 0.0;
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if inner.paused.load(Ordering::SeqCst) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&tmp);
                update(store, job, device_id, json!({"proxy_status": "queued"}));
                notify(Value::Null);
                return true;
            }
            if let Some(v) = line.strip_prefix("out_time_us=").or_else(|| line.strip_prefix("out_time_ms=")) {
                if let Ok(us) = v.trim().parse::<f64>() {
                    if info.duration > 0.0 {
                        let p = (us / 1_000_000.0 / info.duration).clamp(0.0, 1.0);
                        inner.status.lock().unwrap().progress = p;
                        if p - last_emit > 0.01 {
                            last_emit = p;
                            let _ = app.emit("media-progress", json!({"video_id": job.video_id, "progress": p}));
                        }
                    }
                }
            }
        }
    }
    let ok = child.wait().map(|s| s.success()).unwrap_or(false);
    if ok && std::fs::rename(&tmp, &out).is_ok() {
        update(store, job, device_id, json!({"proxy_status": "ready", "proxy_path": out.to_string_lossy()}));
    } else {
        let _ = std::fs::remove_file(&tmp);
        update(store, job, device_id, json!({"proxy_status": "failed", "proxy_error": "ffmpeg could not convert this file"}));
    }
    notify(Value::Null);
    false
}

/// Without an overlay this is a fast stream copy (cuts snap to keyframes); with one it re-encodes.
pub fn export_clip(
    ffmpeg: &Path,
    src: &Path,
    start: f64,
    end: f64,
    out: &Path,
    overlay_png: Option<&Path>,
) -> Result<(), String> {
    let dur = (end - start).max(0.1);
    let mut c = command(ffmpeg);
    c.args(["-hide_banner", "-y", "-ss", &format!("{start:.3}"), "-i"]).arg(src);
    match overlay_png {
        None => {
            c.args(["-t", &format!("{dur:.3}"), "-map", "0:v:0", "-map", "0:a:0?", "-c", "copy", "-avoid_negative_ts", "make_zero"]);
        }
        Some(png) => {
            c.args(["-loop", "1", "-i"]).arg(png).args([
                "-t", &format!("{dur:.3}"),
                // The UI renders the overlay PNG at the source video's own size.
                "-filter_complex", "[0:v][1:v]overlay=0:0:shortest=1,format=yuv420p[v]",
                "-map", "[v]", "-map", "0:a:0?",
                "-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p",
                "-c:a", "aac", "-b:a", "128k",
            ]);
        }
    }
    let status = c
        .arg(out)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("could not run ffmpeg: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("ffmpeg failed to export the clip".into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_ffmpeg_banner() {
        let s = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'a.mp4':\n  Duration: 01:02:03.50, start: 0.000000, bitrate: 5000 kb/s\n  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, bt709, progressive), 1920x1080 [SAR 1:1 DAR 16:9], 4800 kb/s, 25 fps, 25 tbr, 12800 tbn (default)\n";
        let p = super::parse_probe(s);
        assert_eq!(p.duration, 3723.5);
        assert_eq!((p.width, p.height), (1920, 1080));
        assert_eq!(p.fps, 25.0);
        assert_eq!(p.vcodec, "h264");
    }
}
