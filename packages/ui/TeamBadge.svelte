<script>
  // Team logo, or coloured initials when the team has no logo.
  import { initials, nameColor } from "./image.js";
  let { team, size = 32 } = $props();
</script>

{#if team?.logo}
  <img class="tb" src={team.logo} alt="" style="width:{size}px;height:{size}px" />
{:else}
  <span class="tb ini" style="width:{size}px;height:{size}px;font-size:{Math.round(size * 0.4)}px;background:{nameColor(team?.name)}">{initials(team?.name)}</span>
{/if}

<style>
  .tb { flex: none; border-radius: 22%; object-fit: contain; }
  .ini { display: inline-grid; place-items: center; color: #fff; font-weight: 700; letter-spacing: 0.02em; }
</style>
