<script>
  import { askText } from "$shared/api.js";
  import { render } from "../lib/drawing.js";

  // shapes: the drawing being shown/edited; active: capture the mouse to draw.
  let { shapes = $bindable([]), active = false, tool = "arrow", color = "#facc15" } = $props();

  let canvas;
  let box = $state({ w: 0, h: 0 });
  let drawing = null; // shape in progress
  let angleWait = null; // angle shape waiting for its third point

  $effect(() => {
    const ro = new ResizeObserver(([e]) => {
      box = { w: Math.round(e.contentRect.width), h: Math.round(e.contentRect.height) };
    });
    ro.observe(canvas.parentElement);
    return () => ro.disconnect();
  });

  $effect(() => {
    // Redraw whenever the shapes or size change.
    const list = $state.snapshot(shapes);
    if (!canvas || !box.w) return;
    canvas.width = box.w;
    canvas.height = box.h;
    const ctx = canvas.getContext("2d");
    ctx.clearRect(0, 0, box.w, box.h);
    render(ctx, list, box.w, box.h);
  });

  const pos = (e) => {
    const r = canvas.getBoundingClientRect();
    return [Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)), Math.min(1, Math.max(0, (e.clientY - r.top) / r.height))];
  };

  function down(e) {
    if (!active) return;
    canvas.setPointerCapture(e.pointerId);
    const p = pos(e);
    if (angleWait) {
      angleWait.pts = [...angleWait.pts, p];
      shapes = [...shapes];
      angleWait = null;
      return;
    }
    if (tool === "text") {
      askText("Text to place on the video").then((text) => {
        if (text) shapes = [...shapes, { type: "text", color, pts: [p], text }];
      });
      return;
    }
    drawing = { type: tool, color, pts: [p, p] };
    shapes = [...shapes, drawing];
  }

  function move(e) {
    if (!drawing) return;
    const p = pos(e);
    if (drawing.type === "free") drawing.pts = [...drawing.pts, p];
    else drawing.pts = [drawing.pts[0], p];
    shapes = [...shapes.slice(0, -1), { ...drawing }];
    drawing = shapes[shapes.length - 1];
  }

  function up() {
    if (drawing?.type === "angle") angleWait = drawing;
    drawing = null;
  }
</script>

<canvas
  bind:this={canvas}
  class="draw"
  class:active
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
></canvas>

<style>
  .draw { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; z-index: 5; }
  .draw.active { pointer-events: auto; cursor: crosshair; }
</style>
