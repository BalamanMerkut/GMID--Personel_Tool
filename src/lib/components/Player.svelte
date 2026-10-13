<script lang="ts">
  // Uygulama içi oynatıcı. İki görünüm:
  //  • Sahne: solda video (müzikte kapak), sağda zamanla akan yazılar.
  //  • Liste: solda sıradaki şarkılar/videolar, sağda yazılar (video varsa listenin üstünde küçük oynar).
  // Altta her zaman çalma çubuğu: oynat/duraklat, önceki/sonraki, karıştır, tekrar, ilerleme, hız, ses, tam ekran.
  // O an söylenen satır altınla işaretlenir; kelime zamanları varsa kelime kelime (karaoke gibi).
  // Küçültülünce (mini) uygulamanın altında çalmaya devam eder; video köşede küçük oynar.
  // Filmlerde videonun yanındaki altyazılar (.tr.vtt…) seçilip görüntünün üstünde gösterilir.
  import { onMount, tick, untrack } from "svelte";
  import { api, inTauri } from "../api";
  import { emitAll, on } from "../bus";
  import { formatDuration, siteLabel } from "../downloads.svelte";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { fill, wordsOf } from "../karaoke";
  import Karaoke from "./Karaoke.svelte";
  import { countPlay } from "../library.svelte";
  import { controls, nav, player } from "../player.svelte";
  import { notify } from "../toast.svelte";
  import type { Download, GmidPlayerState, Segment, Sidecar, Transcript, TranscriptProgress, WhisperStatus } from "../types";

  let {
    item,
    queue,
    mini = false,
    onclose,
    onexpand,
    onstop,
    onpick,
  }: {
    item: Download;
    queue: Download[];
    /** Küçültülmüş: uygulamanın altında çalma çubuğu. */
    mini?: boolean;
    /** Küçült (çalmaya devam eder). */
    onclose: () => void;
    onexpand: () => void;
    /** Çalmayı bitir ve kapat. */
    onstop: () => void;
    onpick: (d: Download) => void;
  } = $props();

  type Repeat = "off" | "all" | "one";
  const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2];

  let media: HTMLMediaElement | undefined = $state();
  let lines: HTMLDivElement | undefined = $state();
  let queueEl: HTMLDivElement | undefined = $state();
  let time = $state(0);
  let duration = $state(0);
  let playing = $state(false);
  let playable = $state(true);
  let seeking = $state(false);
  let transcript: Transcript | null = $state(null);
  let loading = $state(true);
  let creating: TranscriptProgress | null = $state(null);
  let error: string | null = $state(null);
  let whisper: WhisperStatus | null = $state(null);
  let installing = $state(false);
  let installPct = $state(0);
  let mode = $state("auto");
  let lang = $state("orig");
  let big = $state(getJson("player.bigLyrics", false));
  let showText = $state(getJson("player.showText", true));
  let autoNext = $state(getJson("player.autoNext", true));
  let view: "stage" | "list" = $state(getJson<"stage" | "list">("player.view", "stage"));
  let repeat: Repeat = $state(getJson<Repeat>("player.repeat", "off"));
  let shuffle = $state(getJson("player.shuffle", false));
  let volume = $state(getJson("player.volume", 1));
  let muted = $state(false);
  let speed = $state(1);
  let menuOpen = $state(false);
  let full = $state(false);
  let idle = $state(false);
  /** Filmin altyazıları ve seçilen ("off" | "transcript" | dosya adı). */
  let tracks: Sidecar[] = $state([]);
  let sub = $state("off");
  let subSegs: Segment[] = $state([]);
  let ccOpen = $state(false);
  /** NVIDIA hızlandırma kurulu değilse Whisper beklerken öneri gösterilir. */
  let gpuMissing = $state(false);
  /** Karıştırırken "önceki" düğmesi için çalınanların sırası. */
  let history: string[] = [];
  /** Kullanıcı yazıları elle kaydırdıysa bir süre otomatik kaydırma yapılmaz. */
  let userScrolledAt = 0;

  const list = $derived(queue.filter((d) => d.status === "done" && d.filePath));
  const index = $derived(list.findIndex((d) => d.id === item.id));
  const segments = $derived.by(() => (transcript as Transcript | null)?.segments ?? []);
  const total = $derived(duration || item.duration || 0);
  const active = $derived.by(() => {
    // Başlangıcı geçmiş son satır (ikili arama).
    let lo = 0;
    let hi = segments.length - 1;
    let found = -1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (segments[mid].start <= time + 0.05) {
        found = mid;
        lo = mid + 1;
      } else hi = mid - 1;
    }
    return found;
  });
  const shownView: "stage" | "list" = $derived(full || mini ? "stage" : view);
  /** Görüntünün üstündeki altyazı satırı. */
  const cue = $derived.by(() => {
    const segs = sub === "transcript" ? segments : subSegs;
    if (sub === "off" || !segs.length) return "";
    let lo = 0;
    let hi = segs.length - 1;
    let found = -1;
    while (lo <= hi) {
      const mid = (lo + hi) >> 1;
      if (segs[mid].start <= time) {
        found = mid;
        lo = mid + 1;
      } else hi = mid - 1;
    }
    return found >= 0 && time <= segs[found].end ? segs[found].text : "";
  });
  const hideStage = $derived(shownView === "list" && item.audioOnly && showText);

  function nextItem(): Download | null {
    if (!list.length) return null;
    if (shuffle && list.length > 1) {
      const recent = history.slice(-Math.floor(list.length / 2));
      const pool = list.filter((d) => d.id !== item.id && !recent.includes(d.id));
      const from = pool.length ? pool : list.filter((d) => d.id !== item.id);
      return from[Math.floor(Math.random() * from.length)];
    }
    if (index >= 0 && index < list.length - 1) return list[index + 1];
    return repeat === "all" ? list[0] : null;
  }
  function prevItem(): Download | null {
    if (shuffle) {
      const id = history[history.length - 2];
      return list.find((d) => d.id === id) ?? null;
    }
    if (index > 0) return list[index - 1];
    return repeat === "all" && list.length ? list[list.length - 1] : null;
  }
  const hasNext = $derived(shuffle ? list.length > 1 : index < list.length - 1 || (repeat === "all" && list.length > 1));
  const hasPrev = $derived(index > 0 || shuffle || repeat === "all");

  function goNext() {
    const n = nextItem();
    if (n) onpick(n);
  }
  function goPrev() {
    // Parçanın başından 3 saniye geçtiyse önce başa sar (alışılmış davranış).
    if (time > 3) {
      seekTo(0);
      return;
    }
    const p = prevItem();
    if (!p) return;
    if (shuffle) history = history.slice(0, -2);
    onpick(p);
  }

  async function load() {
    loading = true;
    error = null;
    creating = null;
    playable = true;
    time = 0;
    duration = 0;
    if (history[history.length - 1] !== item.id) history = [...history, item.id].slice(-200);
    try {
      transcript = await api.transcripts.get(item.id);
    } catch {
      transcript = null;
    }
    loading = false;
    void loadTracks();
    void api.dictation
      .status()
      .then((s) => (gpuMissing = s.supported && !s.gpuInstalled))
      .catch(() => {});
    whisper = await api.transcripts.whisperStatus().catch(() => null);
    void tick().then(() =>
      queueEl?.querySelector<HTMLElement>(`[data-id="${item.id}"]`)?.scrollIntoView({ block: "nearest" }),
    );
  }

  // Filmin altyazıları: tercih edilen dil (varsayılan Türkçe) kendiliğinden açılır.
  async function loadTracks() {
    tracks = [];
    subSegs = [];
    sub = "off";
    if (item.audioOnly) return;
    const id = item.id;
    const found = await api.videos.tracks(id).catch(() => [] as Sidecar[]);
    if (id !== item.id) return;
    tracks = found;
    const pref = getJson<string>("player.subLang", "tr");
    const pick = pref === "off" ? undefined : found.find((t) => t.lang.split(/[-_]/)[0].replace(/^tur$/, "tr").replace(/^eng$/, "en") === pref);
    if (pick) await chooseSub(pick.file, false);
  }

  async function chooseSub(value: string, remember = true) {
    ccOpen = false;
    sub = value;
    subSegs = [];
    if (value !== "off" && value !== "transcript") {
      const id = item.id;
      const segs = await api.videos.track(id, value).catch(() => [] as Segment[]);
      if (id === item.id && sub === value) subSegs = segs;
    }
    if (remember) {
      const t = tracks.find((x) => x.file === value);
      void setJson("player.subLang", value === "off" ? "off" : (t?.lang.split(/[-_]/)[0] ?? "tr"));
    }
  }

  // Video değişince her şeyi baştan yükle.
  let loadedId = "";
  $effect(() => {
    if (item.id !== loadedId) {
      loadedId = item.id;
      void load();
    }
  });

  // Ses, sessiz ve tekrar ayarları her zaman oynatıcıya yansısın (yeni parça dahil).
  $effect(() => {
    if (!media) return;
    media.volume = Math.min(1, Math.max(0, volume));
    media.muted = muted;
    media.loop = repeat === "one";
  });
  function applySpeed() {
    if (!media) return;
    media.defaultPlaybackRate = speed;
    media.playbackRate = speed;
  }

  // Oynarken zamanı kare kare takip et (kelime işaretleme akıcı olsun).
  let raf = 0;
  function frame() {
    if (media && playable && !seeking) time = media.currentTime;
    if (playing) raf = requestAnimationFrame(frame);
  }
  $effect(() => {
    if (playing) raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  });

  // Tarayıcıda (geliştirme) dosya oynamaz: yazıların akışını görmek için sahte saat.
  let demo: ReturnType<typeof setInterval> | undefined;
  function startDemo() {
    if (inTauri || demo) return;
    playing = true;
    duration = item.duration ?? 240;
    demo = setInterval(() => {
      if (playing && !seeking) time = Math.min(duration, time + 0.1 * speed);
    }, 100);
  }

  // Etkin satırı ortada tut.
  $effect(() => {
    const i = active;
    if (i < 0 || !lines || Date.now() - userScrolledAt < 4000) return;
    void tick().then(() => {
      const el = lines?.querySelector<HTMLElement>(`[data-i="${i}"]`);
      el?.scrollIntoView({ block: "center", behavior: "smooth" });
    });
  });

  // Windows'un medya tuşları ve ses göstergesi (klavyedeki ⏯ ⏭ ⏮).
  $effect(() => {
    const ms = typeof navigator !== "undefined" ? navigator.mediaSession : undefined;
    if (!ms || typeof MediaMetadata === "undefined") return;
    try {
      ms.metadata = new MediaMetadata({
        title: item.title,
        artist: item.uploader,
        album: item.playlist ?? "",
        artwork: item.thumbnail ? [{ src: thumb(item) }] : [],
      });
      ms.setActionHandler("play", () => void media?.play());
      ms.setActionHandler("pause", () => media?.pause());
      ms.setActionHandler("nexttrack", goNext);
      ms.setActionHandler("previoustrack", goPrev);
    } catch {
      // Eski tarayıcı motoru: medya tuşları olmadan da çalışır.
    }
  });

  onMount(() => {
    const stop = on<TranscriptProgress>("transcript-progress", (p) => {
      if (p.id === item.id && creating) creating = p;
    });
    const stopTools = on<{ tool: string; done: number; total: number }>("tools-progress", (p) => {
      if (installing && p.total) installPct = p.done / p.total;
    });
    return () => {
      clearInterval(demo);
      clearTimeout(idleTimer);
      if (full) void setWindowFull(false);
      try {
        for (const a of ["play", "pause", "nexttrack", "previoustrack"] as const) navigator.mediaSession?.setActionHandler(a, null);
      } catch {
        // yok say
      }
      void stop.then((f) => f());
      void stopTools.then((f) => f());
    };
  });

  /** Karaoke açıkken oynatıcının kendi sesi durur, karaoke kendi müziğini çalar. */
  let karaoke = $state(false);
  function openKaraoke() {
    media?.pause();
    if (!inTauri) playing = false;
    karaoke = true;
  }
  $effect(() => {
    if (mini) karaoke = false;
  });

  function toggle() {
    if (!playable) {
      if (!inTauri) playing = !playing;
      return;
    }
    if (!media) return;
    if (media.paused) void media.play().catch(() => {});
    else media.pause();
  }

  function seekTo(t: number) {
    time = Math.max(0, total ? Math.min(total, t) : t);
    if (media && playable) media.currentTime = time;
  }

  function seekLine(s: Segment) {
    seekTo(s.start);
    if (media?.paused) void media.play().catch(() => {});
  }

  let volTimer: ReturnType<typeof setTimeout> | undefined;
  function setVolume(v: number) {
    volume = Math.round(Math.min(1, Math.max(0, v)) * 100) / 100;
    muted = volume === 0;
    clearTimeout(volTimer);
    volTimer = setTimeout(() => void setJson("player.volume", volume), 400);
  }
  function toggleMute() {
    if (muted && volume === 0) setVolume(0.5);
    else muted = !muted;
  }

  function setSpeed(s: number) {
    speed = s;
    applySpeed();
  }

  function cycleRepeat() {
    repeat = repeat === "off" ? "all" : repeat === "all" ? "one" : "off";
    void setJson("player.repeat", repeat);
  }

  function setView(v: "stage" | "list") {
    view = v;
    void setJson("player.view", v);
  }

  async function setWindowFull(on: boolean) {
    if (!inTauri) return;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().setFullscreen(on);
    } catch {
      // İzin yoksa yalnızca pencere içinde büyür.
    }
  }
  function toggleFull() {
    full = !full;
    void setWindowFull(full);
    wake();
  }

  // Tam ekranda fare durunca kontroller gizlenir; fare kıpırdayınca (nerede olursa olsun) geri gelir.
  let idleTimer: ReturnType<typeof setTimeout> | undefined;
  let lastMove = { x: -1, y: -1 };
  function wake() {
    idle = false;
    clearTimeout(idleTimer);
    if (full) idleTimer = setTimeout(() => (idle = playing && !menuOpen && !overControls), 2500);
  }
  /** Fare alt çubuğun üstündeyken gizlenmez. */
  let overControls = $state(false);
  /** Tam ekranda alt şeridin yüksekliği: görüntü tam olarak şeridin bittiği yerde biter. */
  let ctlH = $state(0);
  function onWindowMove(e: PointerEvent) {
    if (!full) return;
    // Bazı sistemler fare kıpırdamadan da olay gönderir; gerçekten hareket edince uyan.
    if (e.clientX === lastMove.x && e.clientY === lastMove.y) return;
    lastMove = { x: e.clientX, y: e.clientY };
    overControls = e.clientY > window.innerHeight - 130;
    wake();
  }

  // Kenar çubuğundaki "çalıyor" göstergesi gerçek duruma uysun.
  $effect(() => {
    player.playing = playing;
  });
  // Medya ekranındaki "Şimdi çalan" kartı: söz satırı ve oynat/duraklat.
  $effect(() => {
    player.line = item.audioOnly && active >= 0 ? (segments[active]?.text ?? "") : "";
  });
  // Oyun içi panel bu oynatıcıyı görsün ve yönetebilsin (karıştır, tekrar, ses, konum): saniyede bir durum.
  function broadcast() {
    void emitAll("gmid-player", {
      title: item.title,
      artist: item.uploader,
      thumb: item.thumbnail,
      playing,
      shuffle,
      repeat: repeat === "one" ? "track" : repeat === "all" ? "list" : "none",
      position: time,
      duration: total,
      volume,
      muted,
    } satisfies GmidPlayerState);
  }
  onMount(() => {
    const t = setInterval(() => playing && broadcast(), 1000);
    const stop = on<{ action: string }>("gmid-player-cmd", ({ action }) => {
      if (action === "toggle") toggle();
      else if (action === "next") goNext();
      else if (action === "prev") goPrev();
      else if (action === "shuffle") ((shuffle = !shuffle), void setJson("player.shuffle", shuffle));
      else if (action === "repeat") cycleRepeat();
      else if (action.startsWith("seek:")) seekTo(Number(action.slice(5)));
      else if (action.startsWith("volume:")) ((volume = Math.max(0, Math.min(1, Number(action.slice(7))))), (muted = false));
      else if (action === "mute") muted = !muted;
      setTimeout(broadcast, 120);
    });
    return () => {
      clearInterval(t);
      void stop.then((f) => f());
    };
  });
  $effect(() => {
    // Duraklatınca/parça değişince hemen bildir.
    void [playing, item.id, shuffle, repeat];
    untrack(broadcast);
  });

  // "En çok dinlenenler" için: her şarkı açılışında bir kez say.
  let counted = "";
  $effect(() => {
    const id = item.id;
    if (item.audioOnly && id !== counted) {
      counted = id;
      untrack(() => countPlay(id));
    }
  });
  onMount(() => {
    controls.toggle = toggle;
    return () => (controls.toggle = () => {});
  });

  function onKey(e: KeyboardEvent) {
    // Küçültülmüşken uygulamanın geri kalanında yazı yazılabilsin: kısayollar yalnızca açıkken.
    if (mini || karaoke) return;
    const target = e.target as HTMLElement;
    if (target.isContentEditable || target.closest("input:not([type=range]):not([type=checkbox]), select, textarea, [contenteditable]")) return;
    if (e.ctrlKey || e.altKey || e.metaKey) return;
    const k = e.key.toLowerCase();
    if (e.key === "Escape") {
      if (menuOpen) menuOpen = false;
      else if (full) toggleFull();
      else onclose();
    } else if (e.key === " " || k === "k") {
      // Odaktaki düğmeye de "tıklanmasın" diye.
      e.preventDefault();
      toggle();
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      seekTo(time + 5);
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      seekTo(time - 5);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setVolume(volume + 0.05);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setVolume(volume - 0.05);
    } else if (k === "m") toggleMute();
    else if (k === "f") toggleFull();
    else if (k === "n") goNext();
    else if (k === "p") goPrev();
    else return;
    wake();
  }

  function onWindowClick(e: MouseEvent) {
    const t = e.target as HTMLElement;
    if (menuOpen && !t.closest(".menuwrap")) menuOpen = false;
    if (ccOpen && !t.closest(".ccwrap")) ccOpen = false;
  }

  // Küçültülünce tam ekrandan da çık.
  $effect(() => {
    if (mini && full) {
      full = false;
      void setWindowFull(false);
    }
  });

  function ended() {
    playing = false;
    if (repeat === "one") return;
    if (autoNext) goNext();
  }

  async function create() {
    error = null;
    const stage = mode === "whisper" ? "audio" : mode === "subtitles" ? "subtitles" : "lyrics";
    creating = { id: item.id, stage, progress: 0 };
    try {
      transcript = await api.transcripts.create(item.id, mode, lang);
      notify(
        transcript.source === "whisper"
          ? "Yazıya döküldü (Whisper)"
          : transcript.source === "lyrics" || transcript.source === "lyrics-timed"
            ? "Şarkı sözleri bulundu"
            : "Sitenin altyazısı alındı",
        "check",
      );
    } catch (e) {
      const msg = String(e);
      error = msg === "WHISPER_MISSING" ? null : msg;
      if (msg === "WHISPER_MISSING") whisper = await api.transcripts.whisperStatus().catch(() => whisper);
    } finally {
      creating = null;
    }
  }

  async function installWhisper(model: string) {
    installing = true;
    installPct = 0;
    try {
      whisper = await api.transcripts.installWhisper(model);
      notify("Whisper hazır", "check");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      installing = false;
    }
  }

  async function redo() {
    await api.transcripts.remove(item.id);
    transcript = null;
  }

  function stageText(p: TranscriptProgress): string {
    if (p.stage === "lyrics") return "Şarkı sözü veritabanında aranıyor…";
    if (p.stage === "subtitles") return "Sitenin altyazısına bakılıyor…";
    if (p.stage === "audio") return "Ses hazırlanıyor…";
    if (p.stage === "lang") return "Şarkının dili bulunuyor…";
    if (p.stage === "load") return "Whisper hazırlanıyor (model yükleniyor)…";
    return `Whisper dinliyor… %${Math.round(p.progress * 100)}`;
  }

  function thumb(d: Download): string {
    if (!d.thumbnail) return "";
    return /^(https?:|data:)/.test(d.thumbnail) ? d.thumbnail : api.fileSrc(d.thumbnail);
  }
  /** O an söylenen satırın kelimeleri (zamanı yoksa tahmini): kelime kelime soldan sağa altınla dolar. */
  const activeWords = $derived(active >= 0 ? wordsOf(segments, active) : []);
  const src = $derived(item.filePath ? api.fileSrc(item.filePath) : "");
  const sourceLabel = $derived.by(() => {
    const t = transcript as Transcript | null;
    if (!t) return "";
    if (t.source === "whisper") return `Whisper · ${t.language === "auto" ? "dil otomatik" : t.language}`;
    if (t.source === "lyrics") return "Şarkı sözü veritabanı (LRCLIB)";
    if (t.source === "lyrics-timed") return "Şarkı sözü veritabanı · zamanlama bu bilgisayarda";
    return `${siteLabel(item.site)} altyazısı · ${t.language}`;
  });
  const pct = $derived(total ? Math.min(100, (time / total) * 100) : 0);
  const shownVolume = $derived(muted ? 0 : volume);
  const volIcon = $derived(shownVolume === 0 ? "mute" : shownVolume < 0.5 ? "volumelow" : "speaker");
  const repeatTitle = $derived(
    repeat === "off" ? "Tekrar: kapalı" : repeat === "all" ? "Tekrar: tüm liste" : "Tekrar: bu parça",
  );

  // <audio> ve <video> aynı olayları dinler.
  const onplay = () => ((playing = true), wake());
  const onpause = () => ((playing = false), wake());
  const onloadedmetadata = () => {
    if (media && isFinite(media.duration)) duration = media.duration;
    applySpeed();
  };
  const onseeked = () => {
    if (media) time = media.currentTime;
  };
  const onerror = () => {
    playable = false;
    startDemo();
  };
</script>

<svelte:window onkeydown={onKey} onclick={onWindowClick} onpointermove={onWindowMove} onpointerdown={() => full && wake()} />

<div
  class="player"
  class:full
  class:idle
  class:mini
  class:isvideo={!item.audioOnly}
  style:--ctlh={`${full && !idle ? ctlH : 0}px`}
  role={mini ? "region" : "dialog"}
  aria-label="Oynatıcı"
  tabindex="-1"
  onmousemove={wake}
>
  <header class="bar">
    <button class="icon" title="Küçült (Esc) — çalmaya devam eder" onclick={onclose}><Icon name="shrinkdown" size={20} /></button>
    <div class="ttl">
      <b title={item.title}>{item.title}</b>
      <span class="sub small">{[item.uploader, item.playlist, item.quality].filter(Boolean).join(" · ")}</span>
    </div>
    <div class="row">
      <div class="seg views" role="group" aria-label="Görünüm">
        <button aria-pressed={view === "stage"} title="Sahne görünümü" onclick={() => setView("stage")}><Icon name="stage" size={16} />Sahne</button>
        <button aria-pressed={view === "list"} title="Liste görünümü: solda parçalar, sağda yazılar" onclick={() => setView("list")}><Icon name="list" size={16} />Liste</button>
      </div>
      <button class="btn ghost sm kbtn" title="Karaoke: şarkının sesi ayrılır, sözler akar, sen söyler ve kaydedersin" onclick={openKaraoke}><Icon name="mic" size={16} />Karaoke</button>
      <button class="btn ghost sm" onclick={() => ((showText = !showText), void setJson("player.showText", showText))}>
        <Icon name="note" size={16} />{showText ? "Yazıları gizle" : "Yazıları göster"}
      </button>
      <button class="btn ghost sm" title="Varsayılan oynatıcıda aç" onclick={() => api.videos.open(item.id)}><Icon name="monitor" size={16} />Dışarıda aç</button>
      <button class="icon" title="Çalmayı bitir ve kapat" onclick={onstop}><Icon name="x" size={20} /></button>
    </div>
  </header>

  <div class="body v-{shownView}" class:notext={!showText} class:nostage={hideStage}>
    {#if shownView === "list"}
      <section class="queue" aria-label="Sıradakiler">
        <div class="qhead">
          <b>{item.playlist ?? "Sıradakiler"}</b>
          <span class="sub small tnum">{index + 1} / {list.length}</span>
        </div>
        <div class="qlist" bind:this={queueEl}>
          {#each list as d, i (d.id)}
            <button class="qi" class:on={d.id === item.id} data-id={d.id} title={d.title} onclick={() => (d.id === item.id ? toggle() : onpick(d))}>
              <span class="qn tnum">
                {#if d.id === item.id && playing}<span class="eq" aria-label="Çalıyor"><i></i><i></i><i></i></span>{:else}{i + 1}{/if}
              </span>
              <span class="qthumb">
                {#if d.thumbnail}<img src={thumb(d)} alt="" loading="lazy" />{:else}<Icon name={d.audioOnly ? "headphones" : "film"} size={18} />{/if}
              </span>
              <span class="qtx">
                <span class="qt">{d.title}</span>
                <span class="sub small">{d.uploader}</span>
              </span>
              <span class="sub small tnum">{formatDuration(d.duration ?? 0)}</span>
            </button>
          {/each}
        </div>
      </section>
    {/if}

    <div class="stage" class:compact={shownView === "list"} class:hidden={hideStage} class:broken={!playable}>
      {#if playable}
        {#if item.audioOnly}
          <button class="cover" title={playing ? "Duraklat" : "Oynat"} onclick={toggle}>
            {#if item.thumbnail}<img src={thumb(item)} alt="" />{:else}<Icon name="headphones" size={64} />{/if}
            <span class="bigplay" class:show={!playing}><Icon name={playing ? "pause" : "play"} size={40} /></span>
          </button>
        {:else}
          <div class="vwrap">
            <!-- svelte-ignore a11y_media_has_caption, a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
            <video
              bind:this={media}
              {src}
              autoplay
              playsinline
              disablepictureinpicture
              controlslist="nodownload noplaybackrate noremoteplayback nofullscreen"
              oncontextmenu={(e) => e.preventDefault()}
              onclick={toggle}
              ondblclick={toggleFull}
              {onplay}
              {onpause}
              onended={ended}
              {onloadedmetadata}
              ondurationchange={onloadedmetadata}
              {onseeked}
              {onerror}
            ></video>
            {#if cue}<div class="cue">{cue}</div>{/if}
            {#if !playing && !mini}<button class="bigplay show" title="Oynat" onclick={toggle}><Icon name="play" size={40} /></button>{/if}
            {#if mini}
              <button class="minibtn close" title="Kapat" onclick={onstop}><Icon name="x" size={16} /></button>
              <button class="minibtn grow" title="Büyüt" onclick={onexpand}><Icon name="expand" size={16} /></button>
            {/if}
          </div>
        {/if}
      {:else}
        <div class="noplay">
          <Icon name="film" size={36} />
          <p>Bu dosya uygulama içinde oynatılamıyor{item.filePath?.toLowerCase().endsWith(".mkv") ? " (MKV)" : ""}.</p>
          <button class="btn" onclick={() => api.videos.open(item.id)}><Icon name="play" size={18} />Varsayılan oynatıcıda aç</button>
          {#if !inTauri}<span class="sub small">(Tarayıcı önizlemesi: yazılar sahte bir saatle akıyor.)</span>{/if}
        </div>
      {/if}
    </div>
    {#if item.audioOnly && playable}
      <audio
        bind:this={media}
        {src}
        autoplay
        {onplay}
        {onpause}
        onended={ended}
        {onloadedmetadata}
        ondurationchange={onloadedmetadata}
        {onseeked}
        {onerror}
      ></audio>
    {/if}

    {#if showText}
      <aside class="text" class:big>
        <div class="thead">
          <span class="sub small">{sourceLabel || "Yazılar"}</span>
          {#if transcript}
            <span class="row">
              <button class="link" onclick={() => ((big = !big), void setJson("player.bigLyrics", big))}>{big ? "Normal yazı" : "Büyük yazı"}</button>
              <button class="link" onclick={redo}>Yeniden çıkar</button>
            </span>
          {/if}
        </div>

        {#if loading}
          <p class="sub pad">Yükleniyor…</p>
        {:else if transcript && segments.length}
          <div
            class="lines"
            role="list"
            bind:this={lines}
            onwheel={() => (userScrolledAt = Date.now())}
            ontouchmove={() => (userScrolledAt = Date.now())}
          >
            {#each segments as s, i (i)}
              <button class="line" class:active={i === active} class:past={i < active} data-i={i} onclick={() => seekLine(s)}>
                <span class="ts tnum">{formatDuration(s.start) || "0:00"}</span>
                <span class="tx">
                  {#if i === active && activeWords.length}
                    {#each activeWords as w, j (j)}{@const p = fill(w, time)}<span class="w" class:now={p > 0 && p < 1} style:--p={p}>{w.text}</span>{" "}{/each}
                  {:else}
                    {s.text}
                  {/if}
                </span>
              </button>
            {/each}
          </div>
        {:else if creating}
          <div class="make">
            <p>{stageText(creating)}</p>
            <div class="pbar" class:busy={creating.stage !== "whisper" || creating.progress === 0}><i style:width="{creating.stage === 'whisper' ? creating.progress * 100 : 0}%"></i></div>
            <span class="sub small">Uzun videolarda birkaç dakika sürebilir; bu sırada izlemeye devam edebilirsin.</span>
            {#if (creating.stage === "whisper" || creating.stage === "audio") && gpuMissing}
              <p class="tip small">
                Yavaş mı? NVIDIA ekran kartın varsa Whisper'ı ekran kartıyla çalıştır; birkaç dakika yerine saniyeler sürer.
                <button class="link" onclick={() => ((player.open = false), nav.go("settings/dikte"))}>Kontrol Odası → Sesle yazma → NVIDIA ile hızlandır</button>
              </p>
            {/if}
          </div>
        {:else}
          <div class="make">
            <h3 class="h2"><Icon name="note" />Yazıya dök</h3>
            <p class="sub small">
              Şarkı sözleri ya da konuşmalar burada zamanla akar; o an söylenen satır işaretlenir. Önce şarkı sözü
              veritabanına (LRCLIB) bakılır, sonra sitenin kendi altyazısına; ikisi de yoksa Whisper tamamen bu
              bilgisayarda dinleyip yazar.
            </p>
            <div class="field">
              <span>Dil</span>
              <div class="seg">
                {#each [["orig", "Videonun dili"], ["tr", "Türkçe"], ["en", "İngilizce"]] as [k, l] (k)}
                  <button aria-pressed={lang === k} onclick={() => (lang = k)}>{l}</button>
                {/each}
              </div>
            </div>
            <div class="field">
              <span>Kaynak</span>
              <div class="seg wrap">
                {#each [["auto", "Otomatik"], ["lyrics", "Şarkı sözü"], ["subtitles", "Sadece altyazı"], ["whisper", "Sadece Whisper"]] as [k, l] (k)}
                  <button aria-pressed={mode === k} onclick={() => (mode = k)}>{l}</button>
                {/each}
              </div>
            </div>
            {#if error}<p class="err small">{error}</p>{/if}
            <button class="btn" onclick={create}><Icon name="sparkle" size={18} />Yazıya dök</button>

            {#if whisper && !whisper.ready && (mode === "auto" || mode === "whisper")}
              <div class="wbox">
                <b>Whisper kurulu değil</b>
                <span class="sub small">Sözü ve altyazısı bulunamayan kayıtlar için gerekli. Bir kez iner, ses hiçbir yere gönderilmez.</span>
                {#each whisper.models as m (m.id)}
                  <button class="wopt" disabled={installing} onclick={() => installWhisper(m.id)}>
                    <span><b>{m.label}</b><span class="sub small">{m.detail}</span></span>
                    <span class="sub small tnum">{m.installed ? "Kurulu" : `~${m.sizeMb} MB`}</span>
                  </button>
                {/each}
                {#if installing}<div class="pbar"><i style:width="{installPct * 100}%"></i></div>{/if}
              </div>
            {/if}
          </div>
        {/if}
      </aside>
    {/if}
  </div>

  {#if full && idle}
    <!-- Kontroller gizliyken alt kenara gelen fare onları geri getirir. -->
    <div class="hot" role="presentation" onpointermove={wake} onpointerenter={wake}></div>
  {/if}
  <footer class="ctl" bind:offsetHeight={ctlH} onpointerenter={() => (overControls = true)} onpointerleave={() => (overControls = false)}>
    <div class="playing-now">
      <button class="nthumb" title={mini ? "Oynatıcıyı aç" : playing ? "Duraklat" : "Oynat"} onclick={() => (mini ? onexpand() : toggle())}>
        {#if item.thumbnail}<img src={thumb(item)} alt="" />{:else}<Icon name={item.audioOnly ? "headphones" : "film"} size={20} />{/if}
      </button>
      <div class="ntx">
        <b title={item.title}>{item.title}</b>
        <span class="sub small" title={item.uploader}>{item.uploader}</span>
      </div>
    </div>

    <div class="mid">
      <div class="btns">
        <button
          class="icon"
          class:lit={shuffle}
          title={shuffle ? "Karıştır: açık" : "Karıştır: kapalı"}
          aria-pressed={shuffle}
          onclick={() => ((shuffle = !shuffle), void setJson("player.shuffle", shuffle))}
        >
          <Icon name="shuffle" size={18} />
        </button>
        <button class="icon" title="Önceki (P)" disabled={!hasPrev && time <= 3} onclick={goPrev}><Icon name="prev" size={20} /></button>
        <button class="play" title={playing ? "Duraklat (Boşluk)" : "Oynat (Boşluk)"} onclick={toggle}>
          <Icon name={playing ? "pause" : "play"} size={22} />
        </button>
        <button class="icon" title="Sonraki (N)" disabled={!hasNext} onclick={goNext}><Icon name="next" size={20} /></button>
        <button class="icon rep" class:lit={repeat !== "off"} title={repeatTitle} onclick={cycleRepeat}>
          <Icon name="repeat" size={18} />{#if repeat === "one"}<span class="one">1</span>{/if}
        </button>
      </div>
      <div class="seek">
        <span class="sub small tnum">{formatDuration(time) || "0:00"}</span>
        <input
          type="range"
          class="range"
          min="0"
          max={total || 1}
          step="0.1"
          value={time}
          style:--p="{pct}%"
          aria-label="İlerleme"
          oninput={(e) => ((seeking = true), (time = +e.currentTarget.value))}
          onchange={(e) => ((seeking = false), seekTo(+e.currentTarget.value))}
        />
        <span class="sub small tnum">{formatDuration(total) || "0:00"}</span>
      </div>
    </div>

    <div class="right">
      {#if !item.audioOnly && (tracks.length || segments.length)}
        <div class="ccwrap">
          <button class="pill" class:lit={sub !== "off"} title="Altyazı" aria-expanded={ccOpen} onclick={() => (ccOpen = !ccOpen)}>CC</button>
          {#if ccOpen}
            <div class="menu">
              <span class="mlabel">Altyazı</span>
              <button class="mi" class:on={sub === "off"} onclick={() => chooseSub("off")}>Kapalı</button>
              {#each tracks as t (t.file)}
                <button class="mi" class:on={sub === t.file} onclick={() => chooseSub(t.file)}>{t.label}</button>
              {/each}
              {#if segments.length}
                <button class="mi" class:on={sub === "transcript"} onclick={() => chooseSub("transcript")}>Yazıya dökülen</button>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
      <div class="menuwrap">
        <button class="pill" class:lit={speed !== 1} title="Hız ve oynatma ayarları" aria-expanded={menuOpen} onclick={() => (menuOpen = !menuOpen)}>
          <span class="tnum">{speed}×</span>
        </button>
        {#if menuOpen}
          <div class="menu">
            <span class="mlabel">Hız</span>
            <div class="speeds">
              {#each SPEEDS as s (s)}
                <button class:on={speed === s} onclick={() => setSpeed(s)}>{s === 1 ? "Normal" : `${s}×`}</button>
              {/each}
            </div>
            <label class="mrow">
              <input
                type="checkbox"
                checked={autoNext}
                onchange={(e) => ((autoNext = e.currentTarget.checked), void setJson("player.autoNext", autoNext))}
              />
              Bitince sıradakine geç
            </label>
          </div>
        {/if}
      </div>
      <div class="vol" role="group" aria-label="Ses" onwheel={(e) => (e.preventDefault(), setVolume(volume + (e.deltaY < 0 ? 0.05 : -0.05)))}>
        <button class="icon" title={muted ? "Sesi aç (M)" : "Sessiz (M)"} onclick={toggleMute}><Icon name={volIcon} size={20} /></button>
        <input
          type="range"
          class="range"
          min="0"
          max="1"
          step="0.01"
          value={shownVolume}
          style:--p="{shownVolume * 100}%"
          aria-label="Ses düzeyi"
          oninput={(e) => setVolume(+e.currentTarget.value)}
        />
        <span class="sub small tnum vpct">{Math.round(shownVolume * 100)}</span>
      </div>
      {#if mini}
        <button class="icon" title="Oynatıcıyı aç" onclick={onexpand}><Icon name="expand" size={18} /></button>
        <button class="icon" title="Çalmayı bitir" onclick={onstop}><Icon name="x" size={18} /></button>
      {:else}
        <button class="icon" title={full ? "Tam ekrandan çık (F)" : "Tam ekran (F)"} onclick={toggleFull}><Icon name={full ? "shrink" : "expand"} size={18} /></button>
      {/if}
    </div>
  </footer>
  {#if karaoke && !mini}
    <Karaoke {item} {segments} start={time} onclose={(t) => ((karaoke = false), seekTo(t))} />
  {/if}
</div>

<style>
  .player { position: fixed; inset: 0; z-index: 30; display: grid; grid-template-rows: auto minmax(0, 1fr) auto; background: var(--night); outline: none; }
  .bar { display: flex; align-items: center; gap: 14px; padding: 10px 16px; border-bottom: 1px solid var(--line); }
  .ttl { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .ttl b { font-family: var(--f-display); font-size: 16px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .views button { display: inline-flex; align-items: center; gap: 6px; }
  .icon { position: relative; border: 0; background: transparent; color: var(--muted); padding: 8px; border-radius: 9px; display: grid; place-items: center; }
  .icon:hover:not(:disabled) { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .icon:disabled { opacity: 0.3; cursor: default; }
  .icon.lit { color: var(--gold); }
  .rep .one { position: absolute; right: 2px; bottom: 2px; font-size: 9px; font-weight: 800; color: var(--night); background: var(--gold); border-radius: 999px; min-width: 12px; height: 12px; line-height: 12px; text-align: center; }
  .sm { padding: 6px 11px; font-size: 13px; }

  /* Görünümler */
  .body { display: grid; min-height: 0; grid-template-columns: minmax(0, 1fr) minmax(320px, 36%); grid-template-areas: "stage text"; }
  .body.notext { grid-template-columns: minmax(0, 1fr); grid-template-areas: "stage"; }
  .body.v-list { grid-template-columns: minmax(300px, 34%) minmax(0, 1fr); grid-template-rows: auto minmax(0, 1fr); grid-template-areas: "stage text" "queue text"; }
  .body.v-list.nostage { grid-template-rows: minmax(0, 1fr); grid-template-areas: "queue text"; }
  .body.v-list.notext { grid-template-rows: minmax(0, 1fr); grid-template-areas: "queue stage"; }
  .stage { grid-area: stage; display: flex; flex-direction: column; justify-content: center; align-items: center; gap: 12px; padding: 18px; min-height: 0; min-width: 0; }
  .stage.hidden { display: none; }
  .stage.compact { padding: 12px; border-right: 1px solid var(--line); border-bottom: 1px solid var(--line); }
  .body.v-list.notext .stage.compact { border: 0; padding: 18px; }
  .text { grid-area: text; }
  .queue { grid-area: queue; display: flex; flex-direction: column; min-height: 0; border-right: 1px solid var(--line); background: rgba(23, 26, 30, 0.5); }

  .vwrap { position: relative; width: 100%; display: grid; place-items: center; }
  video { width: 100%; max-height: calc(100vh - 200px); background: #000; border-radius: 12px; cursor: pointer; display: block; }
  .compact video { max-height: 34vh; }
  .full video { max-height: calc(100vh - 100px); border-radius: 0; }
  .cover { position: relative; width: min(440px, 70%, 58vh); aspect-ratio: 1; border-radius: 18px; overflow: hidden; background: var(--coal-2); border: 0; padding: 0; display: grid; place-items: center; color: var(--dim); box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), 0 0 40px rgba(227, 194, 112, 0.08); }
  .compact .cover { width: min(200px, 60%); }
  .cover img { width: 100%; height: 100%; object-fit: cover; transition: filter 0.2s; }
  .bigplay { position: absolute; inset: 0; margin: auto; width: 76px; height: 76px; border-radius: 999px; border: 0; display: grid; place-items: center; color: var(--night); background: rgba(227, 194, 112, 0.92); box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45); opacity: 0; transform: scale(0.9); transition: opacity 0.2s, transform 0.2s; pointer-events: none; }
  .bigplay.show { opacity: 1; transform: none; }
  .vwrap .bigplay { pointer-events: auto; }
  .cover:hover .bigplay { opacity: 1; transform: none; }
  .cover:hover img { filter: brightness(0.75); }
  .noplay { display: flex; flex-direction: column; align-items: center; gap: 12px; color: var(--muted); text-align: center; }

  /* Sıradakiler */
  .qhead { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; padding: 12px 16px; border-bottom: 1px solid var(--line); }
  .qhead b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .qlist { overflow: auto; padding: 6px; display: flex; flex-direction: column; gap: 2px; }
  .qi { display: grid; grid-template-columns: 26px 44px minmax(0, 1fr) auto; align-items: center; gap: 10px; text-align: left; border: 0; background: transparent; padding: 6px 8px; border-radius: 10px; color: var(--text); }
  .qi:hover { background: rgba(255, 255, 255, 0.05); }
  .qi.on { background: rgba(227, 194, 112, 0.1); box-shadow: inset 3px 0 0 var(--gold); }
  .qi.on .qt { color: var(--gold-glow); }
  .qn { color: var(--dim); font-size: 12px; text-align: center; }
  .qthumb { width: 44px; height: 44px; border-radius: 8px; overflow: hidden; background: var(--coal-2); display: grid; place-items: center; color: var(--dim); }
  .qthumb img { width: 100%; height: 100%; object-fit: cover; }
  .qtx { display: flex; flex-direction: column; min-width: 0; }
  .qt { font-weight: 600; font-size: 13.5px; line-height: 1.3; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .qtx .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .eq { display: inline-flex; gap: 2px; align-items: flex-end; height: 12px; }
  .eq i { width: 3px; background: var(--gold); border-radius: 1px; animation: eq 0.9s ease-in-out infinite; }
  .eq i:nth-child(2) { animation-delay: 0.2s; }
  .eq i:nth-child(3) { animation-delay: 0.4s; }
  @keyframes eq { 0%, 100% { height: 3px; } 50% { height: 12px; } }

  /* Yazılar */
  .text { border-left: 1px solid var(--line); display: flex; flex-direction: column; min-height: 0; background: rgba(23, 26, 30, 0.6); }
  .v-list .text { border-left: 0; }
  .thead { display: flex; justify-content: space-between; align-items: center; gap: 8px; padding: 12px 16px; border-bottom: 1px solid var(--line); }
  .pad { padding: 16px; }
  .lines { overflow: auto; padding: 30vh 10px; display: flex; flex-direction: column; gap: 2px; scroll-behavior: smooth; }
  .v-list .lines { padding-left: 6%; padding-right: 6%; }
  .line { display: grid; grid-template-columns: 44px minmax(0, 1fr); gap: 10px; text-align: left; border: 0; background: transparent; padding: 7px 10px; border-radius: 10px; color: var(--muted); transition: color 0.2s, background 0.2s; font-size: 15px; line-height: 1.45; }
  .line:hover { background: rgba(255, 255, 255, 0.04); color: var(--text); }
  .line.past { color: var(--dim); }
  .line.active { color: var(--text); background: rgba(227, 194, 112, 0.1); box-shadow: inset 3px 0 0 var(--gold); }
  .line.active .tx { color: var(--gold-glow); font-weight: 600; }
  .ts { font-size: 11px; color: var(--dim); padding-top: 3px; }
  /* Kelime dolumu: --p (0…1) her karede güncellenir; yumuşak kenarlı altın, soldan sağa. */
  .w {
    --p: 0;
    color: transparent;
    background: linear-gradient(90deg, var(--gold-glow) calc(var(--p) * 120% - 20%), rgba(236, 231, 220, 0.55) calc(var(--p) * 120%));
    -webkit-background-clip: text;
    background-clip: text;
    transition: filter 0.25s;
  }
  .w.now { filter: drop-shadow(0 0 10px rgba(240, 216, 120, 0.45)); }
  .big .line { font-size: 22px; font-family: var(--f-display); padding: 10px 12px; grid-template-columns: minmax(0, 1fr); }
  .big .ts { display: none; }
  .make { padding: 18px; display: flex; flex-direction: column; gap: 12px; overflow: auto; }
  .make p { margin: 0; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field > span { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .seg.wrap { flex-wrap: wrap; }
  .err { color: var(--bad); }
  .tip { margin: 4px 0 0; padding: 10px 12px; border-radius: 10px; border: 1px solid rgba(53, 160, 162, 0.4); background: rgba(53, 160, 162, 0.06); color: var(--muted); }
  .tip .link { display: inline; text-align: left; }
  .pbar { height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
  .pbar i { display: block; height: 100%; background: linear-gradient(90deg, var(--teal), var(--neon)); transition: width 0.3s; }
  /* Yüzde henüz yokken (dil bulma, model yükleme) ilerlediğini gösteren kayan şerit. */
  .pbar.busy i { width: 30% !important; animation: busy 1.3s ease-in-out infinite; }
  @keyframes busy { from { transform: translateX(-100%); } to { transform: translateX(340%); } }
  .wbox { display: flex; flex-direction: column; gap: 8px; padding: 12px; border-radius: 12px; border: 1px solid rgba(53, 160, 162, 0.4); background: rgba(53, 160, 162, 0.06); }
  .wopt { display: flex; justify-content: space-between; align-items: center; gap: 10px; text-align: left; padding: 9px 12px; border-radius: 10px; border: 1px solid var(--line-2); background: var(--coal-2); }
  .wopt:hover:not(:disabled) { border-color: var(--gold); }
  .wopt > span:first-child { display: flex; flex-direction: column; }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; font-size: 12.5px; }
  .link:hover { color: var(--neon); }

  /* Alt çalma çubuğu */
  .ctl { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1.6fr) minmax(0, 1fr); align-items: center; gap: 16px; padding: 10px 16px; border-top: 1px solid var(--line); background: var(--night-2); }
  .playing-now { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .nthumb { flex: none; width: 52px; height: 52px; border-radius: 9px; overflow: hidden; border: 0; padding: 0; background: var(--coal-2); color: var(--dim); display: grid; place-items: center; }
  .nthumb img { width: 100%; height: 100%; object-fit: cover; }
  .ntx { display: flex; flex-direction: column; min-width: 0; }
  .ntx b { font-size: 14px; line-height: 1.3; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .ntx .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mid { display: flex; flex-direction: column; align-items: center; gap: 4px; min-width: 0; }
  .btns { display: flex; align-items: center; gap: 6px; }
  .play { width: 42px; height: 42px; border-radius: 999px; border: 0; display: grid; place-items: center; background: var(--gold); color: var(--night); transition: transform 0.12s, background 0.12s; }
  .play:hover { transform: scale(1.06); background: var(--gold-glow); }
  .seek { display: grid; grid-template-columns: 44px minmax(0, 1fr) 44px; align-items: center; gap: 8px; width: 100%; }
  .seek span:first-child { text-align: right; }
  .right { display: flex; align-items: center; justify-content: flex-end; gap: 6px; min-width: 0; }
  .vol { display: flex; align-items: center; gap: 4px; }
  .vol .range { width: 110px; }
  .vpct { width: 26px; text-align: right; }
  .pill { border: 1px solid var(--line-2); background: transparent; color: var(--muted); border-radius: 999px; padding: 4px 10px; font-size: 12.5px; font-weight: 700; }
  .pill:hover, .pill.lit { color: var(--gold); border-color: rgba(227, 194, 112, 0.5); }
  .menuwrap { position: relative; }
  .menu { position: absolute; bottom: calc(100% + 10px); right: 0; z-index: 5; width: 230px; padding: 12px; border-radius: 12px; border: 1px solid var(--line-2); background: var(--coal-2); box-shadow: 0 18px 40px rgba(0, 0, 0, 0.5); display: flex; flex-direction: column; gap: 10px; }
  .mlabel { font-size: 11px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--dim); }
  .speeds { display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px; }
  .speeds button { border: 1px solid var(--line-2); background: transparent; color: var(--text); border-radius: 8px; padding: 6px 0; font-size: 12.5px; }
  .speeds button:hover { border-color: var(--gold); }
  .speeds button.on { background: var(--gold); color: var(--night); border-color: var(--gold); font-weight: 700; }
  .mrow { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--muted); }
  .mrow input { accent-color: var(--gold); }

  /* Kaydırıcılar: dolu kısım altın */
  .range { -webkit-appearance: none; appearance: none; width: 100%; height: 16px; background: transparent; cursor: pointer; margin: 0; }
  .range::-webkit-slider-runnable-track { height: 4px; border-radius: 999px; background: linear-gradient(90deg, var(--gold) var(--p, 0%), rgba(255, 255, 255, 0.14) var(--p, 0%)); }
  .range::-webkit-slider-thumb { -webkit-appearance: none; appearance: none; width: 12px; height: 12px; margin-top: -4px; border-radius: 999px; background: var(--text); opacity: 0; transition: opacity 0.15s; }
  .range:hover::-webkit-slider-thumb, .range:focus-visible::-webkit-slider-thumb { opacity: 1; }
  .range::-moz-range-track { height: 4px; border-radius: 999px; background: linear-gradient(90deg, var(--gold) var(--p, 0%), rgba(255, 255, 255, 0.14) var(--p, 0%)); }
  .range::-moz-range-thumb { width: 12px; height: 12px; border: 0; border-radius: 999px; background: var(--text); }

  /* Görüntünün üstündeki altyazı */
  .cue { position: absolute; left: 50%; bottom: 7%; transform: translateX(-50%); max-width: 84%; padding: 4px 14px; border-radius: 8px; background: rgba(0, 0, 0, 0.62); color: #fff; text-align: center; font-size: clamp(15px, 1.9vw, 30px); line-height: 1.35; font-weight: 600; text-shadow: 0 1px 3px rgba(0, 0, 0, 0.9); pointer-events: none; white-space: pre-line; transition: bottom 0.3s; }
  .ccwrap { position: relative; }
  .mi { display: block; width: 100%; text-align: left; border: 0; background: transparent; color: var(--text); padding: 7px 10px; border-radius: 8px; font-size: 13px; }
  .mi:hover { background: rgba(255, 255, 255, 0.06); }
  .mi.on { color: var(--gold); font-weight: 700; }
  .ccwrap .menu { width: 200px; gap: 2px; }

  /* Tam ekran: başlık gizlenir, fare durunca alt çubuk da */
  .full { grid-template-rows: minmax(0, 1fr) auto; }
  .full .bar { display: none; }
  .full .ctl { transition: opacity 0.3s; }
  .full.idle .ctl { opacity: 0; }
  .full.idle { cursor: none; }
  /* Filmde tam ekran: fare dururken görüntü bütün ekranı kaplar, hiçbir düğme görünmez. Fare kıpırdayınca
     kontroller altta ayrı bir şeritte gelir ve görüntü şeridin bittiği yerde biter: hiçbir şey görüntünün üstüne
     binmez (WebView2 bazı ekran kartlarında videonun üstündeki öğelerin arka planını siliyor, düğmeler yarı
     saydam görünüyordu). Fare şeridin üstündeyken şerit kapanmaz. */
  .full.isvideo { background: #000; }
  .full.isvideo .text { display: none; }
  .full.isvideo .vwrap { position: fixed; left: 0; right: 0; top: 0; bottom: var(--ctlh, 0px); width: auto; height: auto; z-index: 1; display: block; background: #000; }
  /* Kutu ekranın kendisi; görüntü oranı bozulmadan sığar (4:3 film yanlarda siyah bantla, hiçbir yeri kesilmez). */
  .full.isvideo video { position: absolute; inset: 0; width: 100%; height: 100%; max-height: none; object-fit: contain; border-radius: 0; }
  .full.isvideo .ctl { position: fixed; left: 0; right: 0; bottom: 0; z-index: 30; border-top: 1px solid rgba(255, 255, 255, 0.08); background: #0b0c0e; }
  .full.isvideo.idle .ctl { display: none; }
  .hot { position: fixed; left: 0; right: 0; bottom: 0; height: 140px; z-index: 40; }

  /* Küçültülmüş: uygulamanın altında çalma çubuğu, video köşede */
  .mini { inset: auto 0 0 var(--side-w); z-index: 20; grid-template-rows: auto; background: transparent; }
  .mini .bar, .mini .queue, .mini .text { display: none; }
  .mini .body { display: block; height: 0; }
  .mini .stage { display: none; }
  .mini.isvideo .stage { display: block; position: fixed; right: 16px; bottom: 90px; width: 360px; padding: 0; z-index: 21; border-radius: 12px; overflow: hidden; box-shadow: 0 16px 40px rgba(0, 0, 0, 0.6); border: 1px solid var(--line-2); }
  .mini.isvideo .stage.broken { display: none; }
  .mini video { max-height: none; border-radius: 0; }
  .mini .cue { font-size: 13px; bottom: 6%; }
  .mini .ctl { background: rgba(23, 26, 30, 0.97); backdrop-filter: blur(10px); padding: 8px 16px; }
  .mini .nthumb { width: 46px; height: 46px; }
  .minibtn { position: absolute; right: 8px; border: 0; border-radius: 8px; padding: 6px; display: grid; place-items: center; background: rgba(0, 0, 0, 0.65); color: #fff; opacity: 0; transition: opacity 0.2s, background 0.15s; }
  .minibtn.close { top: 8px; }
  .minibtn.grow { bottom: 8px; }
  .minibtn:hover { background: rgba(227, 194, 112, 0.9); color: #1b1608; }
  .vwrap:hover .minibtn { opacity: 1; }

  @media (max-width: 1100px) {
    .mini { left: 76px; }
  }
  @media (max-width: 900px) {
    .body, .body.v-list, .body.v-list.nostage { grid-template-columns: minmax(0, 1fr); grid-template-rows: auto minmax(0, 1fr); grid-template-areas: "stage" "text"; }
    .body.v-list { grid-template-rows: auto auto minmax(0, 1fr); grid-template-areas: "stage" "queue" "text"; }
    .body.v-list.nostage { grid-template-areas: "queue" "text"; }
    .text { border-left: 0; border-top: 1px solid var(--line); }
    .queue { max-height: 30vh; border-right: 0; }
    video { max-height: 40vh; }
    .cover { width: 180px; }
    .ctl { grid-template-columns: minmax(0, 1fr); }
    .right { justify-content: center; }
  }
</style>
