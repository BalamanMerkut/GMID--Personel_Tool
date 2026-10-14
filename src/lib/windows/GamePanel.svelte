<script lang="ts">
  // Oyun İçi Panel (Ctrl+Shift+G): oyunun üstünde yarı saydam katman. Ses, mikrofon, Oyun Modu, sayaç,
  // bu oyuna not, son klip, yaklaşan hatırlatıcılar. Esc ya da aynı tuş kapatır; odak oyuna döner.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { emitAll, on } from "../bus";
  import AppMixer from "../components/AppMixer.svelte";
  import NoteEditor from "../components/NoteEditor.svelte";
  import PowerWarn from "../components/PowerWarn.svelte";
  import Icon from "../Icon.svelte";
  import { msText, summary, tone } from "../ping";
  import { clock, playtime } from "../time";
  import type { AppVolume, FootballMatch, GmidPlayerState, NetReport, NowPlaying, AudioState, Clip, Countdown, Game, GameModeStatus, NoteSummary, PanelOpen, PerfSample, Reminder } from "../types";

  let audio = $state<AudioState | null>(null);
  let game = $state<Game | null>(null);
  // Ping: tek tıkla kısa bağlantı testi (alt-tab yok).
  let ping = $state<NetReport | null>(null);
  let pinging = $state(false);
  async function runPing() {
    if (pinging) return;
    pinging = true;
    try {
      ping = await api.net.test(true);
    } catch (e) {
      flash(String(e));
    } finally {
      pinging = false;
    }
  }
  let gm: GameModeStatus = $state({ active: false, reason: null, since: null, quiet: true, stopBackground: true });
  let countdowns: Countdown[] = $state([]);
  let reminders: Reminder[] = $state([]);
  let clip = $state<Clip | null>(null);
  let noteId: string | null = $state(null);
  let noteText = $state("");
  let noteSaved = $state(true);
  /** Her yüklemede artar: düzenleyici yeni içeriği alsın. */
  let noteLoads = $state(0);
  let now = $state(Date.now());
  let msg = $state("");
  let perf = $state<PerfSample | null>(null);
  /** interactive: panel odakta (Shift+Tab gibi) · glance: oyun önde, panel yalnız gösteriyor. */
  const params = new URLSearchParams(location.search);
  let openInfo = $state<PanelOpen>({
    mode: params.has("glance") ? "glance" : "interactive",
    exclusive: params.has("exclusive"),
    minimizes: false,
    exe: params.has("exclusive") ? "hl.exe" : null,
    borderless: false,
  });
  let fpsBusy = $state(false);
  /** Zamanı gelen uyarılar (oyundayken pencere yerine panelde). */
  let alarmReminders: Reminder[] = $state([]);
  let alarmTimers: Countdown[] = $state([]);
  /** Panel uyarı için kendiliğinden açıldı: uyarılar bitince kapanır, oyuna dönülür. */
  let alarmAuto = $state(false);

  async function loadAlarms() {
    const a = await api.overlay.alerts().catch(() => null);
    alarmReminders = a?.reminders ?? [];
    alarmTimers = a?.timers ?? [];
    // Zamanlı kapatma uyarısı da paneli açık tutar (kapatılana ya da iptal edilene kadar).
    const plan = await api.power.status().catch(() => null);
    const powerSoon = !!plan && !plan.hidden && plan.at - Date.now() < 66_000;
    if (alarmAuto && !alarmReminders.length && !alarmTimers.length && !powerSoon) {
      alarmAuto = false;
      close();
    }
  }
  async function alarmAct(p: Promise<unknown>) {
    await p.catch(() => {});
    await loadAlarms();
  }
  let perfTimer: ReturnType<typeof setInterval> | undefined;

  // Ölçüm yalnız panel açıkken (saniyede bir).
  // Şimdi çalan (Spotify, YouTube…): panel açıkken 2 sn'de bir.
  let media = $state<NowPlaying | null>(null);
  /** Konumun okunduğu an (araya giren saniyeler çalıyorsa eklenir). */
  let mediaAt = $state(0);
  let mixApps = $state<AppVolume[]>([]);
  let mediaTimer: ReturnType<typeof setInterval> | undefined;
  const loadMedia = () => {
    void api.media.now().then((m) => ((media = m), (mediaAt = Date.now()))).catch(() => {});
    void api.audio.apps().then((a) => (mixApps = a)).catch(() => {});
  };
  // GMID'in kendi oynatıcısı çalıyorsa (son 3 sn'de durum geldiyse) onun bilgisi ve komutları kullanılır:
  // Windows'un medya oturumu karıştır/tekrar/ses bilgisini bu oynatıcı için vermiyor.
  let gmid = $state<GmidPlayerState | null>(null);
  let gmidAt = $state(0);
  onMount(() => {
    const stop = on<GmidPlayerState>("gmid-player", (st) => ((gmid = st), (gmidAt = Date.now())));
    return () => void stop.then((f) => f());
  });
  const ownPlayer = $derived(!!gmid && (gmid.playing ? now - gmidAt < 3000 : now - gmidAt < 60_000));
  /** Kartta gösterilen: GMID oynatıcısı ya da Windows'un medya oturumu. */
  const np = $derived<NowPlaying | null>(
    ownPlayer && gmid
      ? {
          app: "GMID Tool",
          title: gmid.title,
          artist: gmid.artist,
          playing: gmid.playing,
          thumb: gmid.thumb ? (/^(https?:|data:)/.test(gmid.thumb) ? gmid.thumb : api.fileSrc(gmid.thumb)) : null,
          source: "gmid",
          shuffle: gmid.shuffle,
          repeat: gmid.repeat,
          position: gmid.position,
          duration: gmid.duration || null,
        }
      : media,
  );
  async function mediaDo(action: string) {
    if (ownPlayer) {
      if (gmid && action === "toggle") gmid = { ...gmid, playing: !gmid.playing };
      if (gmid && action.startsWith("seek:")) ((gmid = { ...gmid, position: Number(action.slice(5)) }), (gmidAt = Date.now()));
      await emitAll("gmid-player-cmd", { action });
      return;
    }
    if (media && action === "toggle") media = { ...media, playing: !media.playing };
    if (media && action.startsWith("seek:")) ((media = { ...media, position: Number(action.slice(5)) }), (mediaAt = Date.now()));
    await api.media.control(action).catch((e) => flash(String(e)));
    setTimeout(loadMedia, action === "next" || action === "prev" ? 700 : 250);
  }
  const mediaPos = $derived(
    np?.position == null
      ? 0
      : Math.min(np.duration ?? Infinity, np.position + (np.playing ? Math.max(0, now - (ownPlayer ? gmidAt : mediaAt)) / 1000 : 0)),
  );
  /** Çalan programın ses karıştırıcısındaki karşılığı (ör. Spotify.exe, Chrome; GMID'in kendi oynatıcısı). */
  const mediaApp = $derived.by(() => {
    if (!media || ownPlayer) return null;
    const src = media.source.toLowerCase();
    const stem = (a: AppVolume) => a.exe.toLowerCase().replace(/\.exe$/, "");
    const key = src.replace(/\.exe$/, "").split(/[!\\._]/)[0];
    return (
      mixApps.find((a) => a.pid && stem(a) && src.includes(stem(a))) ??
      (key ? mixApps.find((a) => a.pid && stem(a).includes(key)) : undefined) ??
      null
    );
  });
  function setMediaVol(level: number) {
    const a = mediaApp;
    if (!a) return;
    mixApps = mixApps.map((x) => (x.pid === a.pid ? { ...x, level } : x));
    void api.audio.setAppVolume(a.pid, level).catch(() => {});
  }
  const mmss = (t: number) => `${Math.floor(t / 60)}:${String(Math.floor(t % 60)).padStart(2, "0")}`;

  function startPerf() {
    stopPerf();
    const tick = () => void api.panel.perf().then((p) => (perf = p)).catch(() => {});
    tick();
    perfTimer = setInterval(tick, 1000);
    loadMedia();
    mediaTimer = setInterval(loadMedia, 2000);
  }
  function stopPerf() {
    clearInterval(perfTimer);
    perfTimer = undefined;
    clearInterval(mediaTimer);
    mediaTimer = undefined;
  }

  const out = $derived(audio?.outputs.find((d) => d.isDefault) ?? null);
  const mic = $derived(audio?.inputs.find((d) => d.isDefault) ?? null);
  const noteTitle = $derived(game ? `🎮 ${game.name}` : "🎮 Oyun notları");

  async function refresh() {
    now = Date.now();
    const [a, games, g, t, r, c] = await Promise.all([
      api.audio.state().catch(() => null),
      api.games.list().catch(() => [] as Game[]),
      api.gamemode.status().catch(() => gm),
      api.timers.state().catch(() => null),
      api.reminders.list().catch(() => [] as Reminder[]),
      api.clips.list().catch(() => [] as Clip[]),
    ]);
    audio = a;
    game = games.find((x) => x.runningSince) ?? null;
    gm = g;
    countdowns = t?.countdowns ?? [];
    reminders = r.filter((x) => !x.done && x.dueAt > now - 3_600_000).sort((x, y) => x.dueAt - y.dueAt).slice(0, 3);
    clip = (game ? c.find((x) => x.gameId === game!.id) : null) ?? c[0] ?? null;
    await loadNote();
  }

  async function loadNote() {
    const list: NoteSummary[] = await api.notes.list().catch(() => []);
    const found = list.find((n) => n.title === noteTitle);
    noteId = found?.id ?? null;
    const body = found ? ((await api.notes.get(found.id).catch(() => null))?.body ?? "") : "";
    // Yazarken panel tazelenirse yazılan kaybolmasın.
    if (!noteSaved) return;
    noteText = body;
    noteSaved = true;
    noteLoads++;
  }

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  function onNote(v: string) {
    noteText = v;
    noteSaved = false;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(saveNote, 700);
  }
  async function saveNote() {
    if (noteSaved) return;
    if (!noteId && !noteText.trim()) return;
    const n = await api.notes
      .save({ id: noteId, title: noteTitle, body: noteText, color: "default", font: "inter", pinned: false })
      .catch(() => null);
    if (n) {
      noteId = n.id;
      noteSaved = true;
    }
  }

  function close() {
    void saveNote();
    void api.panel.hide();
  }

  async function run(p: Promise<unknown>, text?: string) {
    try {
      await p;
      if (text) flash(text);
    } catch (e) {
      flash(String(e));
    }
    audio = await api.audio.state().catch(() => audio);
  }
  let flashTimer: ReturnType<typeof setTimeout> | undefined;
  function flash(text: string) {
    msg = text;
    clearTimeout(flashTimer);
    flashTimer = setTimeout(() => (msg = ""), 2200);
  }

  // Favori takımların canlı / bugünkü maçları (Futbol ekranındaki ☆). "Oyun içi panelde skor" kapalıysa gizli.
  let fbShow = $state(false);
  let fbMatches = $state<FootballMatch[]>([]);
  async function loadFootball() {
    const r = await api.football.live().catch(() => null);
    fbShow = !!r?.show;
    fbMatches = r?.matches ?? [];
  }
  async function hideFootball() {
    fbShow = false;
    await api.setSetting("panel.football", "false").catch(() => {});
    flash("Skor gizlendi · Futbol ekranından yeniden açabilirsin");
  }
  const fbTime = (iso: string) => new Date(iso).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });

  async function addTimer(min: number) {
    const s = await api.timers.add(`${min} dk mola`, min * 60_000).catch(() => null);
    if (s) countdowns = s.countdowns;
    flash(`${min} dakikalık sayaç başladı`);
  }

  /** Bayt/sn → "2,4 MB/s". */
  const rate = (b: number) => (b >= 1e6 ? `${(b / 1e6).toFixed(1).replace(".", ",")} MB/s` : `${Math.round(b / 1e3)} KB/s`);

  /** FPS yoksa nedeni (panelde görünür). */
  const fpsWhy = $derived.by(() => {
    if (!perf || perf.fps != null) return null;
    if (perf.fpsNote) return perf.fpsNote;
    switch (perf.fpsState) {
      case "missing":
        return "FPS ölçümü kapalı.";
      case "permission":
        return "FPS için bir kez Windows izni gerekiyor.";
      case "error":
        return "FPS ölçümü başlatılamadı.";
      default:
        return null;
    }
  });
  const fpsCanSetup = $derived(!!perf && perf.fps == null && ["missing", "permission", "error"].includes(perf.fpsState));

  async function setupFps() {
    fpsBusy = true;
    flash("FPS ölçümü kuruluyor · Windows izin isterse Evet de");
    try {
      const st = await api.fps.setup();
      flash(st.relogin ? "İzin verildi · Windows oturumunu bir kez kapatıp aç" : "FPS ölçümü açıldı");
    } catch (e) {
      flash(String(e));
    } finally {
      fpsBusy = false;
    }
  }

  async function makeBorderless() {
    if (!openInfo.exe) return;
    try {
      await api.borderless.set(openInfo.exe, true);
      openInfo = { ...openInfo, borderless: true };
      flash("Tamam · oyunu Pencereli moda al, GMID kenarlıksız tam ekran yapar");
    } catch (e) {
      flash(String(e));
    }
  }

  async function fit() {
    if (!clip) return;
    flash("Discord'a hazırlanıyor…");
    try {
      await api.clips.fitDiscord(clip.id);
      flash("Panoya kopyalandı · Discord'da Ctrl+V");
    } catch (e) {
      flash(String(e));
    }
  }

  onMount(() => {
    void refresh();
    void loadAlarms();
    void loadFootball();
    startPerf();
    const stops = [
      on<PanelOpen | null>("panel-open", (info) => {
        if (info) openInfo = info;
        void refresh();
        void loadFootball();
        startPerf();
      }),
      on<FootballMatch[]>("football-live", (list) => (fbMatches = list)),
      on("panel-close", () => stopPerf()),
      // Orta tuşa (ya da seçilen tuşa) basılınca panel fareyi aldı.
      on("panel-grabbed", () => (openInfo = { ...openInfo, mode: "interactive" })),
      on("panel-alarm", () => {
        alarmAuto = true;
        void loadAlarms();
      }),
      on("alarm-changed", () => void loadAlarms()),
      on<GameModeStatus>("gamemode", (s) => (gm = s)),
      on("audio-changed", async () => (audio = await api.audio.state().catch(() => audio))),
    ];
    const timer = setInterval(() => (now = Date.now()), 1000);
    const key = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("keydown", key);
    return () => {
      clearInterval(timer);
      stopPerf();
      window.removeEventListener("keydown", key);
      for (const s of stops) void s.then((f) => f());
    };
  });

  const cover = (g: Game) => (!g.cover ? null : /^(https?:|data:)/.test(g.cover) ? g.cover : api.fileSrc(g.cover));
  const timeStr = $derived(new Date(now).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" }));
  const remaining = (c: Countdown) => (c.endsAt ? Math.max(0, c.endsAt - now) : c.remainingMs);
  const pct = (v: number) => `${Math.round(v * 100)}`;
</script>

<div class="shade" role="presentation" onclick={(e) => e.target === e.currentTarget && close()}>
  <div class="wrap">
    <header class="top">
      <div class="brand"><span class="logo">GMID</span><span class="sub small">Oyun Paneli</span></div>
      {#if game}
        <div class="game">
          {#if cover(game)}<img src={cover(game)} alt="" />{/if}
          <div>
            <b>{game.name}</b>
            <span class="sub small">Bu oturum {playtime(Math.max(0, (now - (game.runningSince ?? now)) / 1000))}</span>
          </div>
        </div>
      {:else}
        <span class="sub">Şu an algılanan bir oyun yok</span>
      {/if}
      {#if perf}
        <div class="perf tnum" title={perf.gpuName ?? ""}>
          <span class:dim={perf.fps == null} title={perf.fps != null ? `${perf.fpsSource === "rtss" ? "RivaTuner" : "PresentMon"} · ${perf.fpsApp ?? ""}` : (fpsWhy ?? "Ölçülüyor…")}>
            <b>{perf.fps != null ? Math.round(perf.fps) : "—"}</b><i>FPS</i>
          </span>
          <span title={perf.cpuTemp != null ? "Sıcaklık ACPI sensöründen (yaklaşık)" : ""}>
            <b>{perf.cpu != null ? `%${Math.round(perf.cpu)}` : "—"}</b><i>CPU{perf.cpuMhz != null ? ` · ${(perf.cpuMhz / 1000).toFixed(1).replace(".", ",")} GHz` : ""}{perf.cpuTemp != null ? ` · ${Math.round(perf.cpuTemp)}°C` : ""}</i>
          </span>
          <span>
            <b>{perf.gpu != null ? `%${Math.round(perf.gpu)}` : "—"}</b><i>GPU{perf.gpuTemp != null ? ` · ${Math.round(perf.gpuTemp)}°C` : ""}</i>
          </span>
          {#if perf.ramUsed && perf.ramTotal}<span title="Kullanılan / toplam bellek"><b>{(perf.ramUsed / 1e9).toFixed(1).replace(".", ",")}</b><i>/ {Math.round(perf.ramTotal / 1e9)} GB RAM</i></span>{/if}
          {#if perf.vramUsed && perf.vramTotal}<span title="Ekran kartı belleği"><b>{(perf.vramUsed / 1e9).toFixed(1).replace(".", ",")}</b><i>/ {Math.round(perf.vramTotal / 1e9)} GB VRAM</i></span>{/if}
          {#if perf.disk != null}<span title="Disk etkinliği"><b>%{Math.round(perf.disk)}</b><i>DİSK</i></span>{/if}
          {#if perf.netRx != null}<span title="Ağ: indirme / yükleme"><b>{rate(perf.netRx)}</b><i>↓ · ↑ {rate(perf.netTx ?? 0)}</i></span>{/if}
        </div>
      {/if}
      <span class="clock tnum">{timeStr}</span>
      <button class="x" onclick={close} title="Kapat (Ctrl+Shift+G)"><Icon name="x" size={20} /></button>
    </header>

    <PowerWarn />
    {#each alarmReminders as r (r.id)}
      <div class="alarm">
        <span class="ring"><Icon name="bell" size={20} /></span>
        <div class="atx"><span class="kind">Hatırlatıcı</span><b>{r.title}</b>{#if r.note}<span class="sub small">{r.note}</span>{/if}</div>
        <button class="dev" onclick={() => alarmAct(api.reminders.snooze(r.id, 10))}>10 dk ertele</button>
        <button class="dev" onclick={() => alarmAct(api.reminders.snooze(r.id, 30))}>30 dk</button>
        <button class="dev on" onclick={() => alarmAct(api.reminders.complete(r.id))}>Kapat</button>
        <button class="dev ic" onclick={() => api.alarm.mute()} title="Melodiyi sustur" aria-label="Melodiyi sustur"><Icon name="mute" size={16} /></button>
      </div>
    {/each}
    {#each alarmTimers as t (t.id)}
      <div class="alarm">
        <span class="ring"><Icon name="timer" size={20} /></span>
        <div class="atx"><span class="kind">Süre doldu</span><b>{t.label}</b></div>
        <button class="dev" onclick={() => alarmAct(api.timers.toggle(t.id))}>Yeniden başlat</button>
        <button class="dev on" onclick={() => alarmAct(api.timers.remove(t.id))}>Kapat</button>
        <button class="dev ic" onclick={() => api.alarm.mute()} title="Melodiyi sustur" aria-label="Melodiyi sustur"><Icon name="mute" size={16} /></button>
      </div>
    {/each}

    {#if fpsWhy}
      <div class="hint fpsbar">
        <Icon name="bolt" size={16} />
        <span>{fpsWhy}</span>
        {#if fpsCanSetup}
          <button class="btn sm" disabled={fpsBusy} onclick={setupFps}>{fpsBusy ? "Kuruluyor…" : "FPS ölçümünü aç"}</button>
        {/if}
      </div>
    {/if}

    {#if openInfo.exclusive || openInfo.minimizes}
      <div class="hint warn">
        <Icon name="info" size={16} />
        <span>
          Oyun <b>tam ekran (özel)</b> modda: Windows bu modda hiçbir pencereyi oyunun üstünde göstermez, odak gidince oyun
          küçülür. Oyunun görüntü ayarından <b>Pencereli (Windowed)</b> seç: GMID onu kendiliğinden <b>kenarlıksız tam
          ekrana</b> çevirir, panel Steam'deki Shift+Tab gibi açılır.
        </span>
        {#if openInfo.exe && !openInfo.borderless}
          <button class="btn sm" onclick={makeBorderless}>Kenarlıksız yap</button>
        {:else if openInfo.borderless}
          <span class="ok"><Icon name="check" size={14} />Kenarlıksız listesinde</span>
        {/if}
      </div>
    {/if}

    {#if fbShow && fbMatches.length}
      <div class="fbbar">
        <span class="fbh"><Icon name="trophy" size={16} />Maç</span>
        <div class="fbl">
          {#each fbMatches as m (m.league + m.id)}
            <div class="fbm" class:live={m.state === "in"}>
              {#if m.home.team.logo}<img src={m.home.team.logo} alt="" />{/if}
              <b>{m.home.team.short}</b>
              <span class="fbs tnum">{m.home.score === null ? fbTime(m.date) : `${m.home.score} - ${m.away.score}`}</span>
              <b>{m.away.team.short}</b>
              {#if m.away.team.logo}<img src={m.away.team.logo} alt="" />{/if}
              {#if m.home.score !== null}<span class="fbst tnum">{#if m.state === "in"}<i class="dot"></i>{/if}{m.status}</span>{/if}
            </div>
          {/each}
        </div>
        <button class="x sm" onclick={hideFootball} title="Panelde skoru gizle"><Icon name="x" size={14} /></button>
      </div>
    {/if}

    <div class="grid">
      <section class="card">
        <h3><Icon name="speaker" size={17} />Ses</h3>
        {#if audio?.supported}
          <div class="devs">
            {#each audio.outputs as d (d.id)}
              <button class="dev" class:on={d.isDefault} onclick={() => run(api.audio.setDefault(d.id), `${d.name} seçildi`)}>{d.name}</button>
            {/each}
          </div>
          {#if out}
            <label class="slider">
              <button class="ib" onclick={() => run(api.audio.setMute(out.id, !out.muted))} title={out.muted ? "Sesi aç" : "Sessize al"}><Icon name={out.muted ? "mute" : "speaker"} size={16} /></button>
              <input type="range" min="0" max="1" step="0.01" value={out.level} oninput={(e) => run(api.audio.setVolume(out.id, Number(e.currentTarget.value)))} />
              <span class="tnum">{pct(out.level)}</span>
            </label>
          {/if}
        {:else}
          <p class="sub small">Ses ayarları yalnızca Windows'ta.</p>
        {/if}
      </section>

      <section class="card np" class:on={media?.playing}>
        <h3><Icon name="headphones" size={17} />Şimdi çalan{#if np}<span class="sub small app">{np.app}</span>{/if}</h3>
        {#if np}
          <div class="track">
            {#if np.thumb}<img src={np.thumb} alt="" />{:else}<span class="art"><Icon name="headphones" size={22} /></span>{/if}
            <div class="tt"><b title={np.title}>{np.title}</b><span class="sub small">{np.artist}</span></div>
          </div>
          {#if np.duration}
            <div class="mprog">
              <span class="tnum">{mmss(mediaPos)}</span>
              <input type="range" min="0" max={np.duration} step="1" value={mediaPos} onchange={(e) => mediaDo(`seek:${e.currentTarget.value}`)} aria-label="Konum" />
              <span class="tnum">{mmss(np.duration)}</span>
            </div>
          {/if}
          <div class="mctl">
            <button class="ib sm" class:on={np.shuffle} disabled={np.shuffle === null} onclick={() => mediaDo("shuffle")} title={np.shuffle === null ? "Bu program karıştırmayı desteklemiyor" : np.shuffle ? "Karıştırma açık" : "Karıştır"}><Icon name="shuffle" size={15} /></button>
            <button class="ib" onclick={() => mediaDo("prev")} title="Önceki"><Icon name="prev" size={16} /></button>
            <button class="ib big" onclick={() => mediaDo("toggle")} title={np.playing ? "Duraklat" : "Oynat"}><Icon name={np.playing ? "pause" : "play"} size={18} /></button>
            <button class="ib" onclick={() => mediaDo("next")} title="Sonraki"><Icon name="next" size={16} /></button>
            <button class="ib sm rep" class:on={np.repeat === "list" || np.repeat === "track"} disabled={np.repeat === null} onclick={() => mediaDo("repeat")} title={np.repeat === null ? "Bu program tekrarı desteklemiyor" : np.repeat === "track" ? "Bu parçayı tekrarla" : np.repeat === "list" ? "Listeyi tekrarla" : "Tekrar kapalı"}>
              <Icon name="repeat" size={15} />{#if np.repeat === "track"}<i>1</i>{/if}
            </button>
          </div>
          {#if ownPlayer && gmid}
            <label class="slider" title="GMID oynatıcısının sesi">
              <button class="ib" class:bad={gmid.muted} onclick={(e) => (e.preventDefault(), void emitAll("gmid-player-cmd", { action: "mute" }))} title={gmid.muted ? "Sesi aç" : "Sessize al"}><Icon name={gmid.muted ? "mute" : "speaker"} size={15} /></button>
              <input type="range" min="0" max="1" step="0.01" value={gmid.volume} oninput={(e) => void emitAll("gmid-player-cmd", { action: `volume:${e.currentTarget.value}` })} />
              <span class="tnum">{pct(gmid.volume)}</span>
            </label>
          {:else if mediaApp}
            <label class="slider" title="{mediaApp.name} ses seviyesi">
              <button class="ib" class:bad={mediaApp.muted} onclick={(e) => (e.preventDefault(), void api.audio.setAppMute(mediaApp.pid, !mediaApp.muted).then(loadMedia))} title={mediaApp.muted ? "Sesi aç" : "Sessize al"}><Icon name={mediaApp.muted ? "mute" : "speaker"} size={15} /></button>
              <input type="range" min="0" max="1" step="0.01" value={mediaApp.level} oninput={(e) => setMediaVol(Number(e.currentTarget.value))} />
              <span class="tnum">{pct(mediaApp.level)}</span>
            </label>
          {/if}
        {:else}
          <p class="sub small">Spotify, YouTube ya da başka bir oynatıcıda bir şey çalınca burada görünür; oyundan çıkmadan durdur, geç.</p>
        {/if}
      </section>

      <section class="card wide">
        <h3><Icon name="headphones" size={17} />Uygulamalar</h3>
        <AppMixer compact max={6} />
      </section>
      <section class="card">
        <h3><Icon name="mic" size={17} />Mikrofon</h3>
        {#if mic}
          <label class="slider">
            <button class="ib" class:bad={mic.muted} onclick={() => run(api.audio.setMute(mic.id, !mic.muted), mic.muted ? "Mikrofon açık" : "Mikrofon kapalı")} title="Sessize al / aç"><Icon name={mic.muted ? "micoff" : "mic"} size={16} /></button>
            <input type="range" min="0" max="1" step="0.01" value={mic.level} oninput={(e) => run(api.audio.setVolume(mic.id, Number(e.currentTarget.value)))} />
            <span class="tnum">{pct(mic.level)}</span>
          </label>
          <button class="tog" class:on={audio?.lock != null} onclick={() => run(api.audio.setLock(audio?.lock != null ? null : 1), audio?.lock != null ? "Kilit kapalı" : "Seviye kilitlendi")}>
            <Icon name="lock" size={15} />{audio?.lock != null ? `Kilitli · %${pct(audio.lock)}` : "Seviyeyi kilitle"}
          </button>
        {:else}
          <p class="sub small">Mikrofon bulunamadı.</p>
        {/if}
      </section>

      <section class="card" class:lit={gm.active}>
        <h3><Icon name="bolt" size={17} />Oyun Modu</h3>
        <p class="sub small">{gm.active ? "Açık: yüksek performans, sessiz, kilitli mikrofon." : "Kapalı. Açınca oyuna göre ayarlanır, kapatınca geri alınır."}</p>
        <button class="tog big" class:on={gm.active} onclick={async () => (gm = await api.gamemode.set(!gm.active).catch(() => gm))}>
          <Icon name="bolt" size={16} />{gm.active ? "Kapat" : "Aç"}
        </button>
      </section>

      <section class="card ping {ping && !pinging ? ping.verdict.level : ''}">
        <h3><Icon name="network" size={17} />Ping
          <button class="dev on mini" disabled={pinging} onclick={runPing}>{pinging ? "Ölçülüyor…" : ping ? "Yenile" : "Ölç"}</button>
        </h3>
        {#if ping}
          <b class="verdict" class:fade={pinging}>{ping.verdict.title}</b>
          {#each summary(ping.probes) as p (p.id)}
            <div class="cd" class:fade={pinging}><span>{p.label}</span><b class="tnum {tone(p)}">{msText(p.avg)} ms{p.loss ? ` · %${p.loss}` : ""}</b></div>
          {/each}
        {:else}
          <p class="sub small">{pinging ? "Modem, internet ve sunuculara ping atılıyor…" : "Lag sende mi, sunucuda mı? Tek tıkla ölç (~1 sn)."}</p>
        {/if}
      </section>

      <section class="card">
        <h3><Icon name="timer" size={17} />Sayaç</h3>
        {#each countdowns.slice(0, 3) as c (c.id)}
          <div class="cd"><span>{c.label}</span><b class="tnum">{c.finished ? "Bitti" : clock(remaining(c))}</b></div>
        {:else}
          <p class="sub small">Çalışan sayaç yok.</p>
        {/each}
        <div class="quick">
          {#each [5, 15, 30] as m (m)}<button class="dev" onclick={() => addTimer(m)}>{m} dk</button>{/each}
        </div>
      </section>

      <section class="card wide">
        <h3><Icon name="note" size={17} />{game ? `${game.name} notu` : "Not"}<span class="sub small saved">{noteSaved ? "kaydedildi" : "yazılıyor…"}</span></h3>
        <!-- Zengin not: ekran görüntüsü Ctrl+V ile yapıştırılır (Notlar ekranındakiyle aynı düzenleyici). -->
        <div class="pnote" role="presentation" onpointerdown={() => api.panel.focus()} onfocusin={() => api.panel.focus()} onfocusout={saveNote}>
          <NoteEditor docKey={`${noteTitle}#${noteLoads}`} value={noteText} toolbar={false} placeholder="Build, kod, nerede kaldın… Ekran görüntüsünü Ctrl+V ile yapıştır (Notlar'da da görünür)" onchange={onNote} />
        </div>
      </section>

      <section class="card">
        <h3><Icon name="clip" size={17} />Son klip</h3>
        {#if clip}
          <div class="clip">
            {#if clip.thumb}<img src={api.fileSrc(clip.thumb)} alt="" />{/if}
            <span class="small">{clip.title ?? clip.name}</span>
          </div>
          <div class="quick">
            {#if clip.kind === "clip"}<button class="dev discord" onclick={fit}>Discord'a sığdır</button>{/if}
            <button class="dev" onclick={() => api.clips.reveal(clip!.id)}>Klasör</button>
          </div>
        {:else}
          <p class="sub small">Henüz klip yok. NVIDIA'da Alt+F10, Xbox Game Bar'da Win+Alt+G.</p>
        {/if}
      </section>

      <section class="card">
        <h3><Icon name="bell" size={17} />Hatırlatıcılar</h3>
        {#each reminders as r (r.id)}
          <div class="cd"><span>{r.title}</span><b class="tnum">{new Date(r.dueAt).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" })}</b></div>
        {:else}
          <p class="sub small">Yaklaşan hatırlatıcı yok.</p>
        {/each}
      </section>
    </div>
    {#if msg}<div class="msg">{msg}</div>{/if}
    <p class="hint sub small">Esc ya da Ctrl+Shift+G ile kapat · boş yere tıklamak da kapatır</p>
  </div>
</div>

<style>
  :global(html), :global(body) { background: transparent !important; overflow: hidden; }
  .shade { position: fixed; inset: 0; background: rgba(6, 7, 9, 0.62); display: grid; place-items: center; padding: 32px; animation: fade 0.16s ease-out; }
  .wrap { width: min(1180px, 100%); display: flex; flex-direction: column; gap: 16px; }
  .top { display: flex; align-items: center; gap: 18px; padding: 12px 16px; border-radius: 16px; background: rgba(18, 20, 23, 0.92); border: 1px solid rgba(227, 194, 112, 0.35); }
  .brand { display: flex; flex-direction: column; }
  .logo { font-family: var(--f-display); font-weight: 800; color: var(--gold); letter-spacing: 0.08em; }
  .game { flex: 1; display: flex; align-items: center; gap: 12px; min-width: 0; }
  .game img { width: 32px; height: 48px; object-fit: cover; border-radius: 6px; }
  .game div { display: flex; flex-direction: column; min-width: 0; }
  .game b { font-size: 16px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .top > .sub { flex: 1; }
  .perf { display: flex; gap: 16px; padding: 4px 14px; border-left: 1px solid var(--line-2); border-right: 1px solid var(--line-2); }
  .perf span { display: flex; flex-direction: column; align-items: center; line-height: 1.1; }
  .perf b { font-family: var(--f-display); font-size: 18px; color: var(--text); }
  .perf i { font-style: normal; font-size: 10.5px; color: var(--muted); letter-spacing: 0.04em; }
  .perf .dim b { color: var(--dim); }
  .clock { font-family: var(--f-display); font-size: 22px; font-weight: 700; color: var(--gold-glow); }
  .x { border: 0; background: rgba(255, 255, 255, 0.06); color: var(--muted); padding: 8px; border-radius: 10px; display: grid; }
  .x:hover { color: var(--text); background: rgba(255, 255, 255, 0.12); }
  .hint { display: flex; align-items: center; gap: 10px; padding: 10px 14px; border-radius: 12px; background: rgba(18, 20, 23, 0.94); border: 1px solid var(--line-2); color: var(--muted); font-size: 13px; }
  .hint span { flex: 1; }
  div.hint { text-align: left; }
  /* Çalan uyarı göze çarpsın: çerçeve ve ışıltı kırmızı ile GMID sarısı arasında hızlı, yumuşak gidip gelir. */
  .alarm {
    display: flex; align-items: center; gap: 12px; padding: 12px 16px; border-radius: 16px; background: rgba(18, 20, 23, 0.96);
    border: 2px solid var(--gold); box-shadow: 0 0 30px rgba(227, 194, 112, 0.25);
    animation: fade 0.25s ease-out, alarmPulse 0.9s ease-in-out infinite alternate;
  }
  .alarm .ring { animation: alarmRing 0.9s ease-in-out infinite alternate; }
  @keyframes alarmPulse {
    from { border-color: var(--gold); box-shadow: 0 0 26px rgba(227, 194, 112, 0.35); background: rgba(30, 27, 18, 0.96); }
    to { border-color: #e06a6a; box-shadow: 0 0 34px rgba(224, 106, 106, 0.5); background: rgba(34, 18, 18, 0.96); }
  }
  @keyframes alarmRing { from { color: var(--gold); background: rgba(227, 194, 112, 0.16); } to { color: #ff8a8a; background: rgba(224, 106, 106, 0.2); } }
  @media (prefers-reduced-motion: reduce) { .alarm, .alarm .ring { animation: none; border-color: #e06a6a; } }
  /* Düğmeler aynı yükseklikte ve ortalı; yalnız simgeli "sustur" kare. */
  .alarm .dev { height: 34px; display: inline-flex; align-items: center; justify-content: center; flex: none; }
  .alarm .dev.ic { width: 34px; padding: 0; }
  .alarm .ring { display: grid; place-items: center; width: 40px; height: 40px; border-radius: 50%; color: var(--gold); background: rgba(227, 194, 112, 0.14); flex: none; }
  .atx { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .atx .kind { font-size: 11px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; color: var(--gold-dim); }
  .atx b { font-size: 17px; }
  .hint.warn { border-color: rgba(227, 194, 112, 0.45); }
  .hint.warn > :global(svg), .fpsbar > :global(svg) { color: var(--gold); flex: none; }
  .hint .ok { flex: none; display: inline-flex; align-items: center; gap: 5px; color: var(--good); font-weight: 600; }
  .hint b { color: var(--text); }
  .grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 14px; }
  .fbbar { display: flex; align-items: center; gap: 14px; padding: 8px 10px 8px 16px; border-radius: 14px; background: rgba(18, 20, 23, 0.94); border: 1px solid var(--line-2); }
  .fbh { display: inline-flex; align-items: center; gap: 7px; font-weight: 700; font-size: 14px; color: var(--gold-glow); flex: none; }
  .fbl { flex: 1; display: flex; flex-wrap: wrap; gap: 8px; min-width: 0; }
  .fbm { display: inline-flex; align-items: center; gap: 8px; padding: 5px 12px; border-radius: 10px; background: rgba(255, 255, 255, 0.04); border: 1px solid transparent; font-size: 13.5px; }
  .fbm.live { border-color: rgba(224, 106, 106, 0.45); }
  .fbm img { width: 20px; height: 20px; object-fit: contain; }
  .fbm b { font-weight: 600; }
  .fbs { padding: 2px 8px; border-radius: 7px; background: rgba(0, 0, 0, 0.35); font-weight: 800; font-size: 15px; }
  .fbm.live .fbs { color: #ffd2d2; }
  .fbst { display: inline-flex; align-items: center; gap: 5px; font-size: 12px; font-weight: 700; color: var(--dim); }
  .fbm.live .fbst { color: #ff8a8a; }
  .fbst .dot { width: 7px; height: 7px; border-radius: 50%; background: #ff5a5a; }
  .x.sm { padding: 6px; flex: none; }
  .card { display: flex; flex-direction: column; gap: 10px; padding: 14px 16px; border-radius: 16px; background: rgba(18, 20, 23, 0.94); border: 1px solid var(--line-2); min-width: 0; }
  .card.wide { grid-column: span 2; }
  .card.lit { border-color: var(--gold); box-shadow: 0 0 24px rgba(227, 194, 112, 0.18); }
  h3 { display: flex; align-items: center; gap: 8px; margin: 0; font-size: 14px; font-weight: 700; color: var(--gold-glow); }
  .saved { margin-left: auto; font-weight: 500; }
  .devs, .quick { display: flex; flex-wrap: wrap; gap: 6px; }
  .dev { border: 1px solid var(--line-2); background: transparent; color: var(--text); border-radius: 9px; padding: 6px 10px; font-size: 13px; font-weight: 600; max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dev:hover { border-color: var(--gold-dim); }
  .dev.on { background: rgba(227, 194, 112, 0.16); border-color: var(--gold); color: var(--gold-glow); }
  .dev.discord { background: #5865f2; border-color: #5865f2; color: #fff; }
  .slider { display: grid; grid-template-columns: auto minmax(0, 1fr) 30px; align-items: center; gap: 8px; }
  .slider input { accent-color: var(--gold); width: 100%; }
  .slider span { text-align: right; font-size: 13px; color: var(--muted); }
  .ib { border: 0; background: rgba(255, 255, 255, 0.06); color: var(--text); border-radius: 8px; padding: 6px; display: grid; }
  .ib.bad { color: #fff; background: var(--bad); }
  .tog { display: inline-flex; align-items: center; justify-content: center; gap: 6px; border: 1px solid var(--line-2); background: transparent; color: var(--text); border-radius: 10px; padding: 8px 12px; font-weight: 700; font-size: 13px; }
  .tog.on { background: var(--gold); border-color: var(--gold); color: #1b1608; }
  .tog.big { padding: 12px; font-size: 15px; }
  .cd { display: flex; justify-content: space-between; gap: 8px; font-size: 13.5px; }
  .np .app { margin-left: auto; font-weight: 600; }
  .np.on { border-color: rgba(92, 192, 138, 0.5); }
  .track { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .track img, .track .art { width: 52px; height: 52px; border-radius: 10px; object-fit: cover; flex: none; }
  .track .art { display: grid; place-items: center; background: var(--coal-2); color: var(--dim); }
  .tt { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .tt b, .tt span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mctl { display: flex; justify-content: center; align-items: center; gap: 10px; }
  .mctl .ib.sm { color: var(--muted); background: transparent; position: relative; }
  .mctl .ib.sm.on { color: var(--gold); }
  .mctl .ib.sm:disabled { opacity: 0.3; }
  .mctl .rep i { position: absolute; right: 1px; bottom: 0; font: 700 9px var(--f-body); font-style: normal; color: var(--gold); }
  .mprog { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; align-items: center; gap: 8px; font-size: 11.5px; color: var(--muted); }
  .mprog input { accent-color: var(--gold); width: 100%; }
  .mctl .ib.big { place-items: center; width: 42px; height: 42px; border-radius: 50%; background: var(--gold); color: #1b1608; }
  .ping h3 .mini { margin-left: auto; padding: 4px 10px; font-size: 12px; }
  .ping.good { border-color: rgba(92, 192, 138, 0.55); }
  .ping.warn { border-color: rgba(227, 194, 112, 0.6); }
  .ping.bad { border-color: rgba(224, 106, 106, 0.65); }
  .ping .verdict { font-size: 13.5px; }
  .ping.good .verdict { color: var(--good); }
  .ping.warn .verdict { color: var(--gold); }
  .ping.bad .verdict { color: var(--bad); }
  .ping .cd b.good { color: var(--good); }
  .ping .cd b.warn { color: var(--gold); }
  .ping .cd b.bad { color: var(--bad); }
  .ping .fade { opacity: 0.45; }
  .cd span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pnote { flex: 1; min-height: 120px; max-height: 340px; display: flex; flex-direction: column; background: rgba(255, 255, 255, 0.04); border: 1px solid var(--line); border-radius: 10px; padding: 10px; font-size: 14px; cursor: text; }
  .pnote :global(.editor) { font-size: 14px; }
  .pnote :global(.editor img) { max-height: 220px; }
  .clip { display: flex; flex-direction: column; gap: 6px; }
  .clip img { width: 100%; aspect-ratio: 16 / 9; object-fit: cover; border-radius: 10px; }
  .clip span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .msg { align-self: center; padding: 8px 16px; border-radius: 999px; background: rgba(18, 20, 23, 0.96); border: 1px solid var(--gold-dim); font-weight: 600; }
  .hint { text-align: center; margin: 0; }
  @keyframes fade { from { opacity: 0; } }
  @media (max-width: 1100px) {
    .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
</style>
