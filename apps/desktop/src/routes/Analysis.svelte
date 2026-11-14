<script>
  import Icon from "$shared/Icon.svelte";
  import { db, invoke, fmtTime, fileUrl, byKey, playerLabel, sortPlayers, ask, askText } from "$shared/api.js";
  import PitchDiagram from "$shared/PitchDiagram.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { app, attempt, toast } from "../lib/state.svelte.js";
  import { createPlayer, togglePlay, seek } from "../lib/player.svelte.js";
  import { TOOLS, snapshotPng } from "../lib/drawing.js";
  import VideoStage from "../components/VideoStage.svelte";
  import TagPanel from "../components/TagPanel.svelte";
  import PanelEditor from "../components/PanelEditor.svelte";
  import Timeline from "../components/Timeline.svelte";
  import ClipsPane from "../components/ClipsPane.svelte";
  import StatsPane from "../components/StatsPane.svelte";
  import Compare from "../components/Compare.svelte";
  import SessionsDialog from "../components/SessionsDialog.svelte";
  import { RemoteSource, openPopout, closePopout } from "../lib/popout.js";
  import { emit } from "@tauri-apps/api/event";

  let videos = $state([]);
  let tags = $state([]);
  let clips = $state([]);
  let playlists = $state([]);
  let panels = $state([]);
  let players = $state([]);
  let drawings = $state([]);
  let matches = $state([]);
  let sessions = $state([]);
  let template = $state(null);
  let progress = $state({});
  let mediaPaused = $state(false);

  let currentId = $state(null);
  let panelId = $state(null);
  let period = $state("");
  let tab = $state("timeline");
  let filter = $state("");
  let selected = $state([]);
  let lastTagId = $state(null);
  let editPanel = $state(false);
  let showCompare = $state(false);

  let inPt = $state(null);
  let outPt = $state(null);
  let swStart = $state(null);

  let drawMode = $state(false);
  let tool = $state("arrow");
  let color = $state("#facc15");
  let shapes = $state([]);
  let editingDrawing = null;

  let videoEl = $state(null);

  const player = createPlayer();
  let queue = $state(null); // { items: clips[], i }
  let pendingClip = null;

  const current = $derived(videos.find((v) => v.id === currentId) ?? null);
  const panel = $derived(panels.find((p) => p.id === panelId) ?? panels.find((p) => p.default) ?? panels[0] ?? null);
  const playersById = $derived(byKey(players, "id"));
  const squad = $derived(sortPlayers(players));
  const videoTags = $derived(tags.filter((t) => t.video_id === currentId).sort((a, b) => a.t - b.t));
  const shownTags = $derived(filter.trim() ? videoTags.filter((t) => matches_(t, filter)) : videoTags);
  const videoDrawings = $derived(drawings.filter((d) => d.video_id === currentId).sort((a, b) => a.t - b.t));
  const lastTag = $derived(tags.find((t) => t.id === lastTagId) ?? null);
  let showLocation = $state(false);
  const lastButton = $derived(panel?.buttons?.find((b) => b.code === lastTag?.code) ?? null);
  const colors = $derived(Object.fromEntries((panel?.buttons ?? []).map((b) => [b.code, b.color])));
  const nearDrawing = $derived(player.paused ? videoDrawings.find((d) => Math.abs(d.t - player.time) < 0.25) : null);

  async function load() {
    [videos, tags, clips, playlists, panels, players, drawings, matches, sessions] = await Promise.all(
      ["video", "tag", "clip", "playlist", "panel", "player", "drawing", "match", "analysis_session"].map((k) => db.list(k))
    );
    if (!sessionLoaded) {
      sessionLoaded = true;
      if (sessionId && sessions.some((x) => x.id === sessionId)) openSession(sessionId, true);
      else sessionId = null;
    }
    if (!currentId && shownVideos.length) currentId = shownVideos[shownVideos.length - 1].id;
  }
  load();
  invoke("list_templates").then((all) => {
    template = all.find((t) => t.id === app.team.sport) ?? all[0];
    period = template?.periods?.[0] ?? "";
  });

  $effect(() => {
    const offs = [
      listen("media-changed", () => db.list("video").then((v) => (videos = v))),
      listen("media-progress", (e) => (progress = { ...progress, [e.payload.video_id]: e.payload.progress })),
    ];
    return () => offs.forEach((p) => p.then((off) => off()));
  });

  // Show a saved freeze-frame drawing when paused on it (unless editing).
  $effect(() => {
    if (!drawMode) shapes = nearDrawing ? $state.snapshot(nearDrawing.shapes) : [];
  });

  // ---- analysis sessions ----
  const sessionKey = "session." + app.team.id;
  let sessionId = $state((() => {
    try {
      return localStorage.getItem(sessionKey);
    } catch {
      return null;
    }
  })());
  let sessionLoaded = false;
  let showSessions = $state(false);
  let addPick = $state("");
  const session = $derived(sessions.find((x) => x.id === sessionId) ?? null);
  const shownVideos = $derived(session ? videos.filter((v) => (session.video_ids ?? []).includes(v.id)) : videos);
  const otherVideos = $derived(session ? videos.filter((v) => !(session.video_ids ?? []).includes(v.id)) : []);

  function openSession(id, initial = false) {
    sessionId = id;
    try {
      if (id) localStorage.setItem(sessionKey, id);
      else localStorage.removeItem(sessionKey);
    } catch {}
    showSessions = false;
    const sess = sessions.find((x) => x.id === id);
    const ids = sess ? sess.video_ids ?? [] : videos.map((v) => v.id);
    const want = sess?.last_video_id && ids.includes(sess.last_video_id) ? sess.last_video_id : ids[ids.length - 1] ?? null;
    if (!initial || !currentId) {
      if (want !== currentId) {
        player.source?.pause();
        currentId = want;
      }
      if (sess?.last_time) resumeAt = sess.last_time;
    }
  }
  async function addToSession(ids) {
    if (!session || !ids.length) return;
    const next = [...new Set([...(session.video_ids ?? []), ...ids])];
    session.video_ids = next;
    await db.put("analysis_session", { video_ids: next, updated: new Date().toISOString() }, session.id);
  }
  async function removeFromSession(v) {
    if (!session) return;
    const next = (session.video_ids ?? []).filter((x) => x !== v.id);
    session.video_ids = next;
    await db.put("analysis_session", { video_ids: next, updated: new Date().toISOString() }, session.id);
    if (currentId === v.id) currentId = next[next.length - 1] ?? null;
  }
  // Remember where you were in the session (video and time), quietly.
  let saveTimer;
  $effect(() => {
    const sess = session, vid = currentId, paused = player.paused;
    if (!sess || !vid) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      const t = Math.round((player.source?.time() ?? player.time) * 10) / 10;
      db.put("analysis_session", { last_video_id: vid, last_time: t, updated: new Date().toISOString() }, sess.id).catch(() => {});
    }, paused ? 800 : 5000);
  });

  // ---- search: "turnovers, 2nd half, #7" ----
  function matches_(t, q) {
    const terms = q.split(",").map((s) => s.trim().toLowerCase()).filter(Boolean);
    return terms.every((term) => {
      const sing = term.replace(/s$/, "");
      if (t.code.toLowerCase().includes(sing)) return true;
      if ((t.period || "").toLowerCase().includes(term)) return true;
      if ((t.descriptor || "").toLowerCase() === term) return true;
      const p = playersById[t.player_id];
      if (p) {
        const m = term.match(/^(?:player\s*)?#?(\d+)$/);
        if (m && String(p.number) === m[1]) return true;
        if ((p.name || "").toLowerCase().includes(term)) return true;
      }
      return false;
    });
  }

  // ---- videos ----
  async function importVideos() {
    const paths = await open({
      multiple: true,
      filters: [{ name: "Video", extensions: ["mp4", "mov", "mkv", "avi", "m4v", "MP4", "MOV", "MKV", "AVI"] }],
    });
    if (!paths?.length) return;
    const added = await attempt(() => invoke("import_videos", { paths: [].concat(paths), matchId: current?.match_id ?? null }));
    if (added?.length) {
      await addToSession(added.map((v) => v.id));
      await load();
      currentId = added[0].id;
      toast(`Imported ${added.length} video(s). Smooth-playback copies are being prepared in the background.`);
    }
  }
  async function renameVideo(v) {
    const name = await askText("Video name", v.name);
    if (name) {
      await attempt(() => db.put("video", { name }, v.id));
      load();
    }
  }
  async function removeVideo(v) {
    if (!await ask(`Remove "${v.name}" from the library? The video file itself is not deleted.`)) return;
    await attempt(() => db.del("video", v.id));
    if (currentId === v.id) currentId = null;
    load();
  }
  async function setMatch(id) {
    await attempt(() => db.put("video", { match_id: id || null }, current.id));
    load();
  }
  function toggleMediaPause() {
    mediaPaused = !mediaPaused;
    invoke("media_pause", { paused: mediaPaused });
  }

  // ---- tagging ----
  function addTag(b) {
    if (!current || !player.source) return toast("Open a video first", "error");
    const t = player.source.time();
    const dur = player.duration || Infinity;
    const rec = {
      video_id: current.id,
      match_id: current.match_id ?? null,
      code: b.code,
      color: b.color,
      t,
      start: Math.max(0, t - (Number(b.pre) || 0)),
      end: Math.min(dur, t + (Number(b.post) || 0)),
      period,
    };
    // Show it instantly; the database write happens in the background.
    const tmp = { ...rec, id: "tmp-" + Math.random().toString(36).slice(2) };
    tags.push(tmp);
    lastTagId = tmp.id;
    showLocation = false;
    db.put("tag", rec)
      .then((saved) => {
        const i = tags.findIndex((x) => x.id === tmp.id);
        if (i >= 0) {
          const pending = tags[i];
          tags[i] = { ...saved, ...pending, id: saved.id };
          if (lastTagId === tmp.id) lastTagId = saved.id;
          // Edits made before the save finished.
          const extra = {};
          for (const k of ["player_id", "descriptor", "x", "y"]) if (pending[k] != null) extra[k] = pending[k];
          if (Object.keys(extra).length) db.put("tag", extra, saved.id);
        }
      })
      .catch((e) => toast(e, "error"));
  }
  function editTag(fields) {
    const t = lastTag;
    if (!t) return;
    Object.assign(t, fields);
    if (!t.id.startsWith("tmp-")) db.put("tag", fields, t.id).catch((e) => toast(e, "error"));
  }
  async function deleteTags(ids) {
    for (const id of ids) if (!id.startsWith("tmp-")) await db.del("tag", id);
    tags = tags.filter((t) => !ids.includes(t.id));
    selected = [];
    if (ids.includes(lastTagId)) lastTagId = null;
  }
  function tagLabel(t) {
    return [playerLabel(playersById[t.player_id]), t.descriptor, t.period].filter(Boolean).join(" · ");
  }

  // ---- clips ----
  function clipName(t) {
    return [t.code, playerLabel(playersById[t.player_id]), fmtTime(t.t)].filter(Boolean).join(" ");
  }
  async function clipsFromTags(ids) {
    const list = tags.filter((t) => ids.includes(t.id) && !t.id.startsWith("tmp-"));
    for (const t of list) {
      await db.put("clip", { video_id: t.video_id, tag_id: t.id, player_id: t.player_id ?? null, name: clipName(t), start: t.start, end: t.end, notes: "" });
    }
    clips = await db.list("clip");
    toast(`${list.length} clip(s) created`);
    tab = "clips";
  }
  async function saveInOut() {
    if (inPt == null || outPt == null || outPt <= inPt) return toast("Set an In point and a later Out point first", "error");
    await attempt(() => db.put("clip", { video_id: current.id, name: `Clip ${fmtTime(inPt)}`, start: inPt, end: outPt, notes: "" }), "Clip saved");
    clips = await db.list("clip");
    inPt = outPt = null;
    player.loop = null;
  }
  function toggleLoop() {
    activeClip = null;
    queue = null;
    player.loop = player.loop ? null : inPt != null && outPt > inPt ? { start: inPt, end: outPt } : null;
    if (!player.loop && inPt == null) toast("Set In and Out points to loop a section");
  }

  // Clip playback is ended when leaving the clips tab, so its loop doesn't trap the main video.
  let activeClip = $state(null);
  function exitClip() {
    if (!activeClip && !queue) return;
    activeClip = null;
    queue = null;
    pendingClip = null;
    player.loop = null;
  }
  function selectVideo(id) {
    exitClip();
    currentId = id;
  }
  function playClip(c, inQueue = false) {
    if (!inQueue) queue = null;
    activeClip = c;
    player.loop = inQueue ? null : { start: c.start, end: c.end };
    if (c.video_id !== currentId) {
      pendingClip = c;
      currentId = c.video_id;
    } else {
      seek(player, c.start);
      player.source?.play();
    }
  }
  function playPlaylist(pl) {
    const items = (pl.items ?? []).map((id) => clips.find((c) => c.id === id)).filter(Boolean);
    if (!items.length) return;
    queue = { items, i: 0 };
    playClip(items[0], true);
  }
  function onReady() {
    if (resumeAt != null) {
      seek(player, resumeAt);
      resumeAt = null;
    }
    if (pendingClip) {
      const c = pendingClip;
      pendingClip = null;
      seek(player, c.start);
      player.source.play();
    }
  }

  // ---- drawings & freeze-frames ----
  function startDrawing() {
    player.source?.pause();
    editingDrawing = nearDrawing;
    shapes = nearDrawing ? $state.snapshot(nearDrawing.shapes) : [];
    drawMode = true;
  }
  async function saveDrawing() {
    const fields = { video_id: current.id, t: editingDrawing?.t ?? player.time, shapes: $state.snapshot(shapes) };
    if (!shapes.length && editingDrawing) await db.del("drawing", editingDrawing.id);
    else if (shapes.length) await attempt(() => db.put("drawing", fields, editingDrawing?.id ?? null), "Freeze-frame saved");
    drawings = await db.list("drawing");
    drawMode = false;
  }
  async function snapshotToClip() {
    const c = clips.find((c) => c.video_id === currentId && player.time >= c.start && player.time <= c.end);
    if (!c) return toast("Move into a clip's time range to attach a snapshot", "error");
    const png = snapshotPng(current.source === "local" ? videoEl : null, $state.snapshot(shapes));
    await attempt(() => db.put("clip", { snapshot: png }, c.id), "Snapshot added to " + c.name);
    clips = await db.list("clip");
  }

  // Playback stops on a drawing until the user presses play; it doesn't fire again until playback moves away.
  let prevT = 0;
  let firedId = null;
  let firedT = 0;
  player.onTick = (t) => {
    if (firedId && Math.abs(t - firedT) > 0.75) firedId = null;
    if (!player.source || player.source.paused || drawMode) {
      prevT = t;
      return;
    }
    for (const d of videoDrawings) {
      if (d.id === firedId) continue;
      if (prevT < d.t && t >= d.t && t - d.t < 0.6) {
        const src = player.source;
        src.pause();
        firedId = d.id;
        firedT = d.t;
        // Seeking back a fraction of a second makes the decoder flash, so only do it when well past.
        if (t - d.t > 0.12) src.seek(d.t);
        break;
      }
    }
    if (queue) {
      const c = queue.items[queue.i];
      if (c && c.video_id === currentId && t >= c.end) {
        queue.i += 1;
        if (queue.i < queue.items.length) playClip(queue.items[queue.i], true);
        else {
          player.source.pause();
          queue = null;
          activeClip = null;
        }
      }
    }
    prevT = t;
  };


  function closeCard() {
    lastTagId = null;
    showLocation = false;
  }

  // ---- layout: resizable areas, collapsible library and bottom pane ----
  const pref = (k, d) => {
    try {
      const v = JSON.parse(localStorage.getItem("layout." + k));
      return v ?? d;
    } catch {
      return d;
    }
  };
  let libOpen = $state(pref("libOpen", true));
  let bottomOpen = $state(pref("bottomOpen", true));
  let tagW = $state(pref("tagW", 280));
  let bottomH = $state(pref("bottomH", 250));
  $effect(() => {
    try {
      for (const [k, v] of Object.entries({ libOpen, bottomOpen, tagW, bottomH })) localStorage.setItem("layout." + k, JSON.stringify(v));
    } catch {}
  });
  function dragSplit(e, which) {
    e.preventDefault();
    const startX = e.clientX, startY = e.clientY, w0 = tagW, h0 = bottomH;
    const move = (ev) => {
      if (which === "v") tagW = Math.min(520, Math.max(200, w0 - (ev.clientX - startX)));
      else bottomH = Math.min(window.innerHeight - 260, Math.max(120, h0 - (ev.clientY - startY)));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }
  // Keep the video 16:9 and as large as its area allows.
  let stageBox = $state();
  let stageW = $state(640);
  $effect(() => {
    if (!stageBox) return;
    const ro = new ResizeObserver(([en]) => {
      const { width, height } = en.contentRect;
      stageW = Math.max(240, Math.floor(Math.min(width, (height * 16) / 9)));
    });
    ro.observe(stageBox);
    return () => ro.disconnect();
  });

  // Second screen: "move" plays only in the other window, "mirror" plays on both.
  let popMode = $state(null); // null | "move" | "mirror"
  const popped = $derived(popMode === "move");
  let remote = null;
  let resumeAt = null;
  const offPop = [];
  function sendLoad() {
    const mirror = popMode === "mirror";
    if (!mirror) {
      remote = new RemoteSource(current);
      remote.update({ time: player.time, paused: true });
      player.source = remote;
    }
    lastVideoSent = current;
    emit("pv:load", {
      video: $state.snapshot(current),
      time: player.time,
      paused: player.paused,
      rate: player.rate,
      mirror,
      shapes: $state.snapshot(shapes),
      draw: { drawMode: mirror ? false : drawMode, tool, color },
    });
  }
  async function openSecondScreen(mode) {
    if (!current) return toast("Open a video first", "error");
    if (popMode) await closeSecondScreen();
    const t = player.time, wasPaused = player.paused;
    if (mode === "move") player.source?.pause();
    popMode = mode;
    offPop.push(
      await listen("pv:ready", () => {
        player.time = t;
        player.paused = wasPaused;
        sendLoad();
      }),
      await listen("pv:state", (e) => remote?.update(e.payload)),
      await listen("pv:loaded", () => {
        if (popMode === "move") onReady();
      }),
      await listen("pv:shapes-from-popout", (e) => {
        lastSentShapes = JSON.stringify(e.payload);
        shapes = e.payload;
      }),
      await listen("pv:closed", () => closeSecondScreen(true)),
    );
    await openPopout();
  }
  async function closeSecondScreen(alreadyClosed = false) {
    if (!popMode) return;
    const wasMove = popMode === "move";
    offPop.splice(0).forEach((off) => off());
    popMode = null;
    if (wasMove) {
      resumeAt = player.time;
      remote = null;
      player.source = null;
    }
    if (!alreadyClosed) closePopout();
  }
  // Keep the other window in step with what's chosen here.
  let lastVideoSent = null;
  $effect(() => {
    if (popMode && current && current.id !== lastVideoSent?.id) sendLoad();
  });
  // Mirror: the other screen follows this player.
  $effect(() => {
    if (popMode !== "mirror") return;
    const t = setInterval(() => {
      const s = player.source;
      if (s) emit("pv:follow", { time: s.time(), paused: s.paused, rate: player.rate });
    }, 200);
    return () => clearInterval(t);
  });
  let lastSentShapes = "[]";
  $effect(() => {
    const json = JSON.stringify($state.snapshot(shapes));
    if (popMode && json !== lastSentShapes) {
      lastSentShapes = json;
      emit("pv:shapes", JSON.parse(json));
    }
  });
  $effect(() => {
    const d = { drawMode: popMode === "move" ? drawMode : false, tool, color };
    if (popMode) emit("pv:draw", d);
  });
  $effect(() => () => {
    if (popMode) closePopout();
  });

  // ---- keyboard ----
  function onKey(e) {
    if (["INPUT", "TEXTAREA", "SELECT"].includes(e.target.tagName) || editPanel || showCompare) return;
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    const k = e.key.toLowerCase();
    if (drawMode) {
      if (e.key === "Escape") drawMode = false;
      return;
    }
    if (e.key === "Escape") return closeCard();
    const b = panel?.buttons?.find((b) => b.hotkey && b.hotkey === k);
    if (b) {
      e.preventDefault();
      return addTag(b);
    }
    if (e.key === " ") {
      e.preventDefault();
      togglePlay(player);
    } else if (e.key === "ArrowLeft") seek(player, player.time - (e.shiftKey ? 1 : 5));
    else if (e.key === "ArrowRight") seek(player, player.time + (e.shiftKey ? 1 : 5));
    else if (e.key === ",") player.source?.step(-1);
    else if (e.key === ".") player.source?.step(1);
    else if (e.key === "[") inPt = player.time;
    else if (e.key === "]") outPt = player.time;
    else if (e.key === "Delete" && selected.length) deleteTags(selected);
  }
</script>

<svelte:window onkeydown={onKey} />

<div
  class="analysis"
  style="--lib:{libOpen ? '230px' : '34px'};--tag:{tagW}px;--bottom:{bottomOpen ? bottomH + 'px' : '38px'}"
>
  <aside class="lib col" class:closed={!libOpen}>
    {#if libOpen}
      <button class="session" onclick={() => (showSessions = true)} title="Analysis sessions: open, create, rename">
        <Icon name="layers" />
        <span class="grow">{session?.name ?? "All videos"}</span>
        <Icon name="chevronDown" size={14} />
      </button>
      <div class="row">
        <h3 class="grow">{session ? "Session videos" : "Videos"}</h3>
        <button class="ib" title="Compare two videos" onclick={() => (showCompare = true)}><Icon name="columns" /></button>
        <button class="ib" title="Hide video list" onclick={() => (libOpen = false)}><Icon name="chevronsLeft" /></button>
      </div>
      <button class="primary" onclick={importVideos}>+ Import video files</button>
      <div class="list scroll grow">
        {#each shownVideos as v (v.id)}
          <div class="item vid" class:on={v.id === currentId} role="button" tabindex="0" onclick={() => selectVideo(v.id)} onkeydown={(e) => e.key === "Enter" && selectVideo(v.id)}>
            {#if v.thumb}<img src={fileUrl(v.thumb)} alt="" />{:else}<div class="thumb"><Icon name={v.source === "youtube" ? "youtube" : "film"} size={18} /></div>{/if}
            <div class="grow" style="min-width:0">
              <div class="name" title={v.name}>{v.name}</div>
              <div class="small muted">
                {#if v.source === "youtube"}YouTube
                {:else if v.proxy_status === "working"}Preparing {Math.round((progress[v.id] ?? 0) * 100)}%
                {:else if v.proxy_status === "queued"}Waiting to prepare
                {:else if v.proxy_status === "failed"}<span class="badge bad">Can't read file</span>
                {:else if v.proxy_status === "no-ffmpeg"}<span class="badge warn">FFmpeg missing</span>
                {:else}{fmtTime(v.duration)}{/if}
                · {tags.filter((t) => t.video_id === v.id).length} tags
              </div>
            </div>
            <button class="ghost small" title="Rename" onclick={(e) => { e.stopPropagation(); renameVideo(v); }}><Icon name="pencil" size={14} /></button>
            <button class="ghost small" title={session ? "Remove from this session" : "Remove from library"} onclick={(e) => { e.stopPropagation(); session ? removeFromSession(v) : removeVideo(v); }}><Icon name="x" size={14} /></button>
          </div>
        {:else}
          <div class="empty small">{session ? "No videos in this session yet. Import one, or add one from the library below." : "No videos yet."}</div>
        {/each}
      </div>
      {#if session && otherVideos.length}
        <select bind:value={addPick} onchange={async () => { if (addPick) { await addToSession([addPick]); selectVideo(addPick); addPick = ""; } }}>
          <option value="">+ Add a video from the library…</option>
          {#each otherVideos as v}<option value={v.id}>{v.name}</option>{/each}
        </select>
      {/if}
      {#if current}
        <label class="field">Match for this video
          <select value={current.match_id ?? ""} onchange={(e) => setMatch(e.currentTarget.value)}>
            <option value="">—</option>
            {#each matches as m}<option value={m.id}>{m.date} vs {m.opponent}</option>{/each}
          </select>
        </label>
      {/if}
      <button class="small" onclick={toggleMediaPause}>{mediaPaused ? "Resume" : "Pause"} background processing</button>
    {:else}
      <button class="ib" title="Show video list" onclick={() => (libOpen = true)}><Icon name="chevronsRight" /></button>
    {/if}
  </aside>

  <section class="center">
    <div class="stagebox" bind:this={stageBox}>
      {#if popped}
        <div class="popped" style="width:{stageW}px">
          <div>The video is playing in its own window.</div>
          <div class="muted small">Drag that window to your external display. The controls stay here.</div>
          <button onclick={() => closeSecondScreen()}>Bring video back</button>
        </div>
      {:else}
        <div style="width:{stageW}px">
          <VideoStage video={current} {player} bind:shapes bind:videoEl {drawMode} {tool} {color} progress={progress[currentId] ?? 0} onready={onReady} />
        </div>
      {/if}
    </div>

    <div class="controls" style="width:{stageW}px">
      <div class="seekrow">
        <span class="time">{fmtTime(player.time, true)}</span>
        <input
          type="range" class="seek" min="0" max={player.duration || 1} step="0.05" value={player.time}
          style="--p:{player.duration ? (player.time / player.duration) * 100 : 0}%"
          oninput={(e) => seek(player, Number(e.currentTarget.value))}
        />
        <span class="time muted">{fmtTime(player.duration)}</span>
      </div>

      <div class="ctlrow">
        {#if !drawMode}
          <div class="grp">
            <button class="ib" onclick={() => seek(player, player.time - 5)} title="Back 5 s (←)"><Icon name="rewind" /></button>
            <button class="ib" onclick={() => player.source?.step(-1)} title="Previous frame (,)"><Icon name="frameBack" /></button>
            <button class="ib play" onclick={() => togglePlay(player)} title="Play / pause (Space)"><Icon name={player.paused ? "play" : "pause"} /></button>
            <button class="ib" onclick={() => player.source?.step(1)} title="Next frame (.)"><Icon name="frameFwd" /></button>
            <button class="ib" onclick={() => seek(player, player.time + 5)} title="Forward 5 s (→)"><Icon name="forward" /></button>
            <select class="rate" title="Speed" value={player.rate} onchange={(e) => { player.rate = Number(e.currentTarget.value); player.source?.setRate(player.rate); }}>
              {#each player.source?.rates() ?? [1] as r}<option value={r}>{r}×</option>{/each}
            </select>
          </div>
          <div class="grp">
            <button class="ib txt" class:on={inPt != null} onclick={() => (inPt = player.time)} title="Set In point ([)">In{inPt != null ? " " + fmtTime(inPt) : ""}</button>
            <button class="ib txt" class:on={outPt != null} onclick={() => (outPt = player.time)} title="Set Out point (])">Out{outPt != null ? " " + fmtTime(outPt) : ""}</button>
            <button class="ib" class:on={player.loop} onclick={toggleLoop} title="Loop In→Out"><Icon name="repeat" /></button>
            <button class="ib" onclick={saveInOut} title="Save In→Out as a clip"><Icon name="scissors" /></button>
            <button class="ib" class:on={swStart != null} title={swStart == null ? "Stopwatch: start" : "Stopwatch: stop"} onclick={() => {
              if (swStart == null) swStart = player.time;
              else { toast(`Stopwatch: ${(player.time - swStart).toFixed(2)} s`); swStart = null; }
            }}><Icon name="timer" />{swStart != null ? " " + (player.time - swStart).toFixed(1) + "s" : ""}</button>
          </div>
          <span class="spacer"></span>
          <div class="grp">
            <button class="ib txt" onclick={startDrawing} disabled={!current} title="Draw on the video"><Icon name="pencil" /> {nearDrawing ? "Edit drawing" : "Draw"}</button>
            {#if popped}
              <button class="ib on" onclick={() => closeSecondScreen()} title="Bring the video back into this window"><Icon name="popin" /></button>
            {:else}
              <button class="ib" class:on={popMode === "mirror"} onclick={() => (popMode === "mirror" ? closeSecondScreen() : openSecondScreen("mirror"))} disabled={!current}
                title={popMode === "mirror" ? "Stop showing on the second screen" : "Show on both screens (duplicate, for team talks)"}><Icon name="mirror" /></button>
              <button class="ib" onclick={() => openSecondScreen("move")} disabled={!current} title="Move the video to another window or screen"><Icon name="popout" /></button>
            {/if}
          </div>
        {:else}
          <div class="grp">
            {#each TOOLS as tl}<button class="ib txt" class:on={tool === tl.id} onclick={() => (tool = tl.id)}>{tl.label}</button>{/each}
            <input type="color" bind:value={color} title="Colour" class="swatch" />
          </div>
          <span class="spacer"></span>
          <div class="grp">
            <button class="ib" onclick={() => (shapes = shapes.slice(0, -1))} title="Undo"><Icon name="undo" /></button>
            <button class="ib" onclick={() => (shapes = [])} title="Clear"><Icon name="eraser" /></button>

            <button class="ib txt" onclick={snapshotToClip} title="Attach a snapshot to the clip at this time">Snapshot</button>
            <button class="ib txt primary" onclick={saveDrawing}>Save</button>
            <button class="ib txt" onclick={() => (drawMode = false)}>Cancel</button>
          </div>
        {/if}
      </div>
      {#if nearDrawing && !drawMode}
        <div class="muted small">Stopped on a drawing. Press play (Space) to carry on.</div>
      {/if}
      {#if activeClip}
        <div class="clipbar small">
          <Icon name="film" size={14} />
          <span class="grow">{queue ? `Playlist: clip ${queue.i + 1} of ${queue.items.length}, ` : "Clip: "}{activeClip.name}</span>
          <button class="small" onclick={exitClip}>Back to full video</button>
        </div>
      {/if}
      {#if !player.source?.frameExact && current?.source === "youtube"}<div class="muted small">Frame steps are approximate on YouTube.</div>{/if}
    </div>
  </section>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="split v" onpointerdown={(e) => dragSplit(e, "v")} title="Drag to resize"></div>

  <aside class="tagging col">
    <div class="row">
      <select class="grow" value={panel?.id} onchange={(e) => (panelId = e.currentTarget.value)} title="Tagging panel">
        {#each panels as p}<option value={p.id}>{p.name}</option>{/each}
      </select>
      <button class="ib" title="Edit this panel" onclick={() => (editPanel = true)}><Icon name="pencil" /></button>
      <button class="ib" title="New panel" onclick={() => { panelId = null; editPanel = "new"; }}><Icon name="plus" /></button>
    </div>
    <div class="row small muted">
      Period
      <select class="grow" bind:value={period}>{#each template?.periods ?? [] as p}<option>{p}</option>{/each}</select>
    </div>
    <div class="tagscroll">
      <TagPanel {panel} ontag={addTag} />
    </div>

    {#if lastTag}
      <div class="card col last">
        <div class="row">
          <i class="dot" style="background:{lastTag.color}"></i>
          <b class="grow">{lastTag.code} · {fmtTime(lastTag.t)}</b>
          <button class="ib danger" title="Delete this tag" onclick={() => deleteTags([lastTag.id])}><Icon name="trash" /></button>
          <button class="ib primary" title="Done (Esc)" onclick={closeCard}><Icon name="check" /></button>
        </div>
        {#if lastButton?.descriptors?.length}
          <div class="chips">
            {#each lastButton.descriptors as d}
              <button class="small" class:on={lastTag.descriptor === d} onclick={() => editTag({ descriptor: lastTag.descriptor === d ? null : d })}>{d}</button>
            {/each}
          </div>
        {/if}
        <div class="chips">
          {#each squad as p}
            <button class="small" class:on={lastTag.player_id === p.id} onclick={() => editTag({ player_id: lastTag.player_id === p.id ? null : p.id })} title={p.name}>
              {p.number ?? p.name?.slice(0, 3)}
            </button>
          {:else}
            <span class="muted small">Add players to assign them to tags.</span>
          {/each}
        </div>
        {#if showLocation}
          <PitchDiagram sport={app.team.sport} height={100} points={lastTag.x != null ? [{ x: lastTag.x, y: lastTag.y, color: lastTag.color }] : []} onpick={(x, y) => { editTag({ x, y }); showLocation = false; }} />
        {:else}
          <button class="small ghost" onclick={() => { showLocation = true; }}><Icon name="mapPin" /> {lastTag.x != null ? "Change location" : "Mark location"}</button>
        {/if}
      </div>
    {/if}
  </aside>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="split h" class:hidden={!bottomOpen} onpointerdown={(e) => dragSplit(e, "h")} title="Drag to resize"></div>

  <section class="bottom col">
    <div class="tabs row">
      {#each [["timeline", `Timeline (${shownTags.length})`], ["clips", "Clips & playlists"], ["stats", "Stats"]] as [k, l]}
        <button class:on={bottomOpen && tab === k} onclick={() => { if (k !== "clips") exitClip(); if (bottomOpen && tab === k) bottomOpen = false; else { tab = k; bottomOpen = true; } }}>{l}</button>
      {/each}
      <span class="spacer"></span>
      {#if bottomOpen && tab === "timeline"}
        <input placeholder="Filter: turnovers, 2nd half, #7" bind:value={filter} style="width:260px;padding:4px 8px" />
        {#if selected.length}
          <button class="small" onclick={() => clipsFromTags(selected)}>Clips from {selected.length} selected</button>
          <button class="small danger" onclick={() => deleteTags(selected)}>Delete</button>
        {:else if shownTags.length}
          <button class="small" onclick={() => clipsFromTags(shownTags.map((t) => t.id))}>Clips from all {shownTags.length}</button>
        {/if}
      {/if}
      <button class="ib" title={bottomOpen ? "Collapse" : "Expand"} onclick={() => (bottomOpen = !bottomOpen)}><Icon name={bottomOpen ? "chevronDown" : "chevronUp"} /></button>
    </div>
    {#if bottomOpen}
      <div class="pane scroll">
        {#if tab === "timeline"}
          <Timeline tags={shownTags} duration={player.duration || current?.duration} time={player.time} drawings={videoDrawings} bind:selected labelOf={tagLabel}
            onseek={(t, tag) => { exitClip(); seek(player, t); if (tag) { lastTagId = tag.id; player.source?.play(); } }} />
        {:else if tab === "clips"}
          <ClipsPane {clips} {playlists} {videos} {drawings} {current} onplayclip={(c) => playClip(c)} onplayplaylist={playPlaylist}
            onchange={async () => { clips = await db.list("clip"); playlists = await db.list("playlist"); }} />
        {:else}
          <StatsPane {current} tags={videoTags} {colors} sport={app.team.sport} />
        {/if}
      </div>
    {/if}
  </section>
</div>

{#if editPanel}
  <PanelEditor panel={editPanel === "new" ? null : panel} onclose={async (saved) => { editPanel = false; if (saved) panels = await db.list("panel"); }} />
{/if}
{#if showCompare}<Compare {videos} onclose={() => (showCompare = false)} />{/if}
{#if showSessions}
  <SessionsDialog {sessions} {videos} {tags} {matches} currentId={sessionId} currentVideoId={currentId}
    onopen={(id) => openSession(id)} onclose={() => (showSessions = false)}
    onchange={async () => (sessions = await db.list("analysis_session"))} />
{/if}

<style>
  .analysis {
    display: grid; height: 100%; padding: 8px; column-gap: 0; row-gap: 0;
    grid-template-columns: var(--lib) minmax(0, 1fr) 8px var(--tag);
    grid-template-rows: minmax(0, 1fr) 8px var(--bottom);
    grid-template-areas: "lib center splitv tag" "splith splith splith splith" "bottom bottom bottom bottom";
  }
  .lib { grid-area: lib; margin-right: 8px; }
  .center { grid-area: center; }
  .split.v { grid-area: splitv; }
  .tagging { grid-area: tag; }
  .split.h { grid-area: splith; }
  .bottom { grid-area: bottom; }

  .lib, .tagging { background: var(--panel); border: 1px solid var(--line); border-radius: var(--radius); padding: 8px; min-height: 0; overflow: auto; }
  .lib.closed { padding: 4px; align-items: center; }
  .center { min-width: 0; min-height: 0; display: flex; flex-direction: column; align-items: center; gap: 6px; }
  .stagebox { flex: 1; min-height: 120px; width: 100%; display: flex; align-items: center; justify-content: center; overflow: hidden; }
  .popped { aspect-ratio: 16 / 9; background: var(--input); border: 1px dashed var(--line); border-radius: var(--radius); display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; text-align: center; padding: 12px; }

  .split { position: relative; }
  .split::after { content: ""; position: absolute; border-radius: 2px; background: transparent; transition: background .15s; }
  .split:hover::after { background: var(--accent); }
  .split.v { cursor: col-resize; }
  .split.v::after { left: 3px; width: 2px; top: 30%; bottom: 30%; }
  .split.h { cursor: row-resize; }
  .split.h::after { top: 3px; height: 2px; left: 40%; right: 40%; }
  .split.h.hidden { pointer-events: none; }

  .controls { display: flex; flex-direction: column; gap: 4px; max-width: 100%; min-width: 360px; }
  .seekrow { display: flex; align-items: center; gap: 8px; }
  .time { font-variant-numeric: tabular-nums; font-size: 12px; min-width: 52px; }
  .seekrow .time:last-child { text-align: right; }
  .seek { flex: 1; -webkit-appearance: none; appearance: none; height: 4px; border-radius: 2px; cursor: pointer; border: 0; padding: 0;
    background: linear-gradient(to right, var(--accent) var(--p), var(--line) var(--p)); }
  .seek:hover { height: 6px; }
  .seek::-webkit-slider-thumb { -webkit-appearance: none; width: 12px; height: 12px; border-radius: 50%; background: #fff; border: 0; box-shadow: 0 0 0 2px var(--accent); }
  .ctlrow { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .grp { display: flex; align-items: center; gap: 3px; }
  .ib { min-width: 30px; height: 30px; padding: 0 7px; display: inline-flex; align-items: center; justify-content: center; gap: 4px; font-size: 14px; line-height: 1; }
  .ib.txt { font-size: 12px; }
  .ib.play { min-width: 42px; background: var(--accent-2); border-color: var(--accent); }
  .rate { height: 30px; padding: 0 4px; font-size: 12px; }
  .swatch { width: 30px; height: 30px; padding: 0; }

  .tagging { display: flex; flex-direction: column; gap: 6px; }
  .tagscroll { flex: 1; min-height: 80px; overflow: auto; }
  .last { gap: 6px; padding: 8px; flex: none; border-color: var(--line-2); }
  .dot { width: 10px; height: 10px; border-radius: 2px; display: inline-block; }
  .chips { display: flex; flex-wrap: wrap; gap: 4px; }
  .chips button { min-width: 34px; padding: 3px 6px; }

  .bottom { background: var(--panel); border: 1px solid var(--line); border-radius: var(--radius); padding: 0 10px 6px; min-height: 0; }
  .bottom .tabs { padding-top: 4px; gap: 4px; }
  .pane { flex: 1; min-height: 0; padding-top: 8px; overflow-x: hidden; }

  .clipbar { display: flex; align-items: center; gap: 8px; padding: 4px 8px; border: 1px solid var(--line-2); border-radius: var(--radius); background: var(--selected); }
  .session { display: flex; align-items: center; gap: 8px; width: 100%; text-align: left; font-weight: 600; background: var(--selected); border-color: var(--line-2); }
  .session span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .vid { display: flex; gap: 8px; align-items: center; padding: 6px; }
  .vid img, .thumb { width: 64px; height: 36px; object-fit: cover; border-radius: 3px; background: #000; display: grid; place-items: center; font-size: 11px; flex: none; }
  .name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
