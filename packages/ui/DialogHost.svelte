<script>
  import { dialogs, closeDialog } from "./dialog.svelte.js";

  dialogs.hosted = true;
  let okBtn = $state(null);
  let field = $state(null);
  $effect(() => {
    if (dialogs.current) queueMicrotask(() => (field ? (field.focus(), field.select()) : okBtn?.focus()));
  });
  const done = (yes) => closeDialog(dialogs.current?.input ? (yes ? dialogs.current.input.value : null) : yes);
</script>

<svelte:window onkeydown={(e) => { if (!dialogs.current) return; if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); done(false); } }} />

{#if dialogs.current}
  {@const d = dialogs.current}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="modal-bg dh" onclick={(e) => e.target === e.currentTarget && done(false)}>
    <div class="modal dlg" role="alertdialog" aria-modal="true" aria-labelledby="dlg-t" aria-describedby="dlg-b">
      <h3 id="dlg-t">{d.title}</h3>
      {#if d.body}<p id="dlg-b" class="muted">{d.body}</p>{/if}
      {#if d.input}
        <input bind:this={field} bind:value={d.input.value} placeholder={d.input.placeholder} maxlength={d.input.maxlength} onkeydown={(e) => e.key === "Enter" && done(true)} />
      {/if}
      <div class="row btns">
        {#if d.cancel}<button onclick={() => done(false)}>{d.cancel}</button>{/if}
        <button bind:this={okBtn} class={d.danger ? "danger strong" : "primary"} onclick={() => done(true)}>{d.ok}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dh { z-index: 200; padding: 16px; }
  .dlg { width: min(440px, 100%); display: flex; flex-direction: column; gap: 10px; box-shadow: 0 10px 40px #0005; }
  .dlg h3 { font-size: 16px; margin: 0; line-height: 1.35; }
  .dlg p { margin: 0; white-space: pre-line; font-size: 13px; }
  .btns { justify-content: flex-end; margin-top: 6px; }
  .btns button { min-width: 90px; }
  :global(button.danger.strong) { background: var(--bad); border-color: var(--bad); color: #fff; font-weight: 600; }
</style>
