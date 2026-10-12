<script lang="ts">
  // Uygulama başına ses (Windows ses karıştırıcısı): Discord'u kıs, oyunu aç. Açıkken birkaç saniyede bir yenilenir.
  import { onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import type { AppVolume } from "../types";

  let { compact = false, max = 8 }: { compact?: boolean; max?: number } = $props();

  let apps = $state<AppVolume[]>([]);
  let loaded = $state(false);
  /** Kaydırılırken yenileme gelen değeri ezmesin. */
  let dragging = $state<number | null>(null);

  async function load() {
    try {
      const a = await api.audio.apps();
      apps = dragging == null ? a : a.map((x) => (x.pid === dragging ? (apps.find((y) => y.pid === x.pid) ?? x) : x));
    } catch {
      apps = [];
    }
    loaded = true;
  }

  onMount(() => {
    void load();
    const t = setInterval(() => void load(), 3000);
    return () => clearInterval(t);
  });

  function setLevel(a: AppVolume, v: number) {
    a.level = v;
    void api.audio.setAppVolume(a.pid, v).catch(() => void load());
  }

  function toggle(a: AppVolume) {
    a.muted = !a.muted;
    void api.audio.setAppMute(a.pid, a.muted).catch(() => void load());
  }

  const shown = $derived(apps.slice(0, max));
</script>

<div class="mix" class:compact>
  {#each shown as a (a.pid)}
    <div class="row" class:quiet={!a.active}>
      <button class="ib" class:bad={a.muted} onclick={() => toggle(a)} title={a.muted ? "Sesini aç" : "Sessize al"}>
        <Icon name={a.muted ? "mute" : "speaker"} size={15} />
      </button>
      <span class="nm" title={a.exe || a.name}>{a.name}</span>
      <input
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={a.level}
        aria-label="{a.name} ses seviyesi"
        onpointerdown={() => (dragging = a.pid)}
        onpointerup={() => (dragging = null)}
        oninput={(e) => setLevel(a, Number(e.currentTarget.value))}
      />
      <span class="tnum pc">{Math.round(a.level * 100)}</span>
    </div>
  {:else}
    {#if loaded}<p class="sub small">Şu an ses çıkaran bir uygulama yok.</p>{/if}
  {/each}
</div>

<style>
  .mix { display: flex; flex-direction: column; gap: 6px; }
  .row { display: grid; grid-template-columns: auto minmax(80px, 140px) minmax(0, 1fr) 30px; align-items: center; gap: 8px; }
  .compact .row { grid-template-columns: auto minmax(70px, 110px) minmax(0, 1fr) 28px; }
  .row.quiet .nm { color: var(--muted); }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  .pc { text-align: right; font-size: 12px; color: var(--muted); }
  .ib { border: 0; background: rgba(255, 255, 255, 0.06); color: var(--text); border-radius: 8px; padding: 5px; display: grid; }
  .ib:hover { background: rgba(255, 255, 255, 0.12); }
  .ib.bad { color: var(--bad); }
  input[type="range"] { width: 100%; accent-color: var(--gold); }
</style>
