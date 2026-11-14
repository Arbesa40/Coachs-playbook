<script>
  import { invoke, db, ask } from "$shared/api.js";
  import Icon from "$shared/Icon.svelte";
  import TeamBadge from "$shared/TeamBadge.svelte";
  import { SPORTS, formatsOf, defaultFormat, teamConfig } from "$shared/sports.js";
  import { imageToDataUrl } from "$shared/image.js";
  import { theme } from "$shared/theme.svelte.js";
  import { app, attempt, toast } from "../lib/state.svelte.js";
  import { open } from "@tauri-apps/plugin-dialog";
  import lockupLight from "$shared/brand/lockup-light.png";
  import lockupDark from "$shared/brand/lockup-dark.png";

  const lockup = $derived(theme.resolved === "light" ? lockupDark : lockupLight);
  let teams = $state([]);
  let templates = $state([]);
  let name = $state("");
  let sport = $state("football");
  let format = $state(defaultFormat("football"));
  let logo = $state(null);
  let busy = $state(false);
  let fileInput;

  const load = () => invoke("list_teams").then((t) => (teams = t.sort((a, b) => (a.name || "").localeCompare(b.name || ""))));
  load();
  invoke("list_templates").then((t) => (templates = t));

  $effect(() => {
    if (!formatsOf(sport).some(([id]) => id === format)) format = defaultFormat(sport);
  });

  // The team that was open before "Switch team" (if it still exists).
  const back = $derived(app.prevTeam && teams.some((t) => t.id === app.prevTeam.id) ? app.prevTeam : null);
  function goBack() {
    if (back) openTeam(back.id);
  }

  async function pickLogo(e) {
    const f = e.currentTarget.files?.[0];
    e.currentTarget.value = "";
    if (!f) return;
    try {
      logo = await imageToDataUrl(f);
    } catch (err) {
      toast(err.message, "error");
    }
  }
  async function create() {
    if (!name.trim() || busy) return;
    busy = true;
    try {
      const tm = await attempt(() => invoke("create_team", { name: name.trim(), sport }));
      if (!tm) return;
      const extra = { format, ...(logo ? { logo } : {}) };
      await db.put("team", extra, tm.id);
      app.prevTeam = null;
      app.team = { ...tm, ...extra };
    } finally {
      busy = false;
    }
  }
  async function openTeam(id) {
    const tm = await attempt(() => invoke("open_team", { teamId: id }));
    if (tm) {
      app.prevTeam = null;
      app.team = tm;
    }
  }
  async function removeTeam(tm) {
    const ok = await ask(
      `Delete the team "${tm.name}"?\n\nIts players, matches, sessions, tags and clips will be removed from Coach's Playbook. ` +
        `Video files on your disk are not touched. A copy of the team's data is kept in the "deleted-teams" folder inside the data folder.`
    );
    if (!ok) return;
    const r = await attempt(() => invoke("delete_team", { teamId: tm.id }).then(() => true), `Deleted ${tm.name}`);
    if (!r) return;
    if (app.prevTeam?.id === tm.id) app.prevTeam = null;
    try {
      if (JSON.parse(localStorage.getItem("lastTeamId")) === tm.id) localStorage.removeItem("lastTeamId");
    } catch {}
    load();
  }
  async function restore() {
    const path = await open({ filters: [{ name: "Coach's Playbook backup", extensions: ["cpbackup", "sqlite"] }] });
    if (!path) return;
    const tm = await attempt(() => invoke("restore", { path }), "Backup restored");
    if (tm) {
      app.prevTeam = null;
      app.team = tm;
    }
  }
  const fmtLabel = (tm) => {
    const c = teamConfig(tm);
    return c.formatLabel && c.formatLabel !== c.name ? c.formatLabel : "";
  };
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && back && !(e.target instanceof HTMLInputElement) && goBack()} />

<div class="home">
  <div class="inner">
    <div class="top">
      {#if back}
        <button class="ghost" onclick={goBack}><Icon name="chevronLeft" size={16} /> Back to {back.name}</button>
      {/if}
    </div>

    <div class="hero">
      <img src={lockup} alt="Coach's Playbook: game analysis and management" />
      <p class="muted">Free, open-source video analysis and squad management. Everything stays on this computer.</p>
    </div>

    <section class="card flush">
      <div class="head"><h2>Your teams</h2><span class="muted small">{teams.length}</span></div>
      {#if teams.length === 0}
        <div class="empty">No teams yet. Create one below.</div>
      {/if}
      <div class="list">
        {#each teams as tm (tm.id)}
          <div class="trow" class:cur={tm.id === back?.id}>
            <button class="open" onclick={() => openTeam(tm.id)}>
              <TeamBadge team={tm} size={36} />
              <span class="nm">
                <b>{tm.name}</b>
                <span class="muted small">{SPORTS[tm.sport]?.name ?? tm.sport}{fmtLabel(tm) ? " · " + fmtLabel(tm) : ""} · {tm.players} {tm.players === 1 ? "player" : "players"}</span>
              </span>
              {#if tm.id === back?.id}<span class="badge">Open</span>{/if}
              <Icon name="chevronRight" size={16} />
            </button>
            <button class="ghost danger del" title="Delete team" aria-label={"Delete " + tm.name} onclick={() => removeTeam(tm)}><Icon name="trash" size={16} /></button>
          </div>
        {/each}
      </div>
    </section>

    <section class="card col">
      <h2>New team</h2>
      <div class="newteam">
        <button class="logo" onclick={() => fileInput.click()} title="Choose a logo">
          {#if logo}<img src={logo} alt="Team logo" />{:else}<Icon name="image" size={22} /><span>Logo</span>{/if}
        </button>
        <input type="file" accept="image/*" hidden bind:this={fileInput} onchange={pickLogo} />
        <div class="col grow">
          <input placeholder="Team name, e.g. U14 Girls" bind:value={name} onkeydown={(e) => e.key === "Enter" && create()} />
          <div class="row wrap">
            <select bind:value={sport} title="Sport">
              {#each templates as tp}<option value={tp.id}>{SPORTS[tp.id]?.name ?? tp.name}</option>{/each}
            </select>
            {#if formatsOf(sport).length > 1}
              <select bind:value={format} title="Format">
                {#each formatsOf(sport) as [id, label]}<option value={id}>{label}</option>{/each}
              </select>
            {/if}
            {#if logo}<button class="ghost small" onclick={() => (logo = null)}>Remove logo</button>{/if}
            <span class="spacer"></span>
            <button class="primary" onclick={create} disabled={!name.trim() || busy}><Icon name="plus" size={16} /> Create team</button>
          </div>
        </div>
      </div>
      <div class="muted small">The sport and format set the match rules, positions, drill categories and tactics. You can change the format and logo later in Settings.</div>
    </section>

    <div class="row foot">
      <button onclick={restore}><Icon name="upload" size={16} /> Restore from backup…</button>
    </div>
  </div>
</div>

<style>
  .home { height: 100%; overflow: auto; }
  .inner { max-width: 760px; margin: 0 auto; padding: 16px 20px 40px; display: flex; flex-direction: column; gap: 16px; }
  .top { min-height: 34px; display: flex; align-items: center; }
  .hero { text-align: center; }
  .hero img { height: 150px; width: auto; }
  .hero p { margin: 6px 0 0; }
  .head { display: flex; align-items: baseline; gap: 8px; padding: 14px 14px 6px; }
  .head h2 { margin: 0; }
  .trow { display: flex; align-items: center; border-top: 1px solid var(--line); }
  .trow.cur { box-shadow: inset 3px 0 0 var(--accent); }
  .open { flex: 1; min-width: 0; display: flex; align-items: center; justify-content: flex-start; gap: 12px; text-align: left; background: none; border: 0; border-radius: 0; padding: 10px 14px; }
  .open:hover { background: var(--hover); }
  .nm { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .nm b { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .del { margin-right: 8px; opacity: 0.6; }
  .trow:hover .del, .del:focus-visible { opacity: 1; }
  .newteam { display: flex; gap: 12px; align-items: flex-start; }
  .logo { width: 76px; height: 76px; flex: none; flex-direction: column; gap: 2px; border-style: dashed; color: var(--muted); font-size: 12px; padding: 4px; }
  .logo img { width: 100%; height: 100%; object-fit: contain; }
  .foot { justify-content: center; }
</style>
