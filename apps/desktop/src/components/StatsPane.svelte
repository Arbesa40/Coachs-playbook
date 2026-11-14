<script>
  import { invoke } from "$shared/api.js";
  import PitchDiagram from "$shared/PitchDiagram.svelte";
  import Bars from "./Bars.svelte";

  // Stats generated from tags, for this video or the whole match.
  let { current, tags, colors, sport = "football" } = $props();
  let scope = $state("video");
  let s = $state(null);
  let codeFilter = $state("");

  $effect(() => {
    // Recompute when tags change.
    tags.length;
    const args = scope === "match" && current?.match_id ? { matchId: current.match_id } : { videoId: current?.id ?? null };
    if (!current) return;
    invoke("tag_stats", args).then((r) => (s = r));
  });

  const located = $derived(tags.filter((t) => t.x != null && (!codeFilter || t.code === codeFilter)));
</script>

<div class="grid3" style="align-items:start">
  <div class="col">
    <div class="row">
      <h3 class="grow">Events</h3>
      <select bind:value={scope}>
        <option value="video">This video</option>
        <option value="match" disabled={!current?.match_id}>Whole match</option>
      </select>
    </div>
    {#if s}
      <Bars data={Object.entries(s.by_code).map(([k, v]) => [k, v, colors[k]])} />
      <div class="muted small">{s.total} tags</div>
    {/if}
  </div>
  <div class="col">
    <h3>By player</h3>
    {#if s}
      {#each Object.entries(s.by_player) as [p, codes]}
        <div class="small"><b>{p}</b> <span class="muted">{Object.entries(codes).map(([c, n]) => `${c} ${n}`).join(" · ")}</span></div>
      {:else}
        <div class="muted small">Assign players to tags to see per-player stats.</div>
      {/each}
      <h3 style="margin-top:12px">By period</h3>
      <table class="small">
        <tbody>
          {#each Object.entries(s.by_period) as [p, codes]}
            <tr><td>{p}</td><td class="muted">{Object.entries(codes).map(([c, n]) => `${c} ${n}`).join(" · ")}</td></tr>
          {/each}
        </tbody>
      </table>
      {#if Object.keys(s.outcomes).length}
        <h3 style="margin-top:12px">Outcomes</h3>
        {#each Object.entries(s.outcomes) as [code, o]}
          {@const total = Object.values(o).reduce((a, b) => a + b, 0)}
          <div class="small"><b>{code}</b> <span class="muted">{Object.entries(o).map(([k, n]) => `${k} ${n} (${Math.round((n / total) * 100)}%)`).join(" · ")}</span></div>
        {/each}
      {/if}
    {/if}
  </div>
  <div class="col">
    <div class="row">
      <h3 class="grow">Event map</h3>
      <select bind:value={codeFilter}>
        <option value="">All events</option>
        {#each Object.keys(colors) as c}<option value={c}>{c}</option>{/each}
      </select>
    </div>
    <PitchDiagram {sport} points={located.map((t) => ({ x: t.x, y: t.y, color: t.color, label: t.code }))} height={180} />
    <div class="muted small">Click the pitch after tagging to record where an event happened.</div>
  </div>
</div>
