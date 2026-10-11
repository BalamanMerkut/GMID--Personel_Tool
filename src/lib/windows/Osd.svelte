<script lang="ts">
  // Sağ üstteki odak çalmayan gösterge. Pencereyi Rust gösterir/gizler; burası sadece çizer.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../icons";
  import type { OsdMessage } from "../types";

  let msg: OsdMessage | null = $state(null);

  onMount(() => {
    void api.overlay.takeOsd().then((m) => m && (msg = m));
    const stop = on<OsdMessage>("osd", (m) => (msg = m));
    return () => void stop.then((f) => f());
  });
</script>

{#if msg}
  <!-- Aynı türden art arda gelen mesajlar (dikte süresi, ses seviyesi) yeniden kaymasın. -->
  {#key msg.icon}
    <div class="osd" class:warn={msg.icon === "info"}>
      <span class="ic"><Icon name={msg.icon as IconName} size={22} /></span>
      <span class="txt">{msg.text}</span>
    </div>
  {/key}
{/if}

<style>
  :global(html), :global(body) { background: transparent !important; }
  .osd {
    position: fixed; top: 6px; left: 6px; right: 6px; height: 64px; display: flex; align-items: center; gap: 12px; padding: 0 16px;
    border-radius: 14px; background: rgba(12, 14, 17, 0.92); border: 1px solid rgba(96, 216, 216, 0.5);
    box-shadow: 0 0 18px rgba(96, 216, 216, 0.14); color: var(--text);
    animation: in 0.18s ease-out;
  }
  .osd.warn { border-color: rgba(227, 194, 112, 0.6); }
  .ic { color: var(--neon); display: grid; place-items: center; }
  .warn .ic { color: var(--gold); }
  .txt { font-weight: 600; font-size: 14.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  @keyframes in { from { opacity: 0; transform: translateY(-6px); } }
</style>
