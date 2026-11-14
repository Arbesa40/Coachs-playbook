<script>
  import { fmtTime } from "$shared/api.js";

  // One row per event code; bars span each tag's clip window.
  let { tags = [], duration = 0, time = 0, drawings = [], selected = $bindable([]), onseek, labelOf = () => "" } = $props();

  const rows = $derived.by(() => {
    const m = new Map();
    for (const t of tags) {
      if (!m.has(t.code)) m.set(t.code, { code: t.code, color: t.color, items: [] });
      m.get(t.code).items.push(t);
    }
    return [...m.values()];
  });
  const dur = $derived(duration || Math.max(60, ...tags.map((t) => t.end || 0)));
  const pct = (s) => Math.max(0, Math.min(100, (s / dur) * 100));

  function clickBar(e, t) {
    e.stopPropagation();
    if (e.shiftKey || e.ctrlKey) {
      selected = selected.includes(t.id) ? selected.filter((x) => x !== t.id) : [...selected, t.id];
    } else {
      selected = [t.id];
      onseek(t.start, t);
    }
  }
  function clickTrack(e) {
    const r = e.currentTarget.getBoundingClientRect();
    onseek(((e.clientX - r.left) / r.width) * dur);
  }
  const ticks = $derived(Array.from({ length: 11 }, (_, i) => (dur * i) / 10));
</script>

<div class="tl">
  <div class="row head">
    <div class="label"></div>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="track ruler" onclick={clickTrack}>
      {#each ticks as tk}<span style="left:{pct(tk)}%">{fmtTime(tk)}</span>{/each}
      {#each drawings as d}<i class="dmark" style="left:{pct(d.t)}%" title="Freeze-frame drawing"></i>{/each}
      <div class="head-line" style="left:{pct(time)}%"></div>
    </div>
  </div>
  {#each rows as r (r.code)}
    <div class="row line">
      <div class="label" title={r.code}><i style="background:{r.color}"></i>{r.code} <span class="muted">{r.items.length}</span></div>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="track" onclick={clickTrack}>
        {#each r.items as t (t.id)}
          <div
            class="bar"
            class:sel={selected.includes(t.id)}
            style="left:{pct(t.start)}%;width:max(3px,{pct(t.end) - pct(t.start)}%);background:{t.color}"
            title="{t.code} {fmtTime(t.t)} {labelOf(t)}"
            onclick={(e) => clickBar(e, t)}
          ></div>
        {/each}
        <div class="head-line" style="left:{pct(time)}%"></div>
      </div>
    </div>
  {/each}
  {#if rows.length === 0}<div class="empty">No tags yet. Press a tag button or its key while the video plays.</div>{/if}
</div>

<style>
  .tl { display: flex; flex-direction: column; gap: 2px; user-select: none; }
  .line, .head { gap: 0; }
  .label { width: 150px; flex: none; font-size: 12px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; padding-right: 6px; }
  .label i { display: inline-block; width: 8px; height: 8px; border-radius: 2px; margin-right: 6px; }
  .track { position: relative; flex: 1; height: 18px; background: var(--input); border-radius: 3px; cursor: pointer; }
  .ruler { height: 18px; background: none; font-size: 10px; color: var(--muted); }
  .ruler span { position: absolute; transform: translateX(-50%); top: 2px; }
  .bar { position: absolute; top: 3px; bottom: 3px; border-radius: 2px; opacity: .85; }
  .bar.sel { outline: 2px solid #fff; opacity: 1; }
  .head-line { position: absolute; top: 0; bottom: 0; width: 2px; margin-left: -1px; background: #fff; pointer-events: none; z-index: 1; }
  :global([data-theme="light"]) .head-line { background: #0f172a; width: 2px; box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.7); }
  :global([data-theme="light"]) .bar.sel { outline-color: #0f172a; }
  .dmark { position: absolute; bottom: 0; width: 7px; height: 7px; background: #facc15; transform: translateX(-50%) rotate(45deg); }
</style>
