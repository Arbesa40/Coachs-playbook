<script>
  import { db, today, ask } from "$shared/api.js";
  import Icon from "$shared/Icon.svelte";
  import { attempt } from "../lib/state.svelte.js";

  // Named analysis sessions group the videos (and their tags, clips and drawings) for one piece of work.
  let { sessions, videos, tags, matches, currentId, currentVideoId = null, onopen, onclose, onchange } = $props();

  let name = $state("");
  let matchId = $state("");
  let withVideo = $state(!!currentVideoId);
  let editing = $state(null);
  let editName = $state("");
  let q = $state("");

  const matchName = (id) => {
    const m = matches.find((x) => x.id === id);
    return m ? `${m.date} vs ${m.opponent}` : "";
  };
  const sorted = $derived(
    [...sessions]
      .filter((s) => !q.trim() || (s.name + " " + matchName(s.match_id)).toLowerCase().includes(q.trim().toLowerCase()))
      .sort((a, b) => (b.updated || "").localeCompare(a.updated || ""))
  );
  const tagCount = (s) => tags.filter((t) => (s.video_ids ?? []).includes(t.video_id)).length;

  async function create() {
    const n = name.trim() || (matchId ? `Analysis: ${matchName(matchId)}` : `Analysis ${today()}`);
    const rec = await attempt(() =>
      db.put("analysis_session", {
        name: n,
        match_id: matchId || null,
        video_ids: withVideo && currentVideoId ? [currentVideoId] : [],
        notes: "",
        created: new Date().toISOString(),
        updated: new Date().toISOString(),
      })
    );
    if (rec) {
      await onchange();
      onopen(rec.id);
    }
  }
  async function rename(s) {
    if (!editName.trim()) return;
    await attempt(() => db.put("analysis_session", { name: editName.trim() }, s.id));
    editing = null;
    onchange();
  }
  async function remove(s) {
    if (!await ask(`Delete the session "${s.name}"? Its videos, tags and clips are kept in the library.`)) return;
    await attempt(() => db.del("analysis_session", s.id));
    await onchange();
    if (s.id === currentId) onopen(null);
  }
</script>

<div class="modal-bg">
  <div class="modal col" style="width:min(760px,94vw)">
    <div class="row">
      <h2 class="grow"><Icon name="layers" /> Analysis sessions</h2>
      <button class="ghost" onclick={onclose} title="Close"><Icon name="x" /></button>
    </div>

    <div class="card col">
      <h3>New session</h3>
      <div class="row wrap">
        <input class="grow" placeholder="Session name, e.g. vs Rangers – pressing review" bind:value={name} onkeydown={(e) => e.key === "Enter" && create()} />
        <select bind:value={matchId} title="Link to a match (optional)">
          <option value="">No match</option>
          {#each matches as m}<option value={m.id}>{matchName(m.id)}</option>{/each}
        </select>
        <button class="primary" onclick={create}><Icon name="plus" /> Create</button>
      </div>
      {#if currentVideoId}
        <label class="small row muted"><input type="checkbox" bind:checked={withVideo} /> Start with the video that's open now</label>
      {/if}
    </div>

    <div class="row">
      <input class="grow" placeholder="Search sessions" bind:value={q} />
    </div>

    <div class="list card" style="padding:0;max-height:50vh;overflow:auto">
      <div class="item row" class:on={!currentId} role="button" tabindex="0" onclick={() => onopen(null)} onkeydown={(e) => e.key === "Enter" && onopen(null)}>
        <Icon name="film" />
        <div class="grow"><b>All videos</b><div class="small muted">Every video in this team's library</div></div>
        <span class="muted small">{videos.length} videos</span>
      </div>
      {#each sorted as s (s.id)}
        <div class="item row" class:on={s.id === currentId}>
          <Icon name="layers" />
          <div class="grow" style="min-width:0">
            {#if editing === s.id}
              <div class="row">
                <input class="grow" bind:value={editName} onkeydown={(e) => { if (e.key === "Enter") rename(s); if (e.key === "Escape") editing = null; }} />
                <button class="small" onclick={() => rename(s)} title="Save name"><Icon name="check" size={14} /></button>
              </div>
            {:else}
              <b>{s.name}</b>
              <div class="small muted">
                {matchName(s.match_id) || "No match"} · {(s.video_ids ?? []).length} videos · {tagCount(s)} tags · updated {(s.updated || "").slice(0, 10)}
              </div>
            {/if}
          </div>
          <button class="small primary" onclick={() => onopen(s.id)}>Open</button>
          <button class="ghost small" title="Rename" onclick={() => { editing = s.id; editName = s.name; }}><Icon name="pencil" size={14} /></button>
          <button class="ghost small danger" title="Delete session" onclick={() => remove(s)}><Icon name="trash" size={14} /></button>
        </div>
      {:else}
        <div class="empty">No sessions yet. Create one for each match or topic you analyse.</div>
      {/each}
    </div>
  </div>
</div>
