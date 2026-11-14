<script>
  import Icon from "$shared/Icon.svelte";
  import { db } from "$shared/api.js";
  import { attempt } from "../lib/state.svelte.js";

  // Design tagging buttons: code, colour, hotkey, seconds before/after, descriptors.
  let { panel, onclose } = $props();
  let name = $state(panel?.name ?? "New panel");
  let buttons = $state(
    (panel?.buttons ?? []).map((b) => ({ ...b, descriptorsText: (b.descriptors ?? []).join(", ") }))
  );

  function add() {
    buttons.push({ code: "New event", color: "#64748b", hotkey: "", pre: 5, post: 5, descriptorsText: "" });
  }
  function move(i, d) {
    const j = i + d;
    if (j < 0 || j >= buttons.length) return;
    [buttons[i], buttons[j]] = [buttons[j], buttons[i]];
  }
  async function save() {
    const clean = buttons
      .filter((b) => b.code.trim())
      .map(({ descriptorsText, ...b }) => ({
        ...b,
        code: b.code.trim(),
        hotkey: (b.hotkey || "").slice(0, 1).toLowerCase(),
        pre: Number(b.pre) || 0,
        post: Number(b.post) || 0,
        descriptors: descriptorsText.split(",").map((s) => s.trim()).filter(Boolean),
      }));
    const keys = clean.map((b) => b.hotkey).filter(Boolean);
    if (new Set(keys).size !== keys.length) return attempt(() => Promise.reject("Two buttons share the same hotkey."));
    await attempt(() => db.put("panel", { name, buttons: clean }, panel?.id ?? null), "Panel saved");
    onclose(true);
  }
</script>

<div class="modal-bg">
  <div class="modal col" style="width:min(900px,95vw)">
    <div class="row"><h2 class="grow">Tagging panel</h2><button class="ghost" onclick={() => onclose(false)}><Icon name="x" /></button></div>
    <label class="field">Panel name <input bind:value={name} /></label>
    <table>
      <thead><tr><th>Event</th><th>Colour</th><th>Key</th><th>Secs before</th><th>Secs after</th><th>Descriptors (comma separated)</th><th></th></tr></thead>
      <tbody>
        {#each buttons as b, i}
          <tr>
            <td><input bind:value={b.code} /></td>
            <td><input type="color" bind:value={b.color} style="width:44px;padding:0" /></td>
            <td><input bind:value={b.hotkey} maxlength="1" style="width:40px" /></td>
            <td><input type="number" min="0" max="120" bind:value={b.pre} style="width:64px" /></td>
            <td><input type="number" min="0" max="120" bind:value={b.post} style="width:64px" /></td>
            <td><input bind:value={b.descriptorsText} placeholder="Success, Fail" style="width:100%" /></td>
            <td class="row">
              <button class="ghost" onclick={() => move(i, -1)}><Icon name="arrowUp" size={14} /></button>
              <button class="ghost" onclick={() => move(i, 1)}><Icon name="arrowDown" size={14} /></button>
              <button class="ghost danger" onclick={() => buttons.splice(i, 1)}><Icon name="x" size={14} /></button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
    <div class="row"><button onclick={add}>+ Add button</button><span class="spacer"></span><button onclick={() => onclose(false)}>Cancel</button><button class="primary" onclick={save}>Save panel</button></div>
  </div>
</div>
