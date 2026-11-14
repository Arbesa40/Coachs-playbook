<script>
  import { db, playerLabel, sortPlayers, today, byKey, ask } from "$shared/api.js";
  import Icon from "$shared/Icon.svelte";
  import { EQUIPMENT } from "$shared/sports.js";
  import { normalize } from "$shared/board.js";
  import { STARTER_DRILLS } from "$shared/starterDrills.js";
  import BoardEditor from "../components/BoardEditor.svelte";
  import { app, attempt, toast, teamCfg } from "../lib/state.svelte.js";

  const cfg = $derived(teamCfg());
  let tab = $state("sessions");
  let sessions = $state([]);
  let drills = $state([]);
  let players = $state([]);
  let attendance = $state([]);

  async function load() {
    [sessions, drills, players, attendance] = await Promise.all(["session", "drill", "player", "attendance"].map((k) => db.list(k)));
  }
  load();

  const dById = $derived(byKey(drills, "id"));
  const squad = $derived(sortPlayers(players));
  const sortedSessions = $derived([...sessions].sort((a, b) => (b.date || "").localeCompare(a.date || "")));

  // ================= sessions (explicit save) =================
  let sDraft = $state(null); // the session being edited (not saved until "Save session")
  let sDirty = $state(false);
  const blankSession = () => ({ id: null, date: today(), time: "", title: "Training", duration: 75, focus: "", location: "", drills: [], group_notes: {}, notes: "" });

  async function editSession(s) {
    if (sDirty && !await ask("Discard unsaved changes to this session?")) return;
    sDraft = s ? JSON.parse(JSON.stringify({ group_notes: {}, drills: [], ...s })) : blankSession();
    sDirty = !s;
  }
  function sChange(fields) {
    Object.assign(sDraft, fields);
    sDirty = true;
  }
  async function saveSession() {
    const { id, ...fields } = $state.snapshot(sDraft);
    if (!fields.title?.trim()) return toast("Give the session a title", "error");
    const r = await attempt(() => db.put("session", fields, id), "Session saved");
    if (r) {
      sDraft = { ...sDraft, id: r.id };
      sDirty = false;
      await load();
    }
  }
  async function deleteSession() {
    if (!sDraft?.id) return (sDraft = null);
    if (!await ask(`Delete "${sDraft.title}" on ${sDraft.date}? Attendance taken for it stays in the records.`)) return;
    await attempt(() => db.del("session", sDraft.id), "Session deleted");
    sDraft = null;
    sDirty = false;
    load();
  }
  const planned = $derived((sDraft?.drills ?? []).reduce((a, d) => a + (Number(d.minutes) || 0), 0));
  function addPlanDrill(id) {
    if (!id) return;
    const dr = dById[id];
    sChange({ drills: [...sDraft.drills, { drill_id: id, minutes: dr?.minutes ?? 15, points: "" }] });
  }
  function movePlan(i, d) {
    const list = [...sDraft.drills];
    const j = i + d;
    if (j < 0 || j >= list.length) return;
    [list[i], list[j]] = [list[j], list[i]];
    sChange({ drills: list });
  }

  // Attendance saves immediately (one record per session+player, shared with the phone).
  const att = (pid) => attendance.find((a) => a.id === `${sDraft.id}:${pid}`);
  async function setPresent(pid, present) {
    const r = await db.put("attendance", { session_id: sDraft.id, player_id: pid, date: sDraft.date, present }, `${sDraft.id}:${pid}`);
    attendance = [...attendance.filter((a) => a.id !== r.id), r];
  }


  // ================= drill library (explicit save) =================
  let dDraft = $state(null);
  let dDirty = $state(false);
  let cat = $state("");
  let q = $state("");
  const categories = $derived([...new Set([...(cfg.drillCategories ?? []), ...drills.map((d) => d.category).filter(Boolean)])]);
  const shownDrills = $derived(
    drills
      .filter((d) => !cat || (cat === "__none" ? !d.category : d.category === cat))
      .filter((d) => !q.trim() || `${d.name} ${d.category} ${d.description}`.toLowerCase().includes(q.trim().toLowerCase()))
      .sort((a, b) => (a.category || "").localeCompare(b.category || "") || (a.name || "").localeCompare(b.name || ""))
  );
  const blankDrill = () => ({ id: null, name: "", category: (cat !== "__none" && cat) || cfg.drillCategories?.[0] || "", minutes: 15, players: "", area: "", intensity: "Medium", equipment: [], description: "", coaching_points: "", board: normalize(null) });

  async function editDrill(d) {
    if (dDirty && !await ask("Discard unsaved changes to this drill?")) return;
    dDraft = d ? { equipment: [], ...JSON.parse(JSON.stringify(d)), board: normalize(d.board ?? d.diagram) } : blankDrill();
    dDirty = !d;
  }
  function dChange(fields) {
    Object.assign(dDraft, fields);
    dDirty = true;
  }
  async function saveDrill() {
    const { id, diagram, ...fields } = $state.snapshot(dDraft);
    if (!fields.name?.trim()) return toast("Give the drill a name", "error");
    const r = await attempt(() => db.put("drill", fields, id), "Drill saved");
    if (r) {
      dDraft = { ...dDraft, id: r.id };
      dDirty = false;
      await load();
    }
  }
  async function duplicateDrill() {
    const { id, ...fields } = $state.snapshot(dDraft);
    const r = await attempt(() => db.put("drill", { ...fields, name: fields.name + " (copy)" }), "Drill duplicated");
    if (r) {
      await load();
      editDrill(drills.find((x) => x.id === r.id));
    }
  }
  async function deleteDrill() {
    if (!dDraft?.id) return (dDraft = null);
    if (!await ask(`Delete the drill "${dDraft.name}"?`)) return;
    await attempt(() => db.del("drill", dDraft.id), "Drill deleted");
    dDraft = null;
    dDirty = false;
    load();
  }
  async function addStarters() {
    const have = new Set(drills.map((d) => d.name));
    const list = (STARTER_DRILLS[app.team.sport] ?? []).filter((d) => !have.has(d.name));
    for (const d of list) await db.put("drill", { ...d, area: "", intensity: "Medium", board: normalize(null) });
    toast(`Added ${list.length} starter drills`);
    load();
  }
  function toggleEquip(k) {
    const set = new Set(dDraft.equipment ?? []);
    set.has(k) ? set.delete(k) : set.add(k);
    dChange({ equipment: [...set] });
  }
</script>

<div class="page">
  <div class="ph">
    <div>
      <h1>Training</h1>
      <div class="sub">{cfg.name} · {cfg.formatLabel}</div>
    </div>
    <div class="tabs">
      <button class:on={tab === "sessions"} onclick={() => (tab = "sessions")}>Sessions</button>
      <button class:on={tab === "drills"} onclick={() => (tab = "drills")}>Drill library</button>
    </div>
  </div>

  {#if tab === "sessions"}
    <div class="layout">
      <div class="col">
        <button class="primary" onclick={() => editSession(null)}><Icon name="plus" /> New session</button>
        <div class="list card flush">
          {#each sortedSessions as s (s.id)}
            <button class="item rowbtn" class:on={s.id === sDraft?.id} onclick={() => editSession(s)}>
              <b>{s.title}</b>
              <span class="small muted">{s.date}{s.time ? " · " + s.time : ""} · {s.duration} min{s.focus ? " · " + s.focus : ""}</span>
            </button>
          {:else}
            <div class="empty">No sessions yet.</div>
          {/each}
        </div>
      </div>

      {#if sDraft}
        <div class="col">
          <div class="card col">
            <div class="row">
              <h2 class="grow">{sDraft.id ? "Edit session" : "New session"}</h2>
              {#if sDirty}<span class="badge warn">Unsaved changes</span>{:else if sDraft.id}<span class="badge good">Saved</span>{/if}
            </div>
            <div class="grid3">
              <label class="field">Title <input value={sDraft.title} oninput={(e) => sChange({ title: e.currentTarget.value })} /></label>
              <label class="field">Date <input type="date" value={sDraft.date} oninput={(e) => sChange({ date: e.currentTarget.value })} /></label>
              <label class="field">Start time <input type="time" value={sDraft.time ?? ""} oninput={(e) => sChange({ time: e.currentTarget.value })} /></label>
              <label class="field">Length (min) <input type="number" min="0" value={sDraft.duration} oninput={(e) => sChange({ duration: Number(e.currentTarget.value) })} /></label>
              <label class="field">Location <input value={sDraft.location ?? ""} oninput={(e) => sChange({ location: e.currentTarget.value })} /></label>
              <label class="field">Focus <input value={sDraft.focus ?? ""} placeholder="e.g. Breakdown speed" oninput={(e) => sChange({ focus: e.currentTarget.value })} /></label>
            </div>
          </div>

          <div class="card col">
            <div class="row"><h3 class="grow">Plan</h3><span class="muted small">{planned} of {sDraft.duration} min planned</span></div>
            {#each sDraft.drills as p, i}
              {@const dr = dById[p.drill_id]}
              <div class="plan row">
                <span class="num">{i + 1}</span>
                <div class="grow col" style="gap:4px">
                  <b>{dr?.name ?? "(deleted drill)"}</b>
                  <input placeholder="Coaching points for today" value={p.points ?? ""} oninput={(e) => sChange({ drills: sDraft.drills.map((x, k) => (k === i ? { ...x, points: e.currentTarget.value } : x)) })} />
                </div>
                <label class="row small"><input type="number" min="0" value={p.minutes} style="width:64px" oninput={(e) => sChange({ drills: sDraft.drills.map((x, k) => (k === i ? { ...x, minutes: Number(e.currentTarget.value) } : x)) })} /> min</label>
                <button class="ghost" title="Move up" onclick={() => movePlan(i, -1)}><Icon name="arrowUp" size={14} /></button>
                <button class="ghost" title="Move down" onclick={() => movePlan(i, 1)}><Icon name="arrowDown" size={14} /></button>
                <button class="ghost danger" title="Remove from plan" onclick={() => sChange({ drills: sDraft.drills.filter((_, k) => k !== i) })}><Icon name="x" size={14} /></button>
              </div>
            {/each}
            <div class="row">
              <select onchange={(e) => { addPlanDrill(e.currentTarget.value); e.currentTarget.value = ""; }}>
                <option value="">+ Add a drill from the library…</option>
                {#each categories as c}
                  {@const list = drills.filter((d) => d.category === c)}
                  {#if list.length}<optgroup label={c}>{#each list as d}<option value={d.id}>{d.name}</option>{/each}</optgroup>{/if}
                {/each}
                {#if drills.some((d) => !d.category)}
                  <optgroup label="Uncategorised">{#each drills.filter((d) => !d.category) as d}<option value={d.id}>{d.name}</option>{/each}</optgroup>
                {/if}
              </select>
              <button class="ghost" onclick={() => (tab = "drills")}>Open drill library</button>
            </div>
          </div>

          <div class="card col">
            <h3>Instructions by position group</h3>
            <div class="grid2">
              {#each cfg.groups as g}
                <label class="field">{g}
                  <textarea rows="2" placeholder="What this group should focus on" value={sDraft.group_notes?.[g] ?? ""} oninput={(e) => sChange({ group_notes: { ...sDraft.group_notes, [g]: e.currentTarget.value } })}></textarea>
                </label>
              {/each}
            </div>
            <label class="field">Session notes <textarea rows="3" value={sDraft.notes ?? ""} oninput={(e) => sChange({ notes: e.currentTarget.value })}></textarea></label>
          </div>

          <div class="actions">
            <button class="primary" onclick={saveSession} disabled={!sDirty}><Icon name="save" /> Save session</button>
            <button onclick={async () => { if (!sDirty || await ask("Discard changes?")) { sDraft = null; sDirty = false; } }}>Close</button>
            <span class="spacer"></span>
            {#if sDraft.id}<button class="danger" onclick={deleteSession}><Icon name="trash" /> Delete session</button>{/if}
          </div>

          {#if sDraft.id}
            <div class="card col">
              <h3>Attendance <span class="muted small">(saved as you go)</span></h3>
              <table>
                <thead><tr><th>Player</th><th>Present</th></tr></thead>
                <tbody>
                  {#each squad as p (p.id)}
                    {@const a = att(p.id)}

                    <tr>
                      <td>{playerLabel(p)}</td>
                      <td><input type="checkbox" checked={a?.present ?? false} onchange={(e) => setPresent(p.id, e.currentTarget.checked)} /></td>

                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {:else}
            <div class="muted small">Save the session to take attendance.</div>
          {/if}
        </div>
      {:else}
        <div class="empty">Choose a session, or plan a new one.</div>
      {/if}
    </div>
  {:else}
    <div class="layout">
      <div class="col">
        <button class="primary" onclick={() => editDrill(null)}><Icon name="plus" /> New drill</button>
        <input placeholder="Search drills" bind:value={q} />
        <select bind:value={cat}>
          <option value="">All categories ({drills.length})</option>
          {#each categories as c}<option value={c}>{c} ({drills.filter((d) => d.category === c).length})</option>{/each}
          {#if drills.some((d) => !d.category)}<option value="__none">Uncategorised ({drills.filter((d) => !d.category).length})</option>{/if}
        </select>
        <div class="list card flush">
          {#each shownDrills as d (d.id)}
            <button class="item rowbtn" class:on={d.id === dDraft?.id} onclick={() => editDrill(d)}>
              <b>{d.name}</b>
              <span class="small muted">{d.category || "Uncategorised"}{d.minutes ? " · " + d.minutes + " min" : ""}</span>
            </button>
          {:else}
            <div class="empty col">
              <span>No drills{cat || q ? " match" : " yet"}.</span>
            </div>
          {/each}
        </div>
        {#if (STARTER_DRILLS[app.team.sport] ?? []).some((x) => !drills.some((d) => d.name === x.name))}
          <button class="ghost small" onclick={addStarters}><Icon name="plus" size={14} /> Add {cfg.name.toLowerCase()} starter drills</button>
        {/if}
      </div>

      {#if dDraft}
        <div class="col">
          <div class="card col">
            <div class="row">
              <h2 class="grow">{dDraft.id ? "Edit drill" : "New drill"}</h2>
              {#if dDirty}<span class="badge warn">Unsaved changes</span>{:else if dDraft.id}<span class="badge good">Saved</span>{/if}
            </div>
            <div class="grid3">
              <label class="field">Name <input value={dDraft.name} placeholder="e.g. 3v2 overload" oninput={(e) => dChange({ name: e.currentTarget.value })} /></label>
              <label class="field">Category
                <select value={dDraft.category ?? ""} onchange={(e) => dChange({ category: e.currentTarget.value })}>
                  <option value="">Uncategorised</option>
                  {#each categories as c}<option>{c}</option>{/each}
                </select>
              </label>
              <label class="field">Minutes <input type="number" min="0" value={dDraft.minutes ?? ""} oninput={(e) => dChange({ minutes: Number(e.currentTarget.value) })} /></label>
              <label class="field">Players <input value={dDraft.players ?? ""} placeholder="e.g. 8v4" oninput={(e) => dChange({ players: e.currentTarget.value })} /></label>
              <label class="field">Area <input value={dDraft.area ?? ""} placeholder="e.g. 30×20 m" oninput={(e) => dChange({ area: e.currentTarget.value })} /></label>
              <label class="field">Intensity
                <select value={dDraft.intensity ?? "Medium"} onchange={(e) => dChange({ intensity: e.currentTarget.value })}>
                  {#each ["Low", "Medium", "High", "Match intensity"] as v}<option>{v}</option>{/each}
                </select>
              </label>
            </div>
            <div class="row wrap small">
              <span class="muted">Equipment:</span>
              {#each cfg.equipment as k}
                <button class="small" class:on={(dDraft.equipment ?? []).includes(k)} onclick={() => toggleEquip(k)}>{EQUIPMENT[k] ?? k}</button>
              {/each}
            </div>
          </div>

          <div class="card">
            <BoardEditor board={dDraft.board} title={dDraft.name} onchange={(nb) => dChange({ board: nb })} />
          </div>

          <div class="grid2">
            <label class="field card">Description <textarea rows="5" value={dDraft.description ?? ""} oninput={(e) => dChange({ description: e.currentTarget.value })}></textarea></label>
            <label class="field card">Coaching points <textarea rows="5" value={dDraft.coaching_points ?? ""} oninput={(e) => dChange({ coaching_points: e.currentTarget.value })}></textarea></label>
          </div>

          <div class="actions">
            <button class="primary" onclick={saveDrill} disabled={!dDirty}><Icon name="save" /> Save drill</button>
            {#if dDraft.id}<button onclick={duplicateDrill}><Icon name="copy" /> Duplicate</button>{/if}
            <button onclick={async () => { if (!dDirty || await ask("Discard changes?")) { dDraft = null; dDirty = false; } }}>Close</button>
            <span class="spacer"></span>
            {#if dDraft.id}<button class="danger" onclick={deleteDrill}><Icon name="trash" /> Delete drill</button>{/if}
          </div>
        </div>
      {:else}
        <div class="empty">Choose a drill, or create one. Drills are organised by {cfg.name.toLowerCase()} categories.</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .layout { display: grid; grid-template-columns: 280px 1fr; gap: 16px; margin-top: 4px; align-items: start; }
  .plan { align-items: flex-start; padding: 8px 0; border-bottom: 1px solid var(--line); }
  .num { width: 24px; height: 24px; border-radius: 50%; background: var(--panel-2); display: grid; place-items: center; font-size: 12px; flex: none; }
</style>
