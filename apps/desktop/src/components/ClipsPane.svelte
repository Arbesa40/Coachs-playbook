<script>
  import Icon from "$shared/Icon.svelte";
  import { db, invoke, fmtTime, byKey, ask } from "$shared/api.js";
  import { save } from "@tauri-apps/plugin-dialog";
  import { attempt, toast } from "../lib/state.svelte.js";
  import { youtubeLink } from "../lib/source.js";
  import { overlayPng } from "../lib/drawing.js";

  let { clips, playlists, videos, drawings, current, onplayclip, onplayplaylist, onchange } = $props();

  let onlyThis = $state(true);
  let plSel = $state(null);
  let newPl = $state("");
  const vById = $derived(byKey(videos, "id"));
  const cById = $derived(byKey(clips, "id"));
  const shown = $derived(
    [...clips]
      .filter((c) => !onlyThis || c.video_id === current?.id)
      .sort((a, b) => (a.order ?? a.start) - (b.order ?? b.start))
  );
  // Windows file names can't contain \ / : * ? " < > |
  const fileSafe = (s) => (s || "clip").replace(/[\\/:*?"<>|]+/g, "-").trim();
  const playlist = $derived(playlists.find((p) => p.id === plSel) ?? null);

  async function update(c, fields) {
    Object.assign(c, fields);
    await attempt(() => db.put("clip", fields, c.id));
  }
  async function remove(c) {
    if (!await ask(`Delete clip "${c.name}"?`)) return;
    await attempt(() => db.del("clip", c.id));
    onchange();
  }
  async function move(c, d) {
    const i = shown.indexOf(c);
    const other = shown[i + d];
    if (!other) return;
    const a = other.order ?? other.start, b = c.order ?? c.start;
    await update(c, { order: a });
    await update(other, { order: b === a ? a + d * 0.001 : b });
    onchange();
  }

  function drawingsFor(c) {
    return drawings.filter((d) => d.video_id === c.video_id && d.t >= c.start && d.t <= c.end).flatMap((d) => d.shapes);
  }

  async function exportClip(c, burn) {
    const v = vById[c.video_id];
    if (!v || v.source !== "local") return toast("YouTube clips can't be exported to MP4. Use share links instead.", "error");
    const out = await save({ defaultPath: `${fileSafe(c.name)}.mp4`, filters: [{ name: "MP4 video", extensions: ["mp4"] }] });
    if (!out) return;
    const shapes = burn ? drawingsFor(c) : [];
    const overlay = shapes.length && v.width ? overlayPng(shapes, v.width, v.height) : null;
    toast(overlay ? "Exporting with drawings (re-encoding)…" : "Exporting…");
    await attempt(
      () => invoke("export_clip", { videoId: v.id, start: c.start, end: c.end, out, overlayPng: overlay }),
      "Clip exported"
    );
  }

  async function createPlaylist() {
    if (!newPl.trim()) return;
    const p = await attempt(() => db.put("playlist", { name: newPl.trim(), items: [] }));
    newPl = "";
    if (p) {
      plSel = p.id;
      onchange();
    }
  }
  async function addTo(c) {
    if (!playlist) return toast("Choose or create a playlist first", "error");
    await attempt(() => db.put("playlist", { items: [...(playlist.items ?? []), c.id] }, playlist.id), "Added to " + playlist.name);
    onchange();
  }
  async function plMove(i, d) {
    const items = [...playlist.items];
    const j = i + d;
    if (j < 0 || j >= items.length) return;
    [items[i], items[j]] = [items[j], items[i]];
    await attempt(() => db.put("playlist", { items }, playlist.id));
    onchange();
  }
  async function plRemove(i) {
    const items = playlist.items.filter((_, k) => k !== i);
    await attempt(() => db.put("playlist", { items }, playlist.id));
    onchange();
  }
  async function sharePackage() {
    const out = await save({ defaultPath: `${fileSafe(playlist.name)}.zip`, filters: [{ name: "ZIP", extensions: ["zip"] }] });
    if (!out) return;
    toast("Building share package…");
    const r = await attempt(() => invoke("share_package", { playlistId: playlist.id, out }), "Share package ready");
    if (r) invoke("reveal", { path: r }).catch(() => {});
  }
  function copyLinks() {
    const lines = (playlist.items ?? []).map((id, i) => {
      const c = cById[id];
      const v = c && vById[c.video_id];
      if (!c || !v) return null;
      const where = v.source === "youtube" ? youtubeLink(v.youtube_id, c.start) : `${v.name} @ ${fmtTime(c.start)}–${fmtTime(c.end)}`;
      return `${i + 1}. ${c.name}: ${where}${c.notes ? "\n   " + c.notes : ""}`;
    });
    navigator.clipboard.writeText(`${playlist.name}\n` + lines.filter(Boolean).join("\n"));
    toast("Links copied. Paste them into WhatsApp or any messaging app.");
  }
  async function deletePlaylist() {
    if (!await ask(`Delete playlist "${playlist.name}"? Clips are kept.`)) return;
    await attempt(() => db.del("playlist", playlist.id));
    plSel = null;
    onchange();
  }
</script>

<div class="grid2" style="height:100%;min-height:0">
  <div class="col scroll">
    <div class="row">
      <h3 class="grow">Clips</h3>
      <label class="small muted row"><input type="checkbox" bind:checked={onlyThis} /> This video only</label>
    </div>
    {#each shown as c (c.id)}
      <div class="card col clip">
        <div class="row">
          <button class="ghost" title="Play" onclick={() => onplayclip(c)}><Icon name="play" size={14} /></button>
          <input class="grow" value={c.name} onchange={(e) => update(c, { name: e.currentTarget.value })} />
          <span class="muted small">{fmtTime(c.start)}–{fmtTime(c.end)}</span>
          {#if vById[c.video_id]?.source === "youtube"}<span class="badge">YouTube</span>{/if}
        </div>
        <textarea rows="1" placeholder="Notes for players" value={c.notes ?? ""} onchange={(e) => update(c, { notes: e.currentTarget.value })}></textarea>
        <div class="row wrap small">
          <button class="ghost" onclick={() => move(c, -1)} title="Move up"><Icon name="arrowUp" size={14} /></button>
          <button class="ghost" onclick={() => move(c, 1)} title="Move down"><Icon name="arrowDown" size={14} /></button>
          <button onclick={() => addTo(c)} disabled={!playlist}><Icon name="plus" size={14} /> Playlist</button>
          {#if vById[c.video_id]?.source === "local"}
            <button onclick={() => exportClip(c, false)}>MP4</button>
            <button onclick={() => exportClip(c, true)} title="Re-encodes to burn in drawings">MP4 + drawings</button>
          {/if}
          <span class="spacer"></span>
          <button class="ghost danger" onclick={() => remove(c)}>Delete</button>
        </div>
      </div>
    {:else}
      <div class="empty">No clips. Make clips from tags (select tags in the timeline) or set In/Out points.</div>
    {/each}
  </div>

  <div class="col scroll">
    <h3>Playlists</h3>
    <div class="row">
      <select class="grow" bind:value={plSel}>
        <option value={null}>Choose a playlist…</option>
        {#each playlists as p}<option value={p.id}>{p.name} ({p.items?.length ?? 0})</option>{/each}
      </select>
    </div>
    <div class="row">
      <input class="grow" placeholder="New playlist, e.g. Pressing – Sat vs Rangers" bind:value={newPl} onkeydown={(e) => e.key === "Enter" && createPlaylist()} />
      <button onclick={createPlaylist}>Create</button>
    </div>
    {#if playlist}
      <div class="row wrap">
        <button class="primary" onclick={() => onplayplaylist(playlist)} disabled={!playlist.items?.length}><Icon name="play" size={14} /> Play all</button>
        <button onclick={sharePackage} disabled={!playlist.items?.length}>Share package (.zip)</button>
        <button onclick={copyLinks} disabled={!playlist.items?.length}>Copy links</button>
        <span class="spacer"></span>
        <button class="ghost danger" onclick={deletePlaylist}>Delete</button>
      </div>
      <div class="list">
        {#each playlist.items ?? [] as id, i}
          {@const c = cById[id]}
          <div class="item row">
            <span class="muted">{i + 1}.</span>
            <span class="grow">{c?.name ?? "(deleted clip)"}</span>
            {#if c}<span class="muted small">{vById[c.video_id]?.name}</span>{/if}
            <button class="ghost" onclick={() => plMove(i, -1)}><Icon name="arrowUp" size={14} /></button>
            <button class="ghost" onclick={() => plMove(i, 1)}><Icon name="arrowDown" size={14} /></button>
            <button class="ghost danger" onclick={() => plRemove(i)}><Icon name="x" size={14} /></button>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .clip { padding: 8px; gap: 6px; }
  .clip textarea { min-height: 30px; }
</style>
