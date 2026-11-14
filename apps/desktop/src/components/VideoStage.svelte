<script>
  import { fileUrl } from "$shared/api.js";
  import { LocalSource, YouTubeSource } from "../lib/source.js";
  import DrawingLayer from "./DrawingLayer.svelte";

  let {
    video, // video record
    player, // from createPlayer()
    shapes = $bindable([]),
    drawMode = false,
    tool = "arrow",
    color = "#facc15",
    progress = 0, // proxy progress 0..1
    onready = null,
    videoEl = $bindable(null),
    muted = false, // the mirrored copy on a second screen stays silent
  } = $props();

  let ytBox = $state();
  let error = $state("");

  const localUrl = $derived(
    video?.source === "local" ? fileUrl(video.proxy_path || video.path) : ""
  );
  const preparing = $derived(
    video?.source === "local" && !video.proxy_path && ["queued", "working"].includes(video.proxy_status)
  );

  let lastVideoId = null;
  function localReady() {
    const s = new LocalSource(videoEl, video.fps);
    // Same video reloaded (e.g. its smooth-playback copy just finished): keep the position.
    if (lastVideoId === video.id && player.time > 0) s.seek(player.time);
    lastVideoId = video.id;
    player.source = s;
    player.duration = s.duration();
    error = "";
    onready?.(s);
  }

  $effect(() => {
    // Rebuild the YouTube player when the video changes.
    if (video?.source !== "youtube" || !ytBox) return;
    let src;
    let cancelled = false;
    const holder = document.createElement("div");
    ytBox.replaceChildren(holder);
    error = "";
    YouTubeSource.create(holder, video.youtube_id)
      .then((s) => {
        if (cancelled) return s.destroy();
        src = s;
        s.onError = (code) => (error = code === 101 || code === 150 ? "The owner doesn't allow this video to be embedded." : "YouTube error " + code);
        player.source = s;
        onready?.(s);
      })
      .catch((e) => (error = e.message));
    return () => {
      cancelled = true;
      if (player.source === src) player.source = null;
      src?.destroy();
    };
  });

  $effect(() => {
    // Clear the source when switching away from a local video.
    const url = localUrl;
    return () => {
      if (url && player.source?.kind === "local") player.source = null;
    };
  });
</script>

<div class="stage">
  {#if !video}
    <div class="msg muted">Import a match video to start.</div>
  {:else if video.source === "youtube"}
    <div class="yt" bind:this={ytBox}></div>
  {:else}
    {#key localUrl}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        bind:this={videoEl}
        src={localUrl}
        preload="auto"
        {muted}
        onloadedmetadata={localReady}
        onerror={() => (error = preparing ? "" : "This file can't be played directly. A smooth-playback copy is being prepared.")}
      ></video>
    {/key}
    {#if preparing && error === ""}
      <div class="badge warn corner">Preparing smooth playback {Math.round(progress * 100)}%</div>
    {/if}
  {/if}
  {#if error}<div class="msg">{error}</div>{/if}
  <DrawingLayer bind:shapes active={drawMode} {tool} {color} />
</div>

<style>
  .stage { position: relative; background: #000; aspect-ratio: 16 / 9; width: 100%; flex: none; overflow: hidden; }
  video, .yt { position: absolute; inset: 0; width: 100%; height: 100%; }
  video { object-fit: contain; }
  .yt :global(iframe) { width: 100%; height: 100%; pointer-events: none; }
  .msg { position: absolute; inset: 0; display: grid; place-items: center; text-align: center; padding: 20px; color: #fca5a5; }
  .msg.muted { color: var(--muted); }
  .corner { position: absolute; top: 8px; left: 8px; z-index: 6; }
</style>
