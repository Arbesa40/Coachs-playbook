// A running match more than an hour past its expected end and untouched for an hour is removed.
import { teamConfig } from "./sports.js";

const MIN = 60_000;
const HOUR = 60 * MIN;
const BREAK_MIN = 15; // allowance for each break between periods
const NO_CLOCK_MIN = 120; // sports without a game clock (e.g. volleyball)

// Delete a match and every event recorded for it. Synced devices remove it at the next sync.
export async function deleteMatch(db, matchId) {
  const events = await db.by("match_event", "match_id", matchId);
  for (const e of events) await db.del("match_event", e.id);
  await db.del("match", matchId);
}

// When the match started recording (ms), or null if unknown.
function beganAt(m) {
  return Number(m.live?.began) || Number(m.activity_at) || Number(m.live?.startedAt) || null;
}

// The time (ms) a running match is expected to be over, or null if it can't be worked out.
export function expectedEnd(cfg, m) {
  const began = beganAt(m);
  if (!began) return null;
  const n = cfg.periods?.length ?? 0;
  const len = Number(m.period_minutes) || cfg.periodMinutes || 0;
  const minutes = cfg.clock === "none" || !n || !len ? NO_CLOCK_MIN : n * len + Math.max(0, n - 1) * BREAK_MIN;
  return began + minutes * MIN;
}

export function isStalled(cfg, m, now = Date.now()) {
  if (!["live", "break"].includes(m.live?.phase)) return false;
  const end = expectedEnd(cfg, m);
  if (!end) return false;
  const lastTouch = Math.max(Number(m.activity_at) || 0, Number(m.live?.startedAt) || 0, beganAt(m) || 0);
  return now > end + HOUR && now - lastTouch > HOUR;
}

// Remove abandoned running matches. Returns the matches that were removed.
export async function sweepStalledMatches(db, team, now = Date.now()) {
  if (!team) return [];
  const cfg = teamConfig(team);
  const removed = [];
  for (const m of await db.list("match")) {
    if (!isStalled(cfg, m, now)) continue;
    try {
      await deleteMatch(db, m.id);
      removed.push(m);
    } catch (e) {
      console.error("Could not remove stalled match", m.id, e);
    }
  }
  if (removed.length && typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("cp:matches-changed", { detail: removed.map((m) => m.id) }));
  }
  return removed;
}
