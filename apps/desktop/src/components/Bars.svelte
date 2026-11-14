<script>
  // Simple horizontal bar chart: data = [[label, value, color?], ...]
  let { data = [], unit = "" } = $props();
  const max = $derived(Math.max(1, ...data.map((d) => d[1])));
</script>

<div class="bars">
  {#each data as [label, value, color]}
    <div class="lbl" title={label}>{label}</div>
    <div class="track"><div class="fill" style="width:{(value / max) * 100}%;background:{color || 'var(--accent)'}"></div></div>
    <div class="val">{Math.round(value * 10) / 10}{unit}</div>
  {/each}
</div>

<style>
  .bars { display: grid; grid-template-columns: minmax(80px, 160px) 1fr 48px; gap: 4px 8px; align-items: center; font-size: 12px; }
  .lbl { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .track { height: 12px; background: var(--input); border-radius: 3px; }
  .fill { height: 100%; border-radius: 3px; }
  .val { text-align: right; color: var(--muted); }
</style>
