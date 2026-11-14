<script>
  // Second-screen compare view: follows the main window and stays silent.
  import { emit, listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { fileUrl, fmtTime } from "$shared/api.js";

  let a = $state(null);
  let b = $state(null);
  let elA = $state();
  let elB = $state();
  let ta = $state(0);
  let tb = $state(0);
  let mirror = $state(true);
  let hint = $state(true);
  const win = getCurrentWindow();
  const url = (v) => (v ? fileUrl(v.proxy_path || v.path) : "");

  function follow(el, time, paused, rate) {
    if (!el || !Number.isFinite(time) || el.readyState < 1) return;
    if (rate && el.playbackRate !== rate) el.playbackRate = rate;
    if (Math.abs(el.currentTime - time) > (paused ? 0.04 : 0.3)) el.currentTime = Math.max(0, time + (paused ? 0 : 0.08));
    if (paused && !el.paused) el.pause();
    if (!paused && el.paused) el.play().catch(() => {});
  }

  $effect(() => {
    const offs = [
      listen("cw:load", (e) => {
        a = e.payload.a;
        b = e.payload.b;
        mirror = !!e.payload.mirror;
      }),
      listen("cw:follow", (e) => {
        const p = e.payload;
        ta = p.ta;
        tb = p.tb;
        follow(elA, p.ta, p.pausedA, p.rate);
        follow(elB, p.tb, p.pausedB, p.rate);
      }),
      listen("cw:close", () => win.close()),
    ];
    emit("cw:ready");
    const bye = () => emit("cw:closed");
    window.addEventListener("beforeunload", bye);
    const t = setTimeout(() => (hint = false), 4000);
    return () => {
      offs.forEach((p) => p.then((off) => off()));
      clearTimeout(t);
      window.removeEventListener("beforeunload", bye);
    };
  });

  async function toggleFullscreen() {
    await win.setFullscreen(!(await win.isFullscreen()));
  }
</script>

<svelte:window onkeydown={(e) => { if (e.key === "F11") { e.preventDefault(); toggleFullscreen(); } else if (e.key === "Escape") win.setFullscreen(false); }} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="cw" ondblclick={toggleFullscreen}>
  {#each [[a, ta], [b, tb]] as [v, t], i}
    <div class="side">
      {#if v}
        <!-- svelte-ignore a11y_media_has_caption -->
        {#if i === 0}<video bind:this={elA} src={url(v)} muted preload="auto"></video>
        {:else}<video bind:this={elB} src={url(v)} muted preload="auto"></video>{/if}
        <div class="cap"><span>{v.name}</span><span>{fmtTime(t, true)}</span></div>
      {:else}
        <div class="none">No video chosen</div>
      {/if}
    </div>
  {/each}
  {#if hint}<div class="hint">Double-click or press F11 for full screen · {mirror ? "Same view as the main window" : "Controls are in the main window"}</div>{/if}
</div>

<style>
  :global(body) { background: #000; overflow: hidden; }
  .cw { position: fixed; inset: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 6px; align-items: center; background: #000; padding: 6px; }
  .side { position: relative; display: flex; flex-direction: column; min-width: 0; }
  video { width: 100%; aspect-ratio: 16 / 9; background: #000; object-fit: contain; }
  .cap { display: flex; justify-content: space-between; gap: 8px; color: #cbd5e1; font-size: 14px; padding: 4px 2px; }
  .cap span:first-child { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .none { aspect-ratio: 16 / 9; display: grid; place-items: center; color: #64748b; border: 1px dashed #334155; border-radius: 6px; }
  .hint { position: fixed; bottom: 16px; left: 50%; transform: translateX(-50%); background: #000a; color: #ddd; padding: 6px 12px; border-radius: 6px; font-size: 13px; pointer-events: none; }
</style>
