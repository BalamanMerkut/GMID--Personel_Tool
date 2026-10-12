<script lang="ts">
  import { onMount } from "svelte";
  import gmidLogo from "../../assets/gmid-logo.webp";
  import merkutLogo from "../../assets/merkut-studios.jpg";
  import { api, inTauri, type AppInfo } from "../api";
  import Hotkeys from "../components/Hotkeys.svelte";
  import Switch from "../components/Switch.svelte";
  import ClipsSettings from "../settings/ClipsSettings.svelte";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../icons";
  import Appearance from "../settings/Appearance.svelte";
  import Data from "../settings/Data.svelte";
  import Dictation from "../settings/Dictation.svelte";
  import Downloader from "../settings/Downloader.svelte";
  import Update from "../settings/Update.svelte";
  import ServersList from "../settings/ServersList.svelte";
  import { getJson, save, setJson, settings } from "../settings.svelte";
  import { notify } from "../toast.svelte";

  let {
    info,
    refreshInfo,
    onreplay,
    onnavigate,
    tab = $bindable("genel"),
  }: {
    info: AppInfo | null;
    refreshInfo: () => Promise<void>;
    onreplay: () => void;
    onnavigate: (id: string) => void;
    tab?: string;
  } = $props();

  interface Tab {
    id: string;
    label: string;
    icon: IconName;
    /** Aramada eşleşecek sözcükler. */
    keys: string;
  }
  const TABS: Tab[] = [
    { id: "genel", label: "Genel", icon: "power", keys: "başlangıç windows açılış tepsi pencere kapat çıkış" },
    { id: "gorunum", label: "Görünüm", icon: "palette", keys: "arka plan resim gif video renk bulanık karartma kalite animasyon açılış tema" },
    { id: "kisayollar", label: "Kısayollar", icon: "keyboard", keys: "kısayol tuş atama klavye ctrl alt hotkey mikrofon ses not pano indir" },
    { id: "dikte", label: "Sesle yazma", icon: "mic", keys: "sesle yazma dikte konuş konuşarak yaz mikrofon whisper wispr ctrl alt nvidia yapay zeka toparla" },
    { id: "ses", label: "Ses & Mikrofon", icon: "speaker", keys: "mikrofon kilit seviye gösterge osd kulaklık hoparlör" },
    { id: "bildirimler", label: "Bildirimler", icon: "bell", keys: "bildirim ses uyarı hatırlatıcı windows sayaç indirme bitti" },
    { id: "indirici", label: "Video İndirici", icon: "download", keys: "video indirme yt-dlp ffmpeg deno araç güncelle eklenti chrome opera aynı anda klasör" },
    { id: "oyunlar", label: "Oyunlar", icon: "library", keys: "oyun kütüphane algılama oturum süre steam epic oynama takip klip kayıt nvidia xbox obs medal ekran görüntüsü discord" },
    { id: "sunucular", label: "Sunucular", icon: "server", keys: "sunucu topluluk liste adres github" },
    { id: "veri", label: "Veri ve yedekler", icon: "shield", keys: "veri klasör yedek yedekle günlük log sorun geri yükle format zip" },
    { id: "hakkinda", label: "Hakkında", icon: "info", keys: "sürüm hakkında merkut gmid güncelle güncelleme yeni sürüm" },
  ];

  let search = $state("");
  const shown = $derived.by(() => {
    const q = search.trim().toLocaleLowerCase("tr-TR");
    if (!q) return TABS;
    return TABS.filter((t) => `${t.label} ${t.keys}`.toLocaleLowerCase("tr-TR").includes(q));
  });
  const current = $derived(TABS.find((t) => t.id === tab) ?? TABS[0]);

  function pick(id: string) {
    tab = id;
    try {
      history.replaceState(null, "", `#settings/${id}`);
    } catch {
      /* önemsiz */
    }
  }

  // Aramada tek bölüm kalırsa ona geç.
  $effect(() => {
    if (search.trim() && shown.length && !shown.some((t) => t.id === tab)) pick(shown[0].id);
  });

  // ---- Genel ----
  let autostart = $state(false);
  let autostartReady = $state(false);
  onMount(async () => {
    try {
      autostart = await api.autostart.isEnabled();
      autostartReady = inTauri;
    } catch {
      autostartReady = false;
    }
  });

  async function toggleAutostart(on: boolean) {
    try {
      await api.autostart.set(on);
      autostart = on;
      notify(on ? "Windows açılınca tepside başlayacak" : "Windows açılışında başlamayacak", "power");
    } catch (e) {
      notify(`Ayarlanamadı: ${e}`, "info");
    }
  }

  // ---- Ses & Mikrofon / Bildirimler ----
  let lockLevel = $state(getJson<number>("audio.micLockLevel", 1));
  let osd = $state(getJson("ui.osd", true));
  let reminderSound = $state(getJson("reminders.sound", true));
  let reminderToast = $state(getJson("reminders.toast", true));
  let videosNotify = $state(getJson("videos.notify", true));
  let gamesTrack = $state(getJson("games.track", true));
  let gamesAsk = $state(getJson("games.ask", true));

  let lockTimer: ReturnType<typeof setTimeout> | undefined;
  function setLockLevel(v: number) {
    lockLevel = v;
    clearTimeout(lockTimer);
    lockTimer = setTimeout(async () => {
      await setJson("audio.micLockLevel", v);
      // Kilit açıksa yeni seviyeyi hemen uygula.
      const s = await api.audio.state().catch(() => null);
      if (s?.lock != null) await api.audio.setLock(v).catch(() => {});
    }, 150);
  }

  function toggle(key: string, v: boolean, set: (v: boolean) => void) {
    set(v);
    void setJson(key, v);
  }

  async function testAlert() {
    try {
      await api.reminders.save({ id: null, title: "Deneme uyarısı", note: "Bunu görüyorsan uyarılar çalışıyor 🎉", dueAt: Date.now(), repeat: "none" });
      notify("Birkaç saniye içinde sağ altta kart ve Windows bildirimi çıkmalı", "bell");
    } catch (e) {
      notify(String(e), "info");
    }
  }
</script>

<div class="cc">
  <nav class="tabs" aria-label="Ayar bölümleri">
    <label class="search">
      <Icon name="search" size={16} />
      <input type="search" placeholder="Ayarlarda ara…" bind:value={search} aria-label="Ayarlarda ara" />
    </label>
    {#each shown as t (t.id)}
      <button aria-current={tab === t.id ? "page" : undefined} onclick={() => pick(t.id)}>
        <Icon name={t.icon} size={18} /><span>{t.label}</span>
      </button>
    {:else}
      <p class="sub small none">"{search}" ile eşleşen ayar yok.</p>
    {/each}
  </nav>

  <div class="pane">
    <header class="phead">
      <h2><Icon name={current.icon} size={22} />{current.label}</h2>
    </header>

    {#if tab === "genel"}
      <section class="panel">
        <h3 class="h2"><Icon name="power" />Başlangıç ve pencere</h3>
        <div class="line">
          <div>
            <b>Windows açılınca başlat</b>
            <span class="sub small">Sessizce tepsiye yerleşir, pencere açılmaz. Hatırlatıcıların her zaman çalması için önerilir.</span>
          </div>
          <Switch label="Windows açılınca başlat" checked={autostart} disabled={!autostartReady} onchange={toggleAutostart} />
        </div>
        <div class="line">
          <div>
            <b>Kapatınca tepside çalışmaya devam et</b>
            <span class="sub small">Kapalıysa pencereyi kapatmak uygulamadan tamamen çıkar; hatırlatıcılar ve indirmeler durur.</span>
          </div>
          <Switch label="Kapatınca tepside çalış" checked={settings.closeToTray} onchange={(v) => save("closeToTray", v)} />
        </div>
        <div class="line">
          <div>
            <b>Pencereyi her yerden aç</b>
            <span class="sub small">Kısayolla (varsayılan <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>G</kbd>) ya da tepsideki simgeye tıklayarak.</span>
          </div>
          <button class="btn ghost" onclick={() => pick("kisayollar")}><Icon name="keyboard" />Kısayollar</button>
        </div>
      </section>
    {:else if tab === "gorunum"}
      <Appearance {onreplay} />
    {:else if tab === "kisayollar"}
      <section class="panel">
        <p class="sub">
          Bu kısayollar oyunun içinden de, GMID Tool kapalı (tepsideyken) de çalışır. Değiştirmek için kaleme bas, sonra
          yeni tuş birleşimine bas. En az bir <kbd>Ctrl</kbd>, <kbd>Alt</kbd> ya da <kbd>Shift</kbd> gerekir (F tuşları hariç).
        </p>
        <Hotkeys />
        <p class="sub small">
          "Başka bir program bu kısayolu kullanıyor" yazarsa (Discord, NVIDIA, oyun…) farklı bir birleşim seç.
        </p>
      </section>
    {:else if tab === "ses"}
      <section class="panel">
        <h3 class="h2"><Icon name="lock" />Mikrofon seviye kilidi</h3>
        <label class="field">
          <span class="label">Kilit seviyesi <b class="tnum">%{Math.round(lockLevel * 100)}</b></span>
          <input type="range" min="10" max="100" value={Math.round(lockLevel * 100)} oninput={(e) => setLockLevel(+e.currentTarget.value / 100)} />
        </label>
        <span class="sub small">Kilit açıkken bir program mikrofonu kısarsa bu seviyeye geri çekilir. Kilidi açıp kapatmak için Ses & Mikrofon ekranı ya da kısayol.</span>
      </section>
      <section class="panel">
        <h3 class="h2"><Icon name="monitor" />Ekran göstergesi</h3>
        <div class="line">
          <div>
            <b>Kısayollara sağ üstte küçük bilgi ile cevap ver</b>
            <span class="sub small">"Mikrofon kapalı", "Ses çıkışı: Kulaklık" gibi. Odağı çalmaz, tıklamaları geçirir.</span>
          </div>
          <Switch label="Ekran göstergesi" checked={osd} onchange={(v) => toggle("ui.osd", v, (x) => (osd = x))} />
        </div>
      </section>
      <section class="panel">
        <div class="line">
          <div>
            <b>Cihazlar, takma adlar ve gizleme</b>
            <span class="sub small">Kulaklık/hoparlör adlarını ve simgelerini Ses & Mikrofon ekranından düzenleyebilirsin.</span>
          </div>
          <button class="btn ghost" onclick={() => onnavigate("audio")}><Icon name="speaker" />Ses & Mikrofon</button>
        </div>
      </section>
    {:else if tab === "bildirimler"}
      <section class="panel">
        <h3 class="h2"><Icon name="bell" />Hatırlatıcılar ve sayaçlar</h3>
        <div class="line">
          <div>
            <b>Windows bildirimi gönder</b>
            <span class="sub small">Sağ alttaki karta ek olarak; bildirim merkezinde de kalır.</span>
          </div>
          <Switch label="Windows bildirimi" checked={reminderToast} onchange={(v) => toggle("reminders.toast", v, (x) => (reminderToast = x))} />
        </div>
        <div class="line">
          <div>
            <b>Uyarıda ses çal</b>
            <span class="sub small">Windows'un bildirim sesi.</span>
          </div>
          <Switch label="Uyarıda ses" checked={reminderSound} onchange={(v) => toggle("reminders.sound", v, (x) => (reminderSound = x))} />
        </div>
        <div class="line">
          <div>
            <b>Uyarıları dene</b>
            <span class="sub small">Şu an için bir deneme hatırlatıcısı kurar; birkaç saniyede çalar.</span>
          </div>
          <button class="btn ghost" onclick={testAlert}><Icon name="bell" />Uyarıyı dene</button>
        </div>
      </section>
      <section class="panel">
        <h3 class="h2"><Icon name="download" />Video İndirici</h3>
        <div class="line">
          <div>
            <b>İndirme bitince bildir</b>
            <span class="sub small">Windows bildirimiyle videonun adı.</span>
          </div>
          <Switch label="İndirme bildirimi" checked={videosNotify} onchange={(v) => toggle("videos.notify", v, (x) => (videosNotify = x))} />
        </div>
      </section>
      <p class="sub small">
        Bildirimler hiç gelmiyorsa: Windows Ayarları → Sistem → Bildirimler'de GMID Tool'un açık olduğuna ve
        "Rahatsız etmeyin"in kapalı olduğuna bak.
      </p>
    {:else if tab === "dikte"}
      <Dictation />
    {:else if tab === "indirici"}
      <Downloader {onnavigate} />
    {:else if tab === "oyunlar"}
      <section class="panel">
        <h3 class="h2"><Icon name="library" />Oyun algılama</h3>
        <div class="line">
          <div>
            <b>Oynadığım oyunları algıla</b>
            <span class="sub small">
              Kütüphanendeki bir oyun açılınca oturum başlar, kapanınca biter (5 saniyede bir çalışan programlara bakılır;
              oyuna hiçbir şey enjekte edilmez). Kapatırsan Oturum Geçmişi'ne yeni kayıt düşmez.
            </span>
          </div>
          <Switch label="Oyun algılama" checked={gamesTrack} onchange={(v) => toggle("games.track", v, (x) => (gamesTrack = x))} />
        </div>
        <div class="line">
          <div>
            <b>Yeni oyun açılınca sor</b>
            <span class="sub small">
              Kütüphanende olmayan bir oyun açılınca (bilinen oyunlar listesinde varsa ya da bir dakikadan uzun tam ekran
              çalışırsa) sağ altta "Kütüphaneye ekleyeyim mi?" diye sorar.
            </span>
          </div>
          <Switch label="Yeni oyun açılınca sor" checked={gamesAsk} onchange={(v) => toggle("games.ask", v, (x) => (gamesAsk = x))} />
        </div>
      </section>
      <ClipsSettings />
    {:else if tab === "sunucular"}
      <ServersList />
    {:else if tab === "veri"}
      <Data {info} {refreshInfo} />
    {:else if tab === "hakkinda"}
      <section class="panel about">
        <img class="gmid" src={gmidLogo} alt="GMID" />
        <div class="abouttext">
          <b>GMID Tool <span class="tnum">{info?.version ?? ""}</span></b>
          <span class="sub">Game Mansion In Discord'un alet çantası. Oyunların, film ve dizilerin, notların, kliplerin ve indirdiğin videolar tek bir pencerede.</span>
          <span class="sub small">Merkut Studios</span>
        </div>
        <img class="merkut" src={merkutLogo} alt="Merkut Studios" />
      </section>
      <Update />
    {/if}
  </div>
</div>

<style>
  .cc { display: grid; grid-template-columns: 230px minmax(0, 1fr); gap: 20px; max-width: 1300px; margin: 0 auto; align-items: start; }
  .tabs { position: sticky; top: 0; display: flex; flex-direction: column; gap: 2px; }
  .search { display: flex; align-items: center; gap: 8px; padding: 7px 10px; margin-bottom: 8px; border-radius: 10px; background: var(--coal-2); border: 1px solid var(--line); color: var(--dim); }
  .search:focus-within { border-color: var(--gold-dim); }
  .search input { flex: 1; min-width: 0; border: 0; background: transparent; outline: none; }
  .tabs button { display: flex; align-items: center; gap: 10px; padding: 9px 12px; border: 0; border-radius: 10px; background: transparent; color: var(--muted); text-align: left; font-weight: 500; }
  .tabs button:hover { background: rgba(255, 255, 255, 0.04); color: var(--text); }
  .tabs button[aria-current="page"] { background: linear-gradient(90deg, rgba(227, 194, 112, 0.16), rgba(227, 194, 112, 0.03)); color: var(--gold-glow); }
  .none { padding: 8px 12px; margin: 0; }
  .pane { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  .phead h2 { display: flex; align-items: center; gap: 10px; margin: 0; font-family: var(--f-display); font-size: 20px; color: var(--gold); }
  .panel { display: flex; flex-direction: column; gap: 14px; }
  .panel p { margin: 0; }
  .field { display: flex; flex-direction: column; gap: 8px; }
  .label { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .label b { color: var(--gold); margin-left: 6px; }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .line > div:first-child { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1 1 260px; }
  kbd { font-family: var(--f-body); font-size: 11px; font-weight: 600; padding: 1px 6px; border-radius: 5px; background: var(--coal); border: 1px solid var(--line-2); color: var(--text); }
  .about { flex-direction: row; align-items: center; gap: 18px; }
  .gmid { width: 64px; height: 64px; border-radius: 50%; box-shadow: 0 0 20px rgba(227, 194, 112, 0.2); flex: none; }
  .abouttext { display: flex; flex-direction: column; gap: 4px; flex: 1; min-width: 0; }
  .abouttext b { font-family: var(--f-display); font-size: 17px; }
  .merkut { width: 64px; height: auto; border-radius: 8px; flex: none; }
  @media (max-width: 900px) {
    .cc { grid-template-columns: minmax(0, 1fr); }
    .tabs { position: static; flex-direction: row; flex-wrap: wrap; gap: 4px; }
    .search { flex-basis: 100%; }
  }
</style>
