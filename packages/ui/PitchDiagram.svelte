<script>
  import { surface } from "./surfaces.js";

  let { sport = "football", points = [], onpick = null, height = 200 } = $props();
  const s = $derived(surface(sport));
  const u = $derived(Math.max(s.w, s.h) / 48); // marker unit
  let svg;

  function pick(e) {
    if (!onpick) return;
    const p = svg.createSVGPoint();
    p.x = e.clientX;
    p.y = e.clientY;
    const r = p.matrixTransform(svg.getScreenCTM().inverse());
    const x = Math.min(1, Math.max(0, r.x / s.w));
    const y = Math.min(1, Math.max(0, r.y / s.h));
    onpick(Math.round(x * 1000) / 1000, Math.round(y * 1000) / 1000);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<svg
  bind:this={svg}
  viewBox="{-s.pad} {-s.pad} {s.w + 2 * s.pad} {s.h + 2 * s.pad}"
  style="height:{height}px;width:auto;max-width:100%;cursor:{onpick ? 'crosshair' : 'default'};display:block"
  onclick={pick}
>
  <rect x={-s.pad} y={-s.pad} width={s.w + 2 * s.pad} height={s.h + 2 * s.pad} fill={s.bg} rx={u * 0.6} />
  <g fill="none" stroke={s.line} stroke-opacity=".8" stroke-width={u * 0.16} color={s.line}>
    {@html s.svg}
  </g>
  {#each points as p}
    <circle cx={p.x * s.w} cy={p.y * s.h} r={u * 0.75} fill={p.color || "#fbbf24"} stroke="#000" stroke-width={u * 0.12}>
      {#if p.label}<title>{p.label}</title>{/if}
    </circle>
  {/each}
</svg>
