<script lang="ts">
  // Üst çubukta sesle yazmayı tek tıkla açıp kapatan düğme (kısayolu: Ctrl+Alt+Y, Kısayollar'dan değişir).
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";

  let { onnavigate }: { onnavigate: (route: string) => void } = $props();

  let supported = $state(false);
  let enabled = $state(false);
  let ready = $state(true);
  let live = $state(false);

  async function load() {
    try {
      const s = await api.dictation.status();
      supported = s.supported;
      enabled = s.enabled;
      ready = s.whisperReady;
      live = s.phase !== "idle";
    } catch {
      /* sesle yazma yok */
    }
  }

  onMount(() => {
    void load();
    const stops = [
      on("dictation-toggled", () => void load()),
      on<{ phase: string }>("dictation-state", (p) => (live = p.phase !== "idle")),
    ];
    return () => stops.forEach((s) => void s.then((f) => f()));
  });

  async function click() {
    if (!ready) return onnavigate("settings/dikte");
    try {
      enabled = await api.dictation.toggle();
      notify(enabled ? "Sesle yazma açık: Ctrl + Alt basılı tut, konuş" : "Sesle yazma kapalı", enabled ? "mic" : "info");
    } catch (e) {
      notify(String(e), "info");
    }
  }
</script>

{#if supported}
  <button
    class="dt"
    class:on={enabled}
    class:live
    onclick={click}
    title={ready ? `Sesle yazma ${enabled ? "açık" : "kapalı"} · tıkla ya da Ctrl+Alt+Y` : "Sesle yazma hazır değil: kurmak için tıkla"}
    aria-pressed={enabled}
  >
    <Icon name={enabled ? "mic" : "micoff"} size={15} />
    <span>Sesle yazma</span>
    <i class="sw"></i>
  </button>
{/if}

<style>
  .dt {
    display: inline-flex; align-items: center; gap: 7px; flex: none;
    border: 1px solid var(--line-2); background: rgba(255, 255, 255, 0.03); color: var(--muted);
    border-radius: 999px; padding: 6px 8px 6px 11px; font-size: 13px; font-weight: 600;
  }
  .dt:hover { border-color: var(--gold-dim); color: var(--text); }
  .dt.on { color: var(--gold-glow); border-color: rgba(227, 194, 112, 0.45); background: rgba(227, 194, 112, 0.08); }
  .dt.live { box-shadow: 0 0 0 3px rgba(227, 194, 112, 0.18); }
  .sw { position: relative; width: 26px; height: 15px; border-radius: 999px; background: var(--coal); border: 1px solid var(--line-2); }
  .sw::after { content: ""; position: absolute; top: 2px; left: 2px; width: 9px; height: 9px; border-radius: 50%; background: var(--dim); transition: transform 0.15s; }
  .on .sw { background: rgba(227, 194, 112, 0.25); border-color: var(--gold); }
  .on .sw::after { transform: translateX(11px); background: var(--gold); }
  @media (max-width: 900px) { .dt span { display: none; } }
</style>
