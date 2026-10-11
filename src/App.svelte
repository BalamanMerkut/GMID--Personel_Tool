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

  const quality = $derived(effectiveQuality(settings.quality));
  const screen = $derived(findScreen(route) ?? SCREENS[0]);

  function navigate(id: string) {
    route = findScreen(id) ? id : "home";
    try {
      history.replaceState(null, "", `#${route}`);
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
    if (findScreen(fromHash)) route = fromHash;

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

    const update = () => (active = document.visibilityState === "visible" && document.hasFocus());
    window.addEventListener("focus", update);
    window.addEventListener("blur", update);
    document.addEventListener("visibilitychange", update);
    return () => {
      unlisten();
      window.removeEventListener("focus", update);
      window.removeEventListener("blur", update);
      document.removeEventListener("visibilitychange", update);
    };
  });
</script>

{#if ready}
  <Background active={active && revealed} {quality} />

  <div class="app" class:pre={!revealed} data-quality={quality}>
    <Sidebar current={route} version={info?.version ?? ""} onnavigate={navigate} />
    <div class="content">
      <header class="topbar">
        <span class="crumb">{roomOf(route) ?? "GMID Tool"}</span>
        <h1>{screen.label}</h1>
      </header>
      <main bind:this={main}>
        {#if route === "home"}
          <Home {info} onnavigate={navigate} />
        {:else if route === "settings"}
          <Settings {info} {refreshInfo} onreplay={replaySplash} />
        {:else if route === "audio"}
          <Audio />
        {:else if route === "reminders"}
          <Reminders />
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
  .content { display: grid; grid-template-rows: auto minmax(0, 1fr); min-width: 0; min-height: 0; }
  .topbar {
    display: flex; flex-direction: column; gap: 2px;
    padding: 14px 28px; border-bottom: 1px solid var(--line);
    background: rgba(18, 20, 23, 0.55);
  }
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
