<script lang="ts">
  // Oyun İçi Panel (Ctrl+Shift+G): oyunun üstünde yarı saydam katman. Ses, mikrofon, Oyun Modu, sayaç,
  // bu oyuna not, son klip, yaklaşan hatırlatıcılar. Esc ya da aynı tuş kapatır; odak oyuna döner.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import AppMixer from "../components/AppMixer.svelte";
  import Icon from "../Icon.svelte";
  import { clock, playtime } from "../time";
  import type { AudioState, Clip, Countdown, Game, GameModeStatus, NoteSummary, PerfSample, Reminder } from "../types";

  let audio = $state<AudioState | null>(null);
  let game = $state<Game | null>(null);
  let gm: GameModeStatus = $state({ active: false, reason: null, since: null, quiet: true, stopBackground: true });
  let countdowns: Countdown[] = $state([]);
  let reminders: Reminder[] = $state([]);
  let clip = $state<Clip | null>(null);
  let noteId: string | null = $state(null);
  let noteText = $state("");
  let noteSaved = $state(true);
  let now = $state(Date.now());
  let msg = $state("");
  let perf = $state<PerfSample | null>(null);
  /** interactive: panel odakta (Shift+Tab gibi) · glance: oyun önde, panel yalnız gösteriyor. */
  let openInfo = $state<{ mode: "interactive" | "glance"; exclusive: boolean; minimizes: boolean }>({ mode: "interactive", exclusive: false, minimizes: false });
  let perfTimer: ReturnType<typeof setInterval> | undefined;

  // Ölçüm yalnız panel açıkken (saniyede bir).
  function startPerf() {
    stopPerf();
    const tick = () => void api.panel.perf().then((p) => (perf = p)).catch(() => {});
    tick();
    perfTimer = setInterval(tick, 1000);
  }
  function stopPerf() {
    clearInterval(perfTimer);
    perfTimer = undefined;
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
    noteText = found ? ((await api.notes.get(found.id).catch(() => null))?.body ?? "") : "";
    noteSaved = true;
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

  async function addTimer(min: number) {
    const s = await api.timers.add(`${min} dk mola`, min * 60_000).catch(() => null);
    if (s) countdowns = s.countdowns;
    flash(`${min} dakikalık sayaç başladı`);
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
    startPerf();
    const stops = [
      on<{ mode: "interactive" | "glance"; exclusive: boolean; minimizes: boolean } | null>("panel-open", (info) => {
        if (info) openInfo = info;
        void refresh();
        startPerf();
      }),
      on("panel-close", () => stopPerf()),
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
          <span class:dim={perf.fps == null} title={perf.fps != null ? (perf.fpsSource === "rtss" ? `RivaTuner · ${perf.fpsApp ?? ""}` : "PresentMon") : perf.fpsState === "missing" ? "FPS ölçümü kapalı: GMID → Oyun Modu → FPS ölçümünü aç" : perf.fpsState === "permission" ? "FPS için izin gerekiyor: Oyun Modu → FPS ölçümünü aç (sonra Windows oturumunu yenile)" : "Ölçülüyor…"}>
            <b>{perf.fps != null ? Math.round(perf.fps) : "—"}</b><i>FPS</i>
          </span>
          <span><b>{perf.cpu != null ? `%${Math.round(perf.cpu)}` : "—"}</b><i>CPU</i></span>
          <span>
            <b>{perf.gpu != null ? `%${Math.round(perf.gpu)}` : "—"}</b><i>GPU{perf.gpuTemp != null ? ` · ${Math.round(perf.gpuTemp)}°C` : ""}</i>
          </span>
          {#if perf.ramUsed && perf.ramTotal}<span><b>{(perf.ramUsed / 1e9).toFixed(1).replace(".", ",")}</b><i>/ {Math.round(perf.ramTotal / 1e9)} GB RAM</i></span>{/if}
        </div>
      {/if}
      <span class="clock tnum">{timeStr}</span>
      <button class="x" onclick={close} title="Kapat (Ctrl+Shift+G)"><Icon name="x" size={20} /></button>
    </header>

    {#if openInfo.mode === "glance"}
      <div class="hint">
        <Icon name="info" size={16} />
        <span>
          {#if openInfo.exclusive}
            Oyun <b>özel tam ekranda</b>: Windows bu kipte paneli oyunun üstünde göstermeyebilir. Oyunun görüntü ayarında
            <b>Kenarlıksız / Windowed Fullscreen</b> seçersen Steam'deki Shift+Tab gibi çalışır.
          {:else if openInfo.minimizes}
            Bu oyun odak gidince kendini küçülttüğü için panel oyunu bozmadan açıldı; fare oyunda kalır.
          {:else}
            Oyun önde, fare oyunda.
          {/if}
        </span>
        <button class="btn sm" onclick={() => (api.panel.interact(), (openInfo = { ...openInfo, mode: "interactive" }))}>
          <Icon name="arrow" size={14} />Fareyi serbest bırak
        </button>
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
        <textarea value={noteText} onpointerdown={() => api.panel.focus()} onfocus={() => api.panel.focus()} oninput={(e) => onNote(e.currentTarget.value)} onblur={saveNote} placeholder="Build, kod, nerede kaldın… (Notlar'da da görünür)"></textarea>
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
  .hint b { color: var(--text); }
  .grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 14px; }
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
  .cd span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  textarea { flex: 1; min-height: 120px; resize: none; background: rgba(255, 255, 255, 0.04); border: 1px solid var(--line); border-radius: 10px; padding: 10px; color: var(--text); font: inherit; font-size: 14px; }
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
