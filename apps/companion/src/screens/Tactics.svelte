<script>
  import Icon from "$shared/Icon.svelte";
  import TacticsBoard from "$shared/TacticsBoard.svelte";
  import { db, ask } from "$shared/api.js";
  import { teamConfig, EQUIPMENT, lineLabel } from "$shared/sports.js";
  import { normalize, DEFAULT_STYLE } from "$shared/board.js";
  import { ui, attempt, toast } from "../state.svelte.js";
  import { setImmersiveLandscape } from "../screen.js";

  const cfg = $derived(teamConfig(ui.team));
  const sport = $derived(ui.team?.sport ?? "football");
  let tactics = $state([]);
  let style = $state({ ...DEFAULT_STYLE });
  let q = $state("");
  let type = $state("");

  async function load() {
    tactics = await db.list("tactic");
  }
  load();
  db.get("board_style", "default").then((s) => s && (style = { ...DEFAULT_STYLE, ...s })).catch(() => {});

  const types = $derived([...new Set([...(cfg.tacticTypes ?? []), ...tactics.map((t) => t.type).filter(Boolean)])]);
  const shown = $derived(
    tactics
      .filter((t) => !type || t.type === type)
      .filter((t) => !q.trim() || `${t.name} ${t.type} ${t.call ?? ""} ${t.tags ?? ""}`.toLowerCase().includes(q.trim().toLowerCase()))
      .sort((a, b) => (a.type || "").localeCompare(b.type || "") || (a.name || "").localeCompare(b.name || ""))
  );
  const grouped = $derived(
    shown.reduce((acc, t) => {
      const k = t.type || "Other";
      (acc[k] ??= []).push(t);
      return acc;
    }, {})
  );

  // ---- open tactic ----
  let cur = $state(null); // working copy
  let editing = $state(false);
  let dirty = $state(false);
  let full = $state(false);
  let showInfo = $state(false);

  let tb;
  let tool = $state("select");
  let frame = $state(0);
  let t = $state(null);
  let playing = $state(false);
  let selected = $state(null);

  let vw = $state(window.innerWidth);
  let vh = $state(window.innerHeight);
  // Bigger markers and lines on a phone-sized board so they can be read and tapped.
  const boardStyle = $derived(vw < 700 ? { ...style, size: (style.size ?? 1) * 1.6, lineWidth: (style.lineWidth ?? 1) * 1.4 } : style);
  // In landscape full screen the tools move to a left column and the board takes the height.
  const land = $derived(full && vw > vh);
  const boardH = $derived(
    !full ? Math.round(Math.min(vw - 24, 720) * 0.68)
    : land ? Math.max(160, vh - 104)
    : Math.max(200, vh - (editing ? 176 : 112))
  );
  $effect(() => {
    setImmersiveLandscape(full && !!cur);
  });
  $effect(() => () => setImmersiveLandscape(false));

  function openTactic(x) {
    cur = { group_notes: {}, ...JSON.parse(JSON.stringify(x)), board: normalize(x.board) };
    editing = false;
    dirty = false;
    frame = 0;
    t = null;
    selected = null;
    tool = "select";
  }
  function newTactic(blank = false) {
    cur = { id: null, name: blank ? "" : "New tactic", type: type || types[0] || "", call: "", notes: "", group_notes: {}, board: normalize(null), scratch: blank };
    editing = true;
    dirty = false;
    frame = 0;
    t = null;
    selected = null;
    tool = "player";
  }
  async function close() {
    if (dirty && !(await ask("Discard your changes?"))) return;
    cur = null;
    full = false;
    editing = false;
    dirty = false;
  }
  async function cancelEdit() {
    if (dirty && !(await ask("Discard your changes?"))) return;
    if (!cur.id) return close();
    openTactic(tactics.find((x) => x.id === cur.id));
  }
  async function save() {
    const { id, scratch, ...fields } = $state.snapshot(cur);
    if (!fields.name?.trim()) {
      showInfo = true;
      return toast("Give it a name first", "error");
    }
    const r = await attempt(() => db.put("tactic", fields, id), "Saved");
    if (!r) return;
    await load();
    cur = { ...cur, id: r.id, scratch: false };
    dirty = false;
    editing = false;
  }
  async function remove() {
    if (!cur?.id) return close();
    if (!(await ask(`Delete "${cur.name}"? It will also be removed from the PC at the next sync.`))) return;
    await attempt(() => db.del("tactic", cur.id), "Deleted");
    cur = null;
    full = false;
    load();
  }
  function change(f) {
    Object.assign(cur, f);
    dirty = true;
  }

  const people = [
    ["player", "Player"],
    ["opponent", "Opponent"],
    ["ball", "Ball"],
  ];
  const equipment = $derived((cfg.equipment ?? []).map((k) => [k, EQUIPMENT[k] ?? k]));
  const lineKinds = $derived((cfg.lineKinds ?? ["run", "pass"]).map((k) => ["line:" + k, lineLabel(cfg, k)]));
</script>

<svelte:window bind:innerWidth={vw} bind:innerHeight={vh} />

{#if !cur}
  <section class="col">
    <div class="row">
      <input class="grow" type="search" placeholder="Search tactics" bind:value={q} />
      <select bind:value={type} aria-label="Type" style="max-width:40%">
        <option value="">All types</option>
        {#each types as x}<option value={x}>{x}</option>{/each}
      </select>
    </div>
    <div class="row">
      <button class="primary grow" onclick={() => newTactic(false)}><Icon name="plus" size={18} /> New tactic</button>
      <button class="grow" onclick={() => newTactic(true)}><Icon name="board" size={18} /> Blank board</button>
    </div>
  </section>

  {#each Object.entries(grouped) as [g, list] (g)}
    <h3 class="sec">{g}</h3>
    <div class="list card flush">
      {#each list as x (x.id)}
        <button class="item rowbtn row trow" onclick={() => openTactic(x)}>
          <span class="grow col" style="gap:2px;text-align:left">
            <b>{x.name}</b>
            <span class="muted small">{x.call ? `Call “${x.call}”` : ""}{x.call && x.board?.steps?.length ? " · " : ""}{x.board?.steps?.length ? `${x.board.steps.length + 1} steps` : ""}</span>
          </span>
          <Icon name="chevronRight" size={18} />
        </button>
      {/each}
    </div>
  {:else}
    <div class="empty col" style="align-items:center">
      <Icon name="tactic" size={36} />
      <span>{tactics.length ? "No tactics match." : `No tactics yet. Typical ${cfg.name.toLowerCase()} entries: ${(cfg.tacticTypes ?? []).slice(0, 3).join(", ")}.`}</span>
    </div>
  {/each}
{:else}
  <div class:fs={full} class:land>
    <div class="row head">
      <button class="ghost iconbtn" aria-label="Back" onclick={() => (full ? (full = false) : close())}><Icon name={full ? "x" : "chevronLeft"} size={22} /></button>
      <div class="grow ttl">
        <b>{cur.name || (cur.scratch ? "Blank board" : "Untitled")}</b>
        <span class="muted small">{cur.type}{cur.call ? ` · “${cur.call}”` : ""}</span>
      </div>
      {#if !full}<button class="ghost iconbtn" aria-label="Full screen (landscape)" onclick={() => (full = true)}><Icon name="popout" size={20} /></button>{/if}
      {#if editing}
        <button class="primary" onclick={save} disabled={!dirty && cur.id}><Icon name="save" size={18} /> Save</button>
      {:else}
        <button onclick={() => { editing = true; tool = "select"; }}><Icon name="pencil" size={18} /> Edit</button>
      {/if}
    </div>

    {#if editing}
      <div class="tools">
        <button class="tool" class:on={tool === "select"} onclick={() => (tool = "select")} aria-label="Select and move"><Icon name="pointer" size={18} /></button>
        {#each people as [k, l]}<button class="tool" class:on={tool === k} onclick={() => (tool = k)}>{l}</button>{/each}
        <select class="tool" class:on={tool.startsWith("line:")} value={tool.startsWith("line:") ? tool : ""} onchange={(e) => e.currentTarget.value && (tool = e.currentTarget.value)} aria-label="Line">
          <option value="">Lines…</option>
          {#each lineKinds as [k, l]}<option value={k}>{l}</option>{/each}
        </select>
        <select class="tool" class:on={equipment.some(([k]) => k === tool)} value={equipment.some(([k]) => k === tool) ? tool : ""} onchange={(e) => e.currentTarget.value && (tool = e.currentTarget.value)} aria-label="Equipment">
          <option value="">Kit…</option>
          {#each equipment as [k, l]}<option value={k}>{l}</option>{/each}
        </select>
        <span class="spacer"></span>
        <button class="tool" aria-label="Undo" onclick={() => tb.undo()}><Icon name="undo" size={18} /></button>
        <button class="tool" aria-label="Redo" onclick={() => tb.redo()}><Icon name="redo" size={18} /></button>
        <button class="tool" aria-label="Delete selected" disabled={!selected} onclick={() => tb.deleteSelected()}><Icon name="trash" size={18} /></button>
      </div>
    {/if}

    <div class="boardwrap">
    <TacticsBoard
      bind:this={tb}
      bind:board={cur.board}
      {sport}
      style={boardStyle}
      editable={editing}
      bind:tool
      bind:frame
      bind:t
      bind:playing
      bind:selected
      height={boardH}
      onchange={() => (dirty = true)}
    />
    </div>
    {#if editing && !full}
      <div class="muted small">Tap a tool, then tap the board. Lines: drag from start to end, or tap start then end. Use Select to move or pick something to delete. “+ Step” records the next movement.</div>
    {/if}
  </div>

  {#if !full}
    {#if editing}
      <button class="ghost small" onclick={() => (showInfo = !showInfo)}><Icon name={showInfo ? "chevronUp" : "chevronDown"} size={16} /> Name, type and notes</button>
      {#if showInfo || !cur.name}
        <section class="card col">
          <label class="field">Name <input value={cur.name} placeholder="e.g. Near-post corner" oninput={(e) => change({ name: e.currentTarget.value })} /></label>
          <label class="field">Type
            <select value={cur.type} onchange={(e) => change({ type: e.currentTarget.value })}>
              {#each types as x}<option>{x}</option>{/each}
            </select>
          </label>
          <label class="field">Call / code word <input value={cur.call ?? ""} oninput={(e) => change({ call: e.currentTarget.value })} /></label>
          {#each cfg.groups as g}
            <label class="field">{g} <textarea rows="2" value={cur.group_notes?.[g] ?? ""} oninput={(e) => change({ group_notes: { ...cur.group_notes, [g]: e.currentTarget.value } })}></textarea></label>
          {/each}
          <label class="field">Notes <textarea rows="3" value={cur.notes ?? ""} oninput={(e) => change({ notes: e.currentTarget.value })}></textarea></label>
        </section>
      {/if}
      <div class="row">
        <button class="grow" onclick={cancelEdit}>Cancel</button>
        <button class="primary grow" onclick={save}><Icon name="save" size={18} /> Save</button>
      </div>
    {:else}
      {#if Object.values(cur.group_notes ?? {}).some((v) => v?.trim()) || cur.notes}
        <section class="card col">
          {#each Object.entries(cur.group_notes ?? {}).filter(([, v]) => v?.trim()) as [g, v]}<div><b>{g}</b><div class="small">{v}</div></div>{/each}
          {#if cur.notes}<div><b>Notes</b><div class="small">{cur.notes}</div></div>{/if}
        </section>
      {/if}
    {/if}
    {#if cur.id}
      <section class="dz"><button class="danger" style="width:100%" onclick={remove}><Icon name="trash" size={18} /> Delete this tactic</button></section>
    {/if}
  {/if}
{/if}

<style>
  .head { margin-bottom: 8px; }
  .ttl { display: flex; flex-direction: column; min-width: 0; line-height: 1.25; }
  .ttl b { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .iconbtn { padding: 6px; min-width: 44px; }
  .trow { padding: 12px !important; }
  .tools { display: flex; gap: 6px; overflow-x: auto; padding-bottom: 6px; margin-bottom: 6px; scrollbar-width: none; }
  .tools .tool { flex: none; min-height: 40px; padding: 6px 10px; }
  .tools select.tool { width: auto; }
  .tools select.tool.on { border-color: var(--accent); background: var(--selected-2); }
  .fs { position: fixed; inset: 0; z-index: 90; background: var(--bg); padding: calc(env(safe-area-inset-top) + 8px) 8px 8px; display: flex; flex-direction: column; }
  .fs.land {
    display: grid; grid-template-columns: auto minmax(0, 1fr); grid-template-rows: auto minmax(0, 1fr);
    grid-template-areas: "head head" "tools board"; column-gap: 8px;
    padding: 6px max(8px, env(safe-area-inset-right)) 6px max(8px, env(safe-area-inset-left));
  }
  .fs.land .head { grid-area: head; margin-bottom: 4px; }
  .fs.land .head .iconbtn, .fs.land .head button { min-height: 36px; padding-top: 2px; padding-bottom: 2px; }
  .fs.land .tools { grid-area: tools; flex-direction: column; overflow-x: hidden; overflow-y: auto; padding: 0; margin: 0; width: 104px; }
  .fs.land .tools .tool { min-height: 36px; width: 100%; }
  .fs.land .boardwrap { grid-area: board; min-width: 0; display: flex; flex-direction: column; justify-content: center; }
  .dz { margin-top: 24px; padding-top: 16px; border-top: 1px solid var(--line); }
</style>
