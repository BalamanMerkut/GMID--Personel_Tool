<script lang="ts">
  import { api, formatSize, type AppInfo, type BackupInfo } from "../api";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";

  let { info, refreshInfo }: { info: AppInfo | null; refreshInfo: () => Promise<void> } = $props();

  let busy = $state(false);
  let backups: BackupInfo[] = $state([]);
  $effect(() => {
    if (info) backups = info.backups;
  });

  async function backupNow() {
    busy = true;
    try {
      backups = await api.backupNow();
      notify("Yedek alındı", "shield");
      void refreshInfo();
    } catch (e) {
      notify(String(e), "info");
    } finally {
      busy = false;
    }
  }

  async function openLog() {
    try {
      await api.openLog();
    } catch (e) {
      notify(String(e), "info");
    }
  }
</script>

  <section class="panel">
    <h3 class="h2"><Icon name="shield" />Veri ve yedekler</h3>
    <div class="field">
      <span class="label">Veri klasörü</span>
      <div class="path">{info?.dataDir ?? "…"}</div>
      <div class="spread">
        <span class="sub small tnum">Veritabanı {info ? formatSize(info.dbSize) : "…"}</span>
        <button class="btn ghost" onclick={() => api.openDataFolder()}><Icon name="folder" />Klasörü aç</button>
      </div>
    </div>
    <div class="field">
      <span class="label">Yedekler <span class="sub small">· her gün otomatik, son 7 tanesi saklanır</span></span>
      {#if backups.length}
        <ul class="backups">
          {#each backups as b (b.name)}
            <li><span class="tnum">{b.created}</span><span class="sub small tnum">{formatSize(b.size)}</span></li>
          {/each}
        </ul>
      {:else}
        <p class="sub small">Henüz yedek yok.</p>
      {/if}
      <div class="row">
        <button class="btn" disabled={busy} onclick={backupNow}><Icon name="shield" />Şimdi yedekle</button>
        <button class="btn ghost" onclick={() => api.openDataFolder("Yedekler")}><Icon name="folder" />Yedekler klasörü</button>
      </div>
    </div>
    <div class="field">
      <span class="label">Sorun giderme</span>
      <span class="sub small">Bir şey ters giderse ne olduğu <code>gmid.log</code> dosyasına yazılır. Bana sorun bildirirken bu dosyanın içeriği çok işe yarar.</span>
      <div class="row">
        <button class="btn ghost" onclick={openLog}><Icon name="note" />Günlük dosyasını aç</button>
      </div>
    </div>
  </section>

<style>
  .panel { display: flex; flex-direction: column; gap: 16px; }
  .field { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .label { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .backups { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .backups li { display: flex; justify-content: space-between; padding: 6px 10px; border-radius: 8px; background: rgba(255, 255, 255, 0.03); }
  code { padding: 1px 6px; }
</style>
