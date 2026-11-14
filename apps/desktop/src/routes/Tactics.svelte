<script>
  import { db, ask } from "$shared/api.js";
  import Icon from "$shared/Icon.svelte";
  import { normalize } from "$shared/board.js";
  import BoardEditor from "../components/BoardEditor.svelte";
  import { attempt, toast, teamCfg } from "../lib/state.svelte.js";

  const cfg = $derived(teamCfg());
  let tactics = $state([]);
  let type = $state("");
  let q = $state("");
  let draft = $state(null);
  let dirty = $state(false);

  async function load() {
    tactics = await db.list("tactic");
  }
  load();

  const types = $derived([...new Set([...(cfg.tacticTypes ?? []), ...tactics.map((t) => t.type).filter(Boolean)])]);
  const shown = $derived(
    tactics
      .filter((t) => !type || t.type === type)
      .filter((t) => !q.trim() || `${t.name} ${t.type} ${t.notes} ${t.tags}`.toLowerCase().includes(q.trim().toLowerCase()))
      .sort((a, b) => (a.type || "").localeCompare(b.type || "") || (a.name || "").localeCompare(b.name || ""))
  );

  const blank = () => ({ id: null, name: "", type: type || types[0] || "", call: "", tags: "", notes: "", group_notes: {}, board: normalize(null) });

  async function edit(t) {
    if (dirty && !await ask("Discard unsaved changes to this tactic?")) return;
    draft = t ? { group_notes: {}, ...JSON.parse(JSON.stringify(t)), board: normalize(t.board) } : blank();
    dirty = !t;
  }
  function change(f) {
    Object.assign(draft, f);
    dirty = true;
  }
  async function save() {
    const { id, ...fields } = $state.snapshot(draft);
    if (!fields.name?.trim()) return toast("Give the tactic a name", "error");
    const r = await attempt(() => db.put("tactic", fields, id), "Tactic saved");
    if (r) {
      draft = { ...draft, id: r.id };
      dirty = false;
      load();
    }
  }
  async function duplicate() {
    const { id, ...fields } = $state.snapshot(draft);
    const r = await attempt(() => db.put("tactic", { ...fields, name: fields.name + " (copy)" }), "Tactic duplicated");
    if (r) {
      await load();
      edit(tactics.find((x) => x.id === r.id));
    }
  }
  async function remove() {
    if (!draft.id) return (draft = null);
    if (!await ask(`Delete "${draft.name}" from the library?`)) return;
    await attempt(() => db.del("tactic", draft.id), "Tactic deleted");
    draft = null;
    dirty = false;
    load();
  }
</script>

<div class="page">
  <div class="ph">
    <div>
      <h1>Tactics library</h1>
      <div class="sub">{cfg.name} · {cfg.formatLabel} — plan moves, set pieces and patterns, then animate them step by step</div>
    </div>
  </div>

  <div class="layout">
    <div class="col">
      <button class="primary" onclick={() => edit(null)}><Icon name="plus" /> New tactic</button>
      <input placeholder="Search" bind:value={q} />
      <select bind:value={type}>
        <option value="">All types ({tactics.length})</option>
        {#each types as t}<option value={t}>{t} ({tactics.filter((x) => x.type === t).length})</option>{/each}
      </select>
      <div class="list card flush">
        {#each shown as t (t.id)}
          <button class="item rowbtn" class:on={t.id === draft?.id} onclick={() => edit(t)}>
            <b>{t.name}{t.call ? ` — “${t.call}”` : ""}</b>
            <span class="small muted">{t.type}{t.board?.steps?.length > 1 ? ` · ${t.board.steps.length} steps` : ""}</span>
          </button>
        {:else}
          <div class="empty">Nothing here yet. Typical {cfg.name.toLowerCase()} entries: {(cfg.tacticTypes ?? []).slice(0, 3).join(", ")}.</div>
        {/each}
      </div>
    </div>

    {#if draft}
      <div class="col">
        <div class="card col">
          <div class="row">
            <h2 class="grow">{draft.id ? "Edit tactic" : "New tactic"}</h2>
            {#if dirty}<span class="badge warn">Unsaved changes</span>{:else if draft.id}<span class="badge good">Saved</span>{/if}
          </div>
          <div class="grid3">
            <label class="field">Name <input value={draft.name} placeholder="Name of the move or set piece" oninput={(e) => change({ name: e.currentTarget.value })} /></label>
            <label class="field">Type
              <select value={draft.type} onchange={(e) => change({ type: e.currentTarget.value })}>
                {#each types as t}<option>{t}</option>{/each}
              </select>
            </label>
            <label class="field">Call / code word <input value={draft.call ?? ""} placeholder="What the players shout" oninput={(e) => change({ call: e.currentTarget.value })} /></label>
          </div>
          <label class="field">Tags <input value={draft.tags ?? ""} placeholder="e.g. 22m entry, vs rush defence" oninput={(e) => change({ tags: e.currentTarget.value })} /></label>
        </div>

        <div class="card">
          <BoardEditor board={draft.board} title={draft.name} onchange={(nb) => change({ board: nb })} />
          <div class="muted small" style="margin-top:6px">Add steps to record movement: place everyone, press “Add step”, then drag players and the ball to where they go next. Play it back here or on a second screen.</div>
        </div>

        <div class="card col">
          <h3>Roles by position group</h3>
          <div class="grid2">
            {#each cfg.groups as g}
              <label class="field">{g}
                <textarea rows="2" value={draft.group_notes?.[g] ?? ""} oninput={(e) => change({ group_notes: { ...draft.group_notes, [g]: e.currentTarget.value } })}></textarea>
              </label>
            {/each}
          </div>
          <label class="field">Notes / when to use it <textarea rows="3" value={draft.notes ?? ""} oninput={(e) => change({ notes: e.currentTarget.value })}></textarea></label>
        </div>

        <div class="actions">
          <button class="primary" onclick={save} disabled={!dirty}><Icon name="save" /> Save tactic</button>
          {#if draft.id}<button onclick={duplicate}><Icon name="copy" /> Duplicate</button>{/if}
          <button onclick={async () => { if (!dirty || await ask("Discard changes?")) { draft = null; dirty = false; } }}>Close</button>
          <span class="spacer"></span>
          {#if draft.id}<button class="danger" onclick={remove}><Icon name="trash" /> Delete</button>{/if}
        </div>
      </div>
    {:else}
      <div class="empty">Choose a tactic, or create one.</div>
    {/if}
  </div>
</div>

<style>
  .layout { display: grid; grid-template-columns: 280px 1fr; gap: 16px; align-items: start; }
</style>
