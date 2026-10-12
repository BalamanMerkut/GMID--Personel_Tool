<script lang="ts">
  // Oyun Modu: tek tıkla (ya da oyun açılınca kendiliğinden) oyuna hazırlan; kapanınca her şey geri alınır.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Switch from "../components/Switch.svelte";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { Device, FpsStatus, GameModeProfile, GameModeStatus, RunningApp } from "../types";

  const DEFAULT: GameModeProfile = { auto: false, power: true, output: null, micLock: true, quiet: true, stopBackground: true, closeApps: [] };

  let status: GameModeStatus = $state({ active: false, reason: null, since: null, quiet: true, stopBackground: true });
  let profile: GameModeProfile = $state({ ...DEFAULT, ...getJson<Partial<GameModeProfile>>("gamemode.profile", {}) });
  let outputs: Device[] = $state([]);
  let apps: RunningApp[] = $state([]);
  let newApp = $state("");
  let busy = $state(false);
  let now = $state(Date.now());
  let fps = $state<FpsStatus | null>(null);
  let fpsBusy = $state(false);

  async function setupFps() {
    fpsBusy = true;
    try {
      fps = await api.fps.setup();
      notify(fps.relogin ? "İzin verildi · Windows oturumunu bir kez kapatıp aç" : "FPS ölçümü hazır", "check");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      fpsBusy = false;
    }
  }

  onMount(() => {
    void api.gamemode.status().then((s) => (status = s)).catch(() => {});
    void api.audio.state().then((s) => (outputs = s.outputs)).catch(() => {});
    void api.games.runningApps().then((a) => (apps = a)).catch(() => {});
    void api.fps.status().then((s) => (fps = s)).catch(() => {});
    const stop = on<GameModeStatus>("gamemode", (s) => (status = s));
    const timer = setInterval(() => (now = Date.now()), 30_000);
    return () => {
      clearInterval(timer);
      void stop.then((f) => f());
    };
  });

  function save() {
    void setJson("gamemode.profile", $state.snapshot(profile));
  }

  async function toggle() {
    busy = true;
    try {
      status = await api.gamemode.set(!status.active);
    } catch (e) {
      notify(String(e), "info");
    } finally {
      busy = false;
    }
  }

  function addApp(exe: string) {
    const x = exe.trim().toLowerCase();
    if (!x) return;
    const name = x.endsWith(".exe") ? x : `${x}.exe`;
    if (!profile.closeApps.includes(name)) profile.closeApps = [...profile.closeApps, name];
    newApp = "";
    save();
  }

  const reasonText = $derived(status.reason === "auto" ? "oyun açıldığı için kendiliğinden" : "elle açıldı");
</script>

<div class="gm">
  <section class="panel hero" class:on={status.active}>
    <div class="htx">
      <span class="badge">{status.active ? "Açık" : "Kapalı"}</span>
      <h2>Oyun Modu</h2>
      <p class="sub">
        {#if status.active}
          {reasonText}{status.since ? ` · ${ago(status.since, now)}` : ""}. Kapatınca güç planı, ses çıkışı ve mikrofon kilidi
          eski haline döner.
        {:else}
          Tek tıkla oyuna hazırlan: güç planı yüksek performansa geçer, kulaklığa geçilir, mikrofon kilitlenir, bildirimler susar.
          Kısayol: <b>Ctrl+Alt+O</b>.
        {/if}
      </p>
    </div>
    <button class="big" class:on={status.active} onclick={toggle} disabled={busy}>
      <Icon name="bolt" size={26} />{status.active ? "Kapat" : "Aç"}
    </button>
  </section>

  <section class="panel">
    <h3 class="h2"><Icon name="gear" />Açılınca ne olsun?</h3>
    <div class="line">
      <div><b>Oyun açılınca kendiliğinden</b><span class="sub small">Kütüphanendeki bir oyun açılınca Oyun Modu açılır, son oyun kapanınca kapanır.</span></div>
      <Switch label="Kendiliğinden" checked={profile.auto} onchange={(v) => ((profile.auto = v), save())} />
    </div>
    <div class="line">
      <div><b>Güç planı: Yüksek performans</b><span class="sub small">İşlemci hız düşürmez. Önceki planın hatırlanır, kapatınca geri döner.</span></div>
      <Switch label="Güç planı" checked={profile.power} onchange={(v) => ((profile.power = v), save())} />
    </div>
    <div class="line">
      <div><b>Ses çıkışı</b><span class="sub small">Oyunda bu cihaza geç (ör. kulaklık); kapatınca önceki cihaza döner.</span></div>
      <select value={profile.output ?? ""} onchange={(e) => ((profile.output = e.currentTarget.value || null), save())} aria-label="Ses çıkışı">
        <option value="">Değiştirme</option>
        {#each outputs as d (d.id)}<option value={d.id}>{d.name}</option>{/each}
      </select>
    </div>
    <div class="line">
      <div><b>Mikrofon seviyesi kilidi</b><span class="sub small">Discord ya da oyun mikrofonunu kısamasın.</span></div>
      <Switch label="Mikrofon kilidi" checked={profile.micLock} onchange={(v) => ((profile.micLock = v), save())} />
    </div>
    <div class="line">
      <div><b>Sessiz</b><span class="sub small">GMID'in uyarı sesleri ve Windows bildirimleri susar; ekran göstergesi kalır.</span></div>
      <Switch label="Sessiz" checked={profile.quiet} onchange={(v) => ((profile.quiet = v), save())} />
    </div>
    <div class="line">
      <div><b>Hareketli arka planı durdur</b><span class="sub small">GMID'in video/GIF arka planı ekran kartını yormasın.</span></div>
      <Switch label="Arka plan" checked={profile.stopBackground} onchange={(v) => ((profile.stopBackground = v), save())} />
    </div>
    <div class="line col">
      <div><b>Kapatılacak uygulamalar</b><span class="sub small">Oyun Modu açılınca kapanır (kaydetmen gereken bir şey varsa sorarlar). Kapatınca yeniden açılmaz.</span></div>
      <div class="chips">
        {#each profile.closeApps as x (x)}
          <span class="chip">{x}<button title="Çıkar" onclick={() => ((profile.closeApps = profile.closeApps.filter((y) => y !== x)), save())}><Icon name="x" size={12} /></button></span>
        {:else}
          <span class="sub small">Hiçbiri</span>
        {/each}
      </div>
      <div class="row">
        <select onchange={(e) => (addApp(e.currentTarget.value), (e.currentTarget.value = ""))} aria-label="Açık uygulamalardan ekle">
          <option value="">Açık uygulamalardan seç…</option>
          {#each apps as a (a.path)}<option value={a.name}>{a.title} ({a.name})</option>{/each}
        </select>
        <input bind:value={newApp} placeholder="ya da adı: chrome.exe" onkeydown={(e) => e.key === "Enter" && addApp(newApp)} />
        <button class="btn ghost sm" onclick={() => addApp(newApp)}><Icon name="plus" size={14} />Ekle</button>
      </div>
    </div>
  </section>

  <section class="panel">
    <h3 class="h2"><Icon name="stage" />Oyun İçi Panel</h3>
    <p class="sub">
      Oyundayken <b>Ctrl+Shift+G</b>: oyunun üstünde yarı saydam bir panel açılır. Ses, mikrofon, Oyun Modu, sayaç, oyuna not ve
      son klibin elinin altında. Panel oyundan odağı almaz, yani oyun küçülmez; fare imleci görünmüyorsa oyunun menüsünü
      (Esc) açıp tıklayabilirsin. Aynı tuşla kapanır. Oyuna hiçbir şey eklenmez (anti-cheat güvenli). Panelin görünmesi
      için oyunun görüntü ayarında <b>Kenarlıksız / Borderless</b> ya da <b>Pencereli</b> seçili olmalı; “Tam ekran
      (özel)” modunda Windows hiçbir pencereyi oyunun üstünde göstermez.
    </p>
    <div class="line">
      <div>
        <b>FPS ölçümü</b>
        <span class="sub small">
          {#if !fps}…
          {:else if fps.state === "ready" || fps.state === "running"}Hazır: panelde FPS görünür (PresentMon, oyuna dokunmaz).
          {:else if fps.relogin}İzin verildi. Windows oturumunu bir kez <b>kapatıp aç</b> (ya da yeniden başlat), sonra FPS görünür.
          {:else if fps.state === "permission"}PresentMon kurulu ama Windows izni yok: bir kez yönetici izni gerekiyor.
          {:else}Panelde FPS için küçük bir ölçüm aracı (Intel PresentMon, ~1 MB) kurulur ve bir kez yönetici izni istenir.
            RivaTuner (RTSS) açıksa ona gerek kalmaz.{/if}
        </span>
      </div>
      {#if fps && fps.state !== "ready" && fps.state !== "running" && !fps.relogin}
        <button class="btn sm" disabled={fpsBusy} onclick={setupFps}><Icon name="bolt" size={14} />{fpsBusy ? "Kuruluyor…" : "FPS ölçümünü aç"}</button>
      {/if}
    </div>
  </section>
</div>

<style>
  .gm { display: flex; flex-direction: column; gap: 16px; max-width: 980px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 14px; }
  .hero { flex-direction: row; align-items: center; gap: 24px; padding: 24px; }
  .hero.on { border-color: var(--gold); background: linear-gradient(110deg, rgba(227, 194, 112, 0.14), transparent 60%); }
  .htx { flex: 1; display: flex; flex-direction: column; gap: 6px; }
  .htx h2 { margin: 0; font-family: var(--f-display); font-size: 26px; }
  .htx p { margin: 0; max-width: 620px; }
  .badge { align-self: flex-start; font-size: 11px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; padding: 3px 9px; border-radius: 999px; background: rgba(255, 255, 255, 0.07); color: var(--muted); }
  .hero.on .badge { background: var(--gold); color: #1b1608; }
  .big { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px; width: 120px; height: 120px; border-radius: 50%; border: 2px solid var(--line-2); background: var(--coal-2); color: var(--text); font-weight: 800; font-size: 15px; transition: transform 0.15s, box-shadow 0.15s, background 0.15s; }
  .big:hover:not(:disabled) { transform: scale(1.04); border-color: var(--gold); }
  .big.on { background: var(--gold); color: #1b1608; border-color: var(--gold-glow); box-shadow: 0 0 34px rgba(227, 194, 112, 0.45); }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .line > div:first-child { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1 1 260px; }
  .line.col { flex-direction: column; align-items: stretch; }
  select, input { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; color: var(--text); max-width: 320px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { display: inline-flex; align-items: center; gap: 6px; }
  .chip button { border: 0; background: none; color: var(--muted); padding: 0; display: grid; }
  .chip button:hover { color: var(--bad); }
  @media (max-width: 640px) {
    .hero { flex-direction: column; align-items: stretch; }
    .big { align-self: center; }
  }
</style>
