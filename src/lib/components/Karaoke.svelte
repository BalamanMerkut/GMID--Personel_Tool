<script lang="ts">
  // Karaoke: şarkının sesi ayrılmış müziği çalar, sözler ortada kelime kelime dolar; mikrofonla söyleneni kaydedip
  // müzikle karıştırır. Ayırma bir kez yapılır (Rust: Demucs), sonra hep hazırdır.
  import { onDestroy, onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import { formatDuration } from "../downloads.svelte";
  import Icon from "../Icon.svelte";
  import { fill, wordsOf } from "../karaoke";
  import { getJson, setJson } from "../settings.svelte";
  import { notify } from "../toast.svelte";
  import type { Download, KaraokeHit, KaraokeProgress, KaraokeRecording, KaraokeStatus, Segment } from "../types";

  let {
    item,
    segments,
    start = 0,
    onclose,
  }: { item: Download; segments: Segment[]; start?: number; onclose: (time: number) => void } = $props();

  let status = $state<KaraokeStatus | null>(null);
  let prog = $state<KaraokeProgress | null>(null);
  let preparing = $state(false);
  let error = $state("");

  let music: HTMLAudioElement | undefined = $state();
  let vocals: HTMLAudioElement | undefined = $state();
  let time = $state(0);
  let duration = $state(0);
  let playing = $state(false);
  /** Orijinal sesin seviyesi (0: yalnız müzik). */
  let guide = $state(getJson("karaoke.guide", 0));
  let volume = $state(getJson("karaoke.volume", 0.9));
  /** Kulaklıkla: yankı engelleme kapalı, ses daha temiz. Hoparlörde açık kalmalı (müzik mikrofona girmesin). */
  let headphones = $state(getJson("karaoke.headphones", true));

  // Kayıt
  let recording = $state(false);
  let recStart = 0;
  let recSongStart = 0;
  let recElapsed = $state(0);
  let level = $state(0);
  let stream: MediaStream | null = null;
  let recorder: MediaRecorder | null = null;
  let chunks: Blob[] = [];
  let meterCtx: AudioContext | null = null;
  let latencyGuess = 0.15;

  // Sonuç
  type Result = { voice: string; path: string; songStart: number };
  let result = $state<Result | null>(null);
  let mixing = $state(false);
  let latencyMs = $state(150);
  // Ses ve müzik kaydedilirken aynı yüksekliğe getirilir (Rust); bunlar onun üzerine ayar (1 = dengeli).
  let voiceGain = $state(getJson("karaoke.voiceGain2", 1));
  let musicGain = $state(getJson("karaoke.musicGain", 1));
  /** Kayıt boyunca mikrofonun en yüksek seviyesi: hiç ses gelmediyse uyarılır. */
  let maxLevel = 0;
  let quietMic = $state(false);
  let echo = $state(getJson("karaoke.echo", true));
  let recordings = $state<KaraokeRecording[]>([]);
  let previewKey = $state(0);

  onMount(() => {
    void load();
    const stop = on<KaraokeProgress>("karaoke-progress", (p) => {
      if (p.id === item.id) prog = p;
    });
    return () => void stop.then((f) => f());
  });

  async function load() {
    status = await api.karaoke.status(item.id).catch(() => null);
    recordings = await api.karaoke.recordings().catch(() => []);
  }

  /** `cpu`: ekran kartını hiç deneme (sonuç bozuk duyulduysa). */
  async function prepare(cpu = false) {
    music?.pause();
    vocals?.pause();
    if (status) status = { ...status, ready: false };
    preparing = true;
    error = "";
    prog = null;
    try {
      status = await api.karaoke.prepare(item.id, cpu);
    } catch (e) {
      error = String(e);
    } finally {
      preparing = false;
    }
  }

  function stageText(p: KaraokeProgress | null): string {
    if (!p) return "Başlıyor…";
    const pct = `%${Math.round(p.progress * 100)}`;
    if (p.stage === "runtime") return `Ses ayırma programı indiriliyor… ${pct}`;
    if (p.stage === "model") return `Yapay zekâ modeli indiriliyor (bir kez)… ${pct}`;
    if (p.stage === "download") return "İndiriliyor…";
    if (p.stage === "align") return "Sözlere göre hizalanıyor…";
    if (p.stage === "decode") return "Şarkı okunuyor…";
    if (p.stage === "check") return "Ekran kartının sonucu işlemciyle karşılaştırılıyor…";
    if (p.stage === "separate") return `Ses müzikten ayrılıyor${p.gpu ? " (ekran kartıyla)" : " (işlemciyle)"}… ${pct}`;
    return "Kaydediliyor…";
  }

  // ---- çalma ----
  const src = (p: string | null | undefined) => (p ? api.fileSrc(p) : "");
  let raf = 0;
  let lastSync = 0;
  function frame() {
    if (music) {
      time = music.currentTime;
      // İki dosya aynı anda başlar; kayarsa vokal müziğe yetişir.
      // En çok saniyede bir düzeltilir: sık sık atlatmak cızırtı yapar.
      const now = performance.now();
      if (vocals && !music.paused && now - lastSync > 1000 && Math.abs(vocals.currentTime - music.currentTime) > 0.12) {
        vocals.currentTime = music.currentTime;
        lastSync = now;
      }
    }
    if (recording) recElapsed = (performance.now() - recStart) / 1000;
    raf = requestAnimationFrame(frame);
  }
  onMount(() => {
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  });

  function onMeta() {
    if (!music) return;
    duration = music.duration || item.duration || 0;
    if (start > 0 && start < duration - 1) {
      music.currentTime = start;
      if (vocals) vocals.currentTime = start;
    }
  }
  function toggle() {
    if (!music) return;
    if (music.paused) {
      void music.play().catch((e) => (error = `Çalınamadı: ${e}`));
      void vocals?.play().catch(() => {});
    } else {
      music.pause();
      vocals?.pause();
    }
  }
  function seek(t: number) {
    if (!music) return;
    music.currentTime = Math.max(0, Math.min(duration || t, t));
    if (vocals) vocals.currentTime = music.currentTime;
    time = music.currentTime;
  }
  $effect(() => {
    if (vocals) vocals.volume = Math.min(1, guide * volume);
    if (music) music.volume = volume;
  });

  // ---- sözler ----
  const active = $derived.by(() => {
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
  const words = $derived(active >= 0 ? wordsOf(segments, active) : []);
  /** Sözler başlamadan geri sayım (ilk satıra 3 sn kala). */
  const countdown = $derived(segments.length && active < 0 ? Math.ceil(segments[0].start - time) : 0);

  // ---- kayıt ----
  async function startRec() {
    error = "";
    try {
      stream = await navigator.mediaDevices.getUserMedia({
        audio: { echoCancellation: !headphones, noiseSuppression: false, autoGainControl: true },
      });
    } catch {
      error = "Mikrofona izin verilmedi ya da mikrofon bulunamadı.";
      return;
    }
    const mime = ["audio/webm;codecs=opus", "audio/webm", "audio/ogg;codecs=opus"].find((m) => MediaRecorder.isTypeSupported(m)) ?? "";
    recorder = new MediaRecorder(stream, mime ? { mimeType: mime, audioBitsPerSecond: 160_000 } : undefined);
    chunks = [];
    recorder.ondataavailable = (e) => e.data.size && chunks.push(e.data);
    recorder.onstart = () => {
      recSongStart = music?.currentTime ?? 0;
      recStart = performance.now();
      recording = true;
    };
    recorder.onstop = () => void finishRec(recorder?.mimeType ?? mime);
    // Seviye göstergesi; aynı bağlamdan çıkış gecikmesi tahmini (kulaklıktan duyduğun ile mikrofonun arası).
    meterCtx = new AudioContext();
    const an = meterCtx.createAnalyser();
    an.fftSize = 512;
    meterCtx.createMediaStreamSource(stream).connect(an);
    const data = new Float32Array(an.fftSize);
    const tick = () => {
      if (!meterCtx) return;
      an.getFloatTimeDomainData(data);
      let peak = 0;
      for (const v of data) peak = Math.max(peak, Math.abs(v));
      level = level * 0.7 + peak * 0.3;
      maxLevel = Math.max(maxLevel, peak);
      requestAnimationFrame(tick);
    };
    tick();
    latencyGuess = Math.min(0.4, Math.max(0.08, (meterCtx.outputLatency || 0.04) + (meterCtx.baseLatency || 0.01) + 0.08));
    maxLevel = 0;
    quietMic = false;
    recorder.start(500);
    if (music?.paused) toggle();
  }

  function stopRec() {
    recording = false;
    recorder?.stop();
    stream?.getTracks().forEach((t) => t.stop());
    void meterCtx?.close();
    meterCtx = null;
    level = 0;
    if (music && !music.paused) toggle();
  }

  async function finishRec(mime: string) {
    const blob = new Blob(chunks, { type: mime || "audio/webm" });
    chunks = [];
    if (blob.size < 2000) {
      error = "Kayıt çok kısa.";
      return;
    }
    const data = await new Promise<string>((res) => {
      const r = new FileReader();
      r.onload = () => res(String(r.result));
      r.readAsDataURL(blob);
    });
    quietMic = maxLevel < 0.02;
    mixing = true;
    try {
      const voice = await api.karaoke.voice(data, mime.includes("ogg") ? "ogg" : "webm");
      latencyMs = Math.round(latencyGuess * 1000);
      const path = await api.karaoke.mix(item.id, mixOpts(voice, recSongStart, null));
      result = { voice, path, songStart: recSongStart };
      previewKey++;
      recordings = await api.karaoke.recordings().catch(() => recordings);
    } catch (e) {
      error = String(e);
    } finally {
      mixing = false;
    }
  }

  const mixOpts = (voice: string, songStart: number, output: string | null) => ({
    voice,
    songStart,
    latency: latencyMs / 1000,
    voiceGain,
    musicGain,
    echo,
    output,
    title: item.title,
  });

  async function remix() {
    if (!result) return;
    mixing = true;
    void setJson("karaoke.voiceGain2", voiceGain);
    void setJson("karaoke.musicGain", musicGain);
    void setJson("karaoke.echo", echo);
    try {
      await api.karaoke.mix(item.id, mixOpts(result.voice, result.songStart, result.path));
      previewKey++;
      notify("Kayıt güncellendi", "check");
    } catch (e) {
      error = String(e);
    } finally {
      mixing = false;
    }
  }

  async function removeResult() {
    if (!result) return;
    await api.karaoke.remove(result.path).catch(() => {});
    result = null;
    recordings = await api.karaoke.recordings().catch(() => []);
  }

  function close() {
    if (recording) stopRec();
    onclose(time);
  }

  // ---- internetten temiz müzik (şarkının resmî enstrümantal / karaoke sürümü) ----
  let finder = $state(false);
  let query = $state("");
  let hits = $state<KaraokeHit[]>([]);
  let searching = $state(false);
  let fetching = $state<string | null>(null);
  /** "Sanatçı - Şarkı (Official Video) [4K]" → "Sanatçı - Şarkı" */
  const cleanTitle = (t: string) =>
    t
      .replace(/[([][^)\]]*(official|video|lyric|klip|audio|hd|4k|music|söz|sozleri|şarkı sözleri)[^)\]]*[)\]]/gi, "")
      .replace(/\s+/g, " ")
      .trim();
  function openFinder(kind = "instrumental") {
    finder = true;
    query = `${cleanTitle(item.title)} ${kind}`;
    void search();
  }
  async function search() {
    searching = true;
    error = "";
    try {
      hits = await api.karaoke.search(query);
    } catch (e) {
      error = String(e);
      hits = [];
    } finally {
      searching = false;
    }
  }
  async function useHit(h: KaraokeHit) {
    music?.pause();
    vocals?.pause();
    fetching = h.url;
    error = "";
    prog = null;
    try {
      status = await api.karaoke.online(item.id, h.url, h.title);
      finder = false;
      notify("Temiz müzik hazır", "check");
    } catch (e) {
      error = String(e);
    } finally {
      fetching = null;
    }
  }
  async function dropOnline() {
    music?.pause();
    status = await api.karaoke.onlineRemove(item.id).catch(() => status);
  }
  /** Bulunan sürümün süresi şarkıdan ne kadar farklı (sn). */
  const durDiff = (h: KaraokeHit) => (h.duration && item.duration ? Math.round(h.duration - item.duration) : null);

  function onKey(e: KeyboardEvent) {
    const t = e.target as HTMLElement;
    if (t.closest("input, textarea")) return;
    if (e.key === "Escape") {
      e.preventDefault();
      if (finder) finder = false;
      else if (result) result = null;
      else close();
    } else if (e.key === " " && status?.ready && !result && !finder) {
      e.preventDefault();
      toggle();
    }
  }

  onDestroy(() => {
    if (recording) stopRec();
    stream?.getTracks().forEach((t) => t.stop());
    void meterCtx?.close();
  });

  const cover = $derived(item.thumbnail ? (/^(https?:|data:)/.test(item.thumbnail) ? item.thumbnail : api.fileSrc(item.thumbnail)) : "");
  const pct = $derived(duration ? (time / duration) * 100 : 0);
</script>

<svelte:window onkeydown={onKey} />

<div class="kara" role="dialog" aria-label="Karaoke">
  {#if cover}<div class="kbg" style:background-image={`url("${cover}")`}></div>{/if}
  <div class="kshade"></div>

  <header class="kbar">
    <button class="btn ghost sm" onclick={close}><Icon name="back" size={16} />Oynatıcıya dön</button>
    <div class="kttl">
      <span class="kick"><Icon name="mic" size={14} />Karaoke</span>
      <b title={item.title}>{item.title}</b>
    </div>
    <button class="btn ghost sm" onclick={() => api.openDataFolder("Karaoke")}><Icon name="folder" size={16} />Kayıtlar</button>
  </header>

  {#if !status}
    <div class="center"><p class="sub">Yükleniyor…</p></div>
  {:else if !status.ready}
    <div class="center">
      <div class="setup">
        <span class="bigicon"><Icon name="mic" size={34} /></span>
        <h2>Müzik nereden gelsin?</h2>
        <button class="choice rec" onclick={() => openFinder()} disabled={preparing}>
          <span class="ctop"><Icon name="search" size={18} /><b>İnternetten temiz müzik bul</b><span class="tag">En temiz</span></span>
          <span class="cdesc">Şarkının resmî enstrümantal ya da karaoke sürümü YouTube'da aranır; müzik stüdyodaki gibi temiz olur. Seçtiğin sürüm sözlere göre kendiliğinden hizalanır.</span>
        </button>
        <div class="choice">
          <span class="ctop"><Icon name="sparkle" size={18} /><b>Yapay zekâyla ayır</b></span>
          <span class="cdesc">
            Şarkıcının sesi bu bilgisayarda müzikten ayrılır (Meta'nın Demucs modeli). İnternette sürümü olmayan şarkılar için; ses
            girince müzik biraz boğuklaşabilir. Şarkı başına bir kez, 1–3 dakika.{#if status.downloadMb}{" "}İlk seferde ~{status.downloadMb} MB iner.{/if}
          </span>
          {#if preparing}
            <div class="kprog">
              <span>{stageText(prog)}</span>
              <div class="pbar" class:busy={!prog || prog.stage === "decode" || prog.stage === "save" || prog.progress === 0}>
                <i style:width="{(prog?.progress ?? 0) * 100}%"></i>
              </div>
              <button class="btn ghost sm" onclick={() => api.karaoke.cancel()}>Durdur</button>
            </div>
          {:else}
            <button class="btn sm" onclick={() => prepare()} disabled={!!status.busy && status.busy !== item.id}><Icon name="sparkle" size={15} />Ayır</button>
            {#if status.busy && status.busy !== item.id}<span class="sub small">Başka bir şarkı hazırlanıyor; bitince dene.</span>{/if}
          {/if}
        </div>
        {#if error}<p class="err">{error}</p>{/if}
      </div>
    </div>
  {:else}
    <audio bind:this={music} src={src(status.music)} preload="auto" onloadedmetadata={onMeta} onplay={() => (playing = true)} onpause={() => (playing = false)} onended={() => (recording ? stopRec() : (playing = false))} onerror={() => (error = "Müzik dosyası açılamadı. Karaoke klasörü silinmiş ya da taşınmış olabilir; oynatıcıya dönüp yeniden dene.")}></audio>
    <audio bind:this={vocals} src={src(status.vocals)} preload="auto"></audio>

    <section class="lyrics" aria-live="polite">
      {#if !segments.length}
        <p class="nolyr">Bu şarkının sözleri yok. Oynatıcıya dönüp <b>Yazılar</b>'dan sözleri bul ya da çıkar; müzik yine de hazır.</p>
      {:else}
        {#if countdown > 0 && countdown <= 3}
          <div class="count" aria-hidden="true">{countdown}</div>
        {/if}
        <p class="ln prev">{active > 0 ? segments[active - 1].text : ""}</p>
        <p class="ln cur">
          {#if active >= 0}
            {#each words as w, j (j)}{@const p = fill(w, time)}<span class="kw" class:now={p > 0 && p < 1} style:--p={p}>{w.text}</span>{" "}{/each}
          {:else}
            <span class="sub">♪ ♪ ♪</span>
          {/if}
        </p>
        <p class="ln next">{segments[active + 1]?.text ?? ""}</p>
        <p class="ln next2">{segments[active + 2]?.text ?? ""}</p>
      {/if}
    </section>

    <footer class="kctl">
      <div class="seekrow">
        <span class="tnum sub small">{formatDuration(time) || "0:00"}</span>
        <input class="seek" type="range" min="0" max={duration || 1} step="0.1" value={time} oninput={(e) => seek(Number(e.currentTarget.value))} aria-label="Konum" style:--pct="{pct}%" disabled={recording} />
        <span class="tnum sub small">{formatDuration(duration) || "0:00"}</span>
      </div>
      <div class="ctlrow">
        {#if status.vocals}
          <div class="knob">
            <label for="kguide"><Icon name="mic" size={15} />Orijinal ses <b class="tnum">%{Math.round(guide * 100)}</b></label>
            <input id="kguide" type="range" min="0" max="1" step="0.05" bind:value={guide} onchange={() => void setJson("karaoke.guide", guide)} />
          </div>
        {:else}
          <div></div>
        {/if}
        <div class="mid">
          <button class="kplay" title={playing ? "Duraklat (Boşluk)" : "Çal (Boşluk)"} onclick={toggle} disabled={recording}>
            <Icon name={playing ? "pause" : "play"} size={26} />
          </button>
          {#if recording}
            <button class="krec on" onclick={stopRec} title="Kaydı bitir">
              <span class="dot"></span>Bitir <span class="tnum">{formatDuration(recElapsed) || "0:00"}</span>
              <span class="lvl"><i style:width="{Math.min(100, level * 140)}%"></i></span>
            </button>
          {:else}
            <button class="krec" onclick={startRec} disabled={mixing} title="Mikrofonla kaydet: müzik baştan değil, şu anki yerden çalar">
              <span class="dot"></span>{mixing ? "Karıştırılıyor…" : "Kaydet"}
            </button>
          {/if}
        </div>
        <div class="knob">
          <label for="kvol"><Icon name="speaker" size={15} />Müzik <b class="tnum">%{Math.round(volume * 100)}</b></label>
          <input id="kvol" type="range" min="0" max="1" step="0.05" bind:value={volume} onchange={() => void setJson("karaoke.volume", volume)} />
        </div>
      </div>
      <label class="hp">
        <input type="checkbox" bind:checked={headphones} onchange={() => void setJson("karaoke.headphones", headphones)} disabled={recording} />
        Kulaklık takılı <span class="sub small">(hoparlörden çalıyorsa kapat: müziğin mikrofona girmesi engellenir)</span>
      </label>
      <p class="redo sub small">
        {#if status.online}
          Müzik: <b>{status.online.title}</b> (YouTube){#if status.online.score < 0.12}<span class="warn"> · şarkıyla tam uyuşmayabilir</span>{/if}
          · <button class="link" onclick={() => openFinder()} disabled={recording}>Değiştir</button>
          {#if status.separated} · <button class="link" onclick={dropOnline} disabled={recording}>Yapay zekâyla ayrılana dön</button>{/if}
        {:else}
          Müzik yapay zekâyla ayrıldı. Boğuk mu? <button class="link" onclick={() => openFinder()} disabled={recording}>İnternetten temiz müzik bul</button>
          · <button class="link" onclick={() => prepare(true)} disabled={recording}>İşlemciyle yeniden ayır</button>
        {/if}
      </p>
      {#if error}<p class="err">{error}</p>{/if}
    </footer>
  {/if}

  {#if finder}
    <div class="rback" role="presentation" onclick={(e) => e.target === e.currentTarget && !fetching && (finder = false)}>
      <div class="rcard finder" role="dialog" aria-label="Temiz müzik bul">
        <h3><Icon name="search" size={18} />Temiz müzik bul</h3>
        <form class="frow" onsubmit={(e) => (e.preventDefault(), void search())}>
          <input bind:value={query} aria-label="Ara" disabled={!!fetching} />
          <button class="btn" disabled={searching || !!fetching}>Ara</button>
        </form>
        <div class="kinds">
          {#each ["instrumental", "karaoke", "beat"] as k (k)}
            <button class="chipk" onclick={() => openFinder(k)} disabled={searching || !!fetching}>{k}</button>
          {/each}
        </div>
        {#if searching}
          <p class="sub">YouTube'da aranıyor…</p>
        {:else}
          <ul class="hits">
            {#each hits as h (h.url)}
              {@const d = durDiff(h)}
              <li>
                <div class="htx">
                  <b title={h.title}>{h.title}</b>
                  <span class="sub small">
                    {h.channel}{h.duration ? ` · ${formatDuration(h.duration)}` : ""}
                    {#if d !== null}<span class="dd" class:ok={Math.abs(d) <= 4}>{Math.abs(d) <= 4 ? "süre tutuyor" : `${d > 0 ? "+" : ""}${d} sn`}</span>{/if}
                  </span>
                </div>
                <button class="btn sm" disabled={!!fetching} onclick={() => useHit(h)}>{fetching === h.url ? stageText(prog) : "Kullan"}</button>
              </li>
            {:else}
              <p class="sub">Sonuç yok. Aramayı değiştir (ör. sanatçı adıyla) ya da yukarıdan "karaoke" / "beat" dene.</p>
            {/each}
          </ul>
        {/if}
        <p class="sub small">Süresi şarkıyla aynı olan ("süre tutuyor") genelde doğru sürümdür. Seçtiğin indirilir ve sözlere göre kendiliğinden hizalanır.</p>
        {#if error}<p class="err">{error}</p>{/if}
        <div class="row"><button class="btn ghost" onclick={() => (finder = false)} disabled={!!fetching}>Kapat</button></div>
      </div>
    </div>
  {/if}

  {#if result}
    <div class="rback" role="presentation" onclick={(e) => e.target === e.currentTarget && (result = null)}>
      <div class="rcard" role="dialog" aria-label="Kayıt">
        <h3><Icon name="check" size={18} />Kaydın hazır</h3>
        {#if quietMic}
          <p class="warnbox">
            Mikrofondan neredeyse hiç ses gelmedi. Windows'ta doğru mikrofon seçili mi (Ses & Mikrofon ekranı ya da Windows ses
            ayarları → Giriş)? Mikrofon kilidi açıksa seviyesine de bak.
          </p>
        {/if}
        <p class="sub small">Sesin ve müzik otomatik olarak dengelendi; istersen aşağıdan ince ayar yap ve yenile.</p>
        {#key previewKey}
          <audio class="pv" controls src={api.fileSrc(result.path)}></audio>
        {/key}
        <div class="rset">
          <label>Sesin <b class="tnum">%{Math.round(voiceGain * 100)}</b>
            <input type="range" min="0.3" max="2.5" step="0.05" bind:value={voiceGain} />
          </label>
          <label>Müzik <b class="tnum">%{Math.round(musicGain * 100)}</b>
            <input type="range" min="0.2" max="1.5" step="0.05" bind:value={musicGain} />
          </label>
          <label title="Sesin müzikten geride kalıyorsa artır, önde gidiyorsa azalt">Gecikme düzeltme <b class="tnum">{latencyMs} ms</b>
            <input type="range" min="0" max="500" step="10" bind:value={latencyMs} />
          </label>
          <label class="chk"><input type="checkbox" bind:checked={echo} />Hafif yankı</label>
        </div>
        <div class="row">
          <button class="btn" onclick={remix} disabled={mixing}><Icon name="refresh" size={16} />{mixing ? "Karıştırılıyor…" : "Bu ayarlarla yenile"}</button>
          <button class="btn ghost" onclick={() => result && api.karaoke.reveal(result.path)}><Icon name="folder" size={16} />Klasörde göster</button>
          <button class="btn ghost" onclick={removeResult}><Icon name="trash" size={16} />Sil</button>
          <button class="btn ghost" onclick={() => (result = null)}>Kapat</button>
        </div>
        <p class="sub small">Kayıtlar: Belgeler › GMID Tool › Karaoke › Kayıtlar ({recordings.length})</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .kara { position: fixed; inset: 0; z-index: 60; display: grid; grid-template-rows: auto minmax(0, 1fr) auto; background: #0d0f12; color: var(--text); }
  .kbg { position: absolute; inset: -60px; background-size: cover; background-position: center; filter: blur(60px) saturate(1.2); opacity: 0.35; }
  .kshade { position: absolute; inset: 0; background: radial-gradient(ellipse at center, rgba(13, 15, 18, 0.45), rgba(13, 15, 18, 0.92)); }
  .kbar, .lyrics, .kctl, .center { position: relative; }
  .kbar { display: flex; align-items: center; justify-content: space-between; gap: 14px; padding: 14px 22px; }
  .kttl { display: flex; flex-direction: column; align-items: center; min-width: 0; text-align: center; }
  .kttl b { font-family: var(--f-display); font-size: 16px; max-width: 60vw; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .kick { display: inline-flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; color: var(--gold); }
  .sm { padding: 6px 11px; font-size: 13px; }

  .center { display: grid; place-items: center; padding: 20px; grid-row: 2 / 4; }
  .setup { max-width: 560px; display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; padding: 30px; border-radius: 20px; border: 1px solid var(--line-2); background: rgba(23, 26, 30, 0.8); }
  .setup h2 { margin: 0; font-family: var(--f-display); font-size: 24px; }
  .setup p { margin: 0; color: var(--muted); line-height: 1.55; }
  .bigicon { width: 72px; height: 72px; border-radius: 50%; display: grid; place-items: center; background: rgba(227, 194, 112, 0.14); color: var(--gold); box-shadow: 0 0 30px rgba(227, 194, 112, 0.2); }
  .kprog { width: 100%; display: flex; flex-direction: column; align-items: center; gap: 10px; }
  .pbar { width: 100%; height: 8px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
  .pbar i { display: block; height: 100%; background: linear-gradient(90deg, var(--gold-dim), var(--gold-glow)); transition: width 0.3s; }
  .pbar.busy i { width: 30% !important; animation: kbusy 1.3s ease-in-out infinite; }
  @keyframes kbusy { from { transform: translateX(-100%); } to { transform: translateX(340%); } }
  .err { color: var(--bad); margin: 0; }

  /* Sözler */
  .lyrics { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 14px; padding: 0 6vw; text-align: center; min-height: 0; overflow: hidden; }
  .ln { margin: 0; max-width: 1100px; line-height: 1.3; transition: opacity 0.3s; }
  .prev { font-size: 20px; color: var(--dim); opacity: 0.6; min-height: 1.3em; }
  .cur { font-family: var(--f-display); font-size: clamp(28px, 4.2vw, 54px); font-weight: 700; min-height: 1.3em; }
  .next { font-size: clamp(18px, 2.2vw, 26px); color: var(--muted); min-height: 1.3em; }
  .next2 { font-size: 17px; color: var(--dim); opacity: 0.7; min-height: 1.3em; }
  .kw {
    --p: 0;
    color: transparent;
    background: linear-gradient(90deg, var(--gold-glow) calc(var(--p) * 120% - 20%), rgba(240, 236, 228, 0.92) calc(var(--p) * 120%));
    -webkit-background-clip: text;
    background-clip: text;
    transition: filter 0.25s;
  }
  .kw.now { filter: drop-shadow(0 0 14px rgba(240, 216, 120, 0.55)); }
  .count { font-family: var(--f-display); font-size: 64px; font-weight: 800; color: var(--gold); animation: pop 1s ease-out infinite; }
  @keyframes pop { from { transform: scale(1.25); opacity: 0.4; } to { transform: scale(1); opacity: 1; } }
  .nolyr { max-width: 520px; color: var(--muted); }

  /* Kontroller */
  .kctl { display: flex; flex-direction: column; gap: 12px; padding: 14px 6vw 22px; }
  .seekrow { display: flex; align-items: center; gap: 12px; }
  .seek { flex: 1; accent-color: var(--gold); }
  .ctlrow { display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; gap: 18px; }
  .knob { display: flex; flex-direction: column; gap: 6px; max-width: 260px; }
  .knob:last-child { justify-self: end; }
  .knob label { display: flex; align-items: center; gap: 6px; font-size: 12.5px; color: var(--muted); }
  .knob label b { margin-left: auto; color: var(--text); }
  .knob input { accent-color: var(--gold); width: 100%; }
  .mid { display: flex; align-items: center; gap: 14px; }
  .kplay { width: 60px; height: 60px; border-radius: 50%; border: 0; background: var(--gold); color: #1b1608; display: grid; place-items: center; box-shadow: 0 10px 26px rgba(227, 194, 112, 0.3); }
  .kplay:disabled { opacity: 0.5; }
  .krec { display: inline-flex; align-items: center; gap: 9px; height: 46px; padding: 0 18px; border-radius: 999px; border: 1px solid rgba(224, 106, 106, 0.55); background: rgba(224, 106, 106, 0.12); color: #ffd9d9; font-weight: 700; }
  .krec:hover:not(:disabled) { background: rgba(224, 106, 106, 0.22); }
  .krec .dot { width: 12px; height: 12px; border-radius: 50%; background: #e05555; }
  .krec.on .dot { animation: blink 1s steps(2) infinite; }
  @keyframes blink { 50% { opacity: 0.2; } }
  .lvl { width: 60px; height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.12); overflow: hidden; }
  .lvl i { display: block; height: 100%; background: linear-gradient(90deg, #5cc08a, #e3c270, #e05555); }
  .redo { margin: 0; text-align: center; }
  .redo b { color: var(--text); font-weight: 600; }
  .warn { color: var(--gold); }
  .choice { width: 100%; display: flex; flex-direction: column; align-items: flex-start; gap: 8px; text-align: left; padding: 14px 16px; border-radius: 14px; border: 1px solid var(--line-2); background: rgba(255, 255, 255, 0.025); color: var(--text); }
  .choice.rec { border-color: rgba(227, 194, 112, 0.5); background: rgba(227, 194, 112, 0.07); cursor: pointer; transition: background 0.15s; }
  .choice.rec:hover:not(:disabled) { background: rgba(227, 194, 112, 0.13); }
  .ctop { display: flex; align-items: center; gap: 8px; color: var(--gold); }
  .ctop b { color: var(--text); font-size: 15px; }
  .tag { font-size: 10.5px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: #1b1608; background: var(--gold); border-radius: 999px; padding: 2px 8px; }
  .cdesc { font-size: 13px; color: var(--muted); line-height: 1.5; }
  .finder { width: min(680px, 100%); }
  .frow { display: flex; gap: 8px; }
  .frow input { flex: 1; min-width: 0; background: var(--coal); border: 1px solid var(--line-2); border-radius: 10px; padding: 9px 12px; color: var(--text); }
  .kinds { display: flex; gap: 6px; }
  .chipk { border: 1px solid var(--line-2); background: transparent; color: var(--muted); border-radius: 999px; padding: 4px 11px; font-size: 12.5px; }
  .chipk:hover:not(:disabled) { color: var(--gold-glow); border-color: var(--gold-dim); }
  .hits { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; max-height: 340px; overflow: auto; }
  .hits li { display: flex; align-items: center; gap: 12px; padding: 8px 10px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); }
  .htx { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .htx b { font-size: 13.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dd { margin-left: 6px; padding: 0 6px; border-radius: 6px; background: rgba(255, 255, 255, 0.07); }
  .dd.ok { color: var(--good); background: rgba(92, 192, 138, 0.12); }
  .link { background: none; border: 0; padding: 0; color: var(--gold); font: inherit; font-weight: 600; cursor: pointer; }
  .link:hover { color: var(--gold-glow); }
  .hp { display: flex; align-items: center; gap: 8px; justify-content: center; font-size: 13px; color: var(--muted); }

  /* Sonuç */
  .rback { position: fixed; inset: 0; z-index: 61; display: grid; place-items: center; background: rgba(0, 0, 0, 0.55); padding: 20px; }
  .rcard { width: min(560px, 100%); display: flex; flex-direction: column; gap: 14px; padding: 22px; border-radius: 18px; border: 1px solid var(--line-2); background: var(--coal-2); box-shadow: 0 30px 70px rgba(0, 0, 0, 0.6); }
  .rcard h3 { margin: 0; display: flex; align-items: center; gap: 8px; font-family: var(--f-display); color: var(--gold-glow); }
  .pv { width: 100%; }
  .rset { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .warnbox { margin: 0; padding: 10px 12px; border-radius: 10px; border: 1px solid rgba(227, 194, 112, 0.45); background: rgba(227, 194, 112, 0.08); color: var(--text); font-size: 13px; line-height: 1.5; }
  .rset label { display: flex; flex-direction: column; gap: 6px; font-size: 13px; color: var(--muted); }
  .rset label b { color: var(--text); }
  .rset input[type="range"] { accent-color: var(--gold); }
  .rset .chk { flex-direction: row; align-items: center; gap: 8px; }
  .rcard .row { display: flex; flex-wrap: wrap; gap: 8px; }

  @media (max-width: 760px) {
    .ctlrow { grid-template-columns: 1fr; justify-items: center; }
    .knob, .knob:last-child { justify-self: stretch; max-width: none; }
    .rset { grid-template-columns: 1fr; }
  }
</style>
