<script>
  import Icon from "$shared/Icon.svelte";
  import DialogHost from "$shared/DialogHost.svelte";
  import { invoke } from "$shared/api.js";
  import { t } from "$shared/i18n.js";
  import TeamBadge from "$shared/TeamBadge.svelte";
  import { SPORTS, teamConfig } from "$shared/sports.js";
  import { theme } from "$shared/theme.svelte.js";
  import emblemLight from "$shared/brand/emblem-light.png";
  import emblemDark from "$shared/brand/emblem-dark.png";
  import { app, toast } from "./lib/state.svelte.js";
  import { db } from "$shared/api.js";
  import { sweepStalledMatches } from "$shared/matchCleanup.js";
  import Home from "./routes/Home.svelte";
  import Analysis from "./routes/Analysis.svelte";
  import Matches from "./routes/Matches.svelte";
  import Players from "./routes/Players.svelte";
  import Training from "./routes/Training.svelte";
  import Tactics from "./routes/Tactics.svelte";
  import Reports from "./routes/Reports.svelte";
  import SyncData from "./routes/SyncData.svelte";
  import Settings from "./routes/Settings.svelte";

  const views = [
    ["analysis", "video", Analysis],
    ["matches", "calendar", Matches],
    ["players", "users", Players],
    ["training", "clipboard", Training],
    ["tactics", "tactic", Tactics],
    ["reports", "printer", Reports],
    ["sync", "sync", SyncData],
    ["settings", "settings", Settings],
  ];
  const emblem = $derived(theme.resolved === "light" ? emblemDark : emblemLight);
  const fmt = $derived(app.team ? teamConfig(app.team) : null);
  function switchTeam() {
    app.prevTeam = app.team;
    app.team = null;
  }
  const View = $derived(views.find(([k]) => k === app.view)?.[2] ?? Analysis);

  invoke("app_info").then((i) => (app.info = i));

  // Crash recovery: reopen the last team automatically.
  const stored = (k, d) => {
    try {
      return JSON.parse(localStorage.getItem(k)) ?? d;
    } catch {
      return d;
    }
  };
  const last = stored("lastTeamId", null);
  if (last) invoke("open_team", { teamId: last }).then((tm) => (app.team = tm)).catch(() => {});
  $effect(() => {
    try {
      if (app.team) localStorage.setItem("lastTeamId", JSON.stringify(app.team.id));
    } catch {}
  });

  // Remove running matches that were abandoned (an hour past their expected end, untouched for an hour).
  $effect(() => {
    const team = app.team;
    if (!team) return;
    const sweep = () =>
      sweepStalledMatches(db, team)
        .then((gone) => gone.length && toast(`Removed ${gone.length} abandoned match${gone.length > 1 ? "es" : ""} (vs ${gone.map((m) => m.opponent).join(", ")})`))
        .catch(() => {});
    sweep();
    const i = setInterval(sweep, 5 * 60_000);
    return () => clearInterval(i);
  });

  // The navigation hides itself; hover the left edge to bring it back, or pin it open.
  let pinned = $state(stored("navPinned", false));
  let open = $state(false);
  let hideTimer;
  $effect(() => {
    try {
      localStorage.setItem("navPinned", JSON.stringify(pinned));
    } catch {}
  });
  function show() {
    clearTimeout(hideTimer);
    open = true;
  }
  function hideSoon() {
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => (open = false), 250);
  }
  function go(key) {
    app.view = key;
    if (!pinned) open = false;
  }
</script>

<svelte:window
  onmousemove={(e) => { if (open && !pinned && e.clientX > 230) hideSoon(); }}
  onkeydown={(e) => { if (e.key === "Escape" && open && !pinned) open = false; }}
/>

{#if !app.team}
  <Home />
{:else}
  <div class="shell" class:pinned>
    {#if !pinned}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="edge no-print" onmouseenter={show} onclick={show} onkeydown={() => {}} role="button" tabindex="-1" title="Menu"></div>
    {/if}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <nav class="side no-print" class:open={pinned || open} onmouseenter={show} onmouseleave={() => !pinned && hideSoon()}>
      <div class="brand">
        <img src={emblem} alt="" />
        <div>
          <div class="wm1">COACH'S</div>
          <div class="wm2">PLAYBOOK</div>
        </div>
      </div>
      <div class="team" title={app.team.name}>
        <TeamBadge team={app.team} size={28} />
        <div style="min-width:0"><b>{app.team.name}</b><span>{SPORTS[app.team.sport]?.name ?? app.team.sport}{fmt?.formatLabel && fmt.formatLabel !== SPORTS[app.team.sport]?.name ? " · " + fmt.formatLabel : ""}</span></div>
      </div>
      {#each views as [key, icon]}
        <button class:on={app.view === key} onclick={() => go(key)}><span class="ic"><Icon name={icon} /></span>{t("nav." + key)}</button>
      {/each}
      <div class="spacer"></div>
      <button onclick={() => (pinned = !pinned)} title="Keep this menu open"><span class="ic"><Icon name={pinned ? "pinOff" : "pin"} /></span>{pinned ? "Unpin menu" : "Pin menu"}</button>
      <button onclick={switchTeam}><span class="ic"><Icon name="swap" /></span>{t("nav.teams")}</button>
    </nav>
    <main class="main" onpointerdown={() => { if (open && !pinned) open = false; }}>
      {#key app.team.id}
        <View />
      {/key}
    </main>
  </div>
{/if}

{#if app.toast}
  <div class="toast" class:error={app.toast.kind === "error"}>{app.toast.msg}</div>
{/if}

<DialogHost />
