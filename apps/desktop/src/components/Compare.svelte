<script>
  import Icon from "$shared/Icon.svelte";
  import { fileUrl, fmtTime } from "$shared/api.js";
  import { emit, listen } from "@tauri-apps/api/event";
  import { openCompareWindow, closeCompareWindow } from "../lib/popout.js";
  import { toast } from "../lib/state.svelte.js";

  // Side-by-side comparison of two local videos, synced or independent, optionally on a second screen.
  let { videos, onclose } = $props();
  const locals = $derived(videos.filter((v) => v.source === "local"));
  let a = $state(null), b = $state(null);
  let elA = $state(), elB = $state();
  let synced = $state(true);
  let offset = 0;
  let ta = $state(0), tb = $state(0);

  const url = (v) => (v ? fileUrl(v.proxy_path || v.path) : "");
  const frame = (el) => 1 / ((el === elA ? a?.fps : b?.fps) || 25);

  function lock() {
    if (elA && elB) offset = elB.currentTime - elA.currentTime;
  }
  function both(fn) {
    for (const el of synced ? [elA, elB] : [elA]) if (el) fn(el);
  }
  function play() {
    if (synced && elA && elB && elA.paused) elB.currentTime = Math.max(0, elA.currentTime + offset);
    const go = elA?.paused ?? elB?.paused;
    both((el) => (go ? el.play() : el.pause()));
  }
  function step(dir) {
    both((el) => {
      el.pause();
      el.currentTime = Math.max(0, el.currentTime + dir * frame(el));
    });
  }
  function rate(r) {
    for (const el of [elA, elB]) if (el) el.playbackRate = r;
    speed = r;
  }
  let speed = $state(1);
  function onTimeA() {
    ta = elA.currentTime;
    if (synced && elB && !elA.paused && Math.abs(elB.currentTime - (elA.currentTime + offset)) > 0.25) elB.currentTime = elA.currentTime + offset;
  }
  // Seeking the left video while synced moves the right one with it.
  function onSeekA() {
    ta = elA.currentTime;
    if (synced && elB) elB.currentTime = Math.max(0, elA.currentTime + offset);
  }

  let popMode = $state(null); // null | "mirror" | "move"
  const offPop = [];
  function sendLoad() {
    emit("cw:load", { a: $state.snapshot(a), b: $state.snapshot(b), mirror: popMode === "mirror" });
  }
  async function openScreen(mode) {
    if (!a && !b) return toast("Choose the videos first", "error");
    if (popMode) await closeScreen();
    popMode = mode;
    offPop.push(await listen("cw:ready", sendLoad), await listen("cw:closed", () => closeScreen(true)));
    await openCompareWindow();
  }
  async function closeScreen(alreadyClosed = false) {
    if (!popMode) return;
    offPop.splice(0).forEach((off) => off());
    popMode = null;
    if (!alreadyClosed) await closeCompareWindow();
  }
  $effect(() => {
    const _ = [a?.id, b?.id];
    if (popMode) sendLoad();
  });
  $effect(() => {
    if (!popMode) return;
    const i = setInterval(() => {
      emit("cw:follow", { ta: elA?.currentTime ?? 0, tb: elB?.currentTime ?? 0, pausedA: elA?.paused ?? true, pausedB: elB?.paused ?? true, rate: speed });
    }, 150);
    return () => clearInterval(i);
  });
  $effect(() => () => {
    offPop.splice(0).forEach((off) => off());
    closeCompareWindow();
  });
  async function close() {
    for (const el of [elA, elB]) el?.pause();
    await closeScreen();
    onclose();
  }
</script>

<div class="modal-bg">
  <div class="modal col" style="width:96vw;max-width:1600px">
    <div class="row">
      <h2 class="grow">Compare</h2>
      <label class="row small"><input type="checkbox" bind:checked={synced} onchange={lock} /> Synced</label>
      <button class="ghost" class:on={popMode === "mirror"} title={popMode === "mirror" ? "Stop showing on the second screen" : "Show on both screens"} onclick={() => (popMode === "mirror" ? closeScreen() : openScreen("mirror"))}><Icon name="mirror" /></button>
      {#if popMode === "move"}
        <button class="ghost on" title="Bring the videos back" onclick={() => closeScreen()}><Icon name="popin" /></button>
      {:else}
        <button class="ghost" title="Move to the other screen" onclick={() => openScreen("move")}><Icon name="popout" /></button>
      {/if}
      <button class="ghost" onclick={close}><Icon name="x" /></button>
    </div>
    <div class="grid2">
      {#each [0, 1] as side}
        <div class="col">
          {#if side === 0}
            <select bind:value={a}><option value={null}>Left video…</option>{#each locals as v}<option value={v}>{v.name}</option>{/each}</select>
            <div class="stage" class:away={popMode === "move"}>
              <!-- svelte-ignore a11y_media_has_caption -->
              <video bind:this={elA} src={url(a)} ontimeupdate={onTimeA} onseeked={onSeekA} controls></video>
              {#if popMode === "move"}<div class="away-msg">Showing on the other screen</div>{/if}
            </div>
            <div class="muted small">{fmtTime(ta, true)}</div>
          {:else}
            <select bind:value={b}><option value={null}>Right video…</option>{#each locals as v}<option value={v}>{v.name}</option>{/each}</select>
            <div class="stage" class:away={popMode === "move"}>
              <!-- svelte-ignore a11y_media_has_caption -->
              <video bind:this={elB} src={url(b)} ontimeupdate={() => (tb = elB.currentTime)} controls={!synced} muted={synced}></video>
              {#if popMode === "move"}<div class="away-msg">Showing on the other screen</div>{/if}
            </div>
            <div class="muted small">{fmtTime(tb, true)}</div>
          {/if}
        </div>
      {/each}
    </div>
    <div class="row">
      <button onclick={() => step(-1)}><Icon name="frameBack" size={14} /> Frame</button>
      <button class="primary" onclick={play}>Play / Pause</button>
      <button onclick={() => step(1)}>Frame <Icon name="frameFwd" size={14} /></button>
      {#each [0.25, 0.5, 1] as r}<button class:on={speed === r} onclick={() => rate(r)}>{r}×</button>{/each}
      <span class="muted small">Line both videos up on the same moment, then tick Synced.</span>
    </div>
  </div>
</div>

<style>
  .stage { position: relative; }
  video { width: 100%; aspect-ratio: 16/9; background: #000; }
  .stage.away video { visibility: hidden; }
  .away-msg { position: absolute; inset: 0; display: grid; place-items: center; color: var(--muted); background: var(--input); border-radius: 4px; }
</style>
