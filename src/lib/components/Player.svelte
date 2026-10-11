<script lang="ts">
  // Uygulama içi oynatıcı: solda video (ya da müzik için kapak), sağda zamanla akan yazılar.
  // O an söylenen satır altınla işaretlenir; kelime zamanları varsa kelime kelime (karaoke gibi).
  import { onMount, tick } from "svelte";
  import { api, inTauri } from "../api";
  import { on } from "../bus";
  import { formatDuration, siteLabel } from "../downloads.svelte";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { notify } from "../toast.svelte";
  import type { Download, Segment, Transcript, TranscriptProgress, WhisperStatus } from "../types";

  let {
    item,
    queue,
    onclose,
    onpick,
  }: { item: Download; queue: Download[]; onclose: () => void; onpick: (d: Download) => void } = $props();

  let media: HTMLMediaElement | undefined = $state();
  let lines: HTMLDivElement | undefined = $state();
  let time = $state(0);
  let playing = $state(false);
  let playable = $state(true);
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
  /** Kullanıcı yazıları elle kaydırdıysa bir süre otomatik kaydırma yapılmaz. */
  let userScrolledAt = 0;

  const index = $derived(queue.findIndex((d) => d.id === item.id));
  const prev = $derived(index > 0 ? queue[index - 1] : null);
  const next = $derived(index >= 0 && index < queue.length - 1 ? queue[index + 1] : null);
  const segments = $derived.by(() => (transcript as Transcript | null)?.segments ?? []);
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

  async function load() {
    loading = true;
    error = null;
    creating = null;
    playable = true;
    time = 0;
    try {
      transcript = await api.transcripts.get(item.id);
    } catch {
      transcript = null;
    }
    loading = false;
    whisper = await api.transcripts.whisperStatus().catch(() => null);
  }

  // Video değişince her şeyi baştan yükle.
  let loadedId = "";
  $effect(() => {
    if (item.id !== loadedId) {
      loadedId = item.id;
      void load();
    }
  });

  // Oynarken zamanı kare kare takip et (kelime işaretleme akıcı olsun).
  let raf = 0;
  function frame() {
    if (media) time = media.currentTime;
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
    demo = setInterval(() => (time += 0.1), 100);
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

  onMount(() => {
    const stop = on<TranscriptProgress>("transcript-progress", (p) => {
      if (p.id === item.id && creating) creating = p;
    });
    const stopTools = on<{ tool: string; done: number; total: number }>("tools-progress", (p) => {
      if (installing && p.total) installPct = p.done / p.total;
    });
    return () => {
      clearInterval(demo);
      void stop.then((f) => f());
      void stopTools.then((f) => f());
    };
  });

  function seek(s: Segment) {
    if (!media) return;
    media.currentTime = s.start;
    time = s.start;
    void media.play().catch(() => {});
  }

  function onKey(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    if (target.closest("input, select, textarea")) return;
    if (e.key === "Escape") onclose();
    else if (e.key === " " && media) {
      e.preventDefault();
      if (media.paused) void media.play();
      else media.pause();
    } else if (e.key === "ArrowRight" && media) media.currentTime += 5;
    else if (e.key === "ArrowLeft" && media) media.currentTime -= 5;
  }

  function ended() {
    playing = false;
    if (autoNext && next) onpick(next);
  }

  async function create() {
    error = null;
    creating = { id: item.id, stage: mode === "whisper" ? "audio" : "subtitles", progress: 0 };
    try {
      transcript = await api.transcripts.create(item.id, mode, lang);
      notify(transcript.source === "whisper" ? "Yazıya döküldü (Whisper)" : "Sitenin altyazısı alındı", "check");
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
    if (p.stage === "subtitles") return "Sitenin altyazısına bakılıyor…";
    if (p.stage === "audio") return "Ses hazırlanıyor…";
    return `Whisper dinliyor… %${Math.round(p.progress * 100)}`;
  }

  const wordState = (s: Segment, w: { start: number; end: number }) =>
    time >= w.end ? "sung" : time >= w.start ? "now" : "";
  const src = $derived(item.filePath ? api.fileSrc(item.filePath) : "");
  const sourceLabel = $derived.by(() => {
    const t = transcript as Transcript | null;
    if (!t) return "";
    if (t.source === "whisper") return `Whisper · ${t.language === "auto" ? "dil otomatik" : t.language}`;
    return `${siteLabel(item.site)} altyazısı · ${t.language}`;
  });
</script>

<svelte:window onkeydown={onKey} />

<div class="player" role="dialog" aria-label="Oynatıcı">
  <header class="bar">
    <button class="icon" title="Kapat (Esc)" onclick={onclose}><Icon name="x" size={20} /></button>
    <div class="ttl">
      <b title={item.title}>{item.title}</b>
      <span class="sub small">{[item.uploader, item.playlist, item.quality].filter(Boolean).join(" · ")}</span>
    </div>
    <div class="row">
      <button class="icon flip" title="Önceki" disabled={!prev} onclick={() => prev && onpick(prev)}><Icon name="arrow" size={18} /></button>
      <button class="icon" title="Sonraki" disabled={!next} onclick={() => next && onpick(next)}><Icon name="arrow" size={18} /></button>
      <button class="btn ghost sm" title="Varsayılan oynatıcıda aç" onclick={() => api.videos.open(item.id)}><Icon name="monitor" size={16} />Dışarıda aç</button>
      <button class="btn ghost sm" onclick={() => (showText = !showText, void setJson("player.showText", showText))}>
        <Icon name="note" size={16} />{showText ? "Yazıları gizle" : "Yazıları göster"}
      </button>
    </div>
  </header>

  <div class="body" class:notext={!showText}>
    <div class="stage">
      {#if playable}
        {#if item.audioOnly}
          <div class="cover">
            {#if item.thumbnail}<img src={/^(https?:|data:)/.test(item.thumbnail) ? item.thumbnail : api.fileSrc(item.thumbnail)} alt="" />{/if}
          </div>
          <audio
            bind:this={media}
            {src}
            controls
            autoplay
            onplay={() => (playing = true)}
            onpause={() => (playing = false)}
            onended={ended}
            onseeked={() => media && (time = media.currentTime)}
            onerror={() => (playable = false, startDemo())}
          ></audio>
        {:else}
          <!-- svelte-ignore a11y_media_has_caption -->
          <video
            bind:this={media}
            {src}
            controls
            autoplay
            onplay={() => (playing = true)}
            onpause={() => (playing = false)}
            onended={ended}
            onseeked={() => media && (time = media.currentTime)}
            onerror={() => (playable = false, startDemo())}
          ></video>
        {/if}
      {:else}
        <div class="noplay">
          <Icon name="film" size={36} />
          <p>Bu dosya uygulama içinde oynatılamıyor{item.filePath?.toLowerCase().endsWith(".mkv") ? " (MKV)" : ""}.</p>
          <button class="btn" onclick={() => api.videos.open(item.id)}><Icon name="play" size={18} />Varsayılan oynatıcıda aç</button>
          {#if !inTauri}<span class="sub small">(Tarayıcı önizlemesi: yazılar sahte bir saatle akıyor.)</span>{/if}
        </div>
      {/if}
      <label class="row autonext">
        <input type="checkbox" checked={autoNext} onchange={(e) => { autoNext = e.currentTarget.checked; void setJson("player.autoNext", autoNext); }} />
        <span class="sub small">Bitince sıradakine geç{next ? `: ${next.title}` : ""}</span>
      </label>
    </div>

    {#if showText}
      <aside class="text" class:big>
        <div class="thead">
          <span class="sub small">{sourceLabel || "Yazılar"}</span>
          {#if transcript}
            <span class="row">
              <button class="link" onclick={() => (big = !big, void setJson("player.bigLyrics", big))}>{big ? "Normal yazı" : "Büyük yazı"}</button>
              <button class="link" onclick={redo}>Yeniden çıkar</button>
            </span>
          {/if}
        </div>

        {#if loading}
          <p class="sub">Yükleniyor…</p>
        {:else if transcript && segments.length}
          <div
            class="lines"
            role="list"
            bind:this={lines}
            onwheel={() => (userScrolledAt = Date.now())}
            ontouchmove={() => (userScrolledAt = Date.now())}
          >
            {#each segments as s, i (i)}
              <button class="line" class:active={i === active} class:past={i < active} data-i={i} onclick={() => seek(s)}>
                <span class="ts tnum">{formatDuration(s.start) || "0:00"}</span>
                <span class="tx">
                  {#if i === active && s.words?.length}
                    {#each s.words as w, j (j)}<span class="w {wordState(s, w)}">{w.text}</span>{" "}{/each}
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
            <div class="pbar"><i style:width="{creating.stage === 'whisper' ? creating.progress * 100 : 8}%"></i></div>
            <span class="sub small">Uzun videolarda birkaç dakika sürebilir; bu sırada izlemeye devam edebilirsin.</span>
          </div>
        {:else}
          <div class="make">
            <h3 class="h2"><Icon name="note" />Yazıya dök</h3>
            <p class="sub small">
              Konuşmalar ya da şarkı sözleri burada zamanla akar; o an söylenen satır işaretlenir. Önce sitenin kendi
              altyazısına bakılır (YouTube'da kelime kelime), yoksa Whisper tamamen bu bilgisayarda dinleyip yazar.
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
              <div class="seg">
                {#each [["auto", "Otomatik"], ["subtitles", "Sadece altyazı"], ["whisper", "Sadece Whisper"]] as [k, l] (k)}
                  <button aria-pressed={mode === k} onclick={() => (mode = k)}>{l}</button>
                {/each}
              </div>
            </div>
            {#if error}<p class="err small">{error}</p>{/if}
            <button class="btn" onclick={create}><Icon name="sparkle" size={18} />Yazıya dök</button>

            {#if whisper && !whisper.ready && mode !== "subtitles"}
              <div class="wbox">
                <b>Whisper kurulu değil</b>
                <span class="sub small">Altyazısı olmayan videolar için gerekli. Bir kez iner, ses hiçbir yere gönderilmez.</span>
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
</div>

<style>
  .player { position: fixed; inset: 0; z-index: 30; display: grid; grid-template-rows: auto minmax(0, 1fr); background: var(--night); }
  .bar { display: flex; align-items: center; gap: 14px; padding: 10px 16px; border-bottom: 1px solid var(--line); }
  .ttl { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .ttl b { font-family: var(--f-display); font-size: 16px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .icon { border: 0; background: transparent; color: var(--muted); padding: 8px; border-radius: 9px; display: grid; place-items: center; }
  .icon:hover:not(:disabled) { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .icon:disabled { opacity: 0.3; cursor: default; }
  .flip :global(svg) { transform: scaleX(-1); }
  .sm { padding: 6px 11px; font-size: 13px; }
  .body { display: grid; grid-template-columns: minmax(0, 1fr) minmax(320px, 34%); min-height: 0; }
  .body.notext { grid-template-columns: minmax(0, 1fr); }
  .stage { display: flex; flex-direction: column; justify-content: center; align-items: center; gap: 12px; padding: 18px; min-height: 0; min-width: 0; }
  video { width: 100%; max-height: calc(100vh - 140px); background: #000; border-radius: 12px; }
  audio { width: min(560px, 100%); }
  .cover { width: min(420px, 70%); aspect-ratio: 1; border-radius: 18px; overflow: hidden; background: var(--coal-2); box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5), 0 0 40px rgba(227, 194, 112, 0.08); }
  .cover img { width: 100%; height: 100%; object-fit: cover; }
  .noplay { display: flex; flex-direction: column; align-items: center; gap: 12px; color: var(--muted); text-align: center; }
  .autonext { align-self: center; }
  .autonext input { accent-color: var(--gold); }
  .text { border-left: 1px solid var(--line); display: flex; flex-direction: column; min-height: 0; background: rgba(23, 26, 30, 0.6); }
  .thead { display: flex; justify-content: space-between; align-items: center; gap: 8px; padding: 12px 16px; border-bottom: 1px solid var(--line); }
  .lines { overflow: auto; padding: 30vh 10px; display: flex; flex-direction: column; gap: 2px; scroll-behavior: smooth; }
  .line { display: grid; grid-template-columns: 44px minmax(0, 1fr); gap: 10px; text-align: left; border: 0; background: transparent; padding: 7px 10px; border-radius: 10px; color: var(--muted); transition: color 0.2s, background 0.2s; font-size: 15px; line-height: 1.45; }
  .line:hover { background: rgba(255, 255, 255, 0.04); color: var(--text); }
  .line.past { color: var(--dim); }
  .line.active { color: var(--text); background: rgba(227, 194, 112, 0.1); box-shadow: inset 3px 0 0 var(--gold); }
  .line.active .tx { color: var(--gold-glow); font-weight: 600; }
  .ts { font-size: 11px; color: var(--dim); padding-top: 3px; }
  .w { transition: color 0.12s; color: var(--muted); }
  .w.now { color: var(--gold-glow); text-shadow: 0 0 12px rgba(240, 216, 120, 0.6); }
  .w.sung { color: var(--gold); }
  .big .line { font-size: 22px; font-family: var(--f-display); padding: 10px 12px; }
  .big .ts { display: none; }
  .big .line { grid-template-columns: minmax(0, 1fr); }
  .make { padding: 18px; display: flex; flex-direction: column; gap: 12px; overflow: auto; }
  .make p { margin: 0; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field > span { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .err { color: var(--bad); }
  .pbar { height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
  .pbar i { display: block; height: 100%; background: linear-gradient(90deg, var(--teal), var(--neon)); transition: width 0.3s; }
  .wbox { display: flex; flex-direction: column; gap: 8px; padding: 12px; border-radius: 12px; border: 1px solid rgba(53, 160, 162, 0.4); background: rgba(53, 160, 162, 0.06); }
  .wopt { display: flex; justify-content: space-between; align-items: center; gap: 10px; text-align: left; padding: 9px 12px; border-radius: 10px; border: 1px solid var(--line-2); background: var(--coal-2); }
  .wopt:hover:not(:disabled) { border-color: var(--gold); }
  .wopt > span:first-child { display: flex; flex-direction: column; }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; font-size: 12.5px; }
  .link:hover { color: var(--neon); }
  @media (max-width: 900px) {
    .body { grid-template-columns: minmax(0, 1fr); grid-template-rows: auto minmax(0, 1fr); }
    .text { border-left: 0; border-top: 1px solid var(--line); }
    video { max-height: 40vh; }
    .cover { width: 180px; }
  }
</style>
