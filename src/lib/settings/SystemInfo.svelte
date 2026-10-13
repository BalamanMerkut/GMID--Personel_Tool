<script lang="ts">
  // Sistem özellikleri: işlemci, ekran kartı, bellek, anakart, diskler, monitörler, fare/klavye, ses, ağ.
  // "Kopyala" ile Discord'a yapıştırılabilir özet.
  import { onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../icons";
  import { notify } from "../toast.svelte";
  import type { SysSection } from "../types";

  let sections: SysSection[] = $state([]);
  let busy = $state(false);
  let error: string | null = $state(null);

  async function load(refresh = false) {
    busy = true;
    error = null;
    try {
      sections = await api.system.info(refresh);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => void load());

  async function copy() {
    try {
      await navigator.clipboard.writeText(await api.system.text());
      notify("Sistem özellikleri kopyalandı · Discord'da Ctrl+V", "copy");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  const icon = (s: string) => s as IconName;
</script>

<section class="panel">
  <div class="line">
    <div>
      <span class="sub small">Windows'un kendi kayıtlarından okunur; hiçbir yere gönderilmez. “Kopyala” ile Discord'da paylaşabilirsin.</span>
    </div>
    <div class="acts">
      <button class="btn ghost sm" disabled={busy} onclick={() => load(true)}><Icon name="refresh" size={14} />{busy ? "Okunuyor…" : "Yenile"}</button>
      <button class="btn sm" disabled={busy || !sections.length} onclick={copy}><Icon name="copy" size={14} />Kopyala</button>
    </div>
  </div>
  {#if busy && !sections.length}
    <p class="sub">Aygıtlar okunuyor… (birkaç saniye)</p>
  {:else if error}
    <p class="err">{error}</p>
  {/if}
</section>

<div class="grid">
  {#each sections as s (s.title)}
    <section class="panel card">
      <h4><Icon name={icon(s.icon)} size={16} />{s.title}</h4>
      <dl>
        {#each s.rows as [k, v], i (i)}
          <dt>{k}</dt>
          <dd>{v}</dd>
        {/each}
      </dl>
    </section>
  {/each}
</div>

<style>
  .panel { display: flex; flex-direction: column; gap: 10px; }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .line > div:first-child { display: flex; flex-direction: column; gap: 4px; flex: 1 1 260px; }
  .acts { display: flex; gap: 8px; }
  .err { color: var(--bad); margin: 0; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(340px, 1fr)); gap: 12px; margin-top: 12px; }
  .card { gap: 8px; }
  h4 { display: flex; align-items: center; gap: 8px; margin: 0; font-size: 14px; color: var(--gold); }
  dl { margin: 0; display: grid; grid-template-columns: minmax(90px, max-content) 1fr; gap: 6px 14px; font-size: 13px; }
  dt { color: var(--dim); }
  dd { margin: 0; color: var(--text); overflow-wrap: anywhere; }
</style>
