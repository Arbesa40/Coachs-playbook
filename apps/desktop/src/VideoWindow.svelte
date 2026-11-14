<script>
  // Pop-out video window: shows the video and drawings, driven by the main window.
  import { emit, listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { createPlayer } from "./lib/player.svelte.js";
  import VideoStage from "./components/VideoStage.svelte";

  const player = createPlayer();
  let video = $state(null);
  let shapes = $state([]);
  let draw = $state({ drawMode: false, tool: "arrow", color: "#facc15" });
  let pending = null; // { time, paused, rate } to apply once the source is ready
  let hint = $state(true);
  let mirror = $state(false);
  let lastShapes = "[]";
  const win = getCurrentWindow();

  function onReady(src) {
    if (mirror && src.kind === "youtube") src.player?.mute?.();
    if (pending) {
      src.seek(pending.time || 0);
      if (pending.rate) src.setRate(pending.rate);
      if (!pending.paused) src.play();
      pending = null;
    }
    emit("pv:loaded", { video_id: video?.id });
  }

  $effect(() => {
    const offs = [
      listen("pv:load", (e) => {
        const p = e.payload;
        pending = { time: p.time, paused: p.paused, rate: p.rate };
        mirror = !!p.mirror;
        lastShapes = JSON.stringify(p.shapes ?? []);
        shapes = p.shapes ?? [];
        draw = p.draw ?? draw;
        if (video?.id === p.video?.id && player.source) onReady(player.source);
        else video = p.video;
      }),
      listen("pv:cmd", (e) => {
        const s = player.source;
        if (!s) return;
        const { c, a } = e.payload;
        if (c === "play") s.play();
        else if (c === "pause") s.pause();
        else if (c === "seek") s.seek(a);
        else if (c === "rate") s.setRate(a);
        else if (c === "step") s.step(a);
      }),
      // Mirror mode: follow the main window's player.
      listen("pv:follow", (e) => {
        const s = player.source;
        if (!s || !mirror) return;
        const { time, paused, rate } = e.payload;
        if (rate && rate !== player.rate) {
          player.rate = rate;
          s.setRate(rate);
        }
        if (Math.abs(s.time() - time) > (paused ? 0.05 : 0.35)) s.seek(time + (paused ? 0 : 0.1));
        if (paused && !s.paused) s.pause();
        if (!paused && s.paused) s.play();
      }),
      listen("pv:shapes", (e) => {
        lastShapes = JSON.stringify(e.payload ?? []);
        shapes = e.payload ?? [];
      }),
      listen("pv:draw", (e) => (draw = e.payload)),
      listen("pv:close", () => win.close()),
    ];
    emit("pv:ready");
    const tick = setInterval(() => {
      const s = player.source;
      if (s) emit("pv:state", { time: s.time(), duration: s.duration(), paused: s.paused, rates: s.rates() });
    }, 150);
    const bye = () => emit("pv:closed");
    window.addEventListener("beforeunload", bye);
    const t = setTimeout(() => (hint = false), 4000);
    return () => {
      offs.forEach((p) => p.then((off) => off()));
      clearInterval(tick);
      clearTimeout(t);
      window.removeEventListener("beforeunload", bye);
    };
  });

  // Drawings made in this window go back to the main window.
  $effect(() => {
    const json = JSON.stringify($state.snapshot(shapes));
    if (json !== lastShapes) {
      lastShapes = json;
      emit("pv:shapes-from-popout", JSON.parse(json));
    }
  });

  async function toggleFullscreen() {
    await win.setFullscreen(!(await win.isFullscreen()));
  }
  function onKey(e) {
    if (e.key === "F11") {
      e.preventDefault();
      toggleFullscreen();
    } else if (e.key === "Escape") win.setFullscreen(false);
  }
</script>

<svelte:window onkeydown={onKey} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="vw" ondblclick={toggleFullscreen}>
  <div class="fit">
    <VideoStage {video} {player} bind:shapes muted={mirror} drawMode={draw.drawMode} tool={draw.tool} color={draw.color} onready={onReady} />
  </div>
  {#if hint}<div class="hint">Double-click or press F11 for full screen · {mirror ? "Showing the same video as the main window" : "Controls are in the main window"}</div>{/if}
</div>

<style>
  :global(body) { background: #000; overflow: hidden; }
  .vw { position: fixed; inset: 0; display: grid; place-items: center; background: #000; }
  .fit { width: min(100vw, calc(100vh * 16 / 9)); }
  .hint { position: fixed; bottom: 16px; left: 50%; transform: translateX(-50%); background: #000a; color: #ddd; padding: 6px 12px; border-radius: 6px; font-size: 13px; pointer-events: none; }
</style>
