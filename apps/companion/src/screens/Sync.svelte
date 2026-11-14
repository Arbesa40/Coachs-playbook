<script>
  import Icon from "$shared/Icon.svelte";
  import { invoke, ask } from "$shared/api.js";
  import { ui, attempt, toast, refreshInfo } from "../state.svelte.js";

  let busy = $state(false);
  let last = $state(null);
  let code = $state("");
  let conflicts = $state(0);

  if (ui.info?.paired) invoke("conflict_count").then((n) => (conflicts = n)).catch(() => {});

  // Ask for the camera first, then scan with the camera behind a transparent page.
  let scanning = $state(false);
  let scanError = $state("");
  async function scan() {
    scanError = "";
    let mod;
    try {
      mod = await import("@tauri-apps/plugin-barcode-scanner");
    } catch {
      scanError = "Camera scanning isn't available on this device. Paste the pairing code instead.";
      return;
    }
    const { scan, cancel, Format, checkPermissions, requestPermissions, openAppSettings } = mod;
    try {
      let perm = await checkPermissions();
      if (perm !== "granted") perm = await requestPermissions();
      if (perm !== "granted") {
        scanError = "Camera permission is off. Allow the camera for Coach's Playbook in Android settings, then try again.";
        if (await ask("Camera access is needed to scan the code. Open the app's settings now?")) await openAppSettings();
        return;
      }
      scanning = true;
      document.documentElement.classList.add("scanning");
      cancelScan = () => cancel().catch(() => {});
      const r = await scan({ windowed: true, formats: [Format.QRCode] });
      if (r?.content) await pair(r.content);
    } catch (e) {
      const msg = String(e?.message ?? e ?? "");
      if (!/cancel/i.test(msg)) scanError = "Scanning failed: " + (msg || "unknown error") + ". You can paste the pairing code instead.";
    } finally {
      scanning = false;
      document.documentElement.classList.remove("scanning");
    }
  }
  let cancelScan = () => {};
  async function pair(c) {
    busy = true;
    const r = await attempt(() => invoke("pair", { code: c.trim() }), "Paired and synced");
    busy = false;
    if (r) last = r;
    await refreshInfo();
  }
  async function syncNow() {
    busy = true;
    const r = await attempt(() => invoke("sync_now"));
    busy = false;
    if (r) {
      last = r;
      toast(`Synced: got ${r.pulled.applied}, sent ${r.sent_changes}${r.files_sent ? `, ${r.files_sent} file(s)` : ""}`);
      conflicts = r.conflicts_total;
    }
    await refreshInfo();
  }
</script>

{#if ui.info?.paired}
  <section class="col">
    <div class="card col">
      <div>Paired with <b>{ui.info.pc_name}</b> for <b>{ui.info.team_name}</b>.</div>
      <div class="muted small">On the PC, open Sync & data and press Open sync. Both devices need the same Wi-Fi, or connect the PC to this phone's hotspot. If it can't connect, allow Coach's Playbook through the Windows firewall on private networks.</div>
      <button class="primary big" onclick={syncNow} disabled={busy}><Icon name="sync" /> {busy ? "Syncing…" : "Sync now"}</button>
      {#if ui.info.outbox}<div class="small muted">{ui.info.outbox} file(s) will be sent.</div>{/if}
      {#if conflicts}<div class="badge warn">{conflicts} change(s) need a decision on the PC</div>{/if}
    </div>
    {#if last}
      <div class="small muted">Last sync: received {last.pulled.applied}, sent {last.sent_changes}{last.files_sent ? `, ${last.files_sent} file(s)` : ""}.</div>
    {/if}
  </section>
{/if}

{#if scanning}
  <div class="scanner">
    <div class="frame"></div>
    <div class="msg">Point the camera at the QR code on the PC</div>
    <button class="big" onclick={() => cancelScan()}><Icon name="x" /> Cancel</button>
  </div>
{/if}

<section class="card col">
  <h3>{ui.info?.paired ? "Pair again / switch PC" : "Pair with your PC"}</h3>
  <ol class="small muted" style="padding-left:18px;margin:0">
    <li>On the PC, open Sync & data and press Open sync.</li>
    <li>Tap Scan and point the camera at the QR code.</li>
  </ol>
  <button class="primary big" onclick={scan} disabled={busy || scanning}><Icon name="qr" /> Scan QR code</button>
  {#if scanError}<div class="badge bad" style="white-space:normal;padding:6px 8px">{scanError}</div>{/if}
  {#if busy && !ui.info?.paired}<div class="muted small">Connecting to the PC…</div>{/if}
  <details>
    <summary class="small muted">No camera? Paste the code</summary>
    <div class="col" style="margin-top:6px">
      <input placeholder="coachpb://pair?d=…" bind:value={code} />
      <button onclick={() => pair(code)} disabled={!code.trim() || busy}>Pair</button>
    </div>
  </details>
</section>

<style>
  .scanner { position: fixed; inset: 0; z-index: 100; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 24px; background: transparent; visibility: visible; }
  .scanner .frame { width: 70vw; height: 70vw; max-width: 320px; max-height: 320px; border: 3px solid #fff; border-radius: 16px; box-shadow: 0 0 0 100vmax #0008; }
  .scanner .msg { color: #fff; text-shadow: 0 1px 3px #000; z-index: 1; }
  .scanner button { z-index: 1; background: #000a; border-color: #fff; color: #fff; }
</style>
