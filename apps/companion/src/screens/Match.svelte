<script>
  import Icon from "$shared/Icon.svelte";
  import MatchConsole from "$shared/MatchConsole.svelte";
  import { db, playerLabel, sortPlayers, today, ask } from "$shared/api.js";
  import { teamConfig, groupPlayers } from "$shared/sports.js";
  import { deleteMatch as removeMatch } from "$shared/matchCleanup.js";
  import { ui, attempt, toast, saved, persist } from "../state.svelte.js";

  let matches = $state([]);
  let players = $state([]);
  let events = $state([]);
  let team = $state(null);
  let matchId = $state(saved("matchId", null));
  let mode = $state("live");
  let opponent = $state("");

  const cfg = $derived(teamConfig(team));
  const match = $derived(matches.find((m) => m.id === matchId) ?? null);
  const groups = $derived(groupPlayers(cfg, sortPlayers(players)));

  async function load() {
    [matches, players] = await Promise.all([db.list("match"), db.list("player")]);
    team = await db.get("team", ui.info.team_id);
  }
  load();

  // Abandoned matches can be removed in the background (see matchCleanup.js).
  $effect(() => {
    const onChange = (e) => {
      if (matchId && e.detail?.includes(matchId)) matchId = null;
      load();
    };
    window.addEventListener("cp:matches-changed", onChange);
    return () => window.removeEventListener("cp:matches-changed", onChange);
  });

  $effect(() => {
    persist("matchId", matchId);
    events = [];
    const id = matchId;
    if (id) db.by("match_event", "match_id", id).then((e) => id === matchId && (events = e));
  });

  async function newMatch() {
    if (!opponent.trim()) return toast("Enter the opponent's name", "error");
    const m = await attempt(() => db.put("match", { date: today(), opponent: opponent.trim(), home: true, goals_for: 0, goals_against: 0, starters: [], bench: [] }));
    opponent = "";
    await load();
    if (m) {
      matchId = m.id;
      mode = "lineup";
    }
  }
  async function setMatch(fields) {
    Object.assign(match, fields);
    await db.put("match", fields, match.id);
  }
  function cycle(pid) {
    const s = (match.starters ?? []).filter((x) => x !== pid);
    const b = (match.bench ?? []).filter((x) => x !== pid);
    if (match.starters?.includes(pid)) b.push(pid);
    else if (!match.bench?.includes(pid)) s.push(pid);
    setMatch({ starters: s, bench: b });
  }
  async function deleteMatch() {
    const n = events.filter((e) => !e.auto).length;
    if (!await ask(`Delete this match against ${match.opponent}${n ? ` and its ${n} recorded events` : ""}? This also removes it from the PC at the next sync.`)) return;
    const ok = await attempt(() => removeMatch(db, match.id).then(() => true), "Match deleted");
    if (!ok) return;
    matchId = null;
    load();
  }
  async function discardMatch() {
    if (!await ask(`Discard the running match against ${match.opponent}?\n\nThe score, the clock and every event recorded so far are thrown away, here and on the PC at the next sync.`)) return;
    const ok = await attempt(() => removeMatch(db, match.id).then(() => true), "Match discarded");
    if (!ok) return;
    matchId = null;
    load();
  }
</script>

{#if !match}
  <section class="col">
    <div class="card col">
      <h3>New match</h3>
      <div class="row">
        <input class="grow" placeholder="Opponent" bind:value={opponent} onkeydown={(e) => e.key === "Enter" && newMatch()} />
        <button class="primary" onclick={newMatch}><Icon name="plus" /> Create</button>
      </div>
      <div class="muted small">{cfg.name} · {cfg.formatLabel}</div>
    </div>
    <h3 class="sec">Matches</h3>
    <div class="list card flush">
      {#each [...matches].sort((a, b) => (b.date || "").localeCompare(a.date || "")) as m (m.id)}
        <button class="item rowbtn row mrow" onclick={() => { matchId = m.id; mode = m.starters?.length ? "live" : "lineup"; }}>
          <span class="grow"><b>{m.home === false ? "@" : "vs"} {m.opponent}</b><span class="muted small">{m.date}{m.live?.phase === "live" ? " · live" : m.live?.phase === "done" ? " · full time" : ""}</span></span>
          <span class="sc">{m.goals_for ?? 0}–{m.goals_against ?? 0}</span>
        </button>
      {:else}<div class="empty">No matches yet.</div>{/each}
    </div>
  </section>
{:else}
  <div class="row bar">
    <button class="ghost small" onclick={() => (matchId = null)}><Icon name="chevronLeft" size={16} /> Matches</button>
    <span class="grow"></span>
    <div class="seg">
      <button class:on={mode === "lineup"} onclick={() => (mode = "lineup")}>Line-up</button>
      <button class:on={mode === "live"} onclick={() => (mode = "live")}>Live</button>
    </div>
  </div>

  {#if mode === "lineup"}
    <section class="col">
      <div class="row"><b class="grow">vs {match.opponent}</b><span class="muted small">{match.starters?.length ?? 0}/{cfg.onField} starting · {match.bench?.length ?? 0} bench</span></div>
      <p class="muted small" style="margin:0">Tap once to start, twice for the bench, again to leave out.</p>
      {#each groups as [g, list] (g)}
        <h3 class="sec">{g}</h3>
        <div class="chipgrid">
          {#each list as p (p.id)}
            <button class="chip" class:on={match.starters?.includes(p.id)} class:bench={match.bench?.includes(p.id)} onclick={() => cycle(p.id)}>{playerLabel(p)}</button>
          {/each}
        </div>
      {/each}
      <button class="primary big" onclick={() => (mode = "live")}>Done: go to live</button>
    </section>
  {:else}
    {#key match.id}
      <MatchConsole {match} {cfg} {team} {players} bind:events onmatch={setMatch} ondiscard={discardMatch} phone />
    {/key}
  {/if}

  <section class="danger-zone">
    <button class="danger big" onclick={deleteMatch}><Icon name="trash" /> Delete this match</button>
  </section>
{/if}

<style>
  .bar { margin-bottom: 10px; }
  .bar .seg button { padding: 6px 14px; min-height: 38px; }
  .mrow { display: flex; align-items: center; gap: 8px; padding: 12px !important; }
  .mrow .grow { display: flex; flex-direction: column; text-align: left; }
  .sc { font-size: 18px; font-weight: 700; font-variant-numeric: tabular-nums; }
  .danger-zone { margin-top: 28px; padding-top: 16px; border-top: 1px solid var(--line); }
  .danger-zone button { width: 100%; }
</style>
