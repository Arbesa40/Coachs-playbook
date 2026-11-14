<script>
  // Small multi-series line chart. series = [{ name, color, points: [[label, value], ...] }]
  let { series = [], height = 160, max = null } = $props();
  const W = 520;
  const labels = $derived([...new Set(series.flatMap((s) => s.points.map((p) => p[0])))].sort());
  const top = $derived(max ?? Math.max(1, ...series.flatMap((s) => s.points.map((p) => p[1]))));
  const x = (l) => 30 + (labels.length < 2 ? (W - 40) / 2 : (labels.indexOf(l) / (labels.length - 1)) * (W - 40));
  const y = (v) => height - 20 - (v / top) * (height - 30);
  const palette = ["#3b82f6", "#22c55e", "#f59e0b", "#ef4444", "#a855f7", "#06b6d4", "#e11d48", "#84cc16"];
</script>

<svg viewBox="0 0 {W} {height}" style="width:100%;max-width:{W}px">
  {#each [0, 0.5, 1] as f}
    <line x1="30" x2={W - 10} y1={y(top * f)} y2={y(top * f)} style="stroke: var(--line)" />
    <text x="4" y={y(top * f) + 4} style="fill: var(--muted)" font-size="10">{Math.round(top * f * 10) / 10}</text>
  {/each}
  {#each labels as l, i}
    {#if labels.length < 8 || i % Math.ceil(labels.length / 8) === 0}
      <text x={x(l)} y={height - 4} style="fill: var(--muted)" font-size="10" text-anchor="middle">{String(l).slice(5)}</text>
    {/if}
  {/each}
  {#each series as s, i}
    <polyline fill="none" stroke={s.color || palette[i % palette.length]} stroke-width="2" points={s.points.map((p) => `${x(p[0])},${y(p[1])}`).join(" ")} />
    {#each s.points as p}<circle cx={x(p[0])} cy={y(p[1])} r="3" fill={s.color || palette[i % palette.length]}><title>{s.name}: {p[1]}</title></circle>{/each}
  {/each}
</svg>
<div class="row wrap small">
  {#each series as s, i}<span><i style="display:inline-block;width:10px;height:3px;background:{s.color || palette[i % palette.length]};vertical-align:middle"></i> {s.name}</span>{/each}
</div>
