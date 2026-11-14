<script>
  import Icon from "$shared/Icon.svelte";
  import { db, invoke, playerLabel, sortPlayers, today, ask } from "$shared/api.js";
  import { attempt, toast, refreshInfo } from "../state.svelte.js";
  import { saveForPc } from "../upload.js";

  let notes = $state([]);
  let matches = $state([]);
  let sessions = $state([]);
  let players = $state([]);
  let outbox = $state([]);
  let target = $state("");
  let text = $state("");
  let recorder = $state(null);
  let chunks = [];
  let progress = $state(null);
  let videoInput;

  async function load() {
    [notes, matches, sessions, players] = await Promise.all(["note", "match", "session", "player"].map((k) => db.list(k)));
    outbox = await invoke("outbox");
  }
  load();

  const targets = $derived([
    ...[...matches].sort((a, b) => (b.date || "").localeCompare(a.date || "")).slice(0, 5).map((m) => [`match:${m.id}`, `Match: ${m.date} vs ${m.opponent}`]),
    ...[...sessions].sort((a, b) => (b.date || "").localeCompare(a.date || "")).slice(0, 5).map((s) => [`session:${s.id}`, `Session: ${s.date} ${s.title}`]),
    ...sortPlayers(players).map((p) => [`player:${p.id}`, `Player: ${playerLabel(p)}`]),
  ]);
  function link() {
    const [kind, id] = target.split(":");
    return kind ? { [`${kind}_id`]: id } : {};
  }
  const targetName = (n) => targets.find(([k]) => (n.match_id && k === `match:${n.match_id}`) || (n.session_id && k === `session:${n.session_id}`) || (n.player_id && k === `player:${n.player_id}`))?.[1] ?? "";

  async function addNote() {
    if (!text.trim()) return;
    await attempt(() => db.put("note", { date: today(), text: text.trim(), ...link() }), "Note saved");
    text = "";
    load();
  }

  async function delNote(n) {
    if (!await ask(`Delete this note${n.memo ? " (and its voice memo link)" : ""}?`)) return;
    await attempt(() => db.del("note", n.id), "Note deleted");
    load();
  }

  async function startMemo() {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      chunks = [];
      const r = new MediaRecorder(stream);
      r.ondataavailable = (e) => chunks.push(e.data);
      r.onstop = async () => {
        stream.getTracks().forEach((t) => t.stop());
        const blob = new Blob(chunks, { type: r.mimeType || "audio/webm" });
        const note = await db.put("note", { date: today(), text: text.trim() || "Voice memo", memo: true, ...link() });
        await saveForPc(blob, "memo.webm", "memo", { note_id: note.id });
        text = "";
        toast("Voice memo saved");
        load();
        refreshInfo();
      };
      r.start();
      recorder = r;
    } catch (e) {
      toast("Microphone not available: " + (e.message || e), "error");
    }
  }
  function stopMemo() {
    recorder?.stop();
    recorder = null;
  }

  async function onVideo(e) {
    const file = e.currentTarget.files?.[0];
    if (!file) return;
    progress = 0;
    const meta = { name: file.name || `Phone video ${today()}`, ...link() };
    await attempt(() => saveForPc(file, file.name || "video.mp4", "video", meta, (p) => (progress = p)), "Video queued for the PC");
    progress = null;
    e.currentTarget.value = "";
    load();
    refreshInfo();
  }
</script>

<section class="col">
  <select bind:value={target}>
    <option value="">Not linked</option>
    {#each targets as [k, l]}<option value={k}>{l}</option>{/each}
  </select>
  <textarea rows="3" placeholder="Quick note…" bind:value={text}></textarea>
  <div class="grid2">
    <button class="primary" onclick={addNote}>Save note</button>
    {#if recorder}<button class="danger" onclick={stopMemo}><Icon name="stop" /> Stop memo</button>{:else}<button onclick={startMemo}><Icon name="mic" /> Voice memo</button>{/if}
  </div>
</section>

<section class="card col">
  <h3>Match video</h3>
  <div class="muted small">Record with the camera (or pick a video you filmed). It goes to the PC at the next sync, over the same Wi-Fi.</div>
  <input type="file" accept="video/*" capture="environment" bind:this={videoInput} onchange={onVideo} style="display:none" />
  <button onclick={() => videoInput.click()} disabled={progress != null}><Icon name="video" /> {progress != null ? `Saving ${Math.round(progress * 100)}%` : "Record / choose video"}</button>
  {#each outbox as f}<div class="small muted">{f.kind}: {f.meta?.name ?? ""} · {Math.round(f.size / 1e6)} MB waiting</div>{/each}
</section>

<section class="list card" style="padding:0">
  {#each [...notes].sort((a, b) => (b.date || "").localeCompare(a.date || "")) as n (n.id)}
    <div class="item row">
      <div class="grow"><div>{#if n.memo}<Icon name="mic" size={14} /> {/if}{n.text}</div><div class="small muted">{n.date} {targetName(n)}</div></div>
      <button class="ghost danger small" aria-label="Delete note" onclick={() => delNote(n)}><Icon name="trash" size={16} /></button>
    </div>
  {:else}<div class="empty">No notes yet.</div>{/each}
</section>
