<script lang="ts">
  // Klip kırpma ve Discord'a hazırlama: başlangıç/bitiş seç, hedef boyutu ve kodeği seç, hazırlamadan önce
  // beklenen netliği gör. Kısa seçim = daha net görüntü (aynı boyuta daha az saniye düşer).
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { notify } from "../toast.svelte";
  import type { Clip, FitOptions, FitPlan } from "../types";

  let { clip, onclose }: { clip: Clip; onclose: () => void } = $props();

  const LIMITS: { mb: number | null; label: string; sub: string }[] = [
    { mb: 10, label: "10 MB", sub: "Ücretsiz" },
    { mb: 50, label: "50 MB", sub: "Nitro Basic" },
    { mb: 500, label: "500 MB", sub: "Nitro" },
    { mb: null, label: "Sınırsız", sub: "Sadece kırp" },
  ];
  const QUALITY: Record<FitPlan["quality"], [string, string]> = {
    great: ["Çok net", "good"],
    good: ["Net", "good"],
    ok: ["Orta", "warn"],
    poor: ["Bulanık olur", "bad"],
  };

  const saved = getJson<{ limitMb?: number | null; codec?: FitOptions["codec"] }>("clips.discord", {});
  let limitMb = $state<number | null>(saved.limitMb === undefined ? 10 : saved.limitMb);
  let codec = $state<FitOptions["codec"]>(saved.codec ?? "av1");

  let video = $state<HTMLVideoElement | null>(null);
  let duration = $state(0);
  let current = $state(0);
  let start = $state(0);
  let end = $state(0);
  let looping = $state(false);
  let plan = $state<FitPlan | null>(null);
  let progress = $state<number | null>(null);

  const src = $derived(api.fileSrc(clip.path));
  const length = $derived(Math.max(0, end - start));
  const trimmed = $derived(start > 0.05 || (duration > 0 && end < duration - 0.05));

  onMount(() => {
    duration = clip.duration ?? 0;
    end = duration;
    const stop = on<{ id: string; progress: number }>("clip-fit", (e) => {
      if (e.id === clip.id && progress !== null) progress = e.progress;
    });
    return () => void stop.then((f) => f());
  });

  function loaded() {
    if (!video) return;
    if (Number.isFinite(video.duration) && video.duration > 0) {
      duration = video.duration;
      if (!end || end > duration) end = duration;
    }
  }

  function tick() {
    if (!video) return;
    current = video.currentTime;
    // "Seçimi oynat": seçimin sonunda başa dön.
    if (looping && current >= end) video.currentTime = start;
  }

  function playSelection() {
    if (!video) return;
    looping = true;
    video.currentTime = start;
    void video.play();
  }

  function setStart(t: number) {
    start = Math.max(0, Math.min(t, end - 0.5));
  }
  function setEnd(t: number) {
    end = Math.min(duration, Math.max(t, start + 0.5));
  }

  // Beklenen sonuç (kısa bir beklemeyle; kaydırırken her an istenmesin).
  let planTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const opts = options();
    clearTimeout(planTimer);
    planTimer = setTimeout(() => {
      void api.clips.plan(clip.id, opts).then((p) => (plan = p)).catch(() => (plan = null));
    }, 150);
  });

  function options(): FitOptions {
    return {
      start: trimmed ? start : null,
      end: trimmed ? end : null,
      limitMb,
      codec,
    };
  }

  function choose(mb: number | null) {
    limitMb = mb;
    void setJson("clips.discord", { limitMb, codec });
  }
  function chooseCodec(c: FitOptions["codec"]) {
    codec = c;
    void setJson("clips.discord", { limitMb, codec });
  }

  async function go() {
    progress = 0;
    video?.pause();
    try {
      await api.clips.fitDiscord(clip.id, options());
      if (limitMb === null) {
        notify("Kırpılmış kopya klibin yanına kaydedildi", "check");
        void api.clips.rescan();
      } else {
        notify("Discord'a hazır · panoya kopyalandı, Discord'da Ctrl+V", "check");
      }
      onclose();
    } catch (e) {
      notify(String(e), "info");
    } finally {
      progress = null;
    }
  }

  function fmt(t: number) {
    const m = Math.floor(t / 60);
    const s = t - m * 60;
    return `${m}:${s.toFixed(1).padStart(4, "0")}`;
  }
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
  <div class="dlg panel" role="dialog" aria-label="Kırp ve Discord'a hazırla">
    <div class="spread">
      <h3 class="h2"><Icon name="clip" />Kırp ve hazırla</h3>
      <button class="x" onclick={onclose} disabled={progress !== null} title="Kapat"><Icon name="x" size={18} /></button>
    </div>

    <div class="stage">
      <!-- svelte-ignore a11y_media_has_caption -->
      <video bind:this={video} {src} controls onloadedmetadata={loaded} ontimeupdate={tick} onpause={() => (looping = false)}></video>
    </div>

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

    <div class="opts">
      <div class="grp">
        <span class="label">Boyut</span>
        <div class="seg">
          {#each LIMITS as l (l.label)}
            <button aria-pressed={limitMb === l.mb} onclick={() => choose(l.mb)}><b>{l.label}</b><span>{l.sub}</span></button>
          {/each}
        </div>
      </div>
      {#if limitMb !== null}
        <div class="grp">
          <span class="label">Kodlama</span>
          <div class="seg">
            <button aria-pressed={codec === "av1"} onclick={() => chooseCodec("av1")} title="Aynı boyutta çok daha net. Discord bilgisayar uygulaması, tarayıcı ve yeni telefonlar oynatır.">
              <b>Net (AV1)</b><span>Önerilen</span>
            </button>
            <button aria-pressed={codec === "h264"} onclick={() => chooseCodec("h264")} title="Her cihazda oynar ama aynı boyutta daha bulanık.">
              <b>Uyumlu (H.264)</b><span>Her yerde oynar</span>
            </button>
          </div>
        </div>
      {/if}
    </div>

    {#if plan}
      {@const q = QUALITY[plan.quality]}
      <div class="plan {q[1]}">
        <Icon name={plan.quality === "poor" ? "info" : "check"} size={16} />
        <span>
          {#if limitMb === null}
            Orijinal kalitede ({clip.height ?? plan.height}p) kırpılır, klibin yanına kaydedilir.
          {:else}
            <b>{plan.height}p · {plan.fps} fps</b> · {q[0]}
            {#if plan.quality === "poor"} — seçimi kısalt ya da daha büyük boyut seç{:else if plan.quality === "ok" && !trimmed} — kırparsan daha net olur{/if}
          {/if}
        </span>
      </div>
    {/if}

    <div class="foot">
      {#if progress !== null}
        <div class="prog"><span class="small">Hazırlanıyor · %{Math.round(progress * 100)}</span><span class="pbar"><i style:width="{progress * 100}%"></i></span></div>
      {:else}
        <span class="sub small">{limitMb === null ? "Kopya klibin yanına kaydedilir; orijinale dokunulmaz." : "Kopya panoya gelir: Discord'da Ctrl+V. Orijinal klibe dokunulmaz."}</span>
      {/if}
      <button class="btn" disabled={progress !== null || length < 0.5} onclick={go}>
        <Icon name={limitMb === null ? "clip" : "copy"} size={15} />{limitMb === null ? "Kırpılmış kopyayı kaydet" : "Hazırla ve panoya kopyala"}
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
  .opts { display: flex; gap: 18px; flex-wrap: wrap; }
  .grp { display: flex; flex-direction: column; gap: 6px; }
  .label { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .seg { display: flex; gap: 6px; flex-wrap: wrap; }
  .seg button { display: flex; flex-direction: column; align-items: flex-start; gap: 1px; padding: 7px 12px; border-radius: 10px; border: 1px solid var(--line-2); background: var(--coal-2); color: var(--text); }
  .seg button span { font-size: 11px; color: var(--muted); }
  .seg button[aria-pressed="true"] { border-color: var(--gold); background: rgba(227, 194, 112, 0.12); }
  .seg button[aria-pressed="true"] b { color: var(--gold-glow); }
  .plan { display: flex; align-items: center; gap: 8px; padding: 9px 12px; border-radius: 10px; background: rgba(255, 255, 255, 0.04); }
  .plan.good { color: var(--good); }
  .plan.warn { color: var(--gold-glow); }
  .plan.bad { color: var(--bad); }
  .plan span { color: var(--text); }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
  .prog { flex: 1; display: flex; flex-direction: column; gap: 6px; min-width: 200px; }
  .pbar { display: block; height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
  .pbar i { display: block; height: 100%; background: var(--gold); transition: width 0.25s; }
</style>
