<script>
  import Icon from "$shared/Icon.svelte";
  import { db, playerLabel, sortPlayers, today } from "$shared/api.js";
  import { teamConfig, groupPlayers } from "$shared/sports.js";
  import { ui, attempt } from "../state.svelte.js";

  // Injury/availability updates and player assessments with rating templates.
  let players = $state([]);
  let injuries = $state([]);
  let templates = $state([]);
  let selId = $state(null);
  let tplId = $state(null);
  let scores = $state({});
  let note = $state("");
  let injType = $state("");

  const sel = $derived(players.find((p) => p.id === selId) ?? null);
  const tpl = $derived(templates.find((t) => t.id === tplId) ?? templates[0] ?? null);

  let team = $state(null);
  const cfg = $derived(teamConfig(team));
  async function load() {
    [players, injuries, templates] = await Promise.all(["player", "injury", "assessment_template"].map((k) => db.list(k)));
    team = await db.get("team", ui.info.team_id);
  }
  load();

  const status = (pid) =>
    injuries.filter((i) => i.player_id === pid).sort((a, b) => (b.date || "").localeCompare(a.date || ""))[0]?.status ?? "available";

  async function setStatus(st) {
    const r = await attempt(() => db.put("injury", { player_id: sel.id, date: today(), status: st, type: injType || (st === "available" ? "Cleared" : ""), notes: "" }), "Saved");
    if (r) injuries = [...injuries, r];
    injType = "";
  }
  async function saveAssessment() {
    await attempt(() => db.put("assessment", { player_id: sel.id, template_id: tpl.id, date: today(), scores: $state.snapshot(scores), notes: note }), "Assessment saved");
    scores = {};
    note = "";
  }
</script>

{#if !sel}
  {#each groupPlayers(cfg, sortPlayers(players)) as [g, list] (g)}
    <h3 class="sec">{g}</h3>
    <div class="list card flush">
      {#each list as p (p.id)}
        {@const st = status(p.id)}
        <button class="item rowbtn row" onclick={() => (selId = p.id)}>
          <span class="grow" style="text-align:left">{playerLabel(p)} <span class="muted small">{p.position ?? ""}</span></span>
          <span class="badge {st === 'out' ? 'bad' : st === 'limited' ? 'warn' : 'good'}">{st}</span>
        </button>
      {/each}
    </div>
  {:else}<div class="empty">Add players on the PC, then sync.</div>{/each}
{:else}
  <div class="row" style="margin-bottom:8px">
    <button class="ghost" style="padding:6px;min-width:44px" aria-label="Back" onclick={() => (selId = null)}><Icon name="chevronLeft" size={22} /></button>
    <div class="grow col" style="gap:0"><b>{playerLabel(sel)}</b><span class="muted small">{sel.position ?? ""}</span></div>
    <span class="badge {status(sel.id) === 'out' ? 'bad' : status(sel.id) === 'limited' ? 'warn' : 'good'}">{status(sel.id)}</span>
  </div>
  <section class="card col">
    <h3>Availability</h3>
    <input placeholder="Injury or reason (optional)" bind:value={injType} />
    <div class="grid3">
      <button class:on={status(sel.id) === "available"} onclick={() => setStatus("available")}>Available</button>
      <button class:on={status(sel.id) === "limited"} onclick={() => setStatus("limited")}>Limited</button>
      <button class:on={status(sel.id) === "out"} onclick={() => setStatus("out")}>Out</button>
    </div>
  </section>
  <section class="card col">
    <h3>Assessment</h3>
    {#if tpl}
      <select value={tpl.id} onchange={(e) => (tplId = e.currentTarget.value)}>{#each templates as t}<option value={t.id}>{t.name}</option>{/each}</select>
      {#each tpl.criteria as c}
        <div class="col" style="gap:4px">
          <span class="small">{c}</span>
          <div class="row" style="gap:4px">
            {#each Array.from({ length: tpl.scale ?? 5 }, (_, i) => i + 1) as n}
              <button class="grow small" class:on={scores[c] === n} onclick={() => (scores[c] = n)}>{n}</button>
            {/each}
          </div>
        </div>
      {/each}
      <textarea placeholder="Notes" bind:value={note}></textarea>
      <button class="primary" onclick={saveAssessment} disabled={!Object.keys(scores).length}>Save</button>
    {:else}
      <div class="muted">Rating templates are created on the PC, under Players, Assessments.</div>
    {/if}
  </section>
{/if}
