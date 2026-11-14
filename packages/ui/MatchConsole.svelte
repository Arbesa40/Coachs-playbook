<script>
  import Icon from "./Icon.svelte";
  import { db, playerLabel, sortPlayers, ask, notice } from "./api.js";
  import {
    eventDef, activeEvents, score, setScores, suspensions, playerFouls, teamFouls, timeoutsLeft,
    substitutionsUsed, onField, allPeriods, matchClock, fmtClock, describe, sortEvents,
  } from "./match.js";

  let { match, cfg, team, players = [], events = $bindable([]), onmatch, ondiscard = null, phone = false } = $props();

  const LIVE0 = { phase: "pre", period: 0, running: false, startedAt: 0, elapsed: 0 };
  const live = $derived({ ...LIVE0, ...(match?.live ?? {}) });
  const periods = $derived(allPeriods(cfg));
  const noClock = $derived(cfg.clock === "none");
  const squad = $derived(sortPlayers(players));
  const pById = $derived(Object.fromEntries(players.map((p) => [p.id, p])));
  const name = (id) => (id ? playerLabel(pById[id]) || "?" : "");

  let now = $state(Date.now());
  $effect(() => {
    const i = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(i);
  });

  const periodLen = $derived(
    (live.period < cfg.periods.length ? Number(match?.period_minutes) || cfg.periodMinutes : cfg.extraMinutes || cfg.periodMinutes) * 60
  );
  const periodSec = $derived(live.elapsed + (live.running ? (now - live.startedAt) / 1000 : 0));
  const clockNow = $derived(matchClock(cfg, live.period, periodSec, Number(match?.period_minutes) || undefined));
  const over = $derived(periodLen > 0 && periodSec > periodLen);

  const sc = $derived(score(events));
  const sets = $derived(setScores(cfg, events));
  const susp = $derived(suspensions(cfg, events, clockNow));
  const pf = $derived(playerFouls(cfg, events));
  const tf = $derived(teamFouls(cfg, events, live.period));
  const tos = $derived(timeoutsLeft(cfg, events, live.period));
  const subsUsed = $derived(substitutionsUsed(events));
  const field = $derived(onField(cfg, match, events, clockNow));
  const lineupSet = $derived((match?.starters ?? []).length > 0);
  const sentOff = $derived(new Set(susp.filter((s) => s.sentOff && !s.replaceable).map((s) => s.player_id)));
  const temps = $derived(
    events
      .map((e) => ({ e, d: eventDef(cfg, e.kind) }))
      .filter(({ e, d }) => d?.temporaryMinutes && clockNow < (e.clock ?? 0) + d.temporaryMinutes * 60)
      .map(({ e, d }) => ({ e, label: d.label, left: (e.clock ?? 0) + d.temporaryMinutes * 60 - clockNow }))
  );
  const log = $derived(sortEvents(events).filter((e) => !e.auto).reverse());

  const ours = (list) => activeEvents(list, team).filter((d) => d.team !== "them");
  const theirs = (list) => activeEvents(list, team).filter((d) => d.team === "them");
  const scoreUs = $derived(ours(cfg.scoring));
  const scoreThem = $derived(theirs(cfg.scoring));
  const discipline = $derived(activeEvents(cfg.discipline, team));
  const others = $derived(activeEvents(cfg.other ?? [], team));

  // activity_at records the last touch, used to remove abandoned matches.
  function setLive(f) {
    return onmatch({ live: { ...live, ...f }, activity_at: Date.now() });
  }
  async function syncScore(list = events) {
    const s = score(list);
    const v = setScores(cfg, list);
    const at = { activity_at: Date.now() };
    await onmatch(v ? { goals_for: v.won.us, goals_against: v.won.them, points_for: s.us, points_against: s.them, ...at } : { goals_for: s.us, goals_against: s.them, ...at });
  }
  async function rec(kind, f = {}) {
    const d = eventDef(cfg, kind);
    const isScore = cfg.scoring.some((x) => x.id === kind);
    const e = {
      match_id: match.id,
      kind,
      team: f.team ?? (d?.team === "them" ? "them" : "us"),
      points: isScore ? d.points : 0,
      player_id: null,
      player2_id: null,
      clock: Math.round(clockNow),
      minute: Math.round((clockNow / 60) * 10) / 10,
      period: live.period,
      period_label: periods[live.period] ?? "",
      ...f,
    };
    const r = await db.put("match_event", e);
    events = [...events, r];
    if (isScore || f.points) await syncScore();
    else onmatch({ activity_at: Date.now() });
    return r;
  }

  // ---------- clock ----------
  async function startMatch() {
    if (!lineupSet && !await ask("No starting line-up is set, so minutes played won't be tracked. Start anyway?")) return;
    for (const pid of match.starters ?? []) await rec("start", { player_id: pid, clock: 0, minute: 0, auto: true });
    await rec("period_start", { auto: true });
    await setLive({ phase: "live", period: 0, running: !noClock, startedAt: Date.now(), elapsed: 0, began: Date.now() });
  }
  const pause = () => setLive({ running: false, elapsed: periodSec });
  const resume = () => setLive({ running: true, startedAt: Date.now(), elapsed: periodSec });
  const adjust = (d) => setLive({ elapsed: Math.max(0, periodSec + d), startedAt: Date.now() });
  async function endPeriod() {
    await rec("period_end", { auto: true });
    await setLive({ phase: "break", running: false, elapsed: periodSec });
  }
  async function nextPeriod() {
    const p = live.period + 1;
    await setLive({ phase: "live", period: p, running: !noClock, startedAt: Date.now(), elapsed: 0 });
    await rec("period_start", { auto: true, period: p, period_label: periods[p], clock: Math.round(matchClock(cfg, p, 0, Number(match?.period_minutes) || undefined)) });
  }
  async function fullTime() {
    if (!await ask("Record full time?")) return;
    await rec("end");
    await setLive({ phase: "done", running: false, elapsed: periodSec });
  }
  const reopen = () => setLive({ phase: "break" });

  // ---------- player picking ----------
  let pick = $state(null); // { title, list, extra: [{label, run}], method?, methods?, then(pid, method) }
  function candidates(d) {
    let list = lineupSet ? squad.filter((p) => field.has(p.id)) : squad.filter((p) => !sentOff.has(p.id));
    if (d?.onlyPositions) {
      const only = d.onlyPositions.map((x) => x.toLowerCase());
      const shooters = list.filter((p) => only.includes(String(p.position ?? "").toLowerCase()));
      if (shooters.length) list = shooters;
    }
    return list;
  }
  function act(d) {
    const needsPlayer = d.team !== "them" && (d.player || d.color);
    if (!needsPlayer) {
      if (d.methods?.length) {
        pick = { title: d.label, list: [], extra: [...d.methods.map((m) => ({ label: m, run: () => done(d, null, m) })), { label: "No detail", run: () => done(d, null) }] };
      } else done(d, null);
      return;
    }
    const extra = [];
    if (d.team === "either") extra.push({ label: "By the opponent", run: (m) => done(d, null, m, "them") });
    if (cfg.scoring.some((x) => x.id === d.id)) extra.push({ label: "Unknown player", run: (m) => done(d, null, m) });
    pick = {
      title: d.color ? `${d.label}: which player?` : `${d.label}: who?`,
      list: candidates(d),
      methods: d.methods ?? null,
      method: "",
      extra,
      then: (pid, m) => done(d, pid, m),
      note: d.onlyPositions ? `Only ${d.onlyPositions.join(" / ")} can score.` : null,
    };
  }
  let follow = $state(null); // e.g. conversion after a try
  async function done(d, pid, method = "", teamOverride = null) {
    pick = null;
    const f = { player_id: pid };
    if (method) f.method = method;
    if (teamOverride) f.team = teamOverride;
    const second = d.secondMeans === "red" && pid && events.some((e) => e.kind === d.id && e.player_id === pid && e.team !== "them");
    const r = await rec(d.id, f);
    // Sent off (red card, or a second yellow): take the player out of the minutes count too.
    if ((d.sendsOff || second) && pid) await rec("sub_off", { player_id: pid, auto: true, linked: r.id });
    // Volleyball penalty card: point to the opponent.
    if (d.givesPointTo) await rec("point_lost", { team: d.givesPointTo, points: 1, auto: true, linked: r.id });
    if (cfg.id === "rugby" && d.id === "try") follow = { us: true };
    if (cfg.id === "rugby" && d.id === "try_against") follow = { us: false };
  }
  async function conversion(made) {
    const us = follow.us;
    follow = null;
    if (us && made) act(eventDef(cfg, "conversion"));
    else if (us) {
      pick = { title: "Missed conversion: kicker?", list: candidates(null), extra: [{ label: "Skip", run: () => rec("kick_missed") }], then: (pid) => { pick = null; rec("kick_missed", { player_id: pid }); } };
    } else if (made) rec("conversion_against");
  }
  async function sub() {
    if (!lineupSet) return notice("Set the starting line-up first.\n\nSubstitutions need to know who is on the field.");
    if (cfg.subs?.max != null && subsUsed >= cfg.subs.max && !await ask(`${subsUsed} of ${cfg.subs.max} substitutions used. Record another anyway?`)) return;
    pick = {
      title: "Substitution: who comes off?",
      list: squad.filter((p) => field.has(p.id)),
      then: (off) => {
        pick = {
          title: `${name(off)} off. Who comes on?`,
          list: squad.filter((p) => !field.has(p.id) && !sentOff.has(p.id)),
          then: async (on) => {
            pick = null;
            const o = await rec("sub_off", { player_id: off, auto: true });
            await rec("sub_on", { player_id: on, player2_id: off, linked: o.id });
          },
        };
      },
    };
  }
  async function timeout(us) {
    if (us && tos != null && tos <= 0 && !await ask("No time-outs left in this period. Record anyway?")) return;
    await rec(us ? "timeout" : "timeout_them", { team: us ? "us" : "them" });
    if (!noClock && live.running) pause();
  }
  async function del(e) {
    if (!await ask(`Delete "${describe(cfg, e, name)}" at ${fmtClock(e.clock ?? (e.minute ?? 0) * 60)}?`)) return;
    const gone = events.filter((x) => x.id === e.id || x.id === e.linked || x.linked === e.id);
    for (const x of gone) await db.del("match_event", x.id);
    const ids = new Set(gone.map((x) => x.id));
    events = events.filter((x) => !ids.has(x.id));
    await syncScore();
  }
  const fouls = (pid) => pf[pid]?.total ?? 0;
  const colorOf = (e) => eventDef(cfg, e.kind)?.color ?? (e.points ? (e.team === "them" ? "#f87171" : "#4ade80") : null);
</script>

<div class="mc" class:phone>
  <div class="board card">
    <div class="sb">
      <div class="sbside"><div class="tn">Us</div><div class="pts">{sets ? sets.won.us : sc.us}</div></div>
      <div class="mid">
        {#if !noClock}
          <div class="clk" class:run={live.running} class:over>{fmtClock(clockNow)}</div>
        {/if}
        <div class="per">
          {live.phase === "pre" ? "Not started" : live.phase === "done" ? "Full time" : (periods[live.period] ?? "") + (live.phase === "break" ? " · break" : "")}
          {#if !noClock && live.phase === "live" && periodLen}· {over ? "+" + fmtClock(periodSec - periodLen) : fmtClock(periodLen - periodSec) + " left"}{/if}
        </div>
        {#if sets}<div class="per">Set {live.period + 1}: {sets.sets[live.period]?.us ?? 0}–{sets.sets[live.period]?.them ?? 0}</div>{/if}
      </div>
      <div class="sbside"><div class="tn">{match.opponent || "Them"}</div><div class="pts">{sets ? sets.won.them : sc.them}</div></div>
    </div>

    <div class="ctl row wrap">
      {#if live.phase === "pre"}
        <button class="primary big" onclick={startMatch}><Icon name="whistle" /> Start match</button>
      {:else if live.phase === "live"}
        {#if !noClock}
          {#if live.running}<button class="big" onclick={pause}><Icon name="pause" /> Pause clock</button>
          {:else}<button class="primary big" onclick={resume}><Icon name="play" /> Restart clock</button>{/if}
        {/if}
        <button onclick={endPeriod}><Icon name="flag" /> End {periods[live.period] ?? "period"}</button>
        {#if !noClock && !live.running}
          <span class="row adj">
            <button class="small" onclick={() => adjust(-60)}>−1m</button><button class="small" onclick={() => adjust(-10)}>−10s</button>
            <button class="small" onclick={() => adjust(10)}>+10s</button><button class="small" onclick={() => adjust(60)}>+1m</button>
          </span>
        {/if}
      {:else if live.phase === "break"}
        {#if live.period + 1 < periods.length && !(sets?.over)}
          <button class="primary big" onclick={nextPeriod}><Icon name="play" /> Start {periods[live.period + 1]}</button>
        {/if}
        <button onclick={fullTime}><Icon name="flag" /> Full time</button>
      {:else}
        <button class="ghost small" onclick={reopen}>Reopen match</button>
      {/if}
      {#if ondiscard && (live.phase === "live" || live.phase === "break")}
        <button class="ghost small danger discard" onclick={ondiscard} title="Throw this match away, with everything recorded so far"><Icon name="trash" size={14} /> Discard match</button>
      {/if}
    </div>
    {#if sets && live.phase === "live" && sets.sets[live.period]?.winner}
      <div class="hint">Set {live.period + 1} won by {sets.sets[live.period].winner === "us" ? "us" : "them"}. <button class="small primary" onclick={endPeriod}>End set</button></div>
    {/if}
    {#if cfg.winAt && Math.max(sc.us, sc.them) >= cfg.winAt && live.phase === "live"}
      <div class="hint">{cfg.winAt} points reached: the game is won. <button class="small primary" onclick={fullTime}>Full time</button></div>
    {/if}
  </div>

  {#if follow}
    <div class="card hint row wrap">
      <b class="grow">{follow.us ? "Conversion?" : "Did they convert?"}</b>
      <button class="primary" onclick={() => conversion(true)}>Kicked</button>
      <button onclick={() => conversion(false)}>Missed</button>
      <button class="ghost" onclick={() => (follow = null)}>Later</button>
    </div>
  {/if}

  {#if pick}
    <div class="card pick col">
      <div class="row"><b class="grow">{pick.title}</b><button class="ghost" onclick={() => (pick = null)}>Cancel</button></div>
      {#if pick.note}<div class="muted small">{pick.note}</div>{/if}
      {#if pick.methods}
        <div class="row wrap">
          {#each pick.methods as m}<button class="small" class:on={pick.method === m} onclick={() => (pick.method = pick.method === m ? "" : m)}>{m}</button>{/each}
        </div>
      {/if}
      {#if pick.list.length}
        <div class="chips">
          {#each pick.list as p (p.id)}
            <button class="chip" onclick={() => pick.then(p.id, pick.method)}>
              {playerLabel(p)}{#if cfg.personalFoulLimit && fouls(p.id)}<span class="badge {fouls(p.id) >= cfg.personalFoulLimit - 1 ? 'bad' : ''}">{fouls(p.id)}F</span>{/if}
            </button>
          {/each}
        </div>
      {:else if pick.then}
        <div class="muted small">No players available. Check the line-up.</div>
      {/if}
      {#if pick.extra?.length}<div class="row wrap">{#each pick.extra as x}<button onclick={() => { const m = pick.method; pick = null; x.run(m); }}>{x.label}</button>{/each}</div>{/if}
    </div>
  {:else if live.phase === "live" || live.phase === "break"}
    <div class="pads">
      <div class="card col">
        <h3>Score</h3>
        <div class="btns">
          {#each scoreUs as d}<button class="good" onclick={() => act(d)}>{d.label}{#if d.points} <span class="pt">+{d.points}</span>{/if}</button>{/each}
        </div>
        <div class="btns">
          {#each scoreThem as d}<button class="bad" onclick={() => act(d)}>{d.label}{#if d.points} <span class="pt">+{d.points}</span>{/if}</button>{/each}
        </div>
      </div>
      <div class="card col">
        <h3>Discipline & fouls</h3>
        <div class="btns">
          {#each discipline as d}<button onclick={() => act(d)}>{#if d.color}<Icon name="card" color={d.color} size={16} />{/if} {d.label}</button>{/each}
        </div>
        <h3>Match</h3>
        <div class="btns">
          <button onclick={sub}><Icon name="swap" size={16} /> Substitution{#if cfg.subs?.max != null} <span class="pt">{subsUsed}/{cfg.subs.max}</span>{/if}</button>
          {#if cfg.timeouts}
            <button onclick={() => timeout(true)}><Icon name="timer" size={16} /> Our time-out {#if tos != null}<span class="pt">{tos} left</span>{/if}</button>
            <button onclick={() => timeout(false)}>Their time-out</button>
          {/if}
          {#each others as d}<button onclick={() => act(d)}>{d.label}</button>{/each}
        </div>
      </div>
    </div>
  {/if}

  {#if susp.length || temps.length || tf || (cfg.personalFoulLimit && Object.keys(pf).length)}
    <div class="card col status">
      {#each susp as s}
        <div class="row">
          <span class="dot" style="background:{s.color ?? '#999'}"></span>
          <span class="grow">{name(s.player_id)} · {s.label}</span>
          {#if s.left != null && s.left > 0}<b class="cd">{fmtClock(s.left)}{s.sentOff ? " until replacement" : ""}</b>
          {:else if s.replaceable}<span class="badge good">can be replaced</span>
          {:else}<span class="badge bad">off</span>{/if}
        </div>
      {/each}
      {#each temps as t}
        <div class="row"><span class="dot" style="background:#60a5fa"></span><span class="grow">{name(t.e.player_id)} · {t.label}</span><b class="cd">{fmtClock(t.left)}</b></div>
      {/each}
      {#if tf}
        <div class="row small">
          <span class="grow">Team fouls{tf.limit ? ` (penalty from ${tf.limit})` : ""}: us {tf.us} · them {tf.them}</span>
          {#if tf.themBonus}<span class="badge bad">They're in the bonus</span>{/if}
          {#if tf.usBonus}<span class="badge good">We're in the bonus</span>{/if}
        </div>
      {/if}
      {#if cfg.personalFoulLimit && Object.keys(pf).length}
        <div class="small muted">Personal fouls: {Object.entries(pf).map(([id, n]) => `${name(id)} ${n.total}`).join(" · ")}</div>
      {/if}
    </div>
  {/if}

  {#if sets}
    <table class="card setsT">
      <thead><tr><th></th>{#each cfg.periods as p, i}<th>{i + 1}</th>{/each}</tr></thead>
      <tbody>
        <tr><td>Us</td>{#each sets.sets as s}<td class:win={s.winner === "us"}>{s.us || ""}</td>{/each}</tr>
        <tr><td>Them</td>{#each sets.sets as s}<td class:win={s.winner === "them"}>{s.them || ""}</td>{/each}</tr>
      </tbody>
    </table>
  {/if}

  <div class="card flush log">
    <div class="loghead row"><h3 class="grow">Match log</h3><span class="muted small">{log.length} events</span></div>
    {#each log as e (e.id)}
      <div class="ev row">
        <span class="t">{noClock ? "Set " + ((e.period ?? 0) + 1) : fmtClock(e.clock ?? (e.minute ?? 0) * 60)}</span>
        <span class="dot" style="background:{colorOf(e) ?? 'transparent'}"></span>
        <span class="grow">{describe(cfg, e, name)}{e.team === "them" && !eventDef(cfg, e.kind)?.team ? " (opponent)" : ""}</span>
        {#if e.points}<span class="pt">{e.team === "them" ? "−" : "+"}{e.points}</span>{/if}
        <button class="ghost danger small" title="Delete event" onclick={() => del(e)}><Icon name="trash" size={14} /></button>
      </div>
    {:else}
      <div class="empty">Nothing recorded yet.</div>
    {/each}
  </div>
  {#if cfg.subs?.note}<div class="muted small">{cfg.formatLabel}: {cfg.subs.note}</div>{/if}
</div>

<style>
  .mc { display: flex; flex-direction: column; gap: 12px; }
  .sb { display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; gap: 12px; }
  .sbside { text-align: center; min-width: 0; }
  .tn { color: var(--muted); font-size: 12px; text-transform: uppercase; letter-spacing: 0.06em; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pts { font-size: 44px; font-weight: 800; font-variant-numeric: tabular-nums; line-height: 1.1; }
  .mid { text-align: center; }
  .clk { font-size: 34px; font-weight: 700; font-variant-numeric: tabular-nums; color: var(--muted); }
  .clk.run { color: var(--text); }
  .clk.over { color: var(--warn); }
  .per { color: var(--muted); font-size: 12px; }
  .ctl { justify-content: center; margin-top: 10px; }
  .ctl .discard { margin-left: auto; }
  .adj { gap: 4px; }
  .hint { margin-top: 8px; text-align: center; color: var(--warn); }
  .pads { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .phone .pads { grid-template-columns: 1fr; }
  .btns { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 6px; }
  .btns button { white-space: normal; text-align: left; min-height: 40px; display: flex; align-items: center; gap: 6px; }
  .phone .btns { grid-template-columns: 1fr 1fr; }
  .phone .btns button { min-height: 48px; }
  button.good { border-color: var(--score-us-line); background: var(--score-us); }
  button.bad { border-color: var(--score-them-line); background: var(--score-them); }
  .pt { margin-left: auto; font-size: 11px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .chips { display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 6px; }
  .phone .chips { grid-template-columns: repeat(auto-fill, minmax(100px, 1fr)); }
  .chip { min-height: 46px; white-space: normal; display: flex; gap: 4px; align-items: center; justify-content: center; }
  .pick { border-color: var(--accent); }
  .dot { width: 10px; height: 10px; border-radius: 2px; flex: none; }
  .cd { font-variant-numeric: tabular-nums; color: var(--warn); }
  .status { gap: 6px; }
  .log .loghead { padding: 10px 12px 4px; }
  .ev { padding: 6px 12px; border-top: 1px solid var(--line); }
  .ev .t { width: 48px; font-variant-numeric: tabular-nums; color: var(--muted); }
  .setsT { padding: 0; }
  .setsT td, .setsT th { text-align: center; }
  .setsT td.win { color: var(--good-fg); font-weight: 700; }
  .big { font-size: 15px; padding: 10px 16px; }
  .phone .big { font-size: 17px; padding: 12px 18px; }
</style>
