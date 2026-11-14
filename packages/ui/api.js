// Thin wrappers over the Tauri commands shared by both apps.
import { invoke, convertFileSrc } from "@tauri-apps/api/core";

export { invoke };

// Generic record store (see crates/coach-core/src/store.rs).
export const db = {
  list: (kind) => invoke("rec_list", { kind }),
  by: (kind, field, value) => invoke("rec_list_by", { kind, field, value }),
  get: (kind, id) => invoke("rec_get", { kind, id }),
  // Create (no id) or update only the given fields.
  put: (kind, fields, id = null) => invoke("rec_put", { kind, id, fields }),
  del: (kind, id) => invoke("rec_delete", { kind, id }),
};

// URL a <video>/<img> can load for a local file path.
export const fileUrl = (p) => (p ? convertFileSrc(p) : "");

export function fmtTime(s, withTenths = false) {
  if (!Number.isFinite(s)) s = 0;
  s = Math.max(0, s);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = Math.floor(s % 60);
  const base = (h ? h + ":" + String(m).padStart(2, "0") : m) + ":" + String(sec).padStart(2, "0");
  return withTenths ? base + "." + Math.floor((s % 1) * 10) : base;
}

export const today = () => new Date().toISOString().slice(0, 10);

export function byKey(list, key) {
  const m = {};
  for (const x of list) m[x[key]] = x;
  return m;
}

export function playerLabel(p) {
  if (!p) return "";
  return (p.number != null && p.number !== "" ? "#" + p.number + " " : "") + (p.name || "");
}

export function sortPlayers(list) {
  return [...list].sort((a, b) => (Number(a.number) || 999) - (Number(b.number) || 999) || (a.name || "").localeCompare(b.name || ""));
}

// Yes/no question, shown as an in-app dialog when <DialogHost /> is mounted. Separate a short title from details with a blank line ("Delete X?\n\nDetails..."). Always await it.
export async function ask(message, opts) {
  const { dialogs, confirmDialog } = await import("./dialog.svelte.js");
  if (dialogs.hosted) return confirmDialog(message, opts);
  const r = window.confirm(message);
  return r && typeof r.then === "function" ? Boolean(await r) : Boolean(r);
}

// Asks for a line of text in an in-app dialog. Resolves to the text, or null when cancelled.
export async function askText(title, value = "", opts) {
  const { dialogs, textDialog } = await import("./dialog.svelte.js");
  if (dialogs.hosted) return textDialog(title, value, opts);
  const r = window.prompt(title, value);
  return r && typeof r.then === "function" ? await r : r;
}

// A message with a single OK button.
export async function notice(message) {
  return ask(message, { cancel: null, danger: false });
}
