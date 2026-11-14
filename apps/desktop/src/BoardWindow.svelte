<script>
  // Second-screen view of the tactics board: follows the main window.
  import { emit, listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import TacticsBoard from "$shared/TacticsBoard.svelte";
  import { emptyBoard } from "$shared/board.js";

  let view = $state({ board: emptyBoard(), sport: "football", style: null, frame: 0, t: null, title: "" });
  let h = $state(window.innerHeight - 60);
  const win = getCurrentWindow();

  $effect(() => {
    const offs = [listen("bw:state", (e) => (view = { ...view, ...e.payload })), listen("bw:close", () => win.close())];
    emit("bw:ready");
    const bye = () => emit("bw:closed");
    const resize = () => (h = window.innerHeight - 60);
    window.addEventListener("beforeunload", bye);
    window.addEventListener("resize", resize);
    return () => {
      offs.forEach((p) => p.then((off) => off()));
      window.removeEventListener("beforeunload", bye);
      window.removeEventListener("resize", resize);
    };
  });
  async function toggleFullscreen() {
    await win.setFullscreen(!(await win.isFullscreen()));
  }
</script>

<svelte:window onkeydown={(e) => { if (e.key === "F11") { e.preventDefault(); toggleFullscreen(); } if (e.key === "Escape") win.setFullscreen(false); }} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="bw" ondblclick={toggleFullscreen}>
  {#if view.title}<h1>{view.title}</h1>{/if}
  <TacticsBoard board={view.board} sport={view.sport} style={view.style} frame={view.frame} t={view.t} height={h - (view.title ? 40 : 0)} controls={false} />
</div>

<style>
  :global(body) { background: #0b0f13; overflow: hidden; }
  .bw { position: fixed; inset: 0; padding: 16px; display: flex; flex-direction: column; align-items: center; gap: 8px; }
  h1 { color: #e6e9ee; font-size: 22px; margin: 0; }
</style>
