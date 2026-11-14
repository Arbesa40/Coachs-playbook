<script>
  import { invoke, db, ask } from "$shared/api.js";
  import { SPORTS, formatsOf } from "$shared/sports.js";
  import Icon from "$shared/Icon.svelte";
  import TeamBadge from "$shared/TeamBadge.svelte";
  import { imageToDataUrl } from "$shared/image.js";
  import { theme, setTheme, THEME_OPTIONS } from "$shared/theme.svelte.js";

  import { app, attempt, toast, teamCfg } from "../lib/state.svelte.js";
  let logoInput;
  async function pickLogo(e) {
    const f = e.currentTarget.files?.[0];
    e.currentTarget.value = "";
    if (!f) return;
    try {
      await setTeam({ logo: await imageToDataUrl(f) });
    } catch (err) {
      toast(err.message, "error");
    }
  }
  async function deleteTeam() {
    const tm = app.team;
    if (!(await ask(`Delete the team "${tm.name}"?\n\nIts players, matches, sessions, tags and clips will be removed. Video files on your disk are not touched. A copy is kept in the "deleted-teams" folder inside the data folder.`))) return;
    const ok = await attempt(() => invoke("delete_team", { teamId: tm.id }).then(() => true), `Deleted ${tm.name}`);
    if (!ok) return;
    try { localStorage.removeItem("lastTeamId"); } catch {}
    app.prevTeam = null;
    app.team = null;
  }

  const cfg = $derived(teamCfg());
  const variations = $derived([...cfg.scoring, ...cfg.discipline, ...(cfg.other ?? [])].filter((d) => d.variation));
  async function setTeam(fields) {
    const r = await attempt(() => db.put("team", fields, app.team.id), "Saved");
    if (r) app.team = { ...app.team, ...fields };
  }
</script>

<div class="page">
<div class="col" style="max-width:760px">
  <div class="ph"><div><h1>Settings</h1><div class="sub">{app.team?.name}</div></div></div>
  <div class="card col">
    <h2>Team and competition rules</h2>
    <div class="grid2">
      <label class="field">Team name <input value={app.team?.name ?? ""} onchange={(e) => setTeam({ name: e.currentTarget.value })} /></label>
      <div class="field small muted">Logo
        <div class="row">
          <TeamBadge team={app.team} size={40} />
          <button class="small" onclick={() => logoInput.click()}><Icon name="image" size={14} /> {app.team?.logo ? "Change" : "Choose"}…</button>
          {#if app.team?.logo}<button class="ghost small" onclick={() => setTeam({ logo: null })}>Remove</button>{/if}
          <input type="file" accept="image/*" hidden bind:this={logoInput} onchange={pickLogo} />
        </div>
      </div>
      <label class="field">Sport <input value={SPORTS[app.team?.sport]?.name ?? app.team?.sport} disabled /></label>
      {#if formatsOf(app.team?.sport).length > 1}
        <label class="field">Format
          <select value={cfg.formatId} onchange={(e) => setTeam({ format: e.currentTarget.value })}>
            {#each formatsOf(app.team?.sport) as [id, label]}<option value={id}>{label}</option>{/each}
          </select>
        </label>
      {/if}
    </div>
    <div class="small muted">
      {cfg.formatLabel}: {cfg.periods.join(", ")}{cfg.periodMinutes ? ` of ${cfg.periodMinutes} min` : ""} · {cfg.onField} on the field{cfg.sinBinMinutes ? ` · sin bin ${cfg.sinBinMinutes} min` : ""}{cfg.subs?.note ? ` · ${cfg.subs.note}` : ""}
    </div>
    {#if variations.length}
      <h3 class="sec">Rule variations used in your competition</h3>
      {#each variations as v}
        <label class="row small"><input type="checkbox" checked={app.team?.rule_options?.[v.id] ?? false} onchange={(e) => setTeam({ rule_options: { ...(app.team?.rule_options ?? {}), [v.id]: e.currentTarget.checked } })} /> {v.label}</label>
      {/each}
    {/if}
    <div class="small muted">Period length can also be changed per match (e.g. finals, youth games) in the match details.</div>
  </div>
  <div class="card col">
    <h2>Appearance</h2>
    <div class="seg" role="radiogroup" aria-label="Theme">
      {#each THEME_OPTIONS as [id, label]}
        <button role="radio" aria-checked={theme.pref === id} class:on={theme.pref === id} onclick={() => setTheme(id)}>
          <Icon name={id === "system" ? "monitor" : id === "light" ? "sun" : "moon"} size={16} /> {label}
        </button>
      {/each}
    </div>
    <div class="muted small">“Match the system” follows the Windows light/dark setting. Set per computer.</div>
  </div>
  {#if app.info && !app.info.ffmpeg}
    <div class="card col">
      <div class="badge bad">Video tools are missing, so smooth playback and MP4 export won't work. Reinstall Coach's Playbook to fix this.</div>
    </div>
  {/if}
  <div class="card col dz">
    <h2>Delete this team</h2>
    <div class="small muted">Removes {app.team?.name} from this computer. Paired phones keep their copy until you remove the app's data there.</div>
    <div><button class="danger" onclick={deleteTeam}><Icon name="trash" size={16} /> Delete team…</button></div>
  </div>
  {#if app.info?.version}<div class="small muted">Version {app.info.version}</div>{/if}
</div>
</div>

<style>
  .seg { align-self: flex-start; }
</style>
