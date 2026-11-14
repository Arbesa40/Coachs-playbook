<script>
  // Attendance ids are session+player, so the PC and phone update the same entry.
  import Icon from "$shared/Icon.svelte";
  import TacticsBoard from "$shared/TacticsBoard.svelte";
  import { db, playerLabel, sortPlayers, today, byKey, ask } from "$shared/api.js";
  import { normalize, DEFAULT_STYLE } from "$shared/board.js";
  import { teamConfig, groupPlayers } from "$shared/sports.js";
  import { ui, attempt, toast } from "../state.svelte.js";

  let sessions = $state([]);
  let players = $state([]);
  let drills = $state([]);
  let attendance = $state([]);
  let selId = $state(null);
  let mode = $state("plan"); // plan | attendance | edit
  let style = $state({ ...DEFAULT_STYLE });

  const cfg = $derived(teamConfig(ui.team));
  const squad = $derived(sortPlayers(players));
  const dById = $derived(byKey(drills, "id"));
  const sorted = $derived([...sessions].sort((a, b) => (b.date || "").localeCompare(a.date || "") || (b.time || "").localeCompare(a.time || "")));
  const upcoming = $derived(sorted.filter((s) => (s.date || "") >= today()).reverse());
  const past = $derived(sorted.filter((s) => (s.date || "") < today()));
  const sel = $derived(sessions.find((s) => s.id === selId) ?? null);
  const categories = $derived([...new Set([...(cfg.drillCategories ?? []), ...drills.map((d) => d.category).filter(Boolean)])]);

  async function load() {
    [sessions, players, drills, attendance] = await Promise.all(["session", "player", "drill", "attendance"].map((k) => db.list(k)));
  }
  load();
  db.get("board_style", "default").then((s) => s && (style = { ...DEFAULT_STYLE, ...s })).catch(() => {});

  function open(id, m = null) {
    selId = id;
    const s = sessions.find((x) => x.id === id);
    mode = m ?? (s?.drills?.length ? "plan" : "attendance");
  }
  async function quickSession() {
    const s = await attempt(() => db.put("session", { date: today(), title: "Training", duration: 75, drills: [], group_notes: {}, notes: "" }), "Session added");
    await load();
    if (s) open(s.id, "attendance");
  }

  // ---------- plan editor (saved with the Save button) ----------
  let draft = $state(null);
  let dirty = $state(false);
  const blank = () => ({ id: null, date: today(), time: "", title: "Training", duration: 75, focus: "", location: "", drills: [], group_notes: {}, notes: "" });

  function newPlan() {
    selId = null;
    draft = blank();
    dirty = true;
    mode = "edit";
  }
  function editPlan() {
    draft = JSON.parse(JSON.stringify({ group_notes: {}, drills: [], ...$state.snapshot(sel) }));
    dirty = false;
    mode = "edit";
  }
  function change(fields) {
    Object.assign(draft, fields);
    dirty = true;
  }
  function setItem(i, fields) {
    change({ drills: draft.drills.map((x, k) => (k === i ? { ...x, ...fields } : x)) });
  }
  function moveItem(i, d) {
    const list = [...draft.drills];
    const j = i + d;
    if (j < 0 || j >= list.length) return;
    [list[i], list[j]] = [list[j], list[i]];
    change({ drills: list });
  }
  function addDrill(id) {
    if (!id) return;
    change({ drills: [...draft.drills, { drill_id: id, minutes: dById[id]?.minutes ?? 15, points: "" }] });
  }
  // A new drill made on the phone goes into the shared drill library too.
  let quick = $state(null); // { name, category, minutes, description }
  async function createDrill() {
    if (!quick?.name?.trim()) return toast("Give the drill a name", "error");
    const r = await attempt(() =>
      db.put("drill", {
        name: quick.name.trim(), category: quick.category || "", minutes: Number(quick.minutes) || 15,
        description: quick.description ?? "", coaching_points: "", players: "", area: "", intensity: "Medium",
        equipment: [], board: normalize(null),
      })
    );
    if (!r) return;
    drills = await db.list("drill");
    addDrill(r.id);
    quick = null;
  }
  const draftPlanned = $derived((draft?.drills ?? []).reduce((a, d) => a + (Number(d.minutes) || 0), 0));

  async function savePlan() {
    const { id, ...fields } = $state.snapshot(draft);
    if (!fields.title?.trim()) return toast("Give the session a title", "error");
    const r = await attempt(() => db.put("session", fields, id), "Session saved");
    if (!r) return;
    await load();
    draft = null;
    dirty = false;
    open(r.id, "plan");
  }
  async function cancelEdit() {
    if (dirty && !(await ask("Discard your changes to this plan?"))) return;
    draft = null;
    dirty = false;
    if (sel) mode = "plan";
    else selId = null;
  }
  async function deleteSession() {
    const s = draft?.id ? draft : sel;
    if (!s?.id) return cancelEdit();
    if (!(await ask(`Delete "${s.title}" on ${s.date}?\n\nIt is also removed from the PC at the next sync.`))) return;
    await attempt(() => db.del("session", s.id), "Session deleted");
    draft = null;
    dirty = false;
    selId = null;
    load();
  }

  // ---------- attendance ----------
  const present = (pid) => attendance.find((a) => a.id === `${sel.id}:${pid}`)?.present ?? false;
  const presentCount = $derived(sel ? attendance.filter((a) => a.session_id === sel.id && a.present).length : 0);
  async function toggle(pid) {
    const r = await db.put("attendance", { session_id: sel.id, player_id: pid, date: sel.date, present: !present(pid) }, `${sel.id}:${pid}`);
    attendance = [...attendance.filter((a) => a.id !== r.id), r];
  }
  async function allPresent() {
    for (const p of squad) if (!present(p.id)) await toggle(p.id);
  }
  const planned = $derived((sel?.drills ?? []).reduce((a, d) => a + (Number(d.minutes) || 0), 0));
  const boardH = Math.round(Math.min(window.innerWidth - 48, 680) * 0.62);
</script>

{#if mode === "edit" && draft}
  <div class="row" style="margin-bottom:8px">
    <button class="ghost iconbtn" aria-label="Back" onclick={cancelEdit}><Icon name="chevronLeft" size={22} /></button>
    <b class="grow">{draft.id ? "Edit plan" : "New session plan"}</b>
    <button class="primary" onclick={savePlan} disabled={!dirty}><Icon name="save" size={18} /> Save</button>
  </div>

  <section class="card col">
    <label class="field">Title <input value={draft.title} oninput={(e) => change({ title: e.currentTarget.value })} /></label>
    <div class="two">
      <label class="field">Date <input type="date" value={draft.date} oninput={(e) => change({ date: e.currentTarget.value })} /></label>
      <label class="field">Start time <input type="time" value={draft.time ?? ""} oninput={(e) => change({ time: e.currentTarget.value })} /></label>
      <label class="field">Length (min) <input type="number" min="0" inputmode="numeric" value={draft.duration} oninput={(e) => change({ duration: Number(e.currentTarget.value) })} /></label>
      <label class="field">Location <input value={draft.location ?? ""} oninput={(e) => change({ location: e.currentTarget.value })} /></label>
    </div>
    <label class="field">Focus <input value={draft.focus ?? ""} placeholder="e.g. Pressing from the front" oninput={(e) => change({ focus: e.currentTarget.value })} /></label>
  </section>

  <h3 class="sec">Drills <span class="muted">· {draftPlanned} of {draft.duration} min</span></h3>
  {#each draft.drills as p, i}
    {@const dr = dById[p.drill_id]}
    <section class="card col item">
      <div class="row">
        <span class="num">{i + 1}</span>
        <b class="grow">{dr?.name ?? "(deleted drill)"}</b>
        <button class="ghost iconbtn" aria-label="Move up" disabled={i === 0} onclick={() => moveItem(i, -1)}><Icon name="arrowUp" size={18} /></button>
        <button class="ghost iconbtn" aria-label="Move down" disabled={i === draft.drills.length - 1} onclick={() => moveItem(i, 1)}><Icon name="arrowDown" size={18} /></button>
        <button class="ghost iconbtn danger" aria-label="Remove from plan" onclick={() => change({ drills: draft.drills.filter((_, k) => k !== i) })}><Icon name="x" size={18} /></button>
      </div>
      <div class="row">
        <label class="field mins">Minutes <input type="number" min="0" inputmode="numeric" value={p.minutes} oninput={(e) => setItem(i, { minutes: Number(e.currentTarget.value) })} /></label>
        <label class="field grow">Coaching points for today <input value={p.points ?? ""} oninput={(e) => setItem(i, { points: e.currentTarget.value })} /></label>
      </div>
    </section>
  {:else}
    <div class="empty">No drills yet. Add one from the library, or create a new one.</div>
  {/each}

  <section class="col addbox">
    {#if drills.length}
      <select onchange={(e) => { addDrill(e.currentTarget.value); e.currentTarget.value = ""; }}>
        <option value="">+ Add a drill from the library…</option>
        {#each categories as c}
          {@const list = drills.filter((d) => d.category === c)}
          {#if list.length}<optgroup label={c}>{#each list as d}<option value={d.id}>{d.name}{d.minutes ? ` (${d.minutes} min)` : ""}</option>{/each}</optgroup>{/if}
        {/each}
        {#if drills.some((d) => !d.category)}
          <optgroup label="Uncategorised">{#each drills.filter((d) => !d.category) as d}<option value={d.id}>{d.name}</option>{/each}</optgroup>
        {/if}
      </select>
    {/if}
    {#if quick}
      <div class="card col">
        <b>New drill</b>
        <label class="field">Name <input value={quick.name} placeholder="e.g. 4v2 rondo" oninput={(e) => (quick.name = e.currentTarget.value)} /></label>
        <div class="two">
          <label class="field">Category
            <select value={quick.category} onchange={(e) => (quick.category = e.currentTarget.value)}>
              <option value="">Uncategorised</option>
              {#each categories as c}<option>{c}</option>{/each}
            </select>
          </label>
          <label class="field">Minutes <input type="number" min="0" inputmode="numeric" value={quick.minutes} oninput={(e) => (quick.minutes = e.currentTarget.value)} /></label>
        </div>
        <label class="field">Description <textarea rows="3" value={quick.description} oninput={(e) => (quick.description = e.currentTarget.value)}></textarea></label>
        <div class="muted small">Draw its diagram later on the PC (Training → Drill library).</div>
        <div class="row">
          <button class="grow" onclick={() => (quick = null)}>Cancel</button>
          <button class="primary grow" onclick={createDrill}><Icon name="plus" size={18} /> Add to plan</button>
        </div>
      </div>
    {:else}
      <button onclick={() => (quick = { name: "", category: categories[0] ?? "", minutes: 15, description: "" })}><Icon name="plus" size={18} /> Create a new drill</button>
    {/if}
  </section>

  <h3 class="sec">Instructions by group</h3>
  <section class="card col">
    {#each cfg.groups as g}
      <label class="field">{g} <textarea rows="2" placeholder="What this group should focus on" value={draft.group_notes?.[g] ?? ""} oninput={(e) => change({ group_notes: { ...draft.group_notes, [g]: e.currentTarget.value } })}></textarea></label>
    {/each}
    <label class="field">Session notes <textarea rows="3" value={draft.notes ?? ""} oninput={(e) => change({ notes: e.currentTarget.value })}></textarea></label>
  </section>

  <div class="row" style="margin-top:12px">
    <button class="grow" onclick={cancelEdit}>Cancel</button>
    <button class="primary grow" onclick={savePlan} disabled={!dirty}><Icon name="save" size={18} /> Save plan</button>
  </div>
  {#if draft.id}
    <section class="dz"><button class="danger" style="width:100%" onclick={deleteSession}><Icon name="trash" size={18} /> Delete this session</button></section>
  {/if}
{:else if !sel}
  <div class="row">
    <button class="primary big grow" onclick={newPlan}><Icon name="plus" size={20} /> Plan a session</button>
  </div>
  <button class="quick" onclick={quickSession}><Icon name="clipboard" size={18} /> Quick session today (attendance only)</button>

  {#if upcoming.length}
    <h3 class="sec">Today and upcoming</h3>
    <div class="list card flush">
      {#each upcoming as s (s.id)}
        <button class="item rowbtn row srow" onclick={() => open(s.id)}>
          <span class="date"><b>{s.date?.slice(8)}</b><span>{new Date(s.date + "T00:00").toLocaleString(undefined, { month: "short" })}</span></span>
          <span class="grow col" style="gap:2px;text-align:left"><b>{s.title}</b><span class="muted small">{s.time ? s.time + " · " : ""}{s.duration} min{s.drills?.length ? ` · ${s.drills.length} drills` : ""}{s.date === today() ? " · today" : ""}</span></span>
          <Icon name="chevronRight" size={18} />
        </button>
      {/each}
    </div>
  {/if}
  <h3 class="sec">Past sessions</h3>
  <div class="list card flush">
    {#each past.slice(0, 30) as s (s.id)}
      <button class="item rowbtn row srow" onclick={() => open(s.id)}>
        <span class="date"><b>{s.date?.slice(8)}</b><span>{new Date(s.date + "T00:00").toLocaleString(undefined, { month: "short" })}</span></span>
        <span class="grow col" style="gap:2px;text-align:left"><b>{s.title}</b><span class="muted small">{s.duration} min{s.drills?.length ? ` · ${s.drills.length} drills` : ""}</span></span>
        <Icon name="chevronRight" size={18} />
      </button>
    {:else}
      <div class="empty">No past sessions yet.</div>
    {/each}
  </div>
{:else}
  <div class="row" style="margin-bottom:8px">
    <button class="ghost iconbtn" aria-label="Back" onclick={() => (selId = null)}><Icon name="chevronLeft" size={22} /></button>
    <div class="grow col" style="gap:0;min-width:0"><b>{sel.title}</b><span class="muted small">{sel.date}{sel.time ? " · " + sel.time : ""} · {sel.duration} min{sel.focus ? " · " + sel.focus : ""}</span></div>
    <button onclick={editPlan}><Icon name="pencil" size={18} /> Edit</button>
  </div>
  <div class="seg">
    <button class:on={mode === "plan"} onclick={() => (mode = "plan")}>Plan</button>
    <button class:on={mode === "attendance"} onclick={() => (mode = "attendance")}>Attendance <span class="cnt">{presentCount}</span></button>
  </div>

  {#if mode === "plan"}
    {#each sel.drills ?? [] as d, i}
      {@const dr = dById[d.drill_id]}
      <section class="card col">
        <div class="row"><span class="num">{i + 1}</span><b class="grow">{dr?.name ?? "Drill"}</b><span class="muted small">{d.minutes} min</span></div>
        {#if dr && (dr.board?.items?.length || dr.diagram?.length)}<TacticsBoard board={normalize(dr.board ?? dr.diagram)} sport={ui.team?.sport ?? "football"} style={{ ...style, size: (style.size ?? 1) * 1.5, lineWidth: (style.lineWidth ?? 1) * 1.3 }} height={boardH} />{/if}
        {#if dr?.description}<div class="small">{dr.description}</div>{/if}
        {#if d.points || dr?.coaching_points}<div class="small"><b>Coaching points:</b> {d.points || dr.coaching_points}</div>{/if}
        {#if dr?.players || dr?.area}<div class="muted small">{[dr.players, dr.area].filter(Boolean).join(" · ")}</div>{/if}
      </section>
    {:else}
      <div class="empty col" style="align-items:center">
        <span>No drills planned for this session yet.</span>
        <button class="primary" onclick={editPlan}><Icon name="pencil" size={18} /> Plan it now</button>
      </div>
    {/each}
    {#if sel.drills?.length}<div class="muted small" style="text-align:center">{planned} of {sel.duration} min planned</div>{/if}
    {#if Object.values(sel.group_notes ?? {}).some((v) => v?.trim())}
      <h3 class="sec">Instructions by group</h3>
      <section class="card col">
        {#each Object.entries(sel.group_notes ?? {}).filter(([, v]) => v?.trim()) as [g, v]}<div><b>{g}</b><div class="small">{v}</div></div>{/each}
      </section>
    {/if}
    {#if sel.notes}<section class="card small">{sel.notes}</section>{/if}
  {:else}
    <div class="row" style="margin:10px 0"><span class="muted grow">{presentCount} of {squad.length} present</span><button class="small" onclick={allPresent}><Icon name="check" size={16} /> All present</button></div>
    {#each groupPlayers(cfg, squad) as [g, list] (g)}
      <h3 class="sec">{g}</h3>
      <div class="chipgrid">
        {#each list as p (p.id)}
          <button class="chip" class:on={present(p.id)} onclick={() => toggle(p.id)}>{#if present(p.id)}<Icon name="check" size={14} />{/if} {playerLabel(p)}</button>
        {/each}
      </div>
    {/each}
  {/if}
{/if}

<style>
  .srow { padding: 10px 12px !important; gap: 12px; }
  .date { width: 42px; flex: none; display: flex; flex-direction: column; align-items: center; line-height: 1.1; border-radius: 8px; background: var(--panel-2); padding: 4px 0; }
  .date b { font-size: 17px; }
  .date span { font-size: 11px; color: var(--muted); text-transform: uppercase; }
  .iconbtn { padding: 6px; min-width: 40px; }
  .num { width: 24px; height: 24px; border-radius: 50%; background: var(--panel-2); display: grid; place-items: center; font-size: 12px; flex: none; }
  .cnt { font-size: 11px; background: var(--chip); border-radius: 10px; padding: 0 6px; }
  section.card { margin-top: 10px; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .mins { width: 86px; flex: none; }
  .item { gap: 6px; }
  .addbox { margin-top: 10px; gap: 8px; }
  .quick { width: 100%; margin-top: 8px; }
  .dz { margin-top: 24px; padding-top: 16px; border-top: 1px solid var(--line); }
</style>
