<script>
  import { db, invoke, byKey, ask } from "$shared/api.js";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import { app, attempt, toast } from "../lib/state.svelte.js";

  let server = $state(null);
  let log = $state([]);
  let peers = $state([]);
  let conflicts = $state([]);
  let videos = $state([]);
  let csvKind = $state("player");
  let xmlVideo = $state("");

  const refresh = async () => {
    peers = await invoke("sync_peers");
    conflicts = await invoke("conflicts");
  };
  refresh();
  db.list("video").then((v) => (videos = v.filter((x) => x.source === "local" || x.source === "youtube")));

  $effect(() => {
    const off = listen("sync-event", (e) => {
      const p = e.payload;
      const time = new Date().toLocaleTimeString();
      if (p.state === "connected") log = [`${time} Phone connected (${p.from})`, ...log];
      if (p.state === "error") log = [`${time} Sync failed: ${p.message}`, ...log];
      if (p.state === "done") {
        const r = p.report;
        log = [`${time} Synced with ${r.peer_name}: received ${r.pulled.applied} changes, sent ${r.sent_changes}, ${r.files_received.length} video(s)`, ...log];
        refresh();
      }
    });
    // The sync port is only open while this screen is open.
    return () => {
      off.then((f) => f());
      invoke("sync_close");
    };
  });

  async function openSync() {
    server = await attempt(() => invoke("sync_open"));
  }
  function closeSync() {
    invoke("sync_close");
    server = null;
  }
  async function resolve(c, keepLocal) {
    await attempt(() => invoke("resolve_conflict", { id: c.id, keepLocal }));
    refresh();
  }

  const stamp = () => new Date().toISOString().slice(0, 10);
  const safe = (s) => (s || "team").replace(/[^\w\- ]+/g, "");
  async function exportSyncFile() {
    const path = await save({ defaultPath: `${safe(app.team.name)}-${stamp()}.sync`, filters: [{ name: "Sync file", extensions: ["sync"] }] });
    if (path) await attempt(() => invoke("export_sync_file", { path }), "Sync file saved. Copy it to the phone by USB, Bluetooth or email.");
  }
  async function importSyncFile() {
    const path = await open({ filters: [{ name: "Sync file", extensions: ["sync"] }] });
    if (!path) return;
    const r = await attempt(() => invoke("import_sync_file", { path }));
    if (r) toast(`Merged ${r.applied} changes${r.conflicts ? `, ${r.conflicts} need a decision` : ""}`);
    refresh();
  }
  async function backup() {
    const path = await save({ defaultPath: `${safe(app.team.name)}-${stamp()}.cpbackup`, filters: [{ name: "Backup", extensions: ["cpbackup"] }] });
    if (path) await attempt(() => invoke("backup", { path }), "Backup saved");
  }
  async function restore() {
    if (!await ask("Restoring replaces this team's data on this PC with the backup. Continue?")) return;
    const path = await open({ filters: [{ name: "Backup", extensions: ["cpbackup", "sqlite"] }] });
    if (path) app.team = (await attempt(() => invoke("restore", { path }), "Backup restored")) ?? app.team;
  }
  async function exportCsv() {
    const path = await save({ defaultPath: `${csvKind}s.csv`, filters: [{ name: "CSV", extensions: ["csv"] }] });
    if (path) await attempt(() => invoke("export_csv", { kind: csvKind, path }), "Exported");
  }
  async function importCsv() {
    const path = await open({ filters: [{ name: "CSV", extensions: ["csv"] }] });
    if (path) {
      const n = await attempt(() => invoke("import_csv", { kind: csvKind, path }));
      if (n != null) toast(`Imported ${n} ${csvKind} rows`);
    }
  }
  async function exportXml() {
    const v = videos.find((x) => x.id === xmlVideo);
    if (!v) return;
    const path = await save({ defaultPath: `${v.name}.xml`, filters: [{ name: "Sportscode / Nacsport XML", extensions: ["xml"] }] });
    if (path) await attempt(() => invoke("export_xml", { videoId: v.id, path }), "XML exported");
  }
  const kinds = ["player", "match", "match_event", "session", "attendance", "injury", "assessment", "tag", "clip", "drill", "note"];
</script>

<div class="page">
<div class="col" style="max-width:1100px">
  <div class="ph"><div><h1>Sync & data</h1><div class="sub">Pair the phone, import squads, back up and restore</div></div></div>
  <div class="grid2">
    <div class="card col">
      <h2>Phone sync over Wi-Fi</h2>
      {#if !server}
        <p class="muted">Put the phone and this PC on the same Wi-Fi, or connect the PC to the phone's hotspot. Then open sync and scan the code with the companion app.</p>
        <div><button class="primary" onclick={openSync}>Open sync</button></div>
      {:else}
        <div class="row" style="align-items:start">
          <div class="qr">{@html server.qr_svg}</div>
          <div class="col small">
            <b>Scan with the phone app → Sync → Pair</b>
            <span class="muted">Already paired phones just tap Sync.</span>
            <span class="muted">This PC: {server.hosts.join(", ")} port {server.port}</span>
            <span class="muted">If Windows asks about the firewall, allow access on private networks.</span>
            <button onclick={() => { navigator.clipboard.writeText(server.uri); toast("Pairing code copied"); }}>Copy pairing code</button>
            <button onclick={closeSync}>Close sync</button>
          </div>
        </div>
      {/if}
      {#each log as l}<div class="small muted">{l}</div>{/each}
      {#if peers.length}
        <h3>Paired devices</h3>
        {#each peers as p}<div class="small">{p.name || p.device_id}</div>{/each}
      {/if}
    </div>

    <div class="card col">
      <h2>Needs a decision {#if conflicts.length}<span class="badge warn">{conflicts.length}</span>{/if}</h2>
      {#each conflicts as c (c.id)}
        <div class="card col small" style="background:var(--panel-2)">
          <div><b>{c.kind}</b> field <b>{c.field}</b> was changed on both devices.</div>
          <div class="row">
            <button onclick={() => resolve(c, true)}>Keep this PC: {JSON.stringify(c.local_value)}</button>
            <button onclick={() => resolve(c, false)}>Keep phone: {JSON.stringify(c.remote_value)}</button>
          </div>
        </div>
      {:else}
        <div class="muted">Nothing to decide. Edits from both devices merged cleanly.</div>
      {/each}
      <h3 style="margin-top:12px">No network?</h3>
      <div class="row wrap">
        <button onclick={exportSyncFile}>Save .sync file</button>
        <button onclick={importSyncFile}>Load .sync file</button>
      </div>
      <div class="muted small">Move the file by USB, Bluetooth, email, or a Syncthing / cloud folder.</div>
    </div>

    <div class="card col">
      <h2>Backup</h2>
      <p class="muted">One file with everything for this team (not the video files themselves).</p>
      <div class="row"><button class="primary" onclick={backup}>Back up now</button><button onclick={restore}>Restore…</button></div>
    </div>

    <div class="card col">
      <h2>Import & export</h2>
      <div class="row wrap">
        <select bind:value={csvKind}>{#each kinds as k}<option value={k}>{k.replace("_", " ")}s</option>{/each}</select>
        <button onclick={exportCsv}>Export CSV</button>
        <button onclick={importCsv}>Import CSV</button>
      </div>
      <div class="row wrap">
        <select bind:value={xmlVideo} class="grow"><option value="">Video…</option>{#each videos as v}<option value={v.id}>{v.name}</option>{/each}</select>
        <button onclick={exportXml} disabled={!xmlVideo}>Export tags as XML</button>
      </div>
      <div class="muted small">XML follows the Sportscode format, which Nacsport and most analysis tools can import.</div>
    </div>
  </div>
</div>
</div>

<style>
  .qr { background: #fff; padding: 8px; border-radius: 6px; line-height: 0; }
  .qr { padding: 14px; }
  .qr :global(svg) { width: 300px; height: 300px; }
</style>
