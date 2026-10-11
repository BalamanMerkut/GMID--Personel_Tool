<script lang="ts">
  import { ICONS, type IconName } from "./icons";

  let { name, size = 20 }: { name: IconName; size?: number } = $props();
</script>

<svg
  class="i"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.7"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {@html ICONS[name]}
</svg>

<style>
  .i { flex: none; }
</style>
