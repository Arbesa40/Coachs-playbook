<script>
  import { db, invoke, byKey, playerLabel, sortPlayers } from "$shared/api.js";
  import TacticsBoard from "$shared/TacticsBoard.svelte";
  import { normalize } from "$shared/board.js";
  import emblemDark from "$shared/brand/emblem-dark.png";
  import { app } from "../lib/state.svelte.js";

  // "Print / Save as PDF" uses the Windows print dialog (Microsoft Print to PDF).
  let kind = $state("match");
  let matches = $state([]), players = $state([]), sessions = $state([]), drills = $state([]);
  let events = $state([]), minutes = $state({}), stats = $state(null);
  let assessments = $state([]), injuries = $state([]), attendance = $state([]), templates = $state([]);
  let seasonMinutes = $state({});
  let id = $state("");

  Promise.all(["match", "player", "session", "drill", "assessment", "injury", "attendance", "assessment_template"].map((k) => db.list(k))).then(
    (r) => ([matches, players, sessions, drills, assessments, injuries, attendance, templates] = r)
  );
  invoke("minutes_played", { matchId: null }).then((m) => (seasonMinutes = m));

  const pById = $derived(byKey(players, "id"));
  const dById = $derived(byKey(drills, "id"));
  const tById = $derived(byKey(templates, "id"));
  const options = $derived(
    kind === "match" ? [...matches].sort((a, b) => (b.date || "").localeCompare(a.date || "")).map((m) => [m.id, `${m.date} vs ${m.opponent}`])
    : kind === "player" ? sortPlayers(players).map((p) => [p.id, playerLabel(p)])
    : [...sessions].sort((a, b) => (b.date || "").localeCompare(a.date || "")).map((s) => [s.id, `${s.date} ${s.title}`])
  );
  const match = $derived(kind === "match" ? matches.find((m) => m.id === id) : null);
  const player = $derived(kind === "player" ? players.find((p) => p.id === id) : null);
  const session = $derived(kind === "session" ? sessions.find((s) => s.id === id) : null);

  $effect(() => {
    if (!match) return;
    db.by("match_event", "match_id", match.id).then((e) => (events = e.sort((a, b) => a.minute - b.minute)));
    invoke("minutes_played", { matchId: match.id }).then((m) => (minutes = m));
    invoke("tag_stats", { videoId: null, matchId: match.id }).then((s) => (stats = s));
  });
</script>

<div class="page">
  <div class="row no-print">
    <h1 class="grow">Reports</h1>
    <select bind:value={kind} onchange={() => (id = "")}>
      <option value="match">Match report</option><option value="player">Player report</option><option value="session">Session plan</option>
    </select>
    <select bind:value={id}><option value="">Choose…</option>{#each options as [v, l]}<option value={v}>{l}</option>{/each}</select>
    <button class="primary" onclick={() => window.print()} disabled={!id}>Print / Save as PDF</button>
  </div>

  <div class="print-area report">
    <div class="hdr"><b>{app.team.name}</b><span class="brandmark"><img src={emblemDark} alt="" />COACH'S <b>PLAYBOOK</b></span></div>

    {#if match}
      <h1>{match.home ? "" : "@ "}{match.opponent} — {match.goals_for ?? 0}:{match.goals_against ?? 0}</h1>
      <p>{match.date} · {match.competition ?? ""} · {match.venue ?? ""}</p>
      <h2>Line-up</h2>
      <p><b>Starting:</b> {(match.starters ?? []).map((p) => playerLabel(pById[p])).join(", ")}</p>
      <p><b>Bench:</b> {(match.bench ?? []).map((p) => playerLabel(pById[p])).join(", ")}</p>
      <h2>Events</h2>
      <table><tbody>{#each events as e}<tr><td>{e.minute}'</td><td>{e.kind.replace("_", " ")}</td><td>{playerLabel(pById[e.player_id])}</td></tr>{/each}</tbody></table>
      <h2>Minutes played</h2>
      <table><tbody>{#each Object.entries(minutes) as [p, m]}<tr><td>{playerLabel(pById[p])}</td><td>{Math.round(m)}</td></tr>{/each}</tbody></table>
      {#if stats?.total}
        <h2>Tagged events</h2>
        <table><tbody>{#each Object.entries(stats.by_code) as [c, n]}<tr><td>{c}</td><td>{n}</td></tr>{/each}</tbody></table>
      {/if}
      <h2>Coach's notes</h2><div class="lines"></div>
    {:else if player}
      <h1>{playerLabel(player)}</h1>
      <p>{player.position ?? ""} {player.dob ? "· born " + player.dob : ""}</p>
      <p>Minutes this season: <b>{Math.round(seasonMinutes[player.id] ?? 0)}</b> ·
        Sessions attended: <b>{attendance.filter((a) => a.player_id === player.id && a.present).length}</b> / {attendance.filter((a) => a.player_id === player.id).length}</p>
      <h2>Assessments</h2>
      <table><tbody>
        {#each assessments.filter((a) => a.player_id === player.id).sort((a, b) => b.date.localeCompare(a.date)).slice(0, 6) as a}
          <tr><td>{a.date}</td><td>{tById[a.template_id]?.name ?? ""}</td><td>{Object.entries(a.scores ?? {}).map(([k, v]) => `${k} ${v}`).join(" · ")}</td></tr>
        {/each}
      </tbody></table>
      <h2>Injuries & availability</h2>
      <table><tbody>{#each injuries.filter((i) => i.player_id === player.id) as i}<tr><td>{i.date}</td><td>{i.type}</td><td>{i.status}</td></tr>{/each}</tbody></table>
      {#if player.notes}<h2>Notes</h2><p style="white-space:pre-wrap">{player.notes}</p>{/if}
    {:else if session}
      <h1>{session.title}</h1>
      <p>{session.date} · {session.duration} min · {session.focus ?? ""}</p>
      {#each session.drills ?? [] as d, i}
        {@const dr = dById[d.drill_id]}
        <div class="drill">
          <h2>{i + 1}. {dr?.name ?? "Drill"} <small>({d.minutes} min)</small></h2>
          {#if dr && (dr.board?.items?.length || dr.diagram?.length)}<TacticsBoard board={normalize(dr.board ?? dr.diagram)} sport={app.team.sport} height={240} controls={false} />{/if}
          {#if dr?.description}<p>{dr.description}</p>{/if}
          {#if d.points || dr?.coaching_points}<p><b>Coaching points:</b> {d.points || dr.coaching_points}</p>{/if}
        </div>
      {/each}
      {#if Object.values(session.group_notes ?? {}).some((v) => v?.trim())}
        <h2>Instructions by group</h2>
        {#each Object.entries(session.group_notes ?? {}).filter(([, v]) => v?.trim()) as [g, v]}<p><b>{g}:</b> {v}</p>{/each}
      {/if}
      {#if session.notes}<h2>Notes</h2><p>{session.notes}</p>{/if}
    {:else}
      <p class="muted no-print">Choose a report type and an item above to preview it.</p>
    {/if}
  </div>
</div>

<style>
  .report { background: #fff; color: #111; padding: 28px 36px; margin-top: 16px; border-radius: 4px; max-width: 900px; }
  .report :global(td), .report :global(th) { border-bottom: 1px solid #ddd; color: #111; }
  .report h2 { margin-top: 18px; font-size: 15px; border-bottom: 2px solid #111; padding-bottom: 2px; }
  .brandmark { display: inline-flex; align-items: center; gap: 6px; letter-spacing: 1px; }
  .brandmark img { height: 22px; }
  .hdr { display: flex; justify-content: space-between; font-size: 12px; color: #555; margin-bottom: 12px; }
  .lines { height: 120px; background: repeating-linear-gradient(#fff 0 23px, #ccc 23px 24px); }
  .drill { break-inside: avoid; }
</style>
