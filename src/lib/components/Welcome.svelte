<script lang="ts">
  // İlk açılış rehberi: beş kısa adım. Her adım işe yarar bir şey yaptırır (Discord'a katıl, FPS iznini ver,
  // Windows ile başlat); hepsi atlanabilir, sonra Kontrol Odası'ndan da yapılır.
  import { onMount } from "svelte";
  import gmidLogo from "../../assets/gmid-logo.webp";
  import { api, inTauri } from "../api";
  import { community, joinDiscord } from "../community.svelte";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../icons";
  import type { ClipRoot, FpsStatus, HotkeyStatus } from "../types";
  import { finishWelcome } from "../welcome.svelte";
  import Switch from "./Switch.svelte";

  let { onnavigate }: { onnavigate: (route: string) => void } = $props();

  const STEPS = ["Hoş geldin", "Kısayollar", "Oyun içi", "Klipler", "Son dokunuş"];
  let step = $state(0);

  let keys = $state<HotkeyStatus[]>([]);
  let fps = $state<FpsStatus | null>(null);
  let fpsBusy = $state(false);
  let roots = $state<ClipRoot[]>([]);
  let autostart = $state(false);
  let autostartReady = $state(false);

  const SHOWN: [string, string][] = [
    ["panel.toggle", "Oyunun üstünde GMID paneli"],
    ["app.show", "GMID Tool'u öne getir"],
    ["note.quick", "Hızlı not"],
    ["mic.toggleMute", "Mikrofonu kapat / aç"],
    ["audio.cycleOutput", "Kulaklık ↔ hoparlör"],
    ["videos.clipboard", "Kopyaladığın bağlantıyı indir"],
  ];
  const shortcuts = $derived([
    ["Ctrl+K", "Her şeyde ara (uygulama içinde)"] as [string, string],
    ...SHOWN.flatMap(([id, label]) => {
      const k = keys.find((h) => h.id === id);
      return k?.accel ? [[k.accel, label] as [string, string]] : [];
    }),
  ]);
  const panelKey = $derived(keys.find((h) => h.id === "panel.toggle")?.accel || "Ctrl+Shift+G");
  const fpsOk = $derived(fps?.state === "ready" || fps?.state === "running");

  onMount(() => {
    void api.hotkeys.list().then((k) => (keys = k)).catch(() => {});
    void api.fps.status().then((s) => (fps = s)).catch(() => {});
    void api.clips.roots().then((r) => (roots = r.filter((x) => x.exists))).catch(() => {});
    void api.autostart.isEnabled().then((on) => {
      autostart = on;
      autostartReady = inTauri;
    }).catch(() => {});
  });

  async function setupFps() {
    fpsBusy = true;
    try {
      fps = await api.fps.setup();
    } catch (e) {
      fps = { state: "error", relogin: false, message: String(e) };
    } finally {
      fpsBusy = false;
    }
  }

  async function toggleAutostart(on: boolean) {
    try {
      await api.autostart.set(on);
      autostart = on;
    } catch {
      /* Kontrol Odası'ndan yeniden denenir */
    }
  }

  function go(route: string) {
    finishWelcome();
    onnavigate(route);
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") finishWelcome();
    else if (e.key === "ArrowRight" && step < STEPS.length - 1) step++;
    else if (e.key === "ArrowLeft" && step > 0) step--;
  }

  const ROOMS: [IconName, string, string][] = [
    ["server", "Oyun Salonu", "GMID sunucuları, kütüphanen, oturumların, fırsatlar ve kliplerin."],
    ["film", "Eğlence Salonu", "Oyun ve film arşivin, çark & çekiliş, medya oynatıcı, video indirici."],
    ["note", "Çalışma Odası", "Notlar, hatırlatıcılar, sayaç ve ses ayarları."],
  ];
</script>

<svelte:window onkeydown={onkey} />

<div class="back" role="presentation">
  <div class="card" role="dialog" aria-label="GMID Tool'a hoş geldin" aria-modal="true">
    <div class="dots" aria-hidden="true">
      {#each STEPS as s, i (s)}<button class:on={i === step} class:done={i < step} onclick={() => (step = i)} tabindex="-1" title={s}></button>{/each}
    </div>

    <div class="body">
      {#if step === 0}
        <img class="logo" src={gmidLogo} alt="" />
        <h2>GMID Tool'a hoş geldin</h2>
        <p class="lead">Game Mansion In Discord'un alet çantası. Bir dakikada en işe yarar yerleri gösterelim.</p>
        <div class="rooms">
          {#each ROOMS as [icon, name, text] (name)}
            <div class="room"><span class="ic"><Icon name={icon} size={18} /></span><div><b>{name}</b><span>{text}</span></div></div>
          {/each}
        </div>
        {#if community.discord}
          <button class="btn dc" onclick={joinDiscord}><Icon name="discord" size={17} />GMID Discord'a katıl</button>
        {/if}
      {:else if step === 1}
        <span class="badge"><Icon name="keyboard" size={20} /></span>
        <h2>Kısayollar her yerde çalışır</h2>
        <p class="lead">Oyundayken bile. Hepsi Kontrol Odası → Kısayollar'dan değiştirilebilir.</p>
        <ul class="keys">
          {#each shortcuts as [accel, label] (label)}
            <li><span>{label}</span><span class="kb">{#each accel.split("+") as k, i (i)}{#if i}<i>+</i>{/if}<kbd>{k}</kbd>{/each}</span></li>
          {/each}
        </ul>
        <button class="link" onclick={() => go("settings/kisayollar")}>Kısayolları değiştir →</button>
      {:else if step === 2}
        <span class="badge"><Icon name="monitor" size={20} /></span>
        <h2>Oyunun üstünde GMID paneli</h2>
        <p class="lead">Oyundayken <kbd>{panelKey}</kbd> bas: Steam'deki Shift+Tab gibi panel açılır. FPS, sıcaklık, uygulama sesleri, sunucular ve notlar orada.</p>
        <div class="tip"><Icon name="info" size={16} /><span>En iyi <b>kenarlıksız pencere</b> (borderless) modunda çalışır: fare serbest kalır, oyun arkada görünür. Tam ekran oyunlarda panel sadece bakmalık açılır.</span></div>
        <div class="row">
          <div>
            <b>FPS ölçümü</b>
            <span class="sub">
              {#if !fps}Bakılıyor…
              {:else if fpsOk}Hazır: panelde FPS görünür.
              {:else if fps.relogin}İzin verildi. Windows oturumunu bir kez kapatıp açınca FPS görünür.
              {:else if fps.state === "error"}{fps.message ?? "Kurulamadı"}. Oyun Modu'ndan yeniden dene.
              {:else}Bir kez Windows izni ister (Intel PresentMon, oyuna dokunmaz).{/if}
            </span>
          </div>
          {#if fps && !fpsOk && !fps.relogin}
            <button class="btn sm" disabled={fpsBusy} onclick={setupFps}><Icon name="bolt" size={14} />{fpsBusy ? "Kuruluyor…" : "FPS ölçümünü aç"}</button>
          {:else if fpsOk || fps?.relogin}
            <span class="ok"><Icon name="check" size={16} /></span>
          {/if}
        </div>
      {:else if step === 3}
        <span class="badge"><Icon name="clip" size={20} /></span>
        <h2>Klipler kendiliğinden bulunur</h2>
        <p class="lead">NVIDIA, Xbox Game Bar, AMD, OBS ve Steam kayıtların oyuna göre dizilir. Discord'a göndermek için tek tık: boyuta sığacak şekilde küçültür, istersen kırpar.</p>
        {#if roots.length}
          <ul class="roots">
            {#each roots as r (r.path)}<li><Icon name="folder" size={15} /><span><b>{r.label}</b><small>{r.path}</small></span></li>{/each}
          </ul>
        {:else}
          <p class="sub">Henüz kayıt klasörü bulunamadı. Kliplerini başka yere kaydediyorsan klasörü ekle.</p>
        {/if}
        <button class="link" onclick={() => go("settings/oyunlar")}>Klasör ekle ya da çıkar →</button>
      {:else}
        <span class="badge"><Icon name="sparkle" size={20} /></span>
        <h2>Son dokunuş</h2>
        <p class="lead">Kısayolların, hatırlatıcıların ve sunucu bildirimlerin uygulama açıkken çalışır. Kapatınca sağ alttaki tepside bekler.</p>
        <div class="row">
          <div><b>Windows açılınca başlat</b><span class="sub">Tepside sessizce açılır, pencere çıkmaz.</span></div>
          <Switch label="Windows açılınca başlat" checked={autostart} disabled={!autostartReady} onchange={toggleAutostart} />
        </div>
        <div class="tip"><Icon name="search" size={16} /><span>Bir şey mi arıyorsun? <kbd>Ctrl</kbd>+<kbd>K</kbd> notları, oyunları, filmleri, klipleri ve ayarları tek yerden arar.</span></div>
        <div class="tip"><Icon name="flag" size={16} /><span>Bir şey bozuksa Kontrol Odası → Hakkında → <b>Sorun bildir</b>.</span></div>
      {/if}
    </div>

    <div class="foot">
      <button class="link skip" onclick={finishWelcome}>{step === STEPS.length - 1 ? "" : "Geç"}</button>
      <span class="count tnum">{step + 1} / {STEPS.length}</span>
      <div class="nav">
        {#if step > 0}<button class="btn ghost" onclick={() => step--}>Geri</button>{/if}
        {#if step < STEPS.length - 1}
          <button class="btn" onclick={() => step++}>İleri<Icon name="arrow" size={15} /></button>
        {:else}
          <button class="btn" onclick={finishWelcome}><Icon name="check" size={15} />Başla</button>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .back { position: fixed; inset: 0; z-index: 80; display: grid; place-items: center; padding: 16px; background: rgba(0, 0, 0, 0.55); backdrop-filter: blur(4px); animation: fade 0.3s ease; }
  .card { width: min(560px, 100%); max-height: calc(100vh - 32px); display: flex; flex-direction: column; border-radius: 18px; background: var(--night-2); border: 1px solid var(--line-2); box-shadow: 0 30px 90px rgba(0, 0, 0, 0.6); overflow: hidden; }
  .dots { display: flex; gap: 6px; padding: 16px 24px 0; }
  .dots button { flex: 1; height: 4px; padding: 0; border: 0; border-radius: 999px; background: rgba(255, 255, 255, 0.08); transition: background 0.3s; }
  .dots button.done { background: var(--gold-dim); }
  .dots button.on { background: var(--gold); }
  .body { padding: 22px 28px 8px; overflow-y: auto; display: flex; flex-direction: column; align-items: flex-start; gap: 12px; min-height: 340px; }
  h2 { margin: 0; font-family: var(--f-display); font-size: 22px; }
  .lead { margin: 0; color: var(--muted); line-height: 1.55; }
  .sub { font-size: 12.5px; color: var(--muted); }
  .logo { width: 64px; height: 64px; border-radius: 16px; }
  .badge { display: grid; place-items: center; width: 44px; height: 44px; border-radius: 12px; background: rgba(227, 194, 112, 0.12); color: var(--gold); }
  .rooms { display: flex; flex-direction: column; gap: 8px; width: 100%; }
  .room { display: flex; gap: 12px; align-items: center; padding: 10px 12px; border-radius: 12px; background: var(--coal-2); border: 1px solid var(--line); }
  .room > div { display: flex; flex-direction: column; gap: 2px; }
  .room span:not(.ic) { font-size: 12.5px; color: var(--muted); }
  .ic { display: grid; place-items: center; color: var(--gold); flex: none; }
  .dc { background: #5865f2; border-color: #5865f2; color: #fff; }
  .dc:hover { background: #4752c4; }
  .keys { list-style: none; margin: 0; padding: 0; width: 100%; display: flex; flex-direction: column; }
  .keys li { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 8px 2px; border-bottom: 1px solid var(--line); font-size: 13.5px; }
  .keys li:last-child { border-bottom: 0; }
  .kb { display: inline-flex; align-items: center; gap: 3px; flex: none; }
  .kb i { font-style: normal; color: var(--dim); font-size: 11px; }
  kbd { font: 600 11.5px var(--f-body); padding: 2px 7px; border-radius: 6px; border: 1px solid var(--line-2); border-bottom-width: 2px; background: var(--coal-2); color: var(--text); white-space: nowrap; }
  .tip { display: flex; gap: 10px; align-items: flex-start; width: 100%; padding: 10px 12px; border-radius: 12px; background: rgba(53, 160, 162, 0.08); border: 1px solid rgba(53, 160, 162, 0.3); font-size: 13px; line-height: 1.5; color: var(--muted); }
  .tip :global(svg) { color: var(--neon); flex: none; margin-top: 2px; }
  .tip b { color: var(--text); }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 14px; width: 100%; padding: 12px; border-radius: 12px; background: var(--coal-2); border: 1px solid var(--line); }
  .row > div { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .ok { color: var(--neon); display: grid; place-items: center; }
  .roots { list-style: none; margin: 0; padding: 0; width: 100%; display: flex; flex-direction: column; gap: 6px; }
  .roots li { display: flex; gap: 10px; align-items: center; padding: 8px 10px; border-radius: 10px; background: var(--coal-2); border: 1px solid var(--line); color: var(--gold); }
  .roots span { display: flex; flex-direction: column; min-width: 0; color: var(--text); font-size: 13px; }
  .roots small { color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .link { border: 0; background: none; padding: 0; color: var(--gold); font-weight: 600; font-size: 13px; }
  .link:hover { text-decoration: underline; }
  .foot { display: flex; align-items: center; gap: 12px; padding: 14px 24px 18px; border-top: 1px solid var(--line); }
  .skip { color: var(--dim); font-weight: 500; min-width: 30px; text-align: left; }
  .count { font-size: 12px; color: var(--dim); }
  .nav { margin-left: auto; display: flex; gap: 8px; }
  .nav .btn { display: inline-flex; align-items: center; gap: 6px; }
  @keyframes fade { from { opacity: 0; } }
  @media (prefers-reduced-motion: reduce) { .back { animation: none; } }
</style>
