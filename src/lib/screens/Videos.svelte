<script lang="ts">
  import { onMount } from "svelte";
  import { api, formatSize, inTauri } from "../api";
  import { on } from "../bus";
  import Switch from "../components/Switch.svelte";
  import { downloads, formatDuration, formatEta, formatSpeed, isActive, siteLabel } from "../downloads.svelte";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { BridgeStatus, Download, PlaylistProbe, Probe, QualityOption, ToolProgress, ToolsState } from "../types";

  type Tab = "all" | "YouTube" | "X" | "Instagram" | "audio" | "other";
  const TABS: [Tab, string][] = [["all", "Tümü"], ["YouTube", "YouTube"], ["X", "X"], ["Instagram", "Instagram"], ["audio", "Sesler"], ["other", "Diğer"]];

  let url = $state("");
  let probing = $state(false);
  let probe: Probe | null = $state(null);
  let probeError: string | null = $state(null);
  let playlist: PlaylistProbe | null = $state(null);
  let selected: string[] = $state([]);
  let plQuality = $state("1080");
  let plBusy = $state(false);
  let plFilter: string | null = $state(null);
  const PL_QUALITIES: [string, string][] = [["1080", "1080p"], ["720", "720p"], ["480", "480p"], ["mp3", "MP3 (sadece ses)"]];
  /** Video bağlantısı bir çalma listesinin içinden mi açılmış? */
  const inList = $derived(!!probe && /[?&]list=/.test(url));
  const plDuration = $derived.by(() => {
    const p = playlist as PlaylistProbe | null;
    return p ? p.entries.reduce((sum, e) => sum + (e.duration ?? 0), 0) : 0;
  });
  let tools: ToolsState | null = $state(null);
  let toolProgress: Record<string, ToolProgress> = $state({});
  let installing = $state(false);
  let bridge: BridgeStatus | null = $state(null);
  let pairCode: string | null = $state(null);
  let extPath: string | null = $state(null);
  let showSteps = $state(false);
  let tab: Tab = $state("all");
  let search = $state("");
  let armed: string | null = $state(null);
  let hovering: string | null = $state(null);
  let parallel = $state(getJson("videos.parallel", 2));
  let notifyDone = $state(getJson("videos.notify", true));
  let updating = $state(false);

  const queue = $derived(downloads.list.filter((d) => d.status !== "done"));
  const library = $derived(downloads.list.filter((d) => d.status === "done"));
  const tabOf = (d: Download): Tab => {
    if (d.audioOnly) return "audio";
    const s = siteLabel(d.site);
    return s === "YouTube" || s === "X" || s === "Instagram" ? s : "other";
  };
  const counts = $derived(Object.fromEntries(TABS.map(([t]) => [t, t === "all" ? library.length : library.filter((d) => tabOf(d) === t).length])));
  const shown = $derived.by(() => {
    const q = search.trim().toLocaleLowerCase("tr-TR");
    return library.filter(
      (d) =>
        (tab === "all" || tabOf(d) === tab) &&
        (!plFilter || d.playlist === plFilter) &&
        (!q || `${d.title} ${d.uploader} ${d.playlist ?? ""}`.toLocaleLowerCase("tr-TR").includes(q)),
    );
  });
  const playlists = $derived([...new Set(library.map((d) => d.playlist).filter((p): p is string => !!p))]);
  const totalSize = $derived(library.reduce((s, d) => s + (d.size ?? 0), 0));

  async function loadTools() {
    tools = await api.videos.tools();
    if (tools.ready && !tools.installing) {
      // Günde en fazla bir kez, sessizce.
      void api.videos.update(false).then((t) => (tools = t)).catch(() => {});
    }
  }

  async function loadBridge() {
    bridge = await api.videos.bridge().catch(() => null);
  }

  onMount(() => {
    void loadTools();
    void loadBridge();
    const stops = [
      on<ToolProgress>("tools-progress", (p) => {
        toolProgress = { ...toolProgress, [p.tool]: p };
      }),
      on("bridge-changed", () => {
        pairCode = null;
        notify("Tarayıcı eklentisi bağlandı", "check");
        void loadBridge();
      }),
    ];
    const timer = setInterval(loadBridge, 30_000);
    return () => {
      clearInterval(timer);
      stops.forEach((s) => void s.then((f) => f()));
    };
  });

  // ---- bağlantı ve kalite ----

  async function getQualities(wholeList = false) {
    const u = url.trim();
    if (!u) return;
    probing = true;
    probe = null;
    playlist = null;
    probeError = null;
    try {
      const r = await api.videos.probe(u, wholeList);
      if (r.kind === "playlist") {
        playlist = r;
        selected = r.entries.map((e) => e.id);
      } else {
        probe = r;
      }
    } catch (e) {
      const msg = String(e);
      probeError = msg === "TOOLS_MISSING" ? "Önce aşağıdaki indirme araçlarını kur (bir kez)." : msg;
    } finally {
      probing = false;
    }
  }

  async function pasteFromClipboard() {
    try {
      const text = (await navigator.clipboard.readText()).trim();
      if (!text) return notify("Panoda bir şey yok", "info");
      url = text;
      void getQualities();
    } catch {
      notify("Panoya erişilemedi; Ctrl+V ile yapıştır.", "info");
    }
  }

  function onPaste(e: ClipboardEvent) {
    const text = e.clipboardData?.getData("text")?.trim();
    if (text && /^https?:\/\//.test(text)) {
      e.preventDefault();
      url = text;
      void getQualities();
    }
  }

  async function start(o: QualityOption) {
    if (!probe) return;
    try {
      await api.videos.download(probe.url, o.id);
      notify(`Sıraya eklendi · ${o.audioOnly ? "MP3" : o.label}`, "download");
      probe = null;
      url = "";
    } catch (e) {
      notify(String(e), "info");
    }
  }

  function toggleEntry(id: string) {
    selected = selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id];
  }

  async function startPlaylist() {
    if (!playlist || !selected.length) return;
    plBusy = true;
    try {
      const all = selected.length === playlist.entries.length;
      const n = await api.videos.downloadPlaylist(playlist.url, plQuality, all ? null : selected);
      notify(`${n} ${plQuality === "mp3" ? "şarkı" : "video"} sıraya eklendi · ${playlist.title}`, "download");
      playlist = null;
      url = "";
    } catch (e) {
      notify(String(e), "info");
    } finally {
      plBusy = false;
    }
  }

  // ---- araçlar ----

  async function installTools(force = false) {
    installing = true;
    toolProgress = {};
    try {
      tools = await api.videos.installTools(force);
      notify("İndirme araçları hazır", "check");
      if (url.trim() && !probe) void getQualities();
    } catch (e) {
      notify(String(e), "info");
      tools = await api.videos.tools();
    } finally {
      installing = false;
    }
  }

  async function updateNow() {
    updating = true;
    try {
      const before = tools?.version;
      tools = await api.videos.update(true);
      notify(tools.version === before ? `yt-dlp zaten güncel (${tools.version})` : `yt-dlp güncellendi: ${tools.version}`, "check");
    } finally {
      updating = false;
    }
  }

  // ---- sıra ve kütüphane ----

  async function act(p: Promise<unknown>) {
    try {
      await p;
    } catch (e) {
      notify(String(e), "info");
    }
  }

  function remove(d: Download) {
    if (armed !== d.id) {
      armed = d.id;
      setTimeout(() => armed === d.id && (armed = null), 3000);
      return;
    }
    armed = null;
    void act(api.videos.remove(d.id, true).then(() => notify("Video silindi", "trash")));
  }

  function statusText(d: Download): string {
    const l = downloads.live[d.id];
    switch (d.status) {
      case "queued": return "Sırada bekliyor";
      case "paused": return `Duraklatıldı · %${Math.round(d.progress * 100)}`;
      case "error": return d.error ?? "Hata";
      case "processing": return "Görüntü ve ses birleştiriliyor…";
      default: {
        const pct = Math.round((l?.progress ?? d.progress) * 100);
        const parts = [`%${pct}`, formatSpeed(l?.speed)];
        if (l?.eta) parts.push(`${formatEta(l.eta)} kaldı`);
        return parts.filter(Boolean).join(" · ");
      }
    }
  }

  const progressOf = (d: Download) => downloads.live[d.id]?.progress ?? d.progress;
  const thumbSrc = (t: string | null) => (!t ? null : /^(https?:|data:)/.test(t) ? t : api.fileSrc(t));
  const previewable = (d: Download) => !!d.filePath && /\.(mp4|webm)$/i.test(d.filePath);

  // ---- eklenti ----

  async function prepareExtension() {
    try {
      extPath = await api.videos.installExtension();
    } catch (e) {
      notify(String(e), "info");
    }
  }

  async function copy(text: string, what: string) {
    try {
      await navigator.clipboard.writeText(text);
      notify(`${what} kopyalandı`, "copy");
    } catch {
      notify(text, "info");
    }
  }

  async function startPairing() {
    pairCode = await api.videos.pair();
  }

  async function unpair() {
    await api.videos.unpair();
    await loadBridge();
    notify("Eklenti bağlantısı kaldırıldı", "check");
  }
</script>

<div class="vd">
  <section class="panel paste">
    <div class="row grow">
      <span class="pi"><Icon name="link" size={22} /></span>
      <input
        class="url"
        type="url"
        placeholder="Video bağlantısını yapıştır — YouTube, X, Instagram, TikTok…"
        bind:value={url}
        onpaste={onPaste}
        onkeydown={(e) => e.key === "Enter" && void getQualities()}
        aria-label="Video bağlantısı"
      />
      <button class="btn ghost" onclick={pasteFromClipboard} title="Panodan yapıştır"><Icon name="copy" size={18} />Yapıştır</button>
      <button class="btn" onclick={() => getQualities()} disabled={probing || !url.trim()}>
        {#if probing}<span class="spin"><Icon name="refresh" size={18} /></span>Bakılıyor…{:else}<Icon name="search" size={18} />Kaliteleri getir{/if}
      </button>
    </div>
    {#if probeError}<p class="err small">{probeError}</p>{/if}

    {#if probe}
      <div class="probe">
        <div class="pthumb">
          {#if thumbSrc(probe.thumbnail)}<img src={thumbSrc(probe.thumbnail)} alt="" />{/if}
          {#if probe.duration}<span class="dur tnum">{formatDuration(probe.duration)}</span>{/if}
        </div>
        <div class="pinfo">
          <b class="ptitle">{probe.title}</b>
          <span class="sub small">{[probe.uploader, siteLabel(probe.site)].filter(Boolean).join(" · ")}</span>
          <div class="qgrid">
            {#each probe.options as o (o.id)}
              <button class="q" class:rec={o.recommended} class:audio={o.audioOnly} onclick={() => start(o)}>
                <span class="ql">{o.label}{#if o.recommended}<span class="tag">Önerilen</span>{/if}</span>
                <span class="qd">{o.detail}</span>
                <span class="qs tnum">{o.size ? `~${formatSize(o.size)}` : "boyut bilinmiyor"}</span>
                <span class="qgo"><Icon name="download" size={16} />İndir</span>
              </button>
            {/each}
          </div>
          {#if inList}
            <button class="btn ghost sm listbtn" onclick={() => getQualities(true)}><Icon name="list" size={16} />Bu çalma listesinin tamamını getir</button>
          {/if}
        </div>
      </div>
    {:else if playlist}
      <div class="pl">
        <div class="plhead">
          <div class="pltitle">
            <b class="ptitle">{playlist.title}</b>
            <span class="sub small tnum">
              {[playlist.uploader, siteLabel(playlist.site), `${playlist.entries.length} video`, plDuration ? formatDuration(plDuration) : ""].filter(Boolean).join(" · ")}
            </span>
          </div>
          <div class="seg">
            {#each PL_QUALITIES as [q, label] (q)}
              <button aria-pressed={plQuality === q} onclick={() => (plQuality = q)}>{label}</button>
            {/each}
          </div>
        </div>
        <div class="spread">
          <span class="row">
            <button class="link" onclick={() => (selected = playlist?.entries.map((e) => e.id) ?? [])}>Tümünü seç</button>
            <button class="link" onclick={() => (selected = [])}>Hiçbiri</button>
            <span class="sub small">Dosyalar listenin adıyla ayrı bir klasöre iner.</span>
          </span>
          <button class="btn" onclick={startPlaylist} disabled={plBusy || !selected.length}>
            <Icon name="download" size={18} />Seçilenleri indir ({selected.length})
          </button>
        </div>
        <ol class="entries">
          {#each playlist.entries as e, i (e.id)}
            <li class:off={!selected.includes(e.id)}>
              <label>
                <input type="checkbox" checked={selected.includes(e.id)} onchange={() => toggleEntry(e.id)} />
                <span class="idx tnum">{i + 1}</span>
                <span class="ethumb">{#if thumbSrc(e.thumbnail)}<img src={thumbSrc(e.thumbnail)} alt="" loading="lazy" />{/if}</span>
                <span class="etxt"><span class="et">{e.title}</span><span class="sub small">{e.uploader}</span></span>
                <span class="sub small tnum">{formatDuration(e.duration)}</span>
              </label>
            </li>
          {/each}
        </ol>
      </div>
    {:else if !probing && !probeError}
      <p class="sub small hint">
        İpucu: Tarayıcıda her sitedeki videoyu (üstündeki <b>GMID ile indir</b> düğmesi ya da sağ tık) indirmek için eklentiyi kur (sağda).
        YouTube çalma listesi bağlantısı yapıştırırsan videoları seçerek toplu indirebilirsin.
        İndirilenler <b>Videolar</b> klasörüne, siteye göre ayrılarak iner.
      </p>
    {/if}
  </section>

  {#if tools && !tools.ready}
    <section class="panel tools">
      <div class="spread">
        <h3 class="h2"><Icon name="install" />İndirme araçları (bir kez)</h3>
        <button class="btn" onclick={() => installTools(false)} disabled={installing}>
          <Icon name="download" size={18} />{installing ? "Kuruluyor…" : "Araçları kur"}
        </button>
      </div>
      <p class="sub small">
        Video indirmek için üç ücretsiz araç gerekiyor. Uygulama küçük kalsın diye birlikte gelmediler; resmî
        sayfalarından <b>GMID Tool\Araclar</b> klasörüne indirilir (toplam ~{tools.tools.filter((t) => !t.installed).reduce((s, t) => s + t.sizeMb, 0)} MB).
      </p>
      {#each tools.tools as t (t.id)}
        {@const p = toolProgress[t.id]}
        <div class="tool">
          <span class="tn"><b>{t.name}</b><span class="sub small">{t.purpose}</span></span>
          {#if t.installed || p?.stage === "done"}
            <span class="ok"><Icon name="check" size={16} />Hazır</span>
          {:else if p?.stage === "error"}
            <span class="err small">{p.error}</span>
          {:else if p}
            <div class="tbar"><i style:width="{p.stage === 'extract' ? 100 : p.total ? (p.done / p.total) * 100 : 5}%"></i></div>
            <span class="sub small tnum">{p.stage === "extract" ? "Açılıyor…" : `${formatSize(p.done)}${p.total ? ` / ${formatSize(p.total)}` : ""}`}</span>
          {:else}
            <span class="sub small tnum">~{t.sizeMb} MB</span>
          {/if}
        </div>
      {/each}
    </section>
  {/if}

  <div class="cols">
    <div class="main">
      {#if queue.length}
        <section class="panel">
          <h3 class="h2"><Icon name="download" />İndirmeler</h3>
          {#each queue as d (d.id)}
            <div class="job" class:bad={d.status === "error"}>
              <div class="jthumb">{#if thumbSrc(d.thumbnail)}<img src={thumbSrc(d.thumbnail)} alt="" />{/if}</div>
              <div class="jbody">
                <div class="spread nowrap">
                  <b class="jtitle" title={d.title}>{d.title}</b>
                  <span class="chip">{d.quality}</span>
                </div>
                {#if d.status === "downloading" || d.status === "processing" || d.status === "paused"}
                  <div class="bar" class:paused={d.status === "paused"} class:proc={d.status === "processing"}><i style:width="{progressOf(d) * 100}%"></i></div>
                {/if}
                <span class="small tnum" class:err={d.status === "error"} class:sub={d.status !== "error"}>
                  {siteLabel(d.site)} · {statusText(d)}
                </span>
              </div>
              <div class="jacts">
                {#if d.status === "downloading" || d.status === "queued"}
                  <button class="mini" title="Duraklat" onclick={() => act(api.videos.pause(d.id))}><Icon name="pause" size={17} /></button>
                {:else if d.status === "paused" || d.status === "error"}
                  <button class="mini" title={d.status === "error" ? "Tekrar dene" : "Devam et"} onclick={() => act(api.videos.resume(d.id))}>
                    <Icon name={d.status === "error" ? "refresh" : "play"} size={17} />
                  </button>
                {/if}
                <button class="mini" title="İptal et" onclick={() => act(api.videos.cancel(d.id))}><Icon name="x" size={17} /></button>
              </div>
            </div>
          {/each}
        </section>
      {/if}

      <section class="panel lib">
        <div class="spread">
          <h3 class="h2"><Icon name="film" />Videolarım <span class="sub small tnum">{library.length} video · {formatSize(totalSize)}</span></h3>
          <div class="row">
            <input class="search" type="search" placeholder="Ara…" bind:value={search} aria-label="Videolarda ara" />
            <button class="btn ghost sm" onclick={() => api.openDataFolder("Videolar")}><Icon name="folder" size={16} />Klasör</button>
          </div>
        </div>
        <div class="seg" role="tablist">
          {#each TABS as [t, label] (t)}
            {#if t === "all" || counts[t]}
              <button aria-pressed={tab === t} onclick={() => (tab = t)}>{label} <span class="n">{counts[t]}</span></button>
            {/if}
          {/each}
        </div>
        {#if playlists.length}
          <div class="lists">
            <span class="sub small">Listeler:</span>
            <button class="chipbtn" aria-pressed={!plFilter} onclick={() => (plFilter = null)}>Hepsi</button>
            {#each playlists as p (p)}
              <button class="chipbtn" aria-pressed={plFilter === p} onclick={() => (plFilter = plFilter === p ? null : p)}><Icon name="list" size={13} />{p}</button>
            {/each}
          </div>
        {/if}
        <div class="grid">
          {#each shown as d (d.id)}
            <article
              class="card"
              ondblclick={() => act(api.videos.open(d.id))}
              onmouseenter={() => (hovering = d.id)}
              onmouseleave={() => hovering === d.id && (hovering = null)}
              title="Çift tıkla: oynat"
            >
              <div class="cthumb">
                {#if hovering === d.id && previewable(d) && inTauri}
                  <!-- svelte-ignore a11y_media_has_caption -->
                  <video src={api.fileSrc(d.filePath ?? "")} autoplay muted loop playsinline></video>
                {:else if thumbSrc(d.thumbnail)}
                  <img src={thumbSrc(d.thumbnail)} alt="" loading="lazy" />
                {:else}
                  <span class="ph"><Icon name={d.audioOnly ? "speaker" : "film"} size={30} /></span>
                {/if}
                <span class="qual">{d.quality}</span>
                {#if d.duration}<span class="dur tnum">{formatDuration(d.duration)}</span>{/if}
                <div class="hover">
                  <button class="round" title="Oynat" onclick={() => act(api.videos.open(d.id))}><Icon name="play" size={20} /></button>
                </div>
              </div>
              <div class="cbody">
                <b class="ctitle" title={d.title}>{d.title}</b>
                <span class="sub small">{[d.uploader, siteLabel(d.site)].filter(Boolean).join(" · ")}</span>
                <div class="spread">
                  <span class="sub small tnum">{d.size ? formatSize(d.size) : ""}{d.finishedAt ? ` · ${ago(d.finishedAt)}` : ""}</span>
                  <span class="cacts">
                    <button class="mini" title="Klasörde göster" onclick={() => act(api.videos.reveal(d.id))}><Icon name="folder" size={16} /></button>
                    <button class="mini" class:armed={armed === d.id} title="Videoyu sil" onclick={() => remove(d)}>
                      <Icon name="trash" size={16} />{#if armed === d.id}<span>Sil?</span>{/if}
                    </button>
                  </span>
                </div>
              </div>
            </article>
          {:else}
            <p class="sub empty">
              {library.length ? "Bu süzgeçte video yok." : "Henüz indirilen video yok. Yukarıya bir bağlantı yapıştırarak başla."}
            </p>
          {/each}
        </div>
      </section>
    </div>

    <aside class="side">
      <section class="panel">
        <h3 class="h2"><Icon name="sparkle" />Tarayıcı eklentisi</h3>
        {#if bridge?.paired}
          <p class="state on">
            <span class="dot"></span>Bağlı{bridge.browser ? ` · ${bridge.browser}` : ""}{bridge.lastSeen ? ` · ${ago(bridge.lastSeen)} görüldü` : ""}
          </p>
          <p class="sub small">
            Her sitede bir videonun üstüne gelince sağ üstte <b>GMID ile indir</b> düğmesi çıkar. Videoya sağ tıklayıp
            <b>GMID Tool ile indir</b> menüsünden kalite seçebilir, eklenti simgesinden sayfadaki videoları görebilirsin.
          </p>
          <p class="sub small">Uygulama güncellenince eklenti de güncellenir: <code>chrome://extensions</code> sayfasında GMID'nin ⟳ (yenile) düğmesine bir kez bas.</p>
          <div class="row">
            <button class="link" onclick={() => (showSteps = !showSteps)}>{showSteps ? "Adımları gizle" : "Kurulum adımları"}</button>
            <button class="link dangerlink" onclick={unpair}>Bağlantıyı kaldır</button>
          </div>
        {:else}
          <p class="state"><span class="dot"></span>Henüz bağlı değil</p>
          <p class="sub small">Chrome ve Opera'da videoyu sayfadan çıkmadan indirmek için. Kurulumu bir kez yapılır.</p>
        {/if}

        {#if !bridge?.paired || showSteps}
          <ol class="steps">
            <li>
              <b>Eklenti klasörünü hazırla</b>
              {#if extPath}
                <div class="path small">{extPath}</div>
                <div class="row">
                  <button class="btn ghost sm" onclick={() => copy(extPath ?? "", "Klasör yolu")}><Icon name="copy" size={15} />Yolu kopyala</button>
                  <button class="btn ghost sm" onclick={() => api.openDataFolder("Eklenti")}><Icon name="folder" size={15} />Aç</button>
                </div>
              {:else}
                <button class="btn sm" onclick={prepareExtension}><Icon name="folder" size={15} />Hazırla</button>
              {/if}
            </li>
            <li>
              <b>Tarayıcıda eklentiler sayfasını aç</b>
              <div class="row">
                <button class="chipbtn" onclick={() => copy("chrome://extensions", "Adres")}>chrome://extensions <Icon name="copy" size={13} /></button>
                <button class="chipbtn" onclick={() => copy("opera://extensions", "Adres")}>opera://extensions <Icon name="copy" size={13} /></button>
              </div>
              <span class="sub small">Adresi kopyalayıp tarayıcının adres çubuğuna yapıştır.</span>
            </li>
            <li>
              <b>Geliştirici modu → Paketlenmemiş öğe yükle</b>
              <span class="sub small">Sağ üstteki "Geliştirici modu"nu aç, "Paketlenmemiş öğe yükle"ye bas ve 1. adımdaki klasörü seç.</span>
            </li>
            <li>
              <b>Eşleştir</b>
              {#if pairCode}
                <div class="code tnum" aria-label="Eşleştirme kodu">{pairCode}</div>
                <span class="sub small">Tarayıcıda GMID simgesine tıkla ve bu kodu yaz. 10 dakika geçerli.</span>
              {:else}
                <button class="btn sm" onclick={startPairing}><Icon name="link" size={15} />Eşleştirme kodu al</button>
              {/if}
            </li>
          </ol>
        {/if}
      </section>

      <section class="panel">
        <h3 class="h2"><Icon name="gear" />Ayarlar</h3>
        <div class="field">
          <span>Aynı anda indirme</span>
          <div class="seg">
            {#each [1, 2, 3, 4] as n (n)}
              <button aria-pressed={parallel === n} onclick={() => { parallel = n; void setJson("videos.parallel", n); }}>{n}</button>
            {/each}
          </div>
        </div>
        <label class="row">
          <Switch label="Bitince bildir" checked={notifyDone} onchange={(v) => { notifyDone = v; void setJson("videos.notify", v); }} />
          <span class="small">İndirme bitince Windows bildirimi</span>
        </label>
        {#if tools?.ready}
          <div class="spread">
            <span class="sub small tnum">yt-dlp {tools.version ?? ""}</span>
            <button class="link" onclick={updateNow} disabled={updating}>{updating ? "Güncelleniyor…" : "Şimdi güncelle"}</button>
          </div>
          <button class="link small" onclick={() => installTools(true)} disabled={installing}>
            {installing ? "Araçlar indiriliyor…" : "Araçları yeniden indir"}
          </button>
        {/if}
      </section>
    </aside>
  </div>
</div>

<style>
  .vd { display: flex; flex-direction: column; gap: 16px; max-width: 1500px; margin: 0 auto; }
  .paste { display: flex; flex-direction: column; gap: 14px; border-color: rgba(227, 194, 112, 0.35); }
  .grow { flex-wrap: nowrap; }
  .pi { color: var(--gold); display: grid; }
  .url { flex: 1; min-width: 0; font-size: 15px; padding: 11px 14px; border-radius: 12px; background: var(--coal-2); border: 1px solid var(--line-2); }
  .url:focus { border-color: var(--gold-dim); outline: none; }
  .hint b { color: var(--text); font-weight: 600; }
  .err { color: var(--bad); margin: 0; }
  .spin { display: inline-grid; animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .probe { display: grid; grid-template-columns: minmax(200px, 300px) minmax(0, 1fr); gap: 18px; align-items: start; }
  .pthumb, .jthumb, .cthumb { position: relative; aspect-ratio: 16 / 9; border-radius: 12px; overflow: hidden; background: var(--coal-2); }
  .pthumb img, .jthumb img, .cthumb img, .cthumb video { width: 100%; height: 100%; object-fit: cover; display: block; }
  .dur { position: absolute; right: 6px; bottom: 6px; background: rgba(0, 0, 0, 0.75); font-size: 11.5px; font-weight: 600; padding: 1px 6px; border-radius: 5px; }
  .pinfo { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .ptitle { font-family: var(--f-display); font-size: 16px; }
  .qgrid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 8px; margin-top: 10px; }
  .q {
    display: grid; grid-template-columns: 1fr auto; grid-template-areas: "l l" "d d" "s g"; gap: 2px 8px; text-align: left;
    padding: 10px 12px; border-radius: 12px; border: 1px solid var(--line-2); background: var(--coal-2); transition: border-color 0.15s, background 0.15s;
  }
  .q:hover { border-color: var(--gold); background: rgba(227, 194, 112, 0.08); }
  .q.rec { border-color: rgba(227, 194, 112, 0.6); box-shadow: 0 0 14px rgba(227, 194, 112, 0.12); }
  .q.audio .ql { color: var(--neon); }
  .ql { grid-area: l; font-family: var(--f-display); font-weight: 700; font-size: 15px; display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .qd { grid-area: d; font-size: 12px; color: var(--muted); min-height: 16px; }
  .qs { grid-area: s; font-size: 12px; color: var(--muted); align-self: end; }
  .qgo { grid-area: g; display: inline-flex; align-items: center; gap: 4px; font-size: 12px; font-weight: 700; color: var(--gold); }
  .tag { font-family: var(--f-body); font-size: 9.5px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--gold); border: 1px solid var(--gold-dim); border-radius: 999px; padding: 0 6px; }

  .listbtn { align-self: flex-start; margin-top: 8px; }
  .pl { display: flex; flex-direction: column; gap: 12px; }
  .plhead { display: flex; justify-content: space-between; align-items: flex-end; gap: 14px; flex-wrap: wrap; }
  .pltitle { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .entries { list-style: none; margin: 0; padding: 0; max-height: 360px; overflow: auto; display: flex; flex-direction: column; gap: 2px; border: 1px solid var(--line); border-radius: 12px; padding: 6px; }
  .entries li label { display: grid; grid-template-columns: auto 28px 64px minmax(0, 1fr) auto; gap: 10px; align-items: center; padding: 6px 8px; border-radius: 8px; cursor: pointer; }
  .entries li label:hover { background: rgba(255, 255, 255, 0.04); }
  .entries li.off { opacity: 0.45; }
  .entries input { accent-color: var(--gold); width: 16px; height: 16px; }
  .idx { color: var(--dim); text-align: right; font-size: 12px; }
  .ethumb { aspect-ratio: 16 / 9; border-radius: 5px; overflow: hidden; background: var(--coal-2); }
  .ethumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .etxt { display: flex; flex-direction: column; min-width: 0; }
  .et { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lists { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .lists .chipbtn { font-family: var(--f-body); }
  .chipbtn[aria-pressed="true"] { border-color: var(--gold); color: var(--gold-glow); background: rgba(227, 194, 112, 0.12); }
  .tools { display: flex; flex-direction: column; gap: 10px; border-color: rgba(53, 160, 162, 0.5); }
  .tool { display: grid; grid-template-columns: minmax(180px, 1fr) minmax(0, 2fr) auto; gap: 12px; align-items: center; padding: 8px 10px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); }
  .tn { display: flex; flex-direction: column; }
  .tool > :last-child { justify-self: end; }
  .tool > :nth-child(2):last-child { grid-column: 2 / 4; }
  .ok { color: var(--good); display: inline-flex; gap: 4px; align-items: center; font-weight: 600; font-size: 13px; }
  .tbar, .bar { height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.07); overflow: hidden; }
  .tbar i, .bar i { display: block; height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--teal), var(--neon)); transition: width 0.4s; }
  .bar.paused i { background: var(--dim); }
  .bar.proc i { background: linear-gradient(90deg, var(--gold-dim), var(--gold)); animation: pulse 1.2s ease-in-out infinite; }
  @keyframes pulse { 50% { opacity: 0.55; } }

  .cols { display: grid; grid-template-columns: minmax(0, 1fr) 340px; gap: 16px; align-items: start; }
  .main, .side { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  .panel { display: flex; flex-direction: column; gap: 10px; }
  .job { display: grid; grid-template-columns: 112px minmax(0, 1fr) auto; gap: 12px; align-items: center; padding: 8px; border-radius: 12px; background: rgba(255, 255, 255, 0.03); }
  .job.bad { background: rgba(224, 106, 106, 0.06); }
  .jthumb { border-radius: 8px; }
  .jbody { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .nowrap { flex-wrap: nowrap; }
  .jtitle { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; min-width: 0; }
  .jacts { display: flex; gap: 2px; }
  .mini { border: 0; background: transparent; color: var(--dim); padding: 6px; border-radius: 7px; display: inline-flex; align-items: center; gap: 4px; }
  .mini:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .mini.armed { color: var(--bad); background: rgba(224, 106, 106, 0.12); font-size: 12px; font-weight: 700; }

  .lib .h2 .sub { font-family: var(--f-body); font-weight: 500; margin-left: 4px; }
  .search { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 6px 10px; width: 180px; }
  .sm { padding: 6px 11px; font-size: 13px; }
  .seg .n { color: var(--dim); margin-left: 3px; font-variant-numeric: tabular-nums; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 220px), 1fr)); gap: 14px; }
  .card { display: flex; flex-direction: column; gap: 8px; min-width: 0; cursor: default; }
  .cthumb { border: 1px solid var(--line); }
  .card:hover .cthumb { border-color: var(--gold-dim); }
  .ph { position: absolute; inset: 0; display: grid; place-items: center; color: var(--dim); }
  .qual { position: absolute; left: 6px; top: 6px; background: rgba(18, 20, 23, 0.85); color: var(--gold); font-size: 10.5px; font-weight: 700; padding: 1px 6px; border-radius: 5px; }
  .hover { position: absolute; inset: 0; display: grid; place-items: center; opacity: 0; transition: opacity 0.15s; background: rgba(0, 0, 0, 0.18); }
  .card:hover .hover { opacity: 1; }
  .round { width: 44px; height: 44px; border-radius: 50%; border: 0; background: rgba(227, 194, 112, 0.92); color: #1b1608; display: grid; place-items: center; }
  .cbody { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .ctitle { font-weight: 600; font-size: 13.5px; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .cacts { display: flex; opacity: 0.5; transition: opacity 0.15s; }
  .card:hover .cacts { opacity: 1; }
  .empty { grid-column: 1 / -1; margin: 6px 0; }

  .state { display: flex; align-items: center; gap: 8px; margin: 0; font-weight: 600; color: var(--muted); }
  .state .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--dim); }
  .state.on { color: var(--good); }
  .state.on .dot { background: var(--good); box-shadow: 0 0 8px rgba(92, 192, 138, 0.6); }
  .steps { margin: 0; padding-left: 20px; display: flex; flex-direction: column; gap: 12px; }
  .steps li { display: flex; flex-direction: column; gap: 6px; align-items: flex-start; }
  .steps li::marker { color: var(--gold); font-weight: 700; }
  .steps .path { align-self: stretch; }
  .chipbtn { display: inline-flex; align-items: center; gap: 5px; border: 1px solid var(--line-2); background: var(--coal-2); border-radius: 999px; padding: 4px 10px; font-family: ui-monospace, Consolas, monospace; font-size: 12px; color: var(--text); }
  .chipbtn:hover { border-color: var(--gold-dim); }
  .code { font-family: ui-monospace, Consolas, monospace; font-size: 30px; font-weight: 700; letter-spacing: 0.3em; color: var(--gold-glow); background: rgba(227, 194, 112, 0.08); border: 1px dashed var(--gold-dim); border-radius: 12px; padding: 6px 14px; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field > span { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; align-self: flex-start; }
  .link:hover { color: var(--neon); }
  .link:disabled { opacity: 0.5; }
  .dangerlink { color: var(--dim); }
  .dangerlink:hover { color: var(--bad); }

  @media (max-width: 1150px) {
    .cols { grid-template-columns: minmax(0, 1fr); }
  }
  @media (max-width: 760px) {
    .probe { grid-template-columns: minmax(0, 1fr); }
    .grow { flex-wrap: wrap; }
    .tool { grid-template-columns: minmax(0, 1fr) auto; }
    .tool > :nth-child(2):not(:last-child) { display: none; }
  }
</style>
