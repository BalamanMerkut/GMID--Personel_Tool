<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";

  let url = $state("");
  let defaultUrl = $state("");
  let draft = $state("");
  let error: string | null = $state(null);
  let busy = $state(false);

  onMount(async () => {
    const r = await api.servers.sync(false);
    url = draft = r.url;
    defaultUrl = r.defaultUrl;
    error = r.error;
  });

  async function saveUrl(value: string) {
    busy = true;
    try {
      await api.setSetting("servers.listUrl", value.trim());
      const r = await api.servers.sync(true);
      url = draft = r.url;
      error = r.error;
      if (!r.error) notify(value.trim() ? `Topluluk listesi güncellendi · ${r.servers.filter((s) => s.remote).length} sunucu` : "Topluluk listesi kapatıldı", "check");
    } finally {
      busy = false;
    }
  }
</script>

<section class="panel">
  <h3 class="h2"><Icon name="server" />Topluluk sunucu listesi</h3>
  <p class="sub small">
    Sunucular ekranındaki "GMID" sunucuları bu adresteki listeden gelir. Liste değişince herkesin uygulamasında birkaç
    dakika içinde güncellenir. Boş bırakıp kaydedersen topluluk listesi kapanır; sadece kendi eklediklerin kalır.
  </p>
  <input type="url" class="url" bind:value={draft} placeholder="https://…/sunucular.json" aria-label="Liste adresi" />
  {#if error}<p class="err small">Son deneme: {error}</p>{/if}
  <div class="row">
    <button class="btn" onclick={() => saveUrl(draft)} disabled={busy || draft === url}><Icon name="check" />Kaydet</button>
    <button class="btn ghost" onclick={() => saveUrl(defaultUrl)} disabled={busy || url === defaultUrl}>Varsayılana döndür</button>
    <button class="btn ghost" onclick={() => saveUrl(url)} disabled={busy}><Icon name="refresh" />Şimdi yenile</button>
  </div>
</section>

<style>
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .panel p { margin: 0; }
  .url { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 9px 12px; font-family: ui-monospace, Consolas, monospace; font-size: 12.5px; }
  .err { color: var(--bad); }
</style>
