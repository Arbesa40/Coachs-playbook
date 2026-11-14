//! Share package: a ZIP with an HTML viewer that opens in any browser.

use crate::media;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use coach_core::{Error, Result};
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

pub fn decode_data_url(s: &str) -> Result<Vec<u8>> {
    let b64 = s.split_once(',').map(|(_, b)| b).unwrap_or(s);
    B64.decode(b64.trim()).map_err(|_| Error::Other("bad image data".into()))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub fn build_package(
    ffmpeg: Option<&Path>,
    team: &Value,
    playlist: &Value,
    clips: &[Value],
    videos: &[Value],
    out: &Path,
) -> Result<()> {
    let tmp = std::env::temp_dir().join(format!("cpb-share-{}", coach_core::new_id()));
    std::fs::create_dir_all(&tmp)?;
    let mut zip = zip::ZipWriter::new(File::create(out)?);
    let opts = || SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut items = Vec::new();

    let ids: Vec<&str> = playlist["items"].as_array().into_iter().flatten().filter_map(|v| v.as_str()).collect();
    for (n, cid) in ids.iter().enumerate() {
        let Some(clip) = clips.iter().find(|c| c["id"] == *cid) else { continue };
        let Some(video) = videos.iter().find(|v| v["id"] == clip["video_id"]) else { continue };
        let start = clip["start"].as_f64().unwrap_or(0.0);
        let end = clip["end"].as_f64().unwrap_or(start + 10.0);
        let mut item = json!({
            "title": clip["name"].as_str().unwrap_or("Clip"),
            "notes": clip["notes"].as_str().unwrap_or(""),
            "start": start,
            "end": end,
        });
        if let Some(snap) = clip["snapshot"].as_str().filter(|s| !s.is_empty()) {
            let name = format!("snapshots/{:03}.png", n + 1);
            zip.start_file(name.as_str(), opts()).map_err(|e| Error::Other(e.to_string()))?;
            zip.write_all(&decode_data_url(snap)?)?;
            item["snapshot"] = json!(name);
        }
        if video["source"] == "youtube" {
            item["youtube"] = video["youtube_id"].clone();
        } else if let Some(ff) = ffmpeg {
            let src = Path::new(video["path"].as_str().unwrap_or_default());
            let src = if src.exists() { src } else { Path::new(video["proxy_path"].as_str().unwrap_or_default()) };
            let tmpf = tmp.join(format!("{:03}.mp4", n + 1));
            if media::export_clip(ff, src, start, end, &tmpf, None).is_ok() {
                let name = format!("clips/{:03}.mp4", n + 1);
                zip.start_file(name.as_str(), opts()).map_err(|e| Error::Other(e.to_string()))?;
                let mut f = File::open(&tmpf)?;
                std::io::copy(&mut f, &mut zip)?;
                item["file"] = json!(name);
            }
        }
        items.push(item);
    }

    let data = json!({
        "team": team["name"].as_str().unwrap_or(""),
        "title": playlist["name"].as_str().unwrap_or("Playlist"),
        "items": items,
    });
    let html = VIEWER
        .replace("{{TITLE}}", &esc(data["title"].as_str().unwrap_or("")))
        .replace("{{DATA}}", &data.to_string().replace("</", "<\\/"))
        .replace("{{LOGO}}", &B64.encode(EMBLEM));
    zip.start_file("index.html", opts()).map_err(|e| Error::Other(e.to_string()))?;
    zip.write_all(html.as_bytes())?;
    zip.start_file("README.txt", opts()).map_err(|e| Error::Other(e.to_string()))?;
    zip.write_all(b"Unzip this folder, then open index.html in any web browser.\r\nYouTube clips need an internet connection; everything else works offline.\r\n")?;
    zip.finish().map_err(|e| Error::Other(e.to_string()))?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(())
}

/// The brand emblem, embedded so the viewer works offline.
const EMBLEM: &[u8] = include_bytes!("../../../../packages/ui/brand/emblem-light.png");

const VIEWER: &str = r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{{TITLE}}</title>
<style>
body{margin:0;font:15px/1.45 system-ui,Segoe UI,Roboto,sans-serif;background:#0f1419;color:#e6e9ee;display:flex;min-height:100vh}
aside{width:300px;background:#161c23;overflow:auto;border-right:1px solid #252d37}
aside h1{font-size:16px;margin:16px}.brand{display:flex;align-items:center;gap:8px;padding:14px 16px 0;font-size:12px;letter-spacing:1.5px;color:#cfd6de}.brand img{height:26px}aside small{display:block;margin:-10px 16px 12px;color:#8b96a5}
button.item{display:block;width:100%;text-align:left;background:none;border:0;border-top:1px solid #222a33;color:inherit;padding:10px 16px;cursor:pointer;font:inherit}
button.item.on{background:#22303d}button.item span{color:#8b96a5;font-size:12px;display:block}
main{flex:1;padding:20px;max-width:1100px}
.stage{position:relative;background:#000;aspect-ratio:16/9;width:100%}
.stage video,.stage iframe{position:absolute;inset:0;width:100%;height:100%;border:0}
img.snap{max-width:100%;margin-top:12px;border:1px solid #333}
.notes{white-space:pre-wrap;margin-top:12px;color:#c9d1db}
</style></head><body>
<aside><div class="brand"><img src="data:image/png;base64,{{LOGO}}" alt=""><span>COACH'S <b>PLAYBOOK</b></span></div><h1 id="t"></h1><small id="team"></small><div id="list"></div></aside>
<main><div class="stage" id="stage"></div><h2 id="ct"></h2><div class="notes" id="notes"></div><div id="snap"></div></main>
<script>
const D = {{DATA}};
const $ = (id) => document.getElementById(id);
$("t").textContent = D.title; $("team").textContent = D.team;
const fmt = (s) => { s = Math.max(0, Math.round(s)); return Math.floor(s/60) + ":" + String(s%60).padStart(2,"0"); };
function show(i) {
  const it = D.items[i]; if (!it) return;
  document.querySelectorAll("button.item").forEach((b, j) => b.classList.toggle("on", j === i));
  const st = $("stage"); st.innerHTML = "";
  if (it.youtube) {
    const f = document.createElement("iframe");
    f.allow = "autoplay; encrypted-media; picture-in-picture"; f.allowFullscreen = true;
    f.src = "https://www.youtube-nocookie.com/embed/" + it.youtube + "?start=" + Math.floor(it.start) + "&end=" + Math.ceil(it.end) + "&autoplay=1&rel=0";
    st.appendChild(f);
  } else if (it.file) {
    const v = document.createElement("video"); v.src = it.file; v.controls = true; v.autoplay = true;
    v.onended = () => show(i + 1); st.appendChild(v);
  } else { st.textContent = "Clip not available"; }
  $("ct").textContent = it.title; $("notes").textContent = it.notes || "";
  $("snap").innerHTML = it.snapshot ? '<img class="snap" src="' + it.snapshot + '" alt="Drawing">' : "";
}
D.items.forEach((it, i) => {
  const b = document.createElement("button"); b.className = "item";
  b.innerHTML = "<b></b><span></span>"; b.querySelector("b").textContent = (i+1) + ". " + it.title;
  b.querySelector("span").textContent = (it.youtube ? "YouTube · " : "") + fmt(it.start) + "–" + fmt(it.end);
  b.onclick = () => show(i); $("list").appendChild(b);
});
show(0);
</script></body></html>
"#;
