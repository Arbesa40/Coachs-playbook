<script>
  import Icon from "$shared/Icon.svelte";
  import { db, byKey, fmtTime } from "$shared/api.js";

  let playlists = $state([]);
  let clips = $state([]);
  let openPl = $state(null);
  Promise.all(["playlist", "clip"].map((k) => db.list(k))).then((r) => ([playlists, clips] = r));
  const cById = $derived(byKey(clips, "id"));
</script>

<div class="list card flush">
  {#each playlists as pl (pl.id)}
    <button class="item rowbtn row" style="padding:12px" onclick={() => (openPl = openPl === pl.id ? null : pl.id)} aria-expanded={openPl === pl.id}>
      <Icon name="film" size={20} />
      <span class="grow" style="text-align:left"><b>{pl.name}</b> <span class="muted small">{pl.items?.length ?? 0} clips</span></span>
      <Icon name={openPl === pl.id ? "chevronUp" : "chevronDown"} size={18} />
    </button>
    {#if openPl === pl.id}
      <div class="clips">
        {#each pl.items ?? [] as id, i}
          {@const c = cById[id]}
          {#if c}<div class="row small"><span class="muted">{i + 1}</span><span class="grow">{c.name}</span><span class="muted">{fmtTime(c.start)}</span></div>{/if}
        {/each}
      </div>
    {/if}
  {:else}
    <div class="empty">Playlists made on the PC show up here after syncing. Clips play on the PC.</div>
  {/each}
</div>

<style>
  .clips { padding: 4px 14px 12px 46px; display: flex; flex-direction: column; gap: 6px; border-bottom: 1px solid var(--line); }
</style>
