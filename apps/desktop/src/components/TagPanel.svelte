<script>
  // The coder's buttons. Pressing one (or its hotkey) tags the current moment.
  let { panel, ontag, onedit = null } = $props();
</script>

<div class="col panel">
  {#if onedit}
    <div class="row">
      <b class="grow">{panel?.name ?? "No panel"}</b>
      <button class="ghost small" onclick={onedit}>Edit</button>
    </div>
  {/if}
  <div class="grid">
    {#each panel?.buttons ?? [] as b}
      <button class="tagbtn" style="--c:{b.color}" onpointerdown={(e) => { e.preventDefault(); ontag(b); }}>
        <span class="code">{b.code}</span>
        {#if b.hotkey}<span class="kbd">{b.hotkey.toUpperCase()}</span>{/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .panel { min-height: 0; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
  .tagbtn {
    display: flex; justify-content: space-between; align-items: center; gap: 6px;
    padding: 10px 8px; border-left: 4px solid var(--c); text-align: left; min-height: 44px;
  }
  .tagbtn:active { background: color-mix(in srgb, var(--c) 35%, var(--panel-2)); }
  .code { overflow: hidden; text-overflow: ellipsis; }
</style>
