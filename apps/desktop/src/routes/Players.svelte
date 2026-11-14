<script>
  import { db, invoke, fileUrl, fmtTime, byKey, playerLabel, sortPlayers, today, ask } from "$shared/api.js";
  import { open } from "@tauri-apps/plugin-dialog";
  import Icon from "$shared/Icon.svelte";
  import { groupPlayers, groupOf } from "$shared/sports.js";
  import { app, attempt, teamCfg } from "../lib/state.svelte.js";
  import LineChart from "../components/LineChart.svelte";

  let players = $state([]);
  let templates = $state([]);
  let assessments = $state([]);
  let injuries = $state([]);
  let attendance = $state([]);

  let clips = $state([]);
  let videos = $state([]);
  let minutes = $state({});
  const cfg = $derived(teamCfg());
  let groupFilter = $state("");
  let q = $state("");
  let selId = $state(null);
  let tab = $state("profile");
  let newName = $state("");

  const sel = $derived(players.find((p) => p.id === selId) ?? null);
  const vById = $derived(byKey(videos, "id"));

  async function load() {
    [players, templates, assessments, injuries, attendance, clips, videos] = await Promise.all(
      ["player", "assessment_template", "assessment", "injury", "attendance", "clip", "video"].map((k) => db.list(k))
    );
    minutes = await invoke("minutes_played", { matchId: null });
  }
  load();
  const grouped = $derived(
    groupPlayers(cfg, sortPlayers(players).filter((p) => !q.trim() || playerLabel(p).toLowerCase().includes(q.trim().toLowerCase()))).filter(([g]) => !groupFilter || g === groupFilter)
  );

  function status(pid) {
    const last = injuries.filter((i) => i.player_id === pid).sort((a, b) => (b.date || "").localeCompare(a.date || ""))[0];
    return last?.status ?? "available";
  }
  function attendancePct(pid) {
    const mine = attendance.filter((a) => a.player_id === pid);
    if (!mine.length) return null;
    return Math.round((mine.filter((a) => a.present).length / mine.length) * 100);
  }


  async function addPlayer() {
    if (!newName.trim()) return;
    const p = await attempt(() => db.put("player", { name: newName.trim(), number: null, position: "" }));
    newName = "";
    await load();
    selId = p?.id;
  }
  async function set(fields) {
    Object.assign(sel, fields);
    await attempt(() => db.put("player", fields, sel.id));
  }
  async function pickPhoto() {
    const path = await open({ filters: [{ name: "Image", extensions: ["jpg", "jpeg", "png", "webp"] }] });
    if (path) set({ photo: path });
  }
  async function removePlayer() {
    if (!await ask(`Remove ${sel.name}? Their history stays in old reports.`)) return;
    await db.del("player", sel.id);
    selId = null;
    load();
  }

  // ---- assessments ----
  let tplName = $state("");
  let tplCriteria = $state("");
  let tplId = $state(null);
  let scores = $state({});
  let assessNotes = $state("");
  const tpl = $derived(templates.find((t) => t.id === tplId) ?? templates[0] ?? null);
  async function addTemplate() {
    const criteria = tplCriteria.split(",").map((s) => s.trim()).filter(Boolean);
    if (!tplName.trim() || !criteria.length) return;
    const t = await attempt(() => db.put("assessment_template", { name: tplName.trim(), criteria, scale: 5 }));
    tplName = tplCriteria = "";
    await load();
    tplId = t?.id;
  }
  async function saveAssessment() {
    await attempt(
      () => db.put("assessment", { player_id: sel.id, template_id: tpl.id, date: today(), scores: $state.snapshot(scores), notes: assessNotes }),
      "Assessment saved"
    );
    scores = {};
    assessNotes = "";
    load();
  }
  const myAssessments = $derived(
    sel && tpl ? assessments.filter((a) => a.player_id === sel.id && a.template_id === tpl.id).sort((a, b) => a.date.localeCompare(b.date)) : []
  );
  const progress = $derived(
    (tpl?.criteria ?? []).map((c) => ({ name: c, points: myAssessments.filter((a) => a.scores?.[c] != null).map((a) => [a.date, Number(a.scores[c])]) }))
  );

  // ---- injuries ----
  let inj = $state({ type: "", status: "out", expected_return: "", notes: "" });
  async function delInjury(i) {
    if (!await ask(`Delete the ${i.date} entry "${i.type || i.status}"?`)) return;
    await attempt(() => db.del("injury", i.id), "Entry deleted");
    load();
  }
  async function delAssessment(a) {
    if (!await ask(`Delete the assessment from ${a.date}?`)) return;
    await attempt(() => db.del("assessment", a.id), "Assessment deleted");
    load();
  }
  async function delTemplate() {
    if (!tpl || !await ask(`Delete the rating template "${tpl.name}"? Past assessments made with it are kept but won't show here.`)) return;
    await attempt(() => db.del("assessment_template", tpl.id), "Template deleted");
    tplId = null;
    load();
  }
  async function addInjury() {
    await attempt(() => db.put("injury", { player_id: sel.id, date: today(), ...$state.snapshot(inj) }), "Saved");
    inj = { type: "", status: "out", expected_return: "", notes: "" };
    load();
  }
</script>

<div class="page">
  <div class="ph">
    <div>
      <h1>Players</h1>
      <div class="sub">{players.length} in the squad · grouped by {cfg.name.toLowerCase()} position groups</div>
    </div>
  </div>
  <div class="layout">
    <div class="col">
      <div class="row">
        <input class="grow" placeholder="New player name" bind:value={newName} onkeydown={(e) => e.key === "Enter" && addPlayer()} />
        <button class="primary" onclick={addPlayer}>Add</button>
      </div>
      <div class="row">
        <input class="grow" placeholder="Search" bind:value={q} />
        <select bind:value={groupFilter} title="Position group">
          <option value="">All groups</option>
          {#each cfg.groups as g}<option>{g}</option>{/each}
          <option>Unassigned</option>
        </select>
      </div>
      <div class="list card flush">
        {#each grouped as [g, list] (g)}
          <div class="grouphead">{g} <span class="muted">{list.length}</span></div>
          {#each list as p (p.id)}
            {@const st = status(p.id)}
            <div class="item row" class:on={p.id === selId} role="button" tabindex="0" onclick={() => (selId = p.id)} onkeydown={(e) => e.key === "Enter" && (selId = p.id)}>
              <span class="grow">{playerLabel(p)}</span>
              <span class="muted small">{p.position ?? ""}</span>
              {#if st !== "available"}<span class="badge {st === 'out' ? 'bad' : 'warn'}">{st}</span>{/if}
            </div>
          {/each}
        {:else}
          <div class="empty">{players.length ? "No players match." : "Add your squad to get started. You can also import a CSV under Sync & data."}</div>
        {/each}
      </div>
    </div>

    {#if sel}
      <div class="col">
        <div class="tabs">
          {#each [["profile", "Profile"], ["assess", "Assessments"], ["injury", "Injuries & availability"], ["load", "Attendance"], ["clips", "Clips"]] as [k, l]}
            <button class:on={tab === k} onclick={() => (tab = k)}>{l}</button>
          {/each}
        </div>

        {#if tab === "profile"}
          <div class="card row" style="align-items:start;gap:16px">
            <button class="photo" onclick={pickPhoto} title="Choose photo">
              {#if sel.photo}<img src={fileUrl(sel.photo)} alt="" />{:else}+ Photo{/if}
            </button>
            <div class="grid3 grow">
              <label class="field">Name <input value={sel.name} onchange={(e) => set({ name: e.currentTarget.value })} /></label>
              <label class="field">Shirt number <input type="number" value={sel.number ?? ""} onchange={(e) => set({ number: e.currentTarget.value === "" ? null : Number(e.currentTarget.value) })} /></label>
              <label class="field">Position
                <select value={sel.position ?? ""} onchange={(e) => set({ position: e.currentTarget.value })}>
                  <option value="">—</option>
                  {#each cfg.groups as g}
                    <optgroup label={g}>{#each cfg.positions.filter((x) => x.group === g) as p}<option value={p.code}>{p.code} — {p.name}</option>{/each}</optgroup>
                  {/each}
                  {#if sel.position && groupOf(cfg, sel.position) === "Other"}<option value={sel.position}>{sel.position}</option>{/if}
                </select>
              </label>
              <label class="field">Date of birth <input type="date" value={sel.dob ?? ""} onchange={(e) => set({ dob: e.currentTarget.value })} /></label>
              <label class="field">Preferred foot / hand <input value={sel.foot ?? ""} onchange={(e) => set({ foot: e.currentTarget.value })} /></label>
              <label class="field">Emergency contact <input value={sel.contact ?? ""} onchange={(e) => set({ contact: e.currentTarget.value })} /></label>
            </div>
          </div>
          <label class="field card">Coach notes <textarea rows="5" value={sel.notes ?? ""} onchange={(e) => set({ notes: e.currentTarget.value })}></textarea></label>
          <div class="row muted small">
            Minutes this season: {Math.round(minutes[sel.id] ?? 0)} · Attendance: {attendancePct(sel.id) ?? "—"}%
            <span class="spacer"></span><button class="danger" onclick={removePlayer}><Icon name="trash" size={14} /> Remove player</button>
          </div>
        {:else if tab === "assess"}
          <div class="grid2">
            <div class="card col">
              <div class="row">
                <select class="grow" value={tpl?.id} onchange={(e) => (tplId = e.currentTarget.value)}>
                  {#each templates as t}<option value={t.id}>{t.name}</option>{/each}
                </select>
                {#if tpl}<button class="ghost danger" title="Delete this template" onclick={delTemplate}><Icon name="trash" size={14} /></button>{/if}
              </div>
              {#if tpl}
                {#each tpl.criteria as c}
                  <div class="row">
                    <span class="grow">{c}</span>
                    {#each Array.from({ length: tpl.scale ?? 5 }, (_, i) => i + 1) as n}
                      <button class="small" class:on={scores[c] === n} onclick={() => (scores[c] = n)}>{n}</button>
                    {/each}
                  </div>
                {/each}
                <textarea placeholder="Notes" bind:value={assessNotes}></textarea>
                <button class="primary" onclick={saveAssessment} disabled={!Object.keys(scores).length}>Save assessment</button>
              {:else}
                <div class="muted">Create a rating template first.</div>
              {/if}
              <details>
                <summary class="small muted">New rating template</summary>
                <div class="col" style="margin-top:8px">
                  <input placeholder="Template name, e.g. Technical" bind:value={tplName} />
                  <input placeholder="Criteria, comma separated: First touch, Passing, Scanning" bind:value={tplCriteria} />
                  <button onclick={addTemplate}>Create template</button>
                </div>
              </details>
            </div>
            <div class="card col">
              <h3>Progress</h3>
              {#if myAssessments.length}
                <LineChart series={progress} max={tpl?.scale ?? 5} />
                {#each [...myAssessments].reverse() as a}
                  <div class="small row" style="align-items:flex-start"><div class="grow"><b>{a.date}</b> <span class="muted">{Object.entries(a.scores ?? {}).map(([k, v]) => `${k} ${v}`).join(" · ")}</span>{#if a.notes}<div class="muted">{a.notes}</div>{/if}</div><button class="ghost danger small" title="Delete" onclick={() => delAssessment(a)}><Icon name="trash" size={13} /></button></div>
                {/each}
              {:else}
                <div class="muted">No assessments yet.</div>
              {/if}
            </div>
          </div>
        {:else if tab === "injury"}
          <div class="card col">
            <div class="row wrap">
              <input placeholder="Injury or reason, e.g. Ankle sprain" bind:value={inj.type} class="grow" />
              <select bind:value={inj.status}><option value="out">Out</option><option value="limited">Limited</option><option value="available">Available</option></select>
              <label class="small row">Expected back <input type="date" bind:value={inj.expected_return} /></label>
            </div>
            <textarea placeholder="Notes" bind:value={inj.notes}></textarea>
            <div><button class="primary" onclick={addInjury}>Add entry</button></div>
          </div>
          <table>
            <thead><tr><th>Date</th><th>Injury / reason</th><th>Status</th><th>Expected back</th><th>Notes</th><th></th></tr></thead>
            <tbody>
              {#each injuries.filter((i) => i.player_id === sel.id).sort((a, b) => b.date.localeCompare(a.date)) as i (i.id)}
                <tr><td>{i.date}</td><td>{i.type}</td><td><span class="badge {i.status === 'out' ? 'bad' : i.status === 'limited' ? 'warn' : 'good'}">{i.status}</span></td><td>{i.expected_return ?? ""}</td><td class="muted">{i.notes ?? ""}</td><td><button class="ghost danger small" title="Delete entry" onclick={() => delInjury(i)}><Icon name="trash" size={13} /></button></td></tr>
              {/each}
            </tbody>
          </table>
        {:else if tab === "load"}
          {@const mine = attendance.filter((a) => a.player_id === sel.id).sort((a, b) => (b.date || "").localeCompare(a.date || ""))}
          <div class="muted small">Attendance {attendancePct(sel.id) ?? "—"}% across {mine.length} sessions.</div>
          <table>
            <thead><tr><th>Date</th><th>Training</th></tr></thead>
            <tbody>
              {#each mine as a (a.id)}
                <tr><td>{a.date ?? ""}</td><td><span class="badge {a.present ? 'good' : 'bad'}">{a.present ? "Present" : "Absent"}</span></td></tr>
              {:else}
                <tr><td colspan="2" class="muted">No attendance recorded yet.</td></tr>
              {/each}
            </tbody>
          </table>
        {:else if tab === "clips"}
          <div class="list card" style="padding:0">
            {#each clips.filter((c) => c.player_id === sel.id) as c}
              <div class="item row"><span class="grow">{c.name}</span><span class="muted small">{vById[c.video_id]?.name} · {fmtTime(c.start)}</span></div>
            {:else}
              <div class="empty">Clips appear here when tags with this player are turned into clips.</div>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <div class="empty">Choose a player.</div>
    {/if}
  </div>
</div>

<style>
  .layout { display: grid; grid-template-columns: 300px 1fr; gap: 16px; align-items: start; }
  .grouphead { padding: 8px 12px 4px; font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--muted); background: var(--panel-2); position: sticky; top: 0; }
  .photo { width: 110px; height: 130px; padding: 0; overflow: hidden; display: grid; place-items: center; color: var(--muted); }
  .photo img { width: 100%; height: 100%; object-fit: cover; }
</style>
