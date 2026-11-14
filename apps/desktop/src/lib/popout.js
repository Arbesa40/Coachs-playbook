// Second-screen windows; RemoteSource mirrors the pop-out video's state so the app treats it like any Source.
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { availableMonitors, currentMonitor } from "@tauri-apps/api/window";
import { emit } from "@tauri-apps/api/event";

export const POPOUT_LABEL = "video";

export class RemoteSource {
  constructor(video) {
    this.kind = video?.source ?? "local";
    this.frameExact = this.kind !== "youtube";
    this.st = { time: 0, duration: video?.duration ?? 0, paused: true, rate: 1, rates: [0.1, 0.25, 0.5, 0.75, 1, 1.25, 1.5, 2], at: performance.now() };
  }
  update(s) {
    Object.assign(this.st, s);
    this.st.at = performance.now();
  }
  send(c, a) {
    emit("pv:cmd", { c, a });
  }
  play() {
    this.st.time = this.time();
    this.st.paused = false;
    this.st.at = performance.now();
    this.send("play");
  }
  pause() {
    this.st.time = this.time();
    this.st.paused = true;
    this.send("pause");
  }
  get paused() {
    return this.st.paused;
  }
  seek(t) {
    this.st.time = Math.max(0, t);
    this.st.at = performance.now();
    this.send("seek", t);
  }
  time() {
    const s = this.st;
    return s.paused ? s.time : s.time + ((performance.now() - s.at) / 1000) * s.rate;
  }
  duration() {
    return this.st.duration;
  }
  setRate(r) {
    this.st.time = this.time();
    this.st.at = performance.now();
    this.st.rate = r;
    this.send("rate", r);
  }
  rates() {
    return this.st.rates;
  }
  step(dir) {
    this.st.time = this.time();
    this.st.paused = true;
    this.send("step", dir);
  }
  destroy() {}
}

// Open (or focus) the video window, on another monitor when one is connected.
export function openPopout() {
  return openSecondWindow(POPOUT_LABEL, "#video", "Coach's Playbook · Video");
}

export const BOARD_LABEL = "board";
// The tactics board on a second screen (for team talks).
export function openBoardWindow() {
  return openSecondWindow(BOARD_LABEL, "#board", "Coach's Playbook · Tactics board");
}
export async function closeBoardWindow() {
  const w = await WebviewWindow.getByLabel(BOARD_LABEL);
  if (w) await w.close();
}

export const COMPARE_LABEL = "compare";
// "Compare two videos" on a second screen.
export function openCompareWindow() {
  return openSecondWindow(COMPARE_LABEL, "#compare", "Coach's Playbook · Compare");
}
export async function closeCompareWindow() {
  const w = await WebviewWindow.getByLabel(COMPARE_LABEL);
  if (w) await w.close();
}

async function openSecondWindow(label, hash, title) {
  const existing = await WebviewWindow.getByLabel(label);
  if (existing) {
    await existing.setFocus();
    return existing;
  }
  const opts = { url: "index.html" + hash, title, width: 1280, height: 720, center: true };
  try {
    const [monitors, here] = await Promise.all([availableMonitors(), currentMonitor()]);
    const other = monitors.find((m) => !here || m.position.x !== here.position.x || m.position.y !== here.position.y);
    if (other) {
      const sf = other.scaleFactor || 1;
      opts.center = false;
      opts.x = Math.round(other.position.x / sf) + 40;
      opts.y = Math.round(other.position.y / sf) + 40;
      opts.maximized = true;
    }
  } catch {}
  return new WebviewWindow(label, opts);
}

export async function closePopout() {
  const w = await WebviewWindow.getByLabel(POPOUT_LABEL);
  if (w) await w.close();
}
