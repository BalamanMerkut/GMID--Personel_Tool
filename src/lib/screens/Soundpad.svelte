<script lang="ts">
  import { onMount } from "svelte";
  import { api, inTauri } from "../api";
  import { debounce, on } from "../bus";
  import Switch from "../components/Switch.svelte";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";
  import type { Download, SoundPad, SoundpadConfig, SoundpadStatus } from "../types";

  let { onnavigate }: { onnavigate: (id: string) => void } = $props();

  let st = $state<SoundpadStatus | null>(null);
  let cfg = $state<SoundpadConfig | null>(null);
  /** Yeni tuş beklenen ses ("stop": durdurma tuşu). */
  let capturing = $state<string | null>(null);
  let armed = $state<string | null>(null);
  let renaming = $state<string | null>(null);
  let mediaOpen = $state(false);
  let media = $state<Download[]>([]);

  async function refresh() {
    try {
      st = await api.soundpad.status();
      cfg = $state.snapshot(st.config) as SoundpadConfig;
    } catch (e) {
      notify(String(e), "info");
    }
  }
  onMount(() => {
    void refresh();
    const stop = on("soundpad-changed", debounce(refresh, 80));
    return () => void stop.then((f) => f());
  });

  async function commit(next: SoundpadConfig) {
    cfg = next;
    try {
      st = await api.soundpad.save($state.snapshot(next) as SoundpadConfig);
      cfg = $state.snapshot(st.config) as SoundpadConfig;
      const errs = Object.values(st.keyErrors);
      if (errs.length) notify(errs[0], "info");
    } catch (e) {
      notify(String(e), "info");
      await refresh();
    }
  }
  const commitSoon = debounce((next: SoundpadConfig) => void commit(next), 250);
  function edit(change: (c: SoundpadConfig) => void, soon = false) {
    if (!cfg) return;
    const next = structuredClone($state.snapshot(cfg)) as SoundpadConfig;
    change(next);
    cfg = next;
    if (soon) commitSoon(next);
    else void commit(next);
  }
  const editPad = (id: string, change: (p: SoundPad) => void, soon = false) =>
    edit((c) => {
      const p = c.pads.find((x) => x.id === id);
      if (p) change(p);
    }, soon);

  async function toggle(p: SoundPad) {
    try {
      await api.soundpad.toggle(p.id);
    } catch (e) {
      notify(String(e), "info");
    }
  }

  async function addFiles() {
    const paths = await api.soundpad.pick();
    if (!paths.length) {
      if (!inTauri) notify("Dosya seçmek masaüstü uygulamasında çalışır", "info");
      return;
    }
    await add(paths);
  }
  async function add(paths: string[]) {
    try {
      st = await api.soundpad.add(paths);
      cfg = $state.snapshot(st.config) as SoundpadConfig;
      notify(paths.length > 1 ? `${paths.length} ses eklendi` : "Ses eklendi", "check");
    } catch (e) {
      notify(String(e), "info");
    }
  }
  async function openMedia() {
    mediaOpen = !mediaOpen;
    if (!mediaOpen) return;
    const all = await api.videos.list().catch(() => [] as Download[]);
    media = all.filter((d) => d.status === "done" && d.filePath);
  }

  function remove(id: string) {
    if (armed !== id) {
      armed = id;
      setTimeout(() => armed === id && (armed = null), 3000);
      return;
    }
    armed = null;
    edit((c) => (c.pads = c.pads.filter((p) => p.id !== id)));
  }

  // ---- tuş atama ----
  const MODS = new Set(["Control", "Alt", "Shift", "Meta", "AltGraph", "OS"]);
  /** Numpad ve F tuşları tek başına olabilir; diğerleri (yazarken basılmasın diye) en az bir değiştiriciyle. */
  function toAccel(e: KeyboardEvent): string | null {
    if (MODS.has(e.key)) return null;
    let key = e.code;
    if (key.startsWith("Key")) key = key.slice(3);
    else if (key.startsWith("Digit")) key = key.slice(5);
    const alone = key.startsWith("Numpad") || /^F\d{1,2}$/.test(key);
    const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift"].filter(Boolean);
    if (!mods.length && !alone) return null;
    return [...mods, key].join("+");
  }
  function onKey(e: KeyboardEvent) {
    if (!capturing) return;
    e.preventDefault();
    e.stopPropagation();
    const target = capturing;
    if (e.key === "Escape") return void (capturing = null);
    let accel: string | null;
    if (e.key === "Backspace" || e.key === "Delete") accel = "";
    else accel = toAccel(e);
    if (accel === null) {
      if (!e.code.startsWith("Numpad") && !MODS.has(e.key)) notify("Numpad tuşu, F tuşu ya da Ctrl/Alt'lı bir tuş seç", "keyboard");
      return;
    }
    capturing = null;
    if (target === "stop") edit((c) => (c.stopKey = accel));
    else editPad(target, (p) => (p.key = accel));
  }
  const keyLabel = (k: string) =>
    k
      .replace(/Numpad(\d)/, "Num $1")
      .replace("NumpadAdd", "Num +")
      .replace("NumpadSubtract", "Num −")
      .replace("NumpadMultiply", "Num *")
      .replace("NumpadDivide", "Num /")
      .replace("NumpadDecimal", "Num ,")
      .replace("NumpadEnter", "Num Enter")
      .split("+");

  const fileName = (p: string) => p.split(/[\\/]/).pop() ?? p;
  const pct = (v: number) => `%${Math.round(v * 100)}`;
  const devices = $derived(st?.devices);
  const realMics = $derived(devices?.inputs.filter((d) => !d.virtualCable) ?? []);
  const playingAny = $derived((st?.playing.length ?? 0) > 0);
</script>

<svelte:window onkeydown={onKey} />

<div class="sp">
  {#if st && !st.supported}
    <section class="panel wide warn"><Icon name="info" /><span>Ses Paneli yalnız Windows'ta çalışır.</span></section>
  {/if}

  <!-- Açık/kapalı ve durum -->
  <section class="panel wide hero" class:live={st?.running}>
    <div class="hero-l">
      <span class="ic"><Icon name="mic" size={22} /></span>
      <div>
        <h2 class="h2">Ses Paneli</h2>
        <p class="sub small">
          {#if st?.running}
            Açık. Tuşa basınca ses mikrofondan çalar. Discord'da mikrofon olarak <b>{devices?.cableMic || "CABLE Output"}</b> seçili olmalı.
          {:else if st?.error && cfg?.enabled}
            <span class="bad">{st.error}</span>
          {:else}
            Kapalı. Açınca seçtiğin tuşlar çalışır; kapalıyken mikrofonun hiç dinlenmez.
          {/if}
        </p>
      </div>
    </div>
    <div class="row">
      {#if playingAny}
        <button class="btn ghost" onclick={() => api.soundpad.stop()}><Icon name="pause" size={15} />Hepsini durdur</button>
      {/if}
      {#if cfg}
        <Switch label="Ses Paneli açık" checked={cfg.enabled} disabled={!devices?.cable && !cfg.enabled} onchange={(v) => edit((c) => (c.enabled = v))} />
      {/if}
    </div>
  </section>

  {#if devices && !devices.cable}
    <!-- Kurulum: sanal kablo yok -->
    <section class="panel wide setup">
      <h3 class="h2"><Icon name="install" />Önce bir kez: sanal ses kablosu</h3>
      <p class="sub">
        Windows, bir programın sesini doğrudan mikrofona koymaya izin vermez. Bu yüzden ücretsiz <b>VB-CABLE</b> kurulur: GMID
        sesleri ve senin sesini bu kabloya gönderir, Discord da kabloyu mikrofon olarak dinler. Soundpad ve benzerleri de böyle
        çalışır.
      </p>
      <ol class="steps">
        <li>
          <b>VB-CABLE'ı indir</b> ve zip'i bir klasöre çıkar.
          <button class="btn" onclick={() => api.soundpad.cablePage()}><Icon name="download" size={15} />vb-audio.com/Cable</button>
        </li>
        <li><b>VBCABLE_Setup_x64.exe</b>'ye sağ tıkla → <b>Yönetici olarak çalıştır</b> → <b>Install Driver</b>. Bitince bilgisayarı yeniden başlat.</li>
        <li>Discord → Kullanıcı Ayarları → <b>Ses ve Görüntü</b> → Giriş Aygıtı: <b>CABLE Output</b>.</li>
        <li>Buraya dön, <b>Ses Paneli</b>'ni aç.</li>
      </ol>
      <div class="row"><button class="btn ghost" onclick={refresh}><Icon name="refresh" size={15} />Yeniden denetle</button></div>
    </section>
  {/if}

  {#if devices?.defaultOutputIsCable}
    <section class="panel wide warn">
      <Icon name="info" />
      <span>Windows'un ses çıkışı <b>CABLE Input</b> olmuş; bilgisayardan hiçbir şey duymazsın. Kulaklığını yeniden varsayılan yap.</span>
      <button class="btn ghost" onclick={() => onnavigate("audio")}>Ses & Mikrofon</button>
    </section>
  {/if}
  {#if devices?.defaultInputIsCable}
    <section class="panel wide note">
      <Icon name="info" />
      <span>Windows'un varsayılan mikrofonu <b>CABLE Output</b> olmuş. Ses Paneli gerçek mikrofonunu kendisi seçer; ama diğer programlar için Ses & Mikrofon'dan gerçek mikrofonunu varsayılan yap.</span>
    </section>
  {/if}
  {#if st && !st.ffmpeg}
    <section class="panel wide warn">
      <Icon name="info" />
      <span>Sesleri okumak için FFmpeg gerekli. Video İndirici'nin araçlarını bir kez kur.</span>
      <button class="btn ghost" onclick={() => onnavigate("videos")}>Video İndirici</button>
    </section>
  {/if}

  {#if cfg}
    <!-- Sesler -->
    <section class="panel wide">
      <div class="spread">
        <h3 class="h2"><Icon name="grid" />Seslerin <span class="sub small">· tuşa ya da karta bas: çalar, yine bas: durur</span></h3>
        <div class="row">
          <button class="btn ghost" class:open={mediaOpen} onclick={openMedia}><Icon name="headphones" size={15} />Medyamdan</button>
          <button class="btn" onclick={addFiles}><Icon name="plus" size={15} />Ses ekle</button>
        </div>
      </div>

      {#if mediaOpen}
        <div class="media">
          {#each media as d (d.id)}
            {@const added = cfg.pads.some((p) => p.path === d.filePath)}
            <button class="mrow" disabled={added} onclick={() => d.filePath && add([d.filePath])}>
              <Icon name={d.audioOnly ? "headphones" : "film"} size={15} />
              <span class="mt">{d.title}</span>
              <span class="sub small">{added ? "Eklendi" : d.audioOnly ? "Müzik" : "Video (sesi)"}</span>
            </button>
          {:else}
            <p class="sub small">Video İndirici'den indirdiğin müzik ve videolar burada görünür.</p>
          {/each}
        </div>
      {/if}

      {#if cfg.pads.length}
        <div class="pads">
          {#each cfg.pads as p (p.id)}
            {@const playing = st?.playing.includes(p.id)}
            {@const missing = st?.missing.includes(p.id)}
            {@const kerr = st?.keyErrors[p.id]}
            <div class="pad" class:playing class:missing>
              <div class="pad-top">
                <button class="key" class:wait={capturing === p.id} class:none={!p.key} onclick={() => (capturing = capturing === p.id ? null : p.id)} title="Tuş ata (Sil: tuşu kaldır, Esc: vazgeç)">
                  {#if capturing === p.id}Tuşa bas…{:else if p.key}{#each keyLabel(p.key) as k, i (i)}<kbd>{k}</kbd>{/each}{:else}Tuş yok{/if}
                </button>
                <button class="del" class:armed={armed === p.id} onclick={() => remove(p.id)} title="Sil">
                  <Icon name="trash" size={13} />{#if armed === p.id}<span>Sil?</span>{/if}
                </button>
              </div>
              <button class="play" disabled={missing || !st?.running} onclick={() => toggle(p)} title={st?.running ? (playing ? "Durdur" : "Çal") : "Önce Ses Paneli'ni aç"}>
                <span class="pi"><Icon name={playing ? "pause" : "play"} size={18} /></span>
                {#if renaming === p.id}
                  <!-- svelte-ignore a11y_autofocus -->
                  <input
                    class="rename"
                    value={p.name}
                    autofocus
                    onclick={(e) => e.stopPropagation()}
                    onkeydown={(e) => {
                      e.stopPropagation();
                      if (e.key === "Enter") e.currentTarget.blur();
                      if (e.key === "Escape") renaming = null;
                    }}
                    onblur={(e) => {
                      const v = e.currentTarget.value.trim();
                      renaming = null;
                      if (v && v !== p.name) editPad(p.id, (x) => (x.name = v.slice(0, 60)));
                    }}
                  />
                {:else}
                  <b title={p.name}>{p.name}</b>
                {/if}
              </button>
              <div class="pad-bot">
                {#if missing}
                  <span class="bad small">Dosya bulunamadı</span>
                {:else if kerr}
                  <span class="bad small" title={kerr}>{kerr}</span>
                {:else}
                  <span class="sub small fn" title={p.path}>{fileName(p.path)}</span>
                {/if}
                <button class="mini" onclick={() => (renaming = p.id)} title="Adını değiştir"><Icon name="edit" size={13} /></button>
              </div>
              <label class="pv" title="Bu sesin seviyesi">
                <Icon name="volumelow" size={13} />
                <input type="range" min="0" max="2" step="0.05" value={p.volume} oninput={(e) => editPad(p.id, (x) => (x.volume = +e.currentTarget.value), true)} />
                <span class="tnum small">{pct(p.volume)}</span>
              </label>
            </div>
          {/each}
        </div>
      {:else}
        <div class="empty">
          <Icon name="mic" size={26} />
          <p class="sub">Henüz ses yok. <b>Ses ekle</b>'ye bas ya da <b>Medyamdan</b> seç; ilk ses <kbd>Num 1</kbd>'e, sonraki <kbd>Num 2</kbd>'ye atanır.</p>
        </div>
      {/if}
    </section>

    <!-- Seviyeler -->
    <section class="panel">
      <h3 class="h2"><Icon name="volumelow" />Seviyeler</h3>
      <label class="field">
        <span>Karşıya giden ses <b class="tnum">{pct(cfg.soundVolume)}</b></span>
        <input type="range" min="0" max="1" step="0.01" value={cfg.soundVolume} oninput={(e) => edit((c) => (c.soundVolume = +e.currentTarget.value), true)} />
        <em class="sub small">Discord'dakilerin duyduğu şarkı sesi. Çok yüksek olursa cızırdar; %40–60 iyidir.</em>
      </label>
      <div class="line">
        <div><b>Kendin de duy</b><span class="sub small">Çalan ses kulaklığından da gelir ({devices?.defaultOutputName || "varsayılan çıkış"}).</span></div>
        <Switch label="Kendin de duy" checked={cfg.monitor} onchange={(v) => edit((c) => (c.monitor = v))} />
      </div>
      {#if cfg.monitor}
        <label class="field">
          <span>Kulaklıktaki ses <b class="tnum">{pct(cfg.monitorVolume)}</b></span>
          <input type="range" min="0" max="1" step="0.01" value={cfg.monitorVolume} oninput={(e) => edit((c) => (c.monitorVolume = +e.currentTarget.value), true)} />
        </label>
      {/if}
      <div class="line">
        <div><b>Üst üste çalsın</b><span class="sub small">Kapalıyken yeni ses, çalanı keser.</span></div>
        <Switch label="Üst üste çalsın" checked={cfg.overlap} onchange={(v) => edit((c) => (c.overlap = v))} />
      </div>
      <div class="line">
        <div>
          <b>Hepsini durdur tuşu</b>
          {#if st?.keyErrors.stop}<span class="bad small">{st.keyErrors.stop}</span>{/if}
        </div>
        <button class="key" class:wait={capturing === "stop"} class:none={!cfg.stopKey} onclick={() => (capturing = capturing === "stop" ? null : "stop")}>
          {#if capturing === "stop"}Tuşa bas…{:else if cfg.stopKey}{#each keyLabel(cfg.stopKey) as k, i (i)}<kbd>{k}</kbd>{/each}{:else}Tuş yok{/if}
        </button>
      </div>
    </section>

    <!-- Mikrofon -->
    <section class="panel">
      <h3 class="h2"><Icon name="mic" />Mikrofonun</h3>
      <div class="line">
        <div><b>Sesim de gitsin</b><span class="sub small">Discord kabloyu dinlediği için konuşman da GMID üzerinden geçer. Kapatırsan karşıya yalnız sesler gider.</span></div>
        <Switch label="Sesim de gitsin" checked={cfg.passthrough} onchange={(v) => edit((c) => (c.passthrough = v))} />
      </div>
      {#if cfg.passthrough}
        <label class="field">
          <span>Mikrofon</span>
          <select value={cfg.micId} onchange={(e) => edit((c) => (c.micId = e.currentTarget.value))}>
            <option value="">Windows'un varsayılanı{devices?.defaultInputIsCable ? " (kablo değilse)" : ""}</option>
            {#each realMics as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
          </select>
        </label>
        <label class="field">
          <span>Sesinin seviyesi <b class="tnum">{pct(cfg.micVolume)}</b></span>
          <input type="range" min="0" max="2" step="0.05" value={cfg.micVolume} oninput={(e) => edit((c) => (c.micVolume = +e.currentTarget.value), true)} />
        </label>
      {/if}
      <details class="how">
        <summary>Discord'da şarkı kesiliyor ya da cızırdıyorsa</summary>
        <ul class="sub small">
          <li>Ses ve Görüntü → <b>Gürültü azaltma</b> (Krisp): <b>Kapalı</b>. Açıkken müziği gürültü sanıp keser.</li>
          <li><b>Yankı giderme</b> ve <b>Otomatik kazanç kontrolü</b>: kapalı.</li>
          <li><b>Giriş hassasiyeti</b>: otomatiği kapatıp çubuğu en sola al; yoksa şarkının sessiz yerlerinde ses gider gelir.</li>
          <li>Bas-konuş kullanıyorsan şarkı yalnız tuşa basılıyken gider; <b>Ses etkinliği</b> kipine geç.</li>
        </ul>
      </details>
    </section>

    <section class="panel wide tips">
      <ul class="sub small">
        <li><Icon name="keyboard" size={14} /><span><b>Num Lock açık olmalı</b>; kapalıyken numpad tuşları ok tuşu gibi çalışır.</span></li>
        <li><Icon name="info" size={14} /><span>Panel açıkken atadığın tuşlar başka programlara gitmez (oyunda numpad kullanıyorsan o tuşları değiştir). Panel kapanınca hepsi serbest kalır.</span></li>
        <li><Icon name="shield" size={14} /><span>Ses dosyaları yerinde kalır, kopyalanmaz. Oyun içindeyken bastığında sağ üstte çalan sesin adı görünür.</span></li>
      </ul>
    </section>
  {/if}
</div>

<style>
  .sp { display: grid; grid-template-columns: repeat(auto-fit, minmax(380px, 1fr)); gap: 16px; max-width: 1400px; margin: 0 auto; align-items: start; }
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .wide { grid-column: 1 / -1; }
  .warn, .note { flex-direction: row; align-items: center; gap: 10px; }
  .warn { color: var(--gold); }
  .warn span, .note span { flex: 1; }
  .note { color: var(--muted); }
  .bad { color: var(--bad); }
  .hero { flex-direction: row; align-items: center; justify-content: space-between; gap: 16px; flex-wrap: wrap; }
  .hero-l { display: flex; align-items: center; gap: 14px; min-width: 0; flex: 1; }
  .hero-l > div { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .ic { width: 44px; height: 44px; border-radius: 12px; display: grid; place-items: center; background: var(--coal); color: var(--muted); flex: none; }
  .hero.live { border-color: rgba(227, 194, 112, 0.45); }
  .hero.live .ic { color: var(--gold); background: rgba(227, 194, 112, 0.14); }
  .setup .steps { margin: 0; padding-left: 20px; display: flex; flex-direction: column; gap: 10px; color: var(--muted); }
  .setup .steps li b { color: var(--text); }
  .setup .steps .btn { margin-left: 10px; padding: 6px 12px; font-size: 13px; }
  .btn.open { border-color: var(--gold-dim); color: var(--gold-glow); }

  .media { display: flex; flex-direction: column; gap: 4px; max-height: 260px; overflow: auto; padding: 8px; border-radius: 12px; border: 1px solid var(--line-2); background: rgba(255, 255, 255, 0.02); }
  .mrow { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 10px; align-items: center; padding: 7px 10px; border: 0; border-radius: 8px; background: none; text-align: left; color: var(--muted); }
  .mrow:hover:not(:disabled) { background: rgba(255, 255, 255, 0.05); color: var(--text); }
  .mrow:disabled { opacity: 0.5; cursor: default; }
  .mt { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text); }

  .pads { display: grid; grid-template-columns: repeat(auto-fill, minmax(210px, 1fr)); gap: 10px; }
  .pad { display: flex; flex-direction: column; gap: 8px; padding: 10px 12px 12px; border-radius: 14px; border: 1px solid var(--line); background: rgba(255, 255, 255, 0.025); transition: border-color 0.15s, background 0.15s; }
  .pad.playing { border-color: var(--gold); background: rgba(227, 194, 112, 0.08); box-shadow: 0 0 0 2px rgba(227, 194, 112, 0.18); }
  .pad.missing { opacity: 0.7; }
  .pad-top, .pad-bot { display: flex; align-items: center; justify-content: space-between; gap: 6px; min-width: 0; }
  .key { display: inline-flex; align-items: center; gap: 3px; min-height: 28px; padding: 3px 8px; border-radius: 8px; border: 1px dashed transparent; background: none; font-size: 12px; color: var(--muted); }
  .key:hover { border-color: var(--line-2); }
  .key.none { border-color: var(--line-2); }
  .key.wait { border-color: var(--gold); color: var(--gold-glow); animation: blink 1s ease-in-out infinite; }
  @keyframes blink { 50% { opacity: 0.55; } }
  kbd { font-family: var(--f-body); font-size: 11.5px; font-weight: 700; padding: 2px 7px; border-radius: 6px; background: var(--coal); border: 1px solid var(--line-2); color: var(--text); }
  .pad.playing kbd { border-color: var(--gold-dim); color: var(--gold-glow); }
  .del { display: inline-flex; align-items: center; gap: 4px; border: 0; background: none; color: var(--dim); padding: 5px; border-radius: 6px; font-size: 12px; opacity: 0.6; }
  .pad:hover .del, .del.armed { opacity: 1; }
  .del:hover, .del.armed { color: var(--bad); background: rgba(224, 106, 106, 0.1); }
  .play { display: flex; align-items: center; gap: 10px; padding: 8px 6px; border: 0; border-radius: 10px; background: none; text-align: left; min-width: 0; }
  .play:hover:not(:disabled) { background: rgba(255, 255, 255, 0.04); }
  .play:disabled { cursor: default; }
  .play b { font-weight: 600; font-size: 14.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pi { width: 36px; height: 36px; border-radius: 50%; display: grid; place-items: center; flex: none; background: var(--coal); color: var(--text); }
  .play:hover:not(:disabled) .pi { background: var(--gold); color: #1b1608; }
  .playing .pi { background: var(--gold); color: #1b1608; }
  .play:disabled .pi { opacity: 0.5; }
  .rename { flex: 1; min-width: 0; background: var(--coal-2); border: 1px solid var(--gold-dim); border-radius: 6px; padding: 4px 8px; font-weight: 600; }
  .fn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .mini { border: 0; background: transparent; color: var(--dim); padding: 4px; border-radius: 6px; display: grid; place-items: center; flex: none; }
  .mini:hover { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .pv { display: grid; grid-template-columns: auto minmax(0, 1fr) 40px; gap: 8px; align-items: center; color: var(--dim); }
  .pv .small { text-align: right; color: var(--muted); }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 26px 10px; color: var(--dim); text-align: center; }

  .field { display: flex; flex-direction: column; gap: 6px; }
  .field > span { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .field > span b { color: var(--gold); margin-left: 4px; }
  .field em { font-style: normal; }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  .line > div { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .how summary { cursor: pointer; color: var(--muted); font-size: 13px; }
  .how ul { margin: 8px 0 0; padding-left: 18px; display: flex; flex-direction: column; gap: 6px; }
  .tips ul { margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: 8px; }
  .tips li { display: flex; gap: 10px; align-items: flex-start; }
  .tips li :global(svg) { color: var(--teal); margin-top: 1px; flex: none; }
  @media (max-width: 1000px) { .sp { grid-template-columns: minmax(0, 1fr); } }
</style>
