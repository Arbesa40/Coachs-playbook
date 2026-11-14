// App-wide state shared by every screen.
export const app = $state({
  info: null, // device id, data folder, ffmpeg path
  team: null, // the open team record
  prevTeam: null, // the team that was open before "Switch team", for the Back button
  view: "analysis",
  toast: null,
});

let toastTimer;
export function toast(msg, kind = "info") {
  app.toast = { msg: String(msg), kind };
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (app.toast = null), kind === "error" ? 6000 : 3000);
}

// Run an async action and show any error as a toast instead of failing silently.
export async function attempt(fn, okMsg) {
  try {
    const r = await fn();
    if (okMsg) toast(okMsg);
    return r;
  } catch (e) {
    console.error(e);
    toast(e?.message || e, "error");
  }
}

import { teamConfig } from "$shared/sports.js";
// Sport + format rules for the open team. Use inside $derived(...).
export function teamCfg() {
  return teamConfig(app.team);
}
