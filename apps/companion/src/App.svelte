<script>
  import Icon from "$shared/Icon.svelte";
  import DialogHost from "$shared/DialogHost.svelte";
  import TeamBadge from "$shared/TeamBadge.svelte";
  import { t } from "$shared/i18n.js";
  import { SPORTS, teamConfig } from "$shared/sports.js";
  import { theme, setTheme, THEME_OPTIONS } from "$shared/theme.svelte.js";
  import emblemLight from "$shared/brand/emblem-light.png";
  import emblemDark from "$shared/brand/emblem-dark.png";
  import { ui, refreshInfo, toast } from "./state.svelte.js";
  import { db } from "$shared/api.js";
  import { sweepStalledMatches } from "$shared/matchCleanup.js";
  import Match from "./screens/Match.svelte";
  import Training from "./screens/Training.svelte";
  import Squad from "./screens/Squad.svelte";
  import Tactics from "./screens/Tactics.svelte";
  import Playlists from "./screens/Playlists.svelte";
  import Notes from "./screens/Notes.svelte";
  import Sync from "./screens/Sync.svelte";

  // Five thumb-reachable tabs; less frequent screens live under "More".
  const screens = {
    match: ["whistle", Match],
    attendance: ["clipboard", Training],
    assess: ["users", Squad],
    tactics: ["tactic", Tactics],
    playlists: ["film", Playlists],
    notes: ["notes", Notes],
    sync: ["sync", Sync],
  };
  const primary = ["match", "attendance", "assess", "tactics"];
  const more = [
    ["notes", "Voice, photo and text notes"],
    ["playlists", "Clip playlists from the PC"],
    ["sync", "Pair and sync with the PC"],
  ];
  const Screen = $derived(screens[ui.tab]?.[1] ?? null);
  const inMore = $derived(ui.tab === "more" || more.some(([k]) => k === ui.tab));
  const emblem = $derived(theme.resolved === "light" ? emblemDark : emblemLight);
  const fmt = $derived(ui.team ? teamConfig(ui.team) : null);

  refreshInfo();

  // Remove running matches that were abandoned (an hour past their expected end, untouched for an hour).
  $effect(() => {
    const team = ui.team;
    if (!team) return;
    const sweep = () =>
      sweepStalledMatches(db, team)
        .then((gone) => gone.length && toast(`Removed ${gone.length} abandoned match${gone.length > 1 ? "es" : ""}`))
        .catch(() => {});
    sweep();
    const i = setInterval(sweep, 5 * 60_000);
    return () => clearInterval(i);
  });
</script>

<div class="mshell">
  <header class="mtop">
    {#if more.some(([k]) => k === ui.tab)}
      <button class="ghost back" onclick={() => (ui.tab = "more")} aria-label="Back"><Icon name="chevronLeft" size={22} /></button>
    {:else if ui.team}
      <TeamBadge team={ui.team} size={34} />
    {:else}
      <img src={emblem} alt="" style="height:30px" />
    {/if}
    <div class="ttl">
      <b>{ui.tab === "more" ? t("companion.more") : t("companion." + ui.tab)}</b>
      <span>{ui.team?.name ?? ui.info?.team_name ?? "Coach's Playbook"}{fmt && ui.team ? " · " + (fmt.formatLabel || SPORTS[ui.team.sport]?.name || "") : ""}</span>
    </div>
    {#if ui.info?.outbox}
      <button class="badge warn" onclick={() => (ui.tab = "sync")}><Icon name="sync" size={12} /> {ui.info.outbox}</button>
    {/if}
  </header>
  <main class="mbody">
    {#if ui.info}
      {#if ui.tab === "more"}
        <div class="list card flush">
          {#each more as [k, sub]}
            <button class="item rowbtn row morerow" onclick={() => (ui.tab = k)} disabled={!ui.info?.paired && k !== "sync"}>
              <span class="mi"><Icon name={screens[k][0]} size={20} /></span>
              <span class="grow"><b>{t("companion." + k)}</b><span class="muted small">{sub}</span></span>
              <Icon name="chevronRight" size={18} />
            </button>
          {/each}
        </div>

        <h3 class="sec">{t("companion.appearance")}</h3>
        <div class="seg" role="radiogroup" aria-label="Theme">
          {#each THEME_OPTIONS as [id, label]}
            <button role="radio" aria-checked={theme.pref === id} class:on={theme.pref === id} onclick={() => setTheme(id)}>
              <Icon name={id === "system" ? "monitor" : id === "light" ? "sun" : "moon"} size={16} /> {id === "system" ? "System" : label}
            </button>
          {/each}
        </div>

        <div class="about">
          <img src={emblem} alt="" />
          <div><b>Coach's Playbook</b><span>{ui.info.paired ? `Paired with ${ui.info.pc_name}` : "Not paired yet"}</span><span>Free and open source</span></div>
        </div>
      {:else if Screen}
        {#key ui.info.team_id}<Screen />{/key}
      {/if}
    {/if}
  </main>
  <nav class="mnav">
    {#each primary as k}
      <button class:on={ui.tab === k} onclick={() => (ui.tab = k)} disabled={!ui.info?.paired} aria-current={ui.tab === k ? "page" : undefined}>
        <Icon name={screens[k][0]} size={22} /><span>{t("companion." + k)}</span>
      </button>
    {/each}
    <button class:on={inMore} onclick={() => (ui.tab = "more")} aria-current={inMore ? "page" : undefined}>
      <Icon name="more" size={22} /><span>{t("companion.more")}</span>
    </button>
  </nav>
</div>

{#if ui.toast}<div class="toast" class:error={ui.toast.kind === "error"}>{ui.toast.msg}</div>{/if}

<DialogHost />
