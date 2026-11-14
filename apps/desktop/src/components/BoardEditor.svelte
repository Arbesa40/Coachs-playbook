<script>
  // Desktop editor around the shared TacticsBoard: tools, selection, undo/redo, style, second screen.
  import { db } from "$shared/api.js";
  import Icon from "$shared/Icon.svelte";
  import TacticsBoard from "$shared/TacticsBoard.svelte";
  import { EQUIPMENT, LINE_KINDS, lineLabel } from "$shared/sports.js";
  import { DEFAULT_STYLE, normalize } from "$shared/board.js";
  import { emit, listen } from "@tauri-apps/api/event";
  import { app, teamCfg, toast } from "../lib/state.svelte.js";
  import { openBoardWindow, closeBoardWindow } from "../lib/popout.js";

  let { board = $bindable(), title = "", height = 420, onchange = null } = $props();

  const cfg = $derived(teamCfg());
  let tb;
  let tool = $state("select");
  let frame = $state(0);
  let t = $state(null);
  let playing = $state(false);
  let selected = $state(null);
  let newColor = $state("");
  let style = $state({ ...DEFAULT_STYLE });
  let showStyle = $state(false);
  let presenting = $state(false);

  db.get("board_style", "default").then((s) => s && (style = { ...DEFAULT_STYLE, ...s }));
  async function saveStyle(fields) {
    style = { ...style, ...fields };
    await db.put("board_style", fields, "default");
  }

  // Keep the step index valid when a different board is loaded.
  $effect(() => {
    const n = 1 + (normalize(board).steps?.length ?? 0);
    if (frame > n - 1) frame = n - 1;
  });

  const b = $derived(normalize(board));
  const selItem = $derived(selected?.kind === "item" ? b.items.find((i) => i.id === selected.id) : null);
  const selLine = $derived(selected?.kind === "line" ? b.lines.find((l) => l.id === selected.id) : null);

  const people = [
    ["player", "Player"],
    ["opponent", "Opponent"],
    ["ball", "Ball"],
  ];
  const equipment = $derived((cfg.equipment ?? []).map((k) => [k, EQUIPMENT[k] ?? k]));
  const lines = $derived((cfg.lineKinds ?? ["run", "pass"]).map((k) => ["line:" + k, lineLabel(cfg, k), k]));

  // ---- second screen ----
  let offs = [];
  async function present() {
    if (presenting) return stopPresenting();
    presenting = true;
    offs.push(
      await listen("bw:ready", sendState),
      await listen("bw:closed", () => stopPresenting(true)),
    );
    const w = await openBoardWindow();
    // The window's own close button: stop presenting here too.
    w?.once?.("tauri://destroyed", () => presenting && stopPresenting(true)).then((off) => offs.push(off));
  }
  function stopPresenting(closed = false) {
    presenting = false;
    offs.splice(0).forEach((o) => o());
    if (!closed) closeBoardWindow();
  }
  function sendState() {
    emit("bw:state", { board: $state.snapshot(b), sport: app.team.sport, style: $state.snapshot(style), frame, t, title });
  }
  $effect(() => {
    // re-send whenever what's shown changes
    const _ = [b, frame, t, style, title];
    if (presenting) sendState();
  });
  $effect(() => () => presenting && closeBoardWindow());

  const swatches = ["", "#2563eb", "#dc2626", "#16a34a", "#f59e0b", "#a855f7", "#0f172a", "#ffffff"];
</script>

<div class="be">
  <div class="bar">
    <div class="grp">
      <button class="ib" class:on={tool === "select"} title="Select and move (drag); double-click to relabel" onclick={() => (tool = "select")}><Icon name="pointer" /></button>
    </div>
    <div class="grp">
      {#each people as [k, l]}<button class="ib txt" class:on={tool === k} onclick={() => (tool = k)}>{l}</button>{/each}
    </div>
    <div class="grp">
      <select class="pick" value={equipment.some(([k]) => k === tool) ? tool : ""} onchange={(e) => e.currentTarget.value && (tool = e.currentTarget.value)} title="Equipment">
        <option value="">Equipment…</option>
        {#each equipment as [k, l]}<option value={k}>{l}</option>{/each}
      </select>
    </div>
    <div class="grp">
      {#each lines as [k, l]}<button class="ib txt" class:on={tool === k} onclick={() => (tool = k)} title="Drag from start to end, or click start then end">{l}</button>{/each}
    </div>
    <div class="grp" title="Colour for new items and lines">
      {#each swatches as c}
        <button class="sw" class:on={newColor === c} style="background:{c || 'transparent'}" onclick={() => (newColor = c)} title={c ? c : "Team default colours"}>{#if !c}<Icon name="x" size={10} />{/if}</button>
      {/each}
    </div>
    <span class="spacer"></span>
    <div class="grp">
      <button class="ib" title="Undo (Ctrl+Z)" onclick={() => tb.undo()}><Icon name="undo" /></button>
      <button class="ib" title="Redo (Ctrl+Y)" onclick={() => tb.redo()}><Icon name="redo" /></button>
      <button class="ib" title="Delete selected (Delete)" disabled={!selected} onclick={() => tb.deleteSelected()}><Icon name="trash" /></button>
      <button class="ib" title="Board style for this team" class:on={showStyle} onclick={() => (showStyle = !showStyle)}><Icon name="palette" /></button>
      <button class="ib" class:on={presenting} title={presenting ? "Stop showing on the second screen" : "Show on a second screen (for team talks)"} onclick={present}><Icon name="mirror" /></button>
    </div>
  </div>

  {#if showStyle}
    <div class="stylebox card row wrap">
      <label class="field">Our players <input type="color" value={style.home} onchange={(e) => saveStyle({ home: e.currentTarget.value })} /></label>
      <label class="field">Opponents <input type="color" value={style.away} onchange={(e) => saveStyle({ away: e.currentTarget.value })} /></label>
      <label class="field">Equipment <input type="color" value={style.equipColor} onchange={(e) => saveStyle({ equipColor: e.currentTarget.value })} /></label>
      <label class="field">Lines <input type="color" value={style.lineColor} onchange={(e) => saveStyle({ lineColor: e.currentTarget.value })} /></label>
      <label class="field">Player shape
        <select value={style.shape} onchange={(e) => saveStyle({ shape: e.currentTarget.value })}>
          <option value="circle">Circle</option><option value="shirt">Shirt</option><option value="triangle">Triangle</option><option value="square">Square</option>
        </select>
      </label>
      <label class="field">Marker size <input type="range" min="0.6" max="1.6" step="0.1" value={style.size} onchange={(e) => saveStyle({ size: Number(e.currentTarget.value) })} /></label>
      <label class="field">Line weight <input type="range" min="0.5" max="2.5" step="0.25" value={style.lineWidth} onchange={(e) => saveStyle({ lineWidth: Number(e.currentTarget.value) })} /></label>
      <label class="field row" style="flex-direction:row;align-items:center;gap:6px"><input type="checkbox" checked={style.numbers} onchange={(e) => saveStyle({ numbers: e.currentTarget.checked })} /> Number new players</label>
      <span class="muted small">Saved for this team.</span>
    </div>
  {/if}

  <TacticsBoard bind:this={tb} bind:board sport={app.team.sport} {style} editable bind:tool bind:frame bind:t bind:playing bind:selected newColor={newColor || null} {height} {onchange} />

  {#if selItem || selLine}
    <div class="selbar row wrap small">
      <b>{selItem ? (selItem.type === "player" ? "Player" : selItem.type === "opponent" ? "Opponent" : EQUIPMENT[selItem.type] ?? selItem.type) : lineLabel(cfg, selLine.kind)}</b>
      {#if selItem}
        <label class="row">Label <input value={selItem.label ?? ""} maxlength="4" style="width:60px" onchange={(e) => tb.updateSelected({ label: e.currentTarget.value })} /></label>
        <label class="row">Size <input type="range" min="0.5" max="2" step="0.1" value={selItem.size ?? 1} onchange={(e) => tb.updateSelected({ size: Number(e.currentTarget.value) })} /></label>
      {:else}
        <label class="row">Type
          <select value={selLine.kind} onchange={(e) => tb.updateSelected({ kind: e.currentTarget.value })}>
            {#each cfg.lineKinds as k}<option value={k}>{lineLabel(cfg, k)}</option>{/each}
          </select>
        </label>
        <label class="row">Weight <input type="range" min="0.5" max="2.5" step="0.25" value={selLine.width ?? 1} onchange={(e) => tb.updateSelected({ width: Number(e.currentTarget.value) })} /></label>
      {/if}
      <label class="row">Colour <input type="color" value={(selItem ?? selLine).color ?? "#ffffff"} onchange={(e) => tb.updateSelected({ color: e.currentTarget.value })} /></label>
      <button class="small" onclick={() => tb.updateSelected({ color: null })}>Default colour</button>
      <button class="small danger" onclick={() => tb.deleteSelected()}><Icon name="trash" size={14} /> Delete</button>
    </div>
  {:else}
    <div class="muted small">Pick a tool and click the board. Lines: drag, or click start then end. Select to move or delete (Delete key or right-click). Ctrl+Z undo, Ctrl+Y redo, Ctrl+C/Ctrl+V copy and paste, Ctrl+D duplicate.</div>
  {/if}
</div>

<style>
  .be { display: flex; flex-direction: column; gap: 8px; }
  .bar { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .grp { display: flex; align-items: center; gap: 3px; }
  .ib { min-width: 30px; height: 30px; padding: 0 8px; display: inline-flex; align-items: center; justify-content: center; gap: 4px; }
  .ib.txt { font-size: 12px; }
  .pick { height: 30px; padding: 0 6px; font-size: 12px; }
  .sw { width: 20px; height: 20px; min-width: 0; padding: 0; border-radius: 50%; border: 2px solid var(--line-2); display: inline-grid; place-items: center; }
  .sw.on { border-color: #facc15; }
  .stylebox { gap: 14px; align-items: flex-end; }
  .selbar { gap: 12px; align-items: center; background: var(--panel-2); border: 1px solid var(--line); border-radius: var(--radius); padding: 6px 10px; }
</style>
