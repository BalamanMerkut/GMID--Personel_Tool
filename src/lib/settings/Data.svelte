<script lang="ts">
  import { api, formatSize, type AppInfo, type BackupEvery, type BackupInfo } from "../api";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";
  import type { BackupManifest } from "../types";

  let { info, refreshInfo }: { info: AppInfo | null; refreshInfo: () => Promise<void> } = $props();

  let busy = $state(false);
  let backups: BackupInfo[] = $state([]);
  $effect(() => {
    if (info) backups = info.backups;
  });

  // Kaç yedek tutulsun, ne sıklıkla alınsın. Sınır dolunca yeni yedek en eskisinin yerini alır.
  let keep = $state(3);
  let every = $state<BackupEvery>("day");
  const KEEPS = [1, 3, 5, 7];
  const EVERY: [BackupEvery, string][] = [["day", "Her gün"], ["week", "Haftada bir"], ["off", "Kapalı"]];
  $effect(() => {
    void api.backupPolicy().then((r) => {
      keep = r.policy.keep;
      every = r.policy.every;
    }).catch(() => {});
  });
  async function setPolicy(k?: number, e?: BackupEvery) {
    try {
      const r = await api.backupPolicy(k, e);
      keep = r.policy.keep;
      every = r.policy.every;
      backups = r.backups;
    } catch (err) {
      notify(String(err), "info");
    }
  }
  const total = $derived(backups.reduce((n, b) => n + b.size, 0));

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

  // Tam yedekten (GMID-Yedek.zip) geri yükleme: önce doğrulanır, onaylanınca uygulama yeniden başlar.
  let staged = $state<BackupManifest | null>(null);
  let restoring = $state(false);

  async function pickRestore() {
    const path = await api.restore.pick();
    if (!path) return;
    restoring = true;
    try {
      staged = await api.restore.stage(path);
    } catch (e) {
      notify(String(e), "info");
    } finally {
      restoring = false;
    }
  }

  async function cancelRestore() {
    staged = null;
    await api.restore.cancel().catch(() => {});
  }

  async function applyRestore() {
    restoring = true;
    try {
      await api.restore.apply();
    } catch (e) {
      restoring = false;
      notify(String(e), "info");
    }
  }

  const fmtDate = (ms: number) =>
    new Date(ms).toLocaleString("tr-TR", { day: "numeric", month: "long", year: "numeric", hour: "2-digit", minute: "2-digit" });

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
      <span class="label">Otomatik yedek</span>
      <span class="sub small">
        Yalnız GMID'in kendi bilgileri yedeklenir: ayarlar, notlar, hatırlatıcılar, oyun kütüphanesi listesi, oturum
        geçmişi, sunucular, indirme kayıtları. <b>Videolar, şarkılar, klipler ve resimler yedeğe girmez</b>; onlar yerinde
        durur. Bu yüzden bir yedek birkaç yüz KB tutar.
      </span>
      <div class="pol">
        <div class="pr">
          <span>Ne sıklıkla</span>
          <div class="seg">
            {#each EVERY as [v, l] (v)}<button aria-pressed={every === v} onclick={() => setPolicy(undefined, v)}>{l}</button>{/each}
          </div>
        </div>
        <div class="pr">
          <span>En fazla kaç yedek <span class="sub small">· dolunca yenisi en eskisinin yerine geçer</span></span>
          <div class="seg">
            {#each KEEPS as k (k)}<button class="tnum" aria-pressed={keep === k} onclick={() => setPolicy(k)}>{k}</button>{/each}
          </div>
        </div>
      </div>
      <span class="label">Yedekler <span class="sub small tnum">· {backups.length} tane, toplam {formatSize(total)}</span></span>
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
      <span class="label">Tam yedek <span class="sub small">· formattan önce ya da başka bilgisayara geçerken</span></span>
      {#if staged}
        <div class="restore">
          <span>
            <b>{fmtDate(staged.created)}</b> tarihli yedek hazır{staged.version ? ` (GMID ${staged.version})` : ""}.
            Uygulama yeniden başlayacak; şu anki verilerin <code>Yedekler</code> klasörüne saklanır, sonra yedek yerine konur.
          </span>
          <div class="row">
            <button class="btn" disabled={restoring} onclick={applyRestore}><Icon name="restore" />Geri yükle ve yeniden başlat</button>
            <button class="btn ghost" disabled={restoring} onclick={cancelRestore}>Vazgeç</button>
          </div>
        </div>
      {:else}
        <span class="sub small">
          Ayarlar, notlar, kütüphane, geçmiş ve kapaklarla birlikte tek dosya: <b>Format Sonrası Kurulum → Format Kiti</b>
          ile oluşturulur (<code>GMID-Yedek.zip</code>). Buradan geri yükleyebilirsin.
        </span>
        <div class="row">
          <button class="btn ghost" disabled={restoring} onclick={pickRestore}><Icon name="restore" />{restoring ? "Yedek okunuyor…" : "Yedekten geri yükle"}</button>
        </div>
      {/if}
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
  .pol { display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); }
  .pr { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; font-size: 13.5px; }
  .backups { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .backups li { display: flex; justify-content: space-between; padding: 6px 10px; border-radius: 8px; background: rgba(255, 255, 255, 0.03); }
  code { padding: 1px 6px; }
  .restore { display: flex; flex-direction: column; gap: 10px; padding: 12px 14px; border-radius: 10px; border: 1px solid var(--gold-dim); background: rgba(227, 194, 112, 0.06); }
</style>
