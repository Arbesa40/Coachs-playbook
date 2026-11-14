// Match engine: everything is derived from append-only match_event records, so devices' events simply merge.

export function eventDef(cfg, kind) {
  return (
    cfg.scoring.find((e) => e.id === kind) ||
    cfg.discipline.find((e) => e.id === kind) ||
    cfg.other.find((e) => e.id === kind) ||
    BASE_EVENTS[kind] ||
    null
  );
}

export const BASE_EVENTS = {
  start: { id: "start", label: "Starts" },
  sub_on: { id: "sub_on", label: "Sub on:" },
  sub_off: { id: "sub_off", label: "Off" },
  period_start: { id: "period_start", label: "Period start" },
  period_end: { id: "period_end", label: "Period end" },
  end: { id: "end", label: "Full time" },
  timeout: { id: "timeout", label: "Time-out" },
  timeout_them: { id: "timeout_them", label: "Opponent time-out" },
  set_end: { id: "set_end", label: "Set end" },
};

// Rule variations (e.g. 20-minute red, Super Shot) are hidden unless switched on for the team.
export function activeEvents(list, team) {
  const on = team?.rule_options ?? {};
  return list.filter((e) => !e.variation || on[e.id]);
}

export const sortEvents = (events) =>
  [...events].sort((a, b) => (a.period ?? 0) - (b.period ?? 0) || (a.clock ?? (a.minute ?? 0) * 60) - (b.clock ?? (b.minute ?? 0) * 60));

export function score(events) {
  let us = 0, them = 0;
  for (const e of events) {
    // Events recorded before the sport engine had no points/team: goal = 1 for us, goal_against = 1 for them.
    const pts = e.points ?? (e.kind === "goal" || e.kind === "goal_against" ? 1 : 0);
    if (!pts) continue;
    if (e.team === "them" || (e.team == null && e.kind === "goal_against")) them += pts;
    else us += pts;
  }
  return { us, them };
}

// Volleyball: points per set and sets won.
export function setScores(cfg, events) {
  if (!cfg.sets) return null;
  const sets = cfg.periods.map(() => ({ us: 0, them: 0, winner: null }));
  for (const e of sortEvents(events)) {
    if (!e.points) continue;
    const s = sets[e.period ?? 0];
    if (!s) continue;
    if (e.team === "them") s.them += e.points;
    else s.us += e.points;
  }
  let won = { us: 0, them: 0 };
  sets.forEach((s, i) => {
    const target = i === cfg.sets.bestOf - 1 ? cfg.sets.decidingSetPoints : cfg.sets.pointsToWin;
    if (s.us >= target && s.us - s.them >= cfg.sets.winBy) s.winner = "us";
    if (s.them >= target && s.them - s.us >= cfg.sets.winBy) s.winner = "them";
    if (s.winner) won[s.winner]++;
  });
  const need = Math.ceil(cfg.sets.bestOf / 2);
  return { sets, won, need, over: won.us >= need || won.them >= need };
}

// Players currently off the field: sin-binned (with seconds left) or sent off.
export function suspensions(cfg, events, clockNow) {
  const out = [];
  for (const e of events) {
    const d = eventDef(cfg, e.kind);
    if (!d || e.team === "them") continue;
    if (d.sinBin && d.minutes) {
      const until = (e.clock ?? 0) + d.minutes * 60;
      if (clockNow < until) out.push({ event: e, player_id: e.player_id, label: d.label, color: d.color, left: until - clockNow });
    } else if (d.sendsOff) {
      if (d.replaceAfterMinutes) {
        const until = (e.clock ?? 0) + d.replaceAfterMinutes * 60;
        out.push({ event: e, player_id: e.player_id, label: d.label, color: d.color, left: Math.max(0, until - clockNow), sentOff: true, replaceable: clockNow >= until });
      } else out.push({ event: e, player_id: e.player_id, label: d.label, color: d.color, left: null, sentOff: true });
    }
  }
  // Football: a second yellow is a red.
  const yellows = {};
  for (const e of events) {
    const d = eventDef(cfg, e.kind);
    if (d?.secondMeans === "red" && e.player_id && e.team !== "them") {
      yellows[e.player_id] = (yellows[e.player_id] ?? 0) + 1;
      if (yellows[e.player_id] === 2) out.push({ event: e, player_id: e.player_id, label: "Second yellow: sent off", color: "#ef4444", left: null, sentOff: true });
    }
  }
  // Basketball: personal-foul limit and two unsportsmanlike fouls disqualify.
  if (cfg.personalFoulLimit) {
    for (const [pid, n] of Object.entries(playerFouls(cfg, events))) {
      if (n.total >= cfg.personalFoulLimit) out.push({ player_id: pid, label: `Fouled out (${n.total})`, color: "#ef4444", left: null, sentOff: true });
      else if (n.unsportsmanlike >= 2) out.push({ player_id: pid, label: "2 unsportsmanlike fouls: disqualified", color: "#ef4444", left: null, sentOff: true });
    }
  }
  return out;
}

export function playerFouls(cfg, events) {
  const m = {};
  for (const e of events) {
    const d = eventDef(cfg, e.kind);
    if (!d?.foul || !e.player_id || e.team === "them") continue;
    const r = (m[e.player_id] ??= { total: 0, unsportsmanlike: 0 });
    r.total++;
    if (e.kind === "unsportsmanlike_foul") r.unsportsmanlike++;
  }
  return m;
}

// Team fouls in the current period (or game), for bonus situations.
export function teamFouls(cfg, events, period) {
  if (!cfg.teamFoulLimit) return null;
  const count = (team) =>
    events.filter((e) => {
      const d = eventDef(cfg, e.kind);
      const isFoul = d?.foul || e.kind === "foul" || e.kind === "foul_against";
      const sameTeam = team === "them" ? e.team === "them" : e.team !== "them";
      return isFoul && sameTeam && (!cfg.teamFoulLimit.perPeriod || (e.period ?? 0) === period);
    }).length;
  const us = count("us"), them = count("them");
  return { us, them, limit: cfg.teamFoulLimit.count, usBonus: them >= cfg.teamFoulLimit.count, themBonus: us >= cfg.teamFoulLimit.count, note: cfg.teamFoulLimit.note };
}

// Time-outs left for us in this period (basketball halves, volleyball sets...).
export function timeoutsLeft(cfg, events, period) {
  const t = cfg.timeouts;
  if (!t) return null;
  const ours = events.filter((e) => e.kind === "timeout");
  if (t.perSet != null) return t.perSet - ours.filter((e) => (e.period ?? 0) === period).length;
  if (t.perPeriod != null) return t.perPeriod - ours.filter((e) => (e.period ?? 0) === period).length;
  if (t.perGame != null) return t.perGame - ours.length;
  if (t.firstHalf != null) {
    const regular = cfg.periods.length;
    if (period >= regular) return (t.overtime ?? 1) - ours.filter((e) => (e.period ?? 0) === period).length;
    const firstHalf = period < regular / 2;
    const used = ours.filter((e) => ((e.period ?? 0) < regular / 2) === firstHalf && (e.period ?? 0) < regular).length;
    return (firstHalf ? t.firstHalf : t.secondHalf) - used;
  }
  return null;
}

export function substitutionsUsed(events) {
  return events.filter((e) => e.kind === "sub_on").length;
}

// Who is on the field now: starters, then subs, minus players sent off or sin-binned.
export function onField(cfg, match, events, clockNow) {
  const set = new Set(match?.starters ?? []);
  for (const e of sortEvents(events)) {
    if (e.kind === "sub_off") set.delete(e.player_id);
    if (e.kind === "sub_on") set.add(e.player_id);
  }
  const off = suspensions(cfg, events, clockNow);
  for (const s of off) if (s.player_id && (s.sentOff || s.left > 0)) set.delete(s.player_id);
  return set;
}

export function allPeriods(cfg) {
  return [...cfg.periods, ...(cfg.extraPeriods ?? [])];
}

// Match clock in seconds from the period index and seconds into that period.
export function matchClock(cfg, period, secondsIntoPeriod, periodMinutes) {
  const reg = cfg.periods.length;
  const len = (periodMinutes ?? cfg.periodMinutes ?? 0) * 60;
  if (period < reg) return period * len + secondsIntoPeriod;
  return reg * len + (period - reg) * (cfg.extraMinutes ?? 0) * 60 + secondsIntoPeriod;
}

export function fmtClock(sec) {
  sec = Math.max(0, Math.floor(sec || 0));
  return Math.floor(sec / 60) + ":" + String(sec % 60).padStart(2, "0");
}

// Short description of an event for lists.
export function describe(cfg, e, playerName = () => "") {
  const d = eventDef(cfg, e.kind);
  const who = [playerName(e.player_id), e.player2_id ? "for " + playerName(e.player2_id) : ""].filter(Boolean).join(" ");
  return [d?.label ?? e.kind, e.method ? `(${e.method})` : "", who].filter(Boolean).join(" ");
}
