import { invoke } from "@tauri-apps/api/core";

import { db } from "$shared/api.js";

export const ui = $state({ info: null, team: null, tab: "match", toast: null });

let timer;
export function toast(msg, kind = "info") {
  ui.toast = { msg: String(msg), kind };
  clearTimeout(timer);
  timer = setTimeout(() => (ui.toast = null), kind === "error" ? 6000 : 2500);
}

export async function attempt(fn, ok) {
  try {
    const r = await fn();
    if (ok) toast(ok);
    return r;
  } catch (e) {
    toast(e?.message || e, "error");
  }
}

// Small persisted values (match clock etc.) so a crash or app switch loses nothing.
export function saved(key, fallback) {
  try {
    return JSON.parse(localStorage.getItem(key)) ?? fallback;
  } catch {
    return fallback;
  }
}
export function persist(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {}
}

export async function refreshInfo() {
  ui.info = await invoke("app_info");
  if (!ui.info.paired) ui.tab = "sync";
  await loadTeam();
}

// The team record (name, sport, format, logo, rule options) from the last sync.
export async function loadTeam() {
  try {
    ui.team = ui.info?.team_id ? await db.get("team", ui.info.team_id) : null;
  } catch {
    ui.team = null;
  }
}
