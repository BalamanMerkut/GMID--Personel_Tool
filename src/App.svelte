<script lang="ts">
  import { onMount } from "svelte";
  import { api, type AppInfo } from "./lib/api";
  import Background from "./lib/components/Background.svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Splash from "./lib/components/Splash.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import { findScreen, roomOf, SCREENS } from "./lib/nav";
  import Audio from "./lib/screens/Audio.svelte";
  import Home from "./lib/screens/Home.svelte";
  import Placeholder from "./lib/screens/Placeholder.svelte";
  import Reminders from "./lib/screens/Reminders.svelte";
  import Servers from "./lib/screens/Servers.svelte";
  import Videos from "./lib/screens/Videos.svelte";
  import Media from "./lib/screens/Media.svelte";
  import Library from "./lib/screens/Library.svelte";
  import Sessions from "./lib/screens/Sessions.svelte";
  import Archive from "./lib/screens/Archive.svelte";
  import Clips from "./lib/screens/Clips.svelte";
  import GameMode from "./lib/screens/GameMode.svelte";
  import Films from "./lib/screens/Films.svelte";
  import Install from "./lib/screens/Install.svelte";
  import Deals from "./lib/screens/Deals.svelte";
  import WrappedScreen from "./lib/screens/Wrapped.svelte";
  import Player from "./lib/components/Player.svelte";
  import { nav, player, stopPlayer } from "./lib/player.svelte";
  import { activeSummary, downloads, startDownloadsSync } from "./lib/downloads.svelte";
  import Icon from "./lib/Icon.svelte";
  import Settings from "./lib/screens/Settings.svelte";
  import Timer from "./lib/screens/Timer.svelte";
  import { effectiveQuality, loadSettings, settings } from "./lib/settings.svelte";

  let route = $state("home");
  let ready = $state(false);
  let showSplash = $state(false);
  /** Uygulama görünür mü (açılış animasyonu karardıysa). */
  let revealed = $state(false);
  /** Pencere görünür ve odakta mı; değilse hareketli şeyler durur. */
  let active = $state(true);
  let info: AppInfo | null = $state(null);
  let main: HTMLElement | undefined = $state();

  // Çalan kayıt silinirse oynatıcı kapanır.
  $effect(() => {
    const id = player.item?.id;
    if (id && downloads.loaded && !downloads.list.some((d) => d.id === id)) stopPlayer();
  });

  const quality = $derived(effectiveQuality(settings.quality));
  const screen = $derived(findScreen(route) ?? SCREENS[0]);
  const dl = $derived(activeSummary());

  /** Ayarlar ekranında açık bölüm ("settings/kisayollar" gibi adreslerle de seçilir). */
  let settingsTab = $state("genel");

  nav.go = (r) => navigate(r);

  function navigate(target: string) {
    const [id, sub] = target.split("/");
    route = findScreen(id) ? id : "home";
    if (route === "settings" && sub) settingsTab = sub;
    try {
      history.replaceState(null, "", `#${route === "settings" ? `settings/${settingsTab}` : route}`);
    } catch {
      /* önemsiz */
    }
    main?.scrollTo(0, 0);
  }

  async function refreshInfo() {
    try {
      info = await api.appInfo();
    } catch (e) {
      console.error(e);
    }
  }

  function replaySplash() {
    revealed = false;
    showSplash = true;
  }

  onMount(() => {
    const fromHash = location.hash.slice(1);
    if (findScreen(fromHash.split("/")[0])) navigate(fromHash);

    let unlisten = () => {};
    (async () => {
      try {
        await loadSettings();
      } catch (e) {
        console.error("Ayarlar okunamadı", e);
      }
      const splash = await api.takeSplash().catch(() => false);
      showSplash = splash && settings.splash;
      revealed = !showSplash;
      ready = true;

      const pending = await api.takePendingRoute().catch(() => null);
      if (pending) navigate(pending);
      unlisten = await api.onNavigate((r) => {
        navigate(r);
        void api.takePendingRoute();
      });
      void refreshInfo();
    })();
    const stopDownloads = startDownloadsSync();

    const update = () => (active = document.visibilityState === "visible" && document.hasFocus());
    window.addEventListener("focus", update);
    window.addEventListener("blur", update);
    document.addEventListener("visibilitychange", update);
    return () => {
      stopDownloads();
      unlisten();
      window.removeEventListener("focus", update);
      window.removeEventListener("blur", update);
      document.removeEventListener("visibilitychange", update);
    };
  });
</script>

{#if ready}
  <Background active={active && revealed} {quality} />

  <div class="app" class:pre={!revealed} class:withmini={player.item && !player.open} data-quality={quality}>
    <Sidebar current={route} version={info?.version ?? ""} onnavigate={navigate} />
    <div class="content">
      <header class="topbar">
        <div class="titles">
          <span class="crumb">{roomOf(route) ?? "GMID Tool"}</span>
          <h1>{screen.label}</h1>
        </div>
        {#if dl.count}
          <button class="dlind" onclick={() => navigate("videos")} title="Video İndirici">
            <Icon name="download" size={17} />
            <span class="tnum">{dl.count} indiriliyor · %{dl.percent}</span>
            <span class="ibar"><i style:width="{dl.percent}%"></i></span>
          </button>
        {/if}
      </header>
      <main bind:this={main}>
        {#if route === "home"}
          <Home {info} onnavigate={navigate} />
        {:else if route === "settings"}
          <Settings {info} {refreshInfo} onreplay={replaySplash} onnavigate={navigate} bind:tab={settingsTab} />
        {:else if route === "audio"}
          <Audio onnavigate={navigate} />
        {:else if route === "reminders"}
          <Reminders />
        {:else if route === "videos"}
          <Videos onnavigate={navigate} />
        {:else if route === "media"}
          <Media onnavigate={navigate} />
        {:else if route === "library"}
          <Library />
        {:else if route === "films"}
          <Films />
        {:else if route === "wrapped"}
          <WrappedScreen />
        {:else if route === "deals"}
          <Deals onnavigate={navigate} />
        {:else if route === "install"}
          <Install />
        {:else if route === "gamemode"}
          <GameMode />
        {:else if route === "clips"}
          <Clips onnavigate={navigate} />
        {:else if route === "archive"}
          <Archive />
        {:else if route === "sessions"}
          <Sessions onnavigate={navigate} />
        {:else if route === "servers"}
          <Servers />
        {:else if route === "timer"}
          <Timer />
        {:else if route === "notes"}
          <!-- Not editörü büyük olduğu için sadece Notlar açılınca yüklenir. -->
          {#await import("./lib/screens/Notes.svelte") then m}
            <m.default />
          {/await}
        {:else}
          <Placeholder {screen} onnavigate={navigate} />
        {/if}
      </main>
    </div>
  </div>

  {#if player.item}
    <!-- Ekran değişse de çalmaya devam eder; küçültülünce altta çalma çubuğu. -->
    <Player
      item={player.item}
      queue={player.queue}
      mini={!player.open}
      onclose={() => (player.open = false)}
      onexpand={() => (player.open = true)}
      onstop={stopPlayer}
      onpick={(d) => (player.item = d)}
    />
  {/if}

  {#if showSplash}
    <Splash onleave={() => (revealed = true)} ondone={() => (showSplash = false)} />
  {/if}
  <Toast />
{/if}

<style>
  .app {
    position: relative;
    z-index: 1;
    height: 100%;
    display: grid;
    grid-template-columns: var(--side-w) minmax(0, 1fr);
    transition: opacity 0.6s ease, transform 0.6s ease;
  }
  .app.pre { opacity: 0; transform: scale(0.99); }
  .app.withmini main { padding-bottom: 96px; }
  .content { display: grid; grid-template-rows: auto minmax(0, 1fr); min-width: 0; min-height: 0; }
  .topbar {
    display: flex; justify-content: space-between; align-items: center; gap: 16px;
    padding: 14px 28px; border-bottom: 1px solid var(--line);
    background: rgba(18, 20, 23, 0.55);
  }
  .titles { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .dlind {
    display: inline-flex; align-items: center; gap: 8px; flex: none;
    border: 1px solid rgba(53, 160, 162, 0.55); background: rgba(53, 160, 162, 0.1); color: var(--neon);
    border-radius: 999px; padding: 6px 12px; font-size: 12.5px; font-weight: 600;
  }
  .dlind:hover { background: rgba(53, 160, 162, 0.18); }
  .ibar { width: 60px; height: 5px; border-radius: 999px; background: rgba(255, 255, 255, 0.1); overflow: hidden; }
  .ibar i { display: block; height: 100%; background: var(--neon); transition: width 0.4s; }
  .crumb {
    font-family: var(--f-display); font-size: 10.5px; font-weight: 700;
    letter-spacing: 0.14em; text-transform: uppercase; color: var(--gold-dim);
  }
  h1 { font-family: var(--f-display); font-size: 20px; font-weight: 700; margin: 0; }
  main { overflow: auto; padding: 24px 28px 40px; min-width: 0; }
  @media (max-width: 1100px) {
    .app { grid-template-columns: 76px minmax(0, 1fr); }
    main { padding: 20px; }
    .topbar { padding: 12px 20px; }
  }
</style>
