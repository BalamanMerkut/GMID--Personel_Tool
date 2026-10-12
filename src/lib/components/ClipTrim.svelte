<script lang="ts">
  // Discord'a hazırla: başlangıç/bitiş seç, hazır seçeneklerden birini seç (boyut, çözünürlük, kare hızı, beklenen
  // MB ve netlik yanında yazar), hazırla, sonucu burada izle; beğenmezsen başka seçenekle dene. Kısa seçim = daha
  // net görüntü (aynı boyuta daha az saniye düşer).
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { notify } from "../toast.svelte";
  import type { Clip, FitChoice, FitOptions, FitResult } from "../types";

  let { clip, onclose }: { clip: Clip; onclose: () => void } = $props();

  const QUALITY: Record<FitChoice["quality"], [string, string]> = {
    great: ["Çok net", "good"],
    good: ["Net", "good"],
    ok: ["Orta", "warn"],
    poor: ["Bulanık", "bad"],
  };
  /** Kırpılmışsa en sonda: boyut sınırı yok, orijinal kalitede kopya. */
  const UNLIMITED = "sinirsiz";

  const saved = getJson<{ key?: string }>("clips.discord", {});
  let selected = $state<string>(saved.key ?? "");

  let video = $state<HTMLVideoElement | null>(null);
  let duration = $state(0);
  let current = $state(0);
  let start = $state(0);
  let end = $state(0);
  let looping = $state(false);
  let choices = $state<FitChoice[]>([]);
  let progress = $state<number | null>(null);
  /** Bu pencerede hazırlananlar (karşılaştırmak için). */
  let results = $state<(FitResult & { title: string })[]>([]);
  /** Ortada oynayan: null = orijinal klip. */
  let preview = $state<(FitResult & { title: string }) | null>(null);

  const length = $derived(Math.max(0, end - start));
  const trimmed = $derived(start > 0.05 || (duration > 0 && end < duration - 0.05));
  const src = $derived(api.fileSrc(preview ? preview.path : clip.path));
  const list = $derived.by((): FitChoice[] => {
    if (!trimmed) return choices;
    return [
      ...choices,
      {
        key: UNLIMITED,
        title: "Sınırsız",
        note: "Sadece kırp · orijinal kalite, klibin yanına kaydedilir",
        options: { start, end, limitMb: null, codec: "h264" },
        height: clip.height ?? 1080,
        fps: 0,
        size: clip.size * (length / Math.max(duration, 0.1)),
        quality: "great",
        recommended: false,
      },
    ];
  });
  const choice = $derived(list.find((c) => c.key === selected) ?? list.find((c) => c.recommended) ?? list[0] ?? null);
  /** Ücretsiz sınırda en iyisi bile net değilse: kısaltmayı öner. */
  const suggestCut = $derived(
    length > 40 && !choices.some((c) => c.options.limitMb === 10 && (c.quality === "great" || c.quality === "good")),
  );

  onMount(() => {
    duration = clip.duration ?? 0;
    end = duration;
    const stop = on<{ id: string; progress: number }>("clip-fit", (e) => {
      if (e.id === clip.id && progress !== null) progress = e.progress;
    });
    return () => void stop.then((f) => f());
  });

  function loaded() {
    if (!video || preview) return;
    if (Number.isFinite(video.duration) && video.duration > 0) {
      duration = video.duration;
      if (!end || end > duration) end = duration;
    }
  }

  function tick() {
    if (!video || preview) return;
    current = video.currentTime;
    // "Seçimi oynat": seçimin sonunda başa dön.
    if (looping && current >= end) video.currentTime = start;
  }

  function playSelection() {
    if (!video) return;
    preview = null;
    looping = true;
    queueMicrotask(() => {
      if (!video) return;
      video.currentTime = start;
      void video.play();
    });
  }

  function setStart(t: number) {
    start = Math.max(0, Math.min(t, end - 0.5));
  }
  function setEnd(t: number) {
    end = Math.min(duration, Math.max(t, start + 0.5));
  }
  function lastSeconds(n: number) {
    start = Math.max(0, duration - n);
    end = duration;
  }

  // Seçenekler (kısa bir beklemeyle; kaydırırken her an istenmesin).
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const s = trimmed ? start : null;
    const e = trimmed ? end : null;
    if (!duration) return;
    clearTimeout(timer);
    timer = setTimeout(() => {
      void api.clips.choices(clip.id, s, e).then((c) => (choices = c)).catch(() => (choices = []));
    }, 200);
  });

  function pick(c: FitChoice) {
    selected = c.key;
    if (c.options.limitMb !== null) {
      void setJson("clips.discord", { key: c.key, limitMb: c.options.limitMb, codec: c.options.codec, smooth: !!c.options.smooth });
    }
  }

  async function go() {
    if (!choice) return;
    const c = choice;
    progress = 0;
    video?.pause();
    const opts: FitOptions = { ...c.options, start: trimmed ? start : null, end: trimmed ? end : null };
    try {
      const r = await api.clips.fitDiscord(clip.id, opts);
      const item = { ...r, title: c.title };
      if (c.options.limitMb === null) {
        notify("Kırpılmış kopya klibin yanına kaydedildi", "check");
        void api.clips.rescan();
      } else {
        notify(`${c.title} · ${mb(r.size)} hazır · panoya kopyalandı, Discord'da Ctrl+V`, "check");
      }
      results = [item, ...results.filter((x) => x.path !== r.path)];
      preview = item;
    } catch (e) {
      notify(String(e), "info");
    } finally {
      progress = null;
    }
  }

  async function copyAgain(r: FitResult) {
    try {
      await api.clips.readyCopy(r.path);
      notify("Panoya kopyalandı · Discord'da Ctrl+V", "copy");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  function fmt(t: number) {
    const m = Math.floor(t / 60);
    const s = t - m * 60;
    return `${m}:${s.toFixed(1).padStart(4, "0")}`;
  }
  const mb = (b: number) => `${(b / 1e6).toLocaleString("tr-TR", { maximumFractionDigits: b < 1e8 ? 1 : 0 })} MB`;
  const res = (c: { height: number; fps: number }) => `${c.height}p${c.fps ? ` · ${c.fps} fps` : ""}`;
  const pct = (t: number) => (duration ? (t / duration) * 100 : 0);

  function seekBar(e: MouseEvent) {
    if (!video || !duration) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    video.currentTime = ((e.clientX - r.left) / r.width) * duration;
    looping = false;
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape" && progress === null) {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && progress === null && onclose()}>
  <div class="dlg panel" role="dialog" aria-label="Discord'a hazırla">
    <div class="spread">
      <h3 class="h2"><Icon name="clip" />Discord'a hazırla</h3>
      <button class="x" onclick={onclose} disabled={progress !== null} title="Kapat"><Icon name="x" size={18} /></button>
    </div>

    <div class="stage">
      <!-- svelte-ignore a11y_media_has_caption -->
      <video bind:this={video} {src} controls autoplay={!!preview} onloadedmetadata={loaded} ontimeupdate={tick} onpause={() => (looping = false)}></video>
      {#if preview}
        <div class="pv">
          <span><b>Hazırlanan:</b> {preview.title} · {mb(preview.size)} · {res(preview)}</span>
          <button class="btn ghost sm" onclick={() => (preview = null)}>Orijinale dön</button>
        </div>
      {/if}
    </div>

    {#if !preview}
      <div class="timeline">
        <div class="bar" role="presentation" onclick={seekBar}>
          <span class="sel" style:left="{pct(start)}%" style:width="{pct(end) - pct(start)}%"></span>
          <span class="head" style:left="{pct(current)}%"></span>
        </div>
        <input class="r start" type="range" min="0" max={duration || 1} step="0.1" value={start} oninput={(e) => setStart(Number(e.currentTarget.value))} aria-label="Başlangıç" />
        <input class="r end" type="range" min="0" max={duration || 1} step="0.1" value={end} oninput={(e) => setEnd(Number(e.currentTarget.value))} aria-label="Bitiş" />
      </div>
      <div class="row times">
        <button class="btn ghost sm" onclick={() => setStart(current)}>[ Başlangıç: şu an</button>
        <span class="tnum sub">{fmt(start)} – {fmt(end)} · <b>{fmt(length)}</b></span>
        <button class="btn ghost sm" onclick={() => setEnd(current)}>Bitiş: şu an ]</button>
        <button class="btn ghost sm" onclick={playSelection}><Icon name="play" size={14} />Seçimi oynat</button>
        {#if trimmed}<button class="btn ghost sm" onclick={() => ((start = 0), (end = duration))}>Tamamı</button>{/if}
      </div>
    {/if}

    {#if suggestCut && !preview}
      <div class="tip">
        <Icon name="info" size={16} />
        <span>{fmt(length)} uzunluğunda bir video 10 MB'a ancak bulanık sığar. Kısa kesersen çok daha net olur: anlık kayıtlarda aksiyon genelde sondadır.</span>
        <button class="btn sm" onclick={() => lastSeconds(30)}>Son 30 sn</button>
        <button class="btn ghost sm" onclick={() => lastSeconds(15)}>Son 15 sn</button>
      </div>
    {/if}

    <div class="choices">
      {#each list as c (c.key)}
        {@const q = QUALITY[c.quality]}
        <button class="ch" aria-pressed={choice?.key === c.key} onclick={() => pick(c)}>
          <span class="top"><b>{c.title}</b>{#if c.recommended}<em>Önerilen</em>{/if}</span>
          <span class="size tnum">≈ {mb(c.size)}</span>
          <span class="meta tnum">{res(c)}{c.options.limitMb !== null && c.key !== "orijinal" && !c.key.startsWith("orijinal") ? ` · ${c.options.codec === "av1" ? "AV1" : "H.264"}` : ""}</span>
          <span class="q {q[1]}">{q[0]}</span>
          <span class="note">{c.note}</span>
        </button>
      {:else}
        <p class="sub small">Seçenekler hesaplanıyor…</p>
      {/each}
    </div>

    {#if results.length}
      <div class="results">
        <span class="label">Hazırlananlar</span>
        {#each results as r (r.path)}
          <div class="res" class:on={preview?.path === r.path}>
            <span><b>{r.title}</b> · {mb(r.size)} · {res(r)}</span>
            <button class="btn ghost sm" onclick={() => (preview = r)}><Icon name="play" size={13} />İzle</button>
            {#if !r.original}
              <button class="btn ghost sm" onclick={() => copyAgain(r)}><Icon name="copy" size={13} />Panoya</button>
              <button class="btn ghost sm" onclick={() => api.clips.readyReveal(r.path)}><Icon name="folder" size={13} />Klasör</button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    <div class="foot">
      {#if progress !== null}
        <div class="prog"><span class="small">Hazırlanıyor · %{Math.round(progress * 100)}</span><span class="pbar"><i style:width="{progress * 100}%"></i></span></div>
      {:else}
        <span class="sub small">
          {choice?.options.limitMb === null ? "Kopya klibin yanına kaydedilir; orijinale dokunulmaz." : "Kopya panoya gelir: Discord'da Ctrl+V. Beğenmezsen başka seçenekle yeniden dene; orijinale dokunulmaz."}
        </span>
      {/if}
      <button class="btn" disabled={progress !== null || length < 0.5 || !choice} onclick={go}>
        <Icon name={choice?.options.limitMb === null ? "clip" : "copy"} size={15} />
        {choice?.options.limitMb === null ? "Kırpılmış kopyayı kaydet" : choice ? `${choice.title} · hazırla ve kopyala` : "Hazırla"}
      </button>
    </div>
  </div>
</div>

<style>
  .back { position: fixed; inset: 0; z-index: 70; background: rgba(0, 0, 0, 0.72); display: grid; place-items: center; padding: 24px; }
  .dlg { width: min(980px, 100%); max-height: 100%; overflow: auto; background: var(--night-2); box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6); display: flex; flex-direction: column; gap: 12px; }
  .spread { display: flex; justify-content: space-between; align-items: center; }
  .h2 { margin: 0; }
  .x { border: 0; background: none; color: var(--muted); display: grid; place-items: center; width: 32px; height: 32px; border-radius: 8px; }
  .x:hover { background: rgba(255, 255, 255, 0.06); color: var(--text); }
  .stage { background: #000; border-radius: 12px; overflow: hidden; }
  .stage video { width: 100%; max-height: 52vh; display: block; object-fit: contain; }
  .timeline { position: relative; height: 34px; }
  .bar { position: absolute; left: 0; right: 0; top: 11px; height: 12px; border-radius: 6px; background: rgba(255, 255, 255, 0.08); cursor: pointer; }
  .sel { position: absolute; top: 0; bottom: 0; background: rgba(227, 194, 112, 0.45); border-left: 3px solid var(--gold); border-right: 3px solid var(--gold); border-radius: 4px; }
  .head { position: absolute; top: -5px; bottom: -5px; width: 2px; background: #fff; transform: translateX(-1px); pointer-events: none; }
  /* İki sürgü üst üste: yalnız tutamaklar tıklanır, çubuğa tıklamak oynatma yerini değiştirir. */
  .r { position: absolute; left: 0; right: 0; top: 7px; width: 100%; margin: 0; background: none; pointer-events: none; appearance: none; height: 20px; }
  .r::-webkit-slider-runnable-track { background: none; height: 20px; }
  .r::-webkit-slider-thumb { appearance: none; pointer-events: auto; width: 14px; height: 26px; margin-top: -3px; border-radius: 5px; background: var(--gold); border: 2px solid #1b1608; cursor: ew-resize; }
  .times { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .times .tnum { margin: 0 auto; }
  .stage { position: relative; }
  .pv { position: absolute; left: 10px; right: 10px; top: 10px; display: flex; align-items: center; justify-content: space-between; gap: 10px; padding: 6px 8px 6px 12px; border-radius: 10px; background: rgba(18, 20, 23, 0.86); font-size: 13px; }
  .tip { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-radius: 12px; background: rgba(227, 194, 112, 0.08); border: 1px solid rgba(227, 194, 112, 0.35); font-size: 13px; color: var(--muted); flex-wrap: wrap; }
  .tip > :global(svg) { color: var(--gold); flex: none; }
  .tip span { flex: 1 1 300px; }
  .label { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .choices { display: grid; grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); gap: 8px; }
  .ch { display: flex; flex-direction: column; align-items: flex-start; gap: 3px; padding: 10px 12px; border-radius: 12px; border: 1px solid var(--line-2); background: var(--coal-2); color: var(--text); text-align: left; }
  .ch:hover { border-color: var(--gold-dim); }
  .ch[aria-pressed="true"] { border-color: var(--gold); background: rgba(227, 194, 112, 0.12); }
  .ch .top { display: flex; align-items: center; gap: 6px; width: 100%; }
  .ch em { margin-left: auto; font-style: normal; font-size: 10.5px; font-weight: 700; padding: 1px 7px; border-radius: 999px; background: var(--gold); color: #1b1608; }
  .ch .size { font-family: var(--f-display); font-size: 20px; font-weight: 700; }
  .ch .meta { font-size: 12.5px; color: var(--muted); }
  .ch .q { font-size: 11.5px; font-weight: 700; }
  .ch .q.good { color: var(--good); }
  .ch .q.warn { color: var(--gold-glow); }
  .ch .q.bad { color: var(--bad); }
  .ch .note { font-size: 11.5px; color: var(--dim); line-height: 1.4; }
  .results { display: flex; flex-direction: column; gap: 6px; }
  .res { display: flex; align-items: center; gap: 8px; padding: 6px 8px 6px 12px; border-radius: 10px; background: rgba(255, 255, 255, 0.04); font-size: 13px; flex-wrap: wrap; }
  .res span { flex: 1 1 220px; }
  .res.on { outline: 1px solid var(--gold-dim); }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
  .prog { flex: 1; display: flex; flex-direction: column; gap: 6px; min-width: 200px; }
  .pbar { display: block; height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
  .pbar i { display: block; height: 100%; background: var(--gold); transition: width 0.25s; }
</style>
