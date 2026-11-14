<script>
  import { ICONS } from "./icons.js";
  // <Icon name="play" size={16} />  Colour follows the text colour.
  let { name, size = 16, color = null, title = null } = $props();
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  style={color ? `color:${color}` : ""}
  aria-hidden={title ? undefined : "true"}
  role={title ? "img" : undefined}
>
  {#if title}<title>{title}</title>{/if}
  {@html ICONS[name] ?? ICONS.x}
</svg>

<style>
  .icon { display: inline-block; vertical-align: -0.15em; flex: none; }
</style>
