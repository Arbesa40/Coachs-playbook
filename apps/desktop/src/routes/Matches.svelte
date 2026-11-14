<script>
  import Icon from "$shared/Icon.svelte";
  import MatchConsole from "$shared/MatchConsole.svelte";
  import { db, invoke, playerLabel, sortPlayers, today, ask } from "$shared/api.js";
  import { groupPlayers } from "$shared/sports.js";
  import { deleteMatch } from "$shared/matchCleanup.js";
  import { app, attempt, teamCfg } from "../lib/state.svelte.js";

  const cfg = $derived(teamCfg());
  let matches = $state([]);
  let players = $state([]);
  let events = $state([]);
  let minutes = $state({});
  let selId = $state(null);
  let tab = $state("live");

  const sel = $derived(matches.find((m) => m.id === selId) ?? null);
  const sorted = $derived([...matches].sort((a, b) => (b.date || "").localeCompare(a.date || "")));
  const groups = $derived(groupPlayers(cfg, sortPlayers(players)));

  async function load() {
    [matches, players] = await Promise.all([db.list("match"), db.list("player")]);
  }
  load();

  // Abandoned matches can be removed in the background (see matchCleanup.js).
  $effect(() => {
    const onChange = (e) => {
      if (selId && e.detail?.includes(selId)) {
        selId = null;
        events = [];
      }
      load();
    };
    window.addEventListener("cp:matches-changed", onChange);
    return () => window.removeEventListener("cp:matches-changed", onChange);
  });

  $effect(() => {
    if (!selId) return;
    const id = selId;
    db.by("match_event", "match_id", id).then((e) => id === selId && (events = e));
    invoke("minutes_played", { matchId: id }).then((m) => (minutes = m));
  });
  // Minutes played follow the event list.
  $effect(() => {
    const _ = events.length;
    if (selId) invoke("minutes_played", { matchId: selId }).then((m) => (minutes = m));
  });

  async function add() {
    const m = await attempt(() => db.put("match", { date: today(), opponent: "Opponent", home: true, venue: "", competition: "", goals_for: 0, goals_against: 0, starters: [], bench: [] }));
    await load();
    selId = m?.id;
    tab = "details";
  }
  async function set(fields) {
    Object.assign(sel, fields);
    await attempt(() => db.put("match", fields, sel.id));
  }
  async function remove(m = sel) {
    if (!m) return;
    const n = m.id === selId ? events.filter((e) => !e.auto).length : null;
    const what = n ? `, with its ${n} recorded events` : " and everything recorded for it";
    if (!await ask(`Delete the match against ${m.opponent} on ${m.date}${what}?\n\nThis can't be undone. Paired phones remove it at the next sync.`)) return;
    const ok = await attempt(() => deleteMatch(db, m.id).then(() => true), "Match deleted");
    if (!ok) return;
    if (m.id === selId) {
      selId = null;
      events = [];
    }
    load();
  }
  async function discard() {
    if (!await ask(`Discard the running match against ${sel.opponent}?\n\nThe score, the clock and every event recorded so far are thrown away. This can't be undone.`)) return;
    const ok = await attempt(() => deleteMatch(db, sel.id).then(() => true), "Match discarded");
    if (!ok) return;
    selId = null;
    events = [];
    load();
  }
  function role(pid) {
    return sel.starters?.includes(pid) ? "start" : sel.bench?.includes(pid) ? "bench" : "";
  }
  function setRole(pid, r) {
    const starters = (sel.starters ?? []).filter((x) => x !== pid);
    const bench = (sel.bench ?? []).filter((x) => x !== pid);
    if (r === "start") starters.push(pid);
    if (r === "bench") bench.push(pid);
    set({ starters, bench });
  }
  const scoreText = (m) => (m.goals_for ?? 0) + "–" + (m.goals_against ?? 0);
</script>

<div class="page">
  <div class="ph">
    <div>
      <h1>Matches</h1>
      <div class="sub">{cfg.name} · {cfg.formatLabel} · {cfg.periods.length} × {cfg.periodMinutes ?? "—"} min{cfg.sinBinMinutes ? ` · sin bin ${cfg.sinBinMinutes} min` : ""}</div>
    </div>
    <button class="primary" onclick={add}><Icon name="plus" /> New match</button>
  </div>
  <div class="layout">
    <div class="list card flush">
      {#each sorted as m (m.id)}
        <div class="mitem" class:on={m.id === selId}>
          <button class="item rowbtn grow" class:on={m.id === selId} onclick={() => { selId = m.id; tab = "live"; }}>
            <span class="row"><b class="grow">{m.home ? "vs" : "@"} {m.opponent}</b><span class="score">{scoreText(m)}</span></span>
            <span class="small muted">{m.date}{m.competition ? " · " + m.competition : ""}{m.live?.phase === "live" ? " · live" : m.live?.phase === "done" ? " · full time" : ""}</span>
          </button>
          <button class="ghost small del" title="Delete this match" onclick={() => remove(m)}><Icon name="trash" size={14} /></button>
        </div>
      {:else}
        <div class="empty">No matches yet.</div>
      {/each}
    </div>

    {#if sel}
      <div class="col">
        <div class="row">
          <h2 class="grow">{sel.home ? "vs" : "@"} {sel.opponent} <span class="muted">· {sel.date}</span></h2>
          <button class="ghost small danger" onclick={() => remove()} title="Delete this match"><Icon name="trash" size={14} /> Delete</button>
          <div class="tabs">
            <button class:on={tab === "live"} onclick={() => (tab = "live")}>Match recording</button>
            <button class:on={tab === "lineup"} onclick={() => (tab = "lineup")}>Line-up ({sel.starters?.length ?? 0}/{cfg.onField})</button>
            <button class:on={tab === "details"} onclick={() => (tab = "details")}>Details</button>
          </div>
        </div>

        {#if tab === "live"}
          {#key sel.id}
            <MatchConsole match={sel} {cfg} team={app.team} {players} bind:events onmatch={set} ondiscard={discard} />
          {/key}
        {:else if tab === "lineup"}
          <div class="card col">
            <div class="row">
              <h3 class="grow">Line-up</h3>
              <span class="small {sel.starters?.length > cfg.onField ? 'bad' : 'muted'}">{sel.starters?.length ?? 0} starting (of {cfg.onField}) · {sel.bench?.length ?? 0} on the bench{cfg.squad ? ` (match squad up to ${cfg.squad})` : ""}</span>
            </div>
            <table>
              <thead><tr><th>Player</th><th>Starts</th><th>Bench</th><th>Not involved</th><th>Minutes</th></tr></thead>
              <tbody>
                {#each groups as [g, list] (g)}
                  <tr class="gh"><td colspan="5">{g}</td></tr>
                  {#each list as p (p.id)}
                    <tr>
                      <td>{playerLabel(p)} <span class="muted small">{p.position ?? ""}</span></td>
                      {#each ["start", "bench", ""] as r}
                        <td><input type="radio" name={"r-" + p.id} checked={role(p.id) === r} onchange={() => setRole(p.id, r)} /></td>
                      {/each}
                      <td class="muted">{minutes[p.id] != null ? Math.round(minutes[p.id]) : ""}</td>
                    </tr>
                  {/each}
                {/each}
              </tbody>
            </table>
          </div>
        {:else}
          <div class="card col">
            <div class="grid3">
              <label class="field">Opponent <input value={sel.opponent} onchange={(e) => set({ opponent: e.currentTarget.value })} /></label>
              <label class="field">Date <input type="date" value={sel.date} onchange={(e) => set({ date: e.currentTarget.value })} /></label>
              <label class="field">Kick-off time <input type="time" value={sel.time ?? ""} onchange={(e) => set({ time: e.currentTarget.value })} /></label>
              <label class="field">Competition <input value={sel.competition ?? ""} onchange={(e) => set({ competition: e.currentTarget.value })} /></label>
              <label class="field">Venue <input value={sel.venue ?? ""} onchange={(e) => set({ venue: e.currentTarget.value })} /></label>
              <label class="field">Home / away
                <select value={sel.home ? "1" : "0"} onchange={(e) => set({ home: e.currentTarget.value === "1" })}><option value="1">Home</option><option value="0">Away</option></select>
              </label>
              {#if cfg.clock !== "none"}
                <label class="field">Period length (min) <input type="number" min="1" placeholder={String(cfg.periodMinutes)} value={sel.period_minutes ?? ""} onchange={(e) => set({ period_minutes: e.currentTarget.value ? Number(e.currentTarget.value) : null })} /></label>
              {/if}
            </div>
            <label class="field">Notes <textarea rows="3" value={sel.notes ?? ""} onchange={(e) => set({ notes: e.currentTarget.value })}></textarea></label>
            <div class="muted small">Changes save automatically. The score comes from the match log; use the Match recording tab to add or delete scores.</div>
          </div>
          <div class="actions">
            <span class="spacer"></span>
            <button class="danger" onclick={() => remove()}><Icon name="trash" /> Delete match</button>
          </div>
        {/if}
      </div>
    {:else}
      <div class="empty">Choose a match, or create one.</div>
    {/if}
  </div>
</div>

<style>
  .layout { display: grid; grid-template-columns: 260px 1fr; gap: 16px; align-items: start; }
  .score { font-variant-numeric: tabular-nums; font-weight: 700; }
  .mitem { display: flex; align-items: stretch; border-bottom: 1px solid var(--line); }
  .mitem:last-child { border-bottom: 0; }
  .mitem.on { box-shadow: inset 3px 0 0 var(--accent); background: var(--selected); }
  .mitem:hover { background: var(--hover); }
  .mitem .rowbtn { display: flex; flex-direction: column; align-items: stretch; gap: 2px; text-align: left; min-width: 0; background: none; border: 0; border-radius: 0; padding: 10px 12px; white-space: normal; }
  .mitem .del { opacity: 0; flex: none; }
  .mitem:hover .del, .mitem.on .del, .mitem .del:focus-visible { opacity: 1; }
  tr.gh td { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); background: var(--panel-2); }
  .bad { color: var(--bad-fg); }
</style>
