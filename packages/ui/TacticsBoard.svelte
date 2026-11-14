<script>
  import { askText } from "./api.js";
  import { surface } from "./surfaces.js";
  import { LINE_KINDS } from "./sports.js";
  import { normalize, uid, positionsAt, positionsAtT, frameCount, secondsToT, timeline, createHistory, DEFAULT_STYLE } from "./board.js";
  import Icon from "./Icon.svelte";

  let {
    board = $bindable(),
    sport = "football",
    style = DEFAULT_STYLE,
    editable = false,
    tool = $bindable("select"), // "select" | item type | "line:<kind>"
    frame = $bindable(0),
    t = $bindable(null), // fractional step while playing (null = show `frame`)
    playing = $bindable(false),
    selected = $bindable(null), // { kind: "item"|"line", id }
    newColor = null, // colour for new items/lines (null = style default)
    height = 380,
    controls = true,
    onchange = null,
  } = $props();

  const st = $derived({ ...DEFAULT_STYLE, ...(style ?? {}) });
  const s = $derived(surface(sport));
  const u = $derived((Math.max(s.w, s.h) / 48) * (st.size || 1));
  const b = $derived(normalize(board));
  const frames = $derived(frameCount(b));
  const shownT = $derived(t ?? frame);
  const pos = $derived(t != null ? positionsAtT(b, t) : positionsAt(b, frame));
  const lineStep = $derived(Math.min(Math.floor(shownT + 1e-6), frames - 1));
  const visibleLines = $derived(b.lines.filter((l) => (l.step ?? 0) === lineStep));

  const history = createHistory();
  let canUndo = $state(false);
  let canRedo = $state(false);
  let svg;
  let drag = null; // { id, ox, oy, moved }
  let lineStart = $state(null); // [x, y] of a line being drawn
  let linePreview = $state(null); // [x, y] under the pointer while drawing
  let clip = null; // copied item/line

  const X = (n) => n * s.w;
  const Y = (n) => n * s.h;

  function commit(next, record = true) {
    if (record) {
      history.push(b);
      canUndo = true;
      canRedo = false;
    }
    board = next;
    onchange?.(next);
  }
  const clone = () => JSON.parse(JSON.stringify(b));

  export function undo() {
    const prev = history.undo(b);
    if (prev) {
      board = prev;
      onchange?.(prev);
    }
    canUndo = history.canUndo;
    canRedo = history.canRedo;
    selected = null;
  }
  export function redo() {
    const next = history.redo(b);
    if (next) {
      board = next;
      onchange?.(next);
    }
    canUndo = history.canUndo;
    canRedo = history.canRedo;
  }
  export function deleteSelected() {
    if (!selected) return;
    const nb = clone();
    if (selected.kind === "item") {
      nb.items = nb.items.filter((i) => i.id !== selected.id);
      for (const st of nb.steps) delete st.pos[selected.id];
    } else nb.lines = nb.lines.filter((l) => l.id !== selected.id);
    selected = null;
    commit(nb);
  }
  export function clearAll() {
    commit({ ...clone(), items: [], lines: [], steps: [] });
    frame = 0;
    selected = null;
  }
  export function addStep() {
    const nb = clone();
    nb.steps.splice(frame, 0, { dur: 1.5, pos: {} });
    commit(nb);
    frame = frame + 1;
  }
  export function removeStep() {
    if (frame === 0 || !b.steps.length) return;
    const nb = clone();
    nb.steps.splice(frame - 1, 1);
    nb.lines = nb.lines.filter((l) => l.step !== frame).map((l) => ((l.step ?? 0) > frame ? { ...l, step: l.step - 1 } : l));
    commit(nb);
    frame = frame - 1;
  }
  export function setStepDuration(sec) {
    if (frame === 0) return;
    const nb = clone();
    nb.steps[frame - 1].dur = Math.max(0.3, Number(sec) || 1.5);
    commit(nb);
  }
  export function updateSelected(fields) {
    if (!selected) return;
    const nb = clone();
    const list = selected.kind === "item" ? nb.items : nb.lines;
    const x = list.find((i) => i.id === selected.id);
    if (x) Object.assign(x, fields);
    commit(nb);
  }
  export const history_ = { get canUndo() { return canUndo; }, get canRedo() { return canRedo; } };

  // ---------- playback ----------
  let raf;
  export function play() {
    if (frames < 2) return;
    const tl = timeline(b);
    let start = performance.now() - (frame >= frames - 1 ? 0 : tl.starts[frame] * 1000);
    playing = true;
    const tick = (now) => {
      const sec = (now - start) / 1000;
      if (sec >= tl.total) {
        t = null;
        frame = frames - 1;
        playing = false;
        return;
      }
      t = secondsToT(b, sec);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
  }
  export function stop() {
    cancelAnimationFrame(raf);
    if (t != null) frame = Math.round(t);
    t = null;
    playing = false;
  }
  $effect(() => () => cancelAnimationFrame(raf));

  // ---------- pointer ----------
  const pt = (e) => {
    const p = svg.createSVGPoint();
    p.x = e.clientX;
    p.y = e.clientY;
    const r = p.matrixTransform(svg.getScreenCTM().inverse());
    const c = (v) => Math.round(Math.min(1.06, Math.max(-0.06, v)) * 1000) / 1000;
    return [c(r.x / s.w), c(r.y / s.h)];
  };
  const lineKindOf = (tl) => (tl?.startsWith("line:") ? tl.slice(5) : null);

  function down(e) {
    if (!editable || e.button === 2) return;
    if (playing) stop();
    svg.parentElement.focus({ preventScroll: true });
    const [x, y] = pt(e);
    const kind = lineKindOf(tool);
    const hitItem = e.target.closest("[data-item]")?.dataset.item;
    const hitLine = e.target.closest("[data-line]")?.dataset.line;

    if (kind) {
      if (!lineStart) {
        lineStart = [x, y];
        linePreview = [x, y];
        svg.setPointerCapture(e.pointerId);
      } else finishLine(x, y);
      return;
    }
    if (tool === "select" || hitItem) {
      if (hitItem) {
        selected = { kind: "item", id: hitItem };
        const p = pos[hitItem];
        drag = { id: hitItem, ox: x - p.x, oy: y - p.y, moved: false, before: clone() };
        svg.setPointerCapture(e.pointerId);
      } else if (hitLine) selected = { kind: "line", id: hitLine };
      else selected = null;
      return;
    }
    // Place a new item.
    const nb = clone();
    const sameType = nb.items.filter((i) => i.type === tool).length;
    const isPlayer = tool === "player" || tool === "opponent";
    const item = { id: uid(), type: tool, x, y, label: isPlayer && st.numbers ? String(sameType + 1) : "" };
    if (newColor && tool !== "ball") item.color = newColor;
    nb.items.push(item);
    // An item added at a later step starts there in every earlier step too.
    selected = { kind: "item", id: item.id };
    commit(nb);
  }

  function move(e) {
    if (lineStart) {
      linePreview = pt(e);
      return;
    }
    if (!drag) return;
    const [x, y] = pt(e);
    const nx = Math.round((x - drag.ox) * 1000) / 1000, ny = Math.round((y - drag.oy) * 1000) / 1000;
    const nb = clone();
    if (frame === 0 || t != null) {
      const it = nb.items.find((i) => i.id === drag.id);
      if (it) Object.assign(it, { x: nx, y: ny });
    } else nb.steps[frame - 1].pos[drag.id] = { x: nx, y: ny };
    drag.moved = true;
    board = nb; // history is recorded once on release
  }

  function up(e) {
    if (lineStart) {
      const [x, y] = pt(e);
      const dist = Math.hypot((x - lineStart[0]) * s.w, (y - lineStart[1]) * s.h);
      if (dist > u * 0.8) finishLine(x, y); // dragged: finish now; otherwise wait for a second click
      return;
    }
    if (drag?.moved) {
      history.push(drag.before);
      canUndo = true;
      canRedo = false;
      onchange?.(board);
    }
    drag = null;
  }

  function finishLine(x, y) {
    const kind = lineKindOf(tool);
    const [x1, y1] = lineStart;
    lineStart = null;
    linePreview = null;
    if (Math.hypot((x - x1) * s.w, (y - y1) * s.h) < u * 0.5) return;
    const nb = clone();
    const line = { id: uid(), kind, x1, y1, x2: x, y2: y, step: frame };
    if (newColor) line.color = newColor;
    nb.lines.push(line);
    selected = { kind: "line", id: line.id };
    commit(nb);
  }

  function context(e, kind, id) {
    if (!editable) return;
    e.preventDefault();
    selected = { kind, id };
    deleteSelected();
  }

  async function relabel(id) {
    if (!editable) return;
    const it = b.items.find((i) => i.id === id);
    const label = await askText("Label (number or initials, up to 4 characters)", it?.label ?? "", { maxlength: 4 });
    if (label != null) {
      selected = { kind: "item", id };
      updateSelected({ label: label.slice(0, 4) });
    }
  }

  function key(e) {
    if (!editable) return;
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === "z" && !e.shiftKey) {
      e.preventDefault();
      undo();
    } else if (mod && (e.key.toLowerCase() === "y" || (e.key.toLowerCase() === "z" && e.shiftKey))) {
      e.preventDefault();
      redo();
    } else if ((e.key === "Delete" || e.key === "Backspace") && selected) {
      e.preventDefault();
      deleteSelected();
    } else if (mod && e.key.toLowerCase() === "c" && selected) {
      const list = selected.kind === "item" ? b.items : b.lines;
      clip = { kind: selected.kind, data: JSON.parse(JSON.stringify(list.find((x) => x.id === selected.id))) };
    } else if (mod && (e.key.toLowerCase() === "v" || e.key.toLowerCase() === "d")) {
      e.preventDefault();
      const src = e.key.toLowerCase() === "d" && selected
        ? { kind: selected.kind, data: JSON.parse(JSON.stringify((selected.kind === "item" ? b.items : b.lines).find((x) => x.id === selected.id))) }
        : clip;
      if (!src?.data) return;
      const nb = clone();
      const copy = { ...src.data, id: uid() };
      if (src.kind === "item") {
        const p = pos[src.data.id] ?? copy;
        Object.assign(copy, { x: p.x + 0.03, y: p.y + 0.03 });
        nb.items.push(copy);
      } else {
        Object.assign(copy, { x1: copy.x1 + 0.03, y1: copy.y1 + 0.03, x2: copy.x2 + 0.03, y2: copy.y2 + 0.03, step: frame });
        nb.lines.push(copy);
      }
      selected = { kind: src.kind, id: copy.id };
      commit(nb);
    } else if (e.key === "Escape") {
      lineStart = null;
      linePreview = null;
      selected = null;
    }
  }

  // ---------- drawing helpers ----------
  function wavy(x1, y1, x2, y2) {
    const L = Math.hypot(x2 - x1, y2 - y1);
    const n = Math.max(4, Math.round(L / (u * 0.7)));
    const ux = (x2 - x1) / L, uy = (y2 - y1) / L;
    const amp = u * 0.28;
    let d = `M${x1} ${y1}`;
    for (let i = 1; i <= n; i++) {
      const k = i / n;
      const off = i === n ? 0 : Math.sin(k * n * Math.PI) * amp;
      d += ` L${x1 + (x2 - x1) * k - uy * off} ${y1 + (y2 - y1) * k + ux * off}`;
    }
    return d;
  }
  function linePath(l) {
    const x1 = X(l.x1), y1 = Y(l.y1), x2 = X(l.x2), y2 = Y(l.y2);
    const def = LINE_KINDS[l.kind] ?? LINE_KINDS.run;
    if (def.wave) return wavy(x1, y1, x2, y2);
    if (def.curve) {
      const mx = (x1 + x2) / 2, my = (y1 + y2) / 2;
      const L = Math.hypot(x2 - x1, y2 - y1);
      return `M${x1} ${y1} Q${mx - ((y2 - y1) / L) * L * 0.25} ${my + ((x2 - x1) / L) * L * 0.25} ${x2} ${y2}`;
    }
    return `M${x1} ${y1} L${x2} ${y2}`;
  }
  const lw = (l) => u * 0.2 * (l.width ?? st.lineWidth ?? 1) * ((LINE_KINDS[l.kind]?.thick ?? 1)) * (LINE_KINDS[l.kind]?.thin ? 0.6 : 1);
  const dashOf = (l) => (LINE_KINDS[l.kind]?.dash ?? "").split(" ").filter(Boolean).map((n) => Number(n) * u).join(" ");
  const colorOf = (it) => it.color ?? (it.type === "opponent" ? st.away : it.type === "player" ? st.home : st.equipColor);
  const bar = (l) => {
    const x1 = X(l.x1), y1 = Y(l.y1), x2 = X(l.x2), y2 = Y(l.y2);
    const L = Math.hypot(x2 - x1, y2 - y1) || 1;
    const px = -(y2 - y1) / L, py = (x2 - x1) / L;
    return `M${x2 + px * u * 0.7} ${y2 + py * u * 0.7} L${x2 - px * u * 0.7} ${y2 - py * u * 0.7}`;
  };
  const ballSport = $derived(sport);
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="tb" tabindex={editable ? 0 : -1} onkeydown={key} role="application" aria-label="Tactics board">
  <svg
    bind:this={svg}
    viewBox="{-s.pad} {-s.pad} {s.w + 2 * s.pad} {s.h + 2 * s.pad}"
    style="height:{height}px"
    class:edit={editable}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
  >
    <defs>
      <marker id="tb-arrow" viewBox="0 0 10 10" refX="7" refY="5" markerWidth="4" markerHeight="4" orient="auto-start-reverse">
        <path d="M0 0L10 5L0 10z" fill="context-stroke" />
      </marker>
    </defs>
    <rect x={-s.pad} y={-s.pad} width={s.w + 2 * s.pad} height={s.h + 2 * s.pad} fill={s.bg} />
    <g fill="none" stroke={s.line} stroke-opacity=".75" stroke-width={u * 0.14} color={s.line}>
      {@html s.svg}
    </g>

    {#each visibleLines as l (l.id)}
      {@const sel = selected?.kind === "line" && selected.id === l.id}
      <g data-line={l.id} oncontextmenu={(e) => context(e, "line", l.id)} class="ln">
        <path d={linePath(l)} stroke="transparent" stroke-width={u * 1.2} fill="none" />
        <path
          d={linePath(l)}
          fill="none"
          stroke={l.color ?? st.lineColor}
          stroke-width={lw(l)}
          stroke-dasharray={dashOf(l)}
          stroke-linecap="round"
          marker-end={LINE_KINDS[l.kind]?.head === "arrow" ? "url(#tb-arrow)" : null}
          class:sel
        />
        {#if LINE_KINDS[l.kind]?.head === "bar"}<path d={bar(l)} stroke={l.color ?? st.lineColor} stroke-width={lw(l) * 1.3} />{/if}
      </g>
    {/each}
    {#if lineStart && linePreview}
      <line x1={X(lineStart[0])} y1={Y(lineStart[1])} x2={X(linePreview[0])} y2={Y(linePreview[1])} stroke="#facc15" stroke-width={u * 0.15} stroke-dasharray="{u * 0.4} {u * 0.3}" />
      <circle cx={X(lineStart[0])} cy={Y(lineStart[1])} r={u * 0.3} fill="#facc15" />
    {/if}

    {#each b.items as it (it.id)}
      {@const p = pos[it.id] ?? it}
      {@const sel = selected?.kind === "item" && selected.id === it.id}
      {@const r = u * (it.size ?? 1)}
      {@const c = colorOf(it)}
      <g
        data-item={it.id}
        transform="translate({X(p.x)} {Y(p.y)})"
        class="it"
        class:sel
        oncontextmenu={(e) => context(e, "item", it.id)}
        ondblclick={() => relabel(it.id)}
        role="img"
        aria-label={it.type}
      >
        {#if sel}<circle r={r * 1.45} fill="none" stroke="#facc15" stroke-width={u * 0.12} stroke-dasharray="{u * 0.3} {u * 0.2}" />{/if}
        {#if it.type === "player" || it.type === "opponent"}
          {#if st.shape === "shirt"}
            <path d="M{-r} {-r * 0.55} L{-r * 0.45} {-r} L{-r * 0.2} {-r * 0.75} L{r * 0.2} {-r * 0.75} L{r * 0.45} {-r} L{r} {-r * 0.55} L{r * 0.72} {-r * 0.1} L{r * 0.55} {-r * 0.25} L{r * 0.55} {r} L{-r * 0.55} {r} L{-r * 0.55} {-r * 0.25} L{-r * 0.72} {-r * 0.1}Z" fill={c} stroke="#fff" stroke-width={u * 0.1} />
          {:else if st.shape === "triangle"}
            <path d="M0 {-r * 1.1} L{r} {r * 0.8} L{-r} {r * 0.8}Z" fill={c} stroke="#fff" stroke-width={u * 0.1} />
          {:else if st.shape === "square"}
            <rect x={-r * 0.9} y={-r * 0.9} width={r * 1.8} height={r * 1.8} rx={r * 0.25} fill={c} stroke="#fff" stroke-width={u * 0.1} />
          {:else}
            <circle r={r} fill={c} stroke="#fff" stroke-width={u * 0.12} />
          {/if}
          {#if it.label}<text y={r * 0.38} font-size={r * (it.label.length > 2 ? 0.75 : 1.05)} text-anchor="middle" fill="#fff" font-weight="700">{it.label}</text>{/if}
        {:else if it.type === "ball"}
          {#if ballSport === "rugby"}
            <ellipse rx={r * 0.7} ry={r * 0.45} fill="#f8f5ec" stroke="#5b4636" stroke-width={u * 0.08} />
            <path d="M{-r * 0.35} 0 L{r * 0.35} 0" stroke="#5b4636" stroke-width={u * 0.06} />
          {:else if ballSport === "basketball"}
            <circle r={r * 0.6} fill="#ea7a1e" stroke="#2b1a0e" stroke-width={u * 0.07} />
            <path d="M{-r * 0.6} 0 L{r * 0.6} 0 M0 {-r * 0.6} L0 {r * 0.6}" stroke="#2b1a0e" stroke-width={u * 0.05} />
          {:else if ballSport === "volleyball"}
            <circle r={r * 0.6} fill="#fde047" stroke="#1e3a8a" stroke-width={u * 0.08} />
            <path d="M0 {-r * 0.6} Q{r * 0.2} 0 0 {r * 0.6} M{-r * 0.55} {-r * 0.2} Q0 {r * 0.1} {r * 0.55} {-r * 0.2}" fill="none" stroke="#1e3a8a" stroke-width={u * 0.06} />
          {:else if ballSport === "hockey"}
            <circle r={r * 0.4} fill="#fff" stroke="#111" stroke-width={u * 0.06} />
          {:else if ballSport === "netball"}
            <circle r={r * 0.6} fill="#fff" stroke="#1d4ed8" stroke-width={u * 0.1} />
          {:else}
            <circle r={r * 0.55} fill="#fff" stroke="#111" stroke-width={u * 0.08} />
            <path d="M0 {-r * 0.22} L{r * 0.21} {-r * 0.07} L{r * 0.13} {r * 0.18} L{-r * 0.13} {r * 0.18} L{-r * 0.21} {-r * 0.07}Z" fill="#111" />
          {/if}
        {:else if it.type === "cone"}
          <path d="M0 {-r * 0.9} L{r * 0.75} {r * 0.6} L{-r * 0.75} {r * 0.6}Z" fill={c} stroke="#0006" stroke-width={u * 0.05} />
        {:else if it.type === "disc"}
          <circle r={r * 0.5} fill="none" stroke={it.color ?? "#facc15"} stroke-width={u * 0.25} />
        {:else if it.type === "pole" || it.type === "flag" || it.type === "post"}
          <circle r={r * 0.45} fill="#0008" />
          <circle r={r * 0.22} fill={it.color ?? (it.type === "post" ? "#e5e7eb" : "#facc15")} />
          {#if it.type === "flag"}<path d="M0 0 L{r * 0.9} {-r * 0.35} L0 {-r * 0.7}Z" fill={it.color ?? "#ef4444"} />{/if}
        {:else if it.type === "mannequin"}
          <rect x={-r * 0.45} y={-r * 0.8} width={r * 0.9} height={r * 1.6} rx={r * 0.45} fill={it.color ?? "#64748b"} stroke="#fff" stroke-width={u * 0.06} />
        {:else if it.type === "mini_goal" || it.type === "goal"}
          {@const g = it.type === "goal" ? r * 2.2 : r * 1.3}
          <path d="M{-g} {-r * 0.5} L{-g} {r * 0.5} L{g} {r * 0.5} L{g} {-r * 0.5}" fill="#ffffff22" stroke="#fff" stroke-width={u * 0.14} />
        {:else if it.type === "ladder"}
          <rect x={-r * 0.5} y={-r * 2} width={r} height={r * 4} fill="none" stroke={it.color ?? "#facc15"} stroke-width={u * 0.1} />
          {#each [-1.2, -0.4, 0.4, 1.2] as k}<line x1={-r * 0.5} x2={r * 0.5} y1={r * k} y2={r * k} stroke={it.color ?? "#facc15"} stroke-width={u * 0.08} />{/each}
        {:else if it.type === "hurdle"}
          <rect x={-r * 0.9} y={-r * 0.18} width={r * 1.8} height={r * 0.36} rx={r * 0.1} fill={it.color ?? "#f97316"} />
        {:else if it.type === "rebounder"}
          <rect x={-r} y={-r * 0.3} width={r * 2} height={r * 0.6} fill="#ffffff33" stroke="#e5e7eb" stroke-width={u * 0.1} />
        {:else if it.type === "tackle_bag"}
          <rect x={-r * 0.5} y={-r * 1.1} width={r} height={r * 2.2} rx={r * 0.5} fill={it.color ?? "#1e293b"} stroke="#fde047" stroke-width={u * 0.1} />
        {:else if it.type === "tackle_shield"}
          <rect x={-r * 0.75} y={-r * 0.45} width={r * 1.5} height={r * 0.9} rx={r * 0.2} fill={it.color ?? "#dc2626"} stroke="#fff" stroke-width={u * 0.08} />
        {:else if it.type === "ruck_pad"}
          <rect x={-r * 0.9} y={-r * 0.6} width={r * 1.8} height={r * 1.2} rx={r * 0.15} fill={it.color ?? "#0ea5e9"} stroke="#fff" stroke-width={u * 0.08} />
        {:else if it.type === "kicking_tee"}
          <circle r={r * 0.4} fill={it.color ?? "#f97316"} stroke="#fff" stroke-width={u * 0.06} />
          <circle r={r * 0.15} fill="#fff" />
        {:else if it.type === "chair"}
          <rect x={-r * 0.6} y={-r * 0.6} width={r * 1.2} height={r * 1.2} rx={r * 0.15} fill={it.color ?? "#78716c"} stroke="#fff" stroke-width={u * 0.06} />
        {:else if it.type === "target"}
          <rect x={-r * 1.6} y={-r * 1.1} width={r * 3.2} height={r * 2.2} fill={(it.color ?? "#22c55e") + "44"} stroke={it.color ?? "#22c55e"} stroke-width={u * 0.08} stroke-dasharray="{u * 0.3} {u * 0.2}" />
        {:else if it.type === "ball_cart"}
          <rect x={-r * 0.8} y={-r * 0.6} width={r * 1.6} height={r * 1.2} rx={r * 0.15} fill="#334155" stroke="#fff" stroke-width={u * 0.06} />
          {#each [[-0.35, -0.2], [0.35, -0.2], [0, 0.25]] as [dx, dy]}<circle cx={r * dx} cy={r * dy} r={r * 0.22} fill="#fde047" />{/each}
        {:else}
          <circle r={r * 0.5} fill={c} />
        {/if}
        {#if it.label && !(it.type === "player" || it.type === "opponent")}<text y={r * 1.5} font-size={r * 0.8} text-anchor="middle" fill="#fff">{it.label}</text>{/if}
      </g>
    {/each}
  </svg>

  {#if controls && (frames > 1 || editable)}
    <div class="steps">
      {#if frames > 1}
        <button class="ib" title={playing ? "Stop" : "Play the move"} onclick={() => (playing ? stop() : play())}><Icon name={playing ? "stop" : "play"} size={14} /></button>
      {/if}
      <button class="ib" title="Previous step" disabled={frame === 0 || playing} onclick={() => (frame = frame - 1)}><Icon name="chevronLeft" size={14} /></button>
      <span class="small">Step {Math.round(shownT) + 1} / {frames}</span>
      <button class="ib" title="Next step" disabled={frame >= frames - 1 || playing} onclick={() => (frame = frame + 1)}><Icon name="chevronRight" size={14} /></button>
      {#if editable}
        <button class="ib txt" title="Add a step after this one: move players to show what happens next" onclick={addStep} disabled={playing}><Icon name="plus" size={14} /> Step</button>
        {#if frame > 0}
          <label class="small muted row" title="Seconds to move into this step">
            <input type="number" min="0.3" max="10" step="0.1" value={b.steps[frame - 1]?.dur ?? 1.5} onchange={(e) => setStepDuration(e.currentTarget.value)} style="width:56px;padding:2px 4px" /> s
          </label>
          <button class="ib" title="Delete this step" onclick={removeStep}><Icon name="trash" size={14} /></button>
        {/if}
        <span class="muted small hint">{frame === 0 ? "Starting positions" : "Drag players to where they move in this step"}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tb { outline: none; display: flex; flex-direction: column; gap: 6px; }
  .tb:focus-visible { box-shadow: 0 0 0 2px var(--accent); border-radius: 4px; }
  svg { width: 100%; display: block; touch-action: pan-x pan-y pinch-zoom; user-select: none; border-radius: 6px; }
  svg.edit { touch-action: none; }
  svg.edit { cursor: crosshair; }
  svg.edit .it, svg.edit .ln { cursor: grab; }
  .sel { filter: drop-shadow(0 0 0.4px #facc15); }
  .steps { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .ib { min-width: 28px; height: 28px; padding: 0 6px; display: inline-flex; align-items: center; justify-content: center; gap: 4px; }
  .ib.txt { font-size: 12px; }
  .hint { margin-left: 4px; }
</style>
