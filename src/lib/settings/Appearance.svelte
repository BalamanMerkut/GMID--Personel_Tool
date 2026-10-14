<script lang="ts">
  // Görünüm: arka plan (Malikâne / düz renk / kendi resmin, GIF'in ya da videon), canlı önizleme, eski arka
  // planlara tek tıkla dönüş, kalite ve açılış animasyonu.
  import { onMount } from "svelte";
  import { api, formatSize, inTauri, isVideo } from "../api";
  import Switch from "../components/Switch.svelte";
  import Icon from "../Icon.svelte";
  import { save, settings, type BgFocus, type BgKind, type Quality } from "../settings.svelte";
  import { notify } from "../toast.svelte";

  let { onreplay }: { onreplay: () => void } = $props();

  const KINDS: [BgKind, string, string][] = [
    ["default", "Malikâne", "GMID'in kendi arka planı"],
    ["color", "Düz renk", "Sade, en hafifi"],
    ["file", "Kendi arka planın", "Resim, GIF ya da video"],
  ];
  const FOCI: [BgFocus, string][] = [
    ["left", "Sol"],
    ["top", "Üst"],
    ["center", "Orta"],
    ["bottom", "Alt"],
    ["right", "Sağ"],
  ];
  const QUALITIES: [Quality, string, string, "sparkle" | "bolt" | "gear"][] = [
    ["auto", "Otomatik", "Windows'taki “animasyonları azalt” ayarına göre kendisi seçer.", "gear"],
    ["high", "Yüksek", "Buzlu cam paneller, hareketli arka plan ve bulanıklık açık.", "sparkle"],
    ["light", "Hafif", "Bulanıklık ve hareket kapalı; eski bilgisayarlarda en akıcısı.", "bolt"],
  ];
  const SWATCHES = ["#16181c", "#0b203e", "#1d1a12", "#1f1430", "#0f2422", "#2a1214", "#101820"];
  const FOCUS_POS: Record<BgFocus, string> = { center: "50% 50%", top: "50% 18%", bottom: "50% 82%", left: "18% 50%", right: "82% 50%" };

  let library = $state<{ path: string; size: number }[]>([]);
  let armed = $state<string | null>(null);

  async function loadLibrary() {
    library = await api.backgrounds().catch(() => []);
  }
  onMount(() => void loadLibrary());

  const src = (p: string) => (p ? api.fileSrc(p) : "");
  const fileName = (p: string) => (p.split(/[\\/]/).pop() ?? "").replace(/^\d{8}-\d{6}-/, "");
  const isGif = (p: string) => /\.gif$/i.test(p);
  const current = $derived(settings.bgKind === "file" ? settings.bgPath : "");

  async function pickFile() {
    try {
      const path = await api.pickBackground();
      if (!path) {
        if (!inTauri) notify("Dosya seçmek masaüstü uygulamasında çalışır", "info");
        return;
      }
      await use(path);
      await loadLibrary();
      notify("Arka plan değişti", "image");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  async function use(path: string) {
    await save("bgPath", path);
    await save("bgKind", "file");
  }

  async function setKind(kind: BgKind) {
    if (kind === "file" && !settings.bgPath) return pickFile();
    await save("bgKind", kind);
  }

  /** İki tıkla silme: ilk tık "Sil?" sorar. */
  async function remove(path: string) {
    if (armed !== path) {
      armed = path;
      setTimeout(() => armed === path && (armed = null), 3000);
      return;
    }
    armed = null;
    try {
      await api.removeBackground(path);
      await loadLibrary();
    } catch (e) {
      notify(String(e), "info");
    }
  }

  // Önizleme küçük olduğu için bulanıklık oranla küçültülür (pencere ~1400 px, önizleme ~340 px).
  const previewBlur = $derived(settings.bgKind === "file" ? Math.round(settings.bgBlur * 0.25 * 10) / 10 : 0);
</script>

<section class="panel">
  <h3 class="h2"><Icon name="palette" />Görünüm</h3>

  <div class="top">
    <div class="kinds">
      <span class="label">Arka plan</span>
      {#each KINDS as [k, l, d] (k)}
        <button class="kind" aria-pressed={settings.bgKind === k} onclick={() => setKind(k)}>
          <span class="thumb {k}">
            {#if k === "color"}
              <i style:background={settings.bgColor}></i>
            {:else if k === "file"}
              {#if settings.bgPath && isVideo(settings.bgPath)}
                <video src={src(settings.bgPath)} muted playsinline preload="metadata"></video>
              {:else if settings.bgPath}
                <img src={src(settings.bgPath)} alt="" />
              {:else}
                <Icon name="image" size={20} />
              {/if}
            {:else}
              <svg viewBox="0 0 520 300" class="castle"><path d="M10 290h500M40 290V120h34v22h22v-22h22v22h22v-22h34v170M150 290V170h220v120M212 170V86h96v84M200 86l60-66 60 66M372 290V120h34v22h22v-22h22v22h22v-22h34v170" /></svg>
            {/if}
          </span>
          <span class="kt"><b>{l}</b><span class="sub small">{d}</span></span>
          {#if settings.bgKind === k}<span class="tick"><Icon name="check" size={14} /></span>{/if}
        </button>
      {/each}
    </div>

    <!-- Canlı önizleme: GMID penceresinin küçük bir taslağı, seçili arka planın üstünde. -->
    <figure class="mock" aria-label="Önizleme">
      <div class="mbg">
        {#if settings.bgKind === "color"}
          <div class="fill" style:background={settings.bgColor}></div>
        {:else if settings.bgKind === "file" && settings.bgPath}
          {#if isVideo(settings.bgPath)}
            <video src={src(settings.bgPath)} muted loop autoplay playsinline style:object-position={FOCUS_POS[settings.bgFocus]} style:filter={previewBlur ? `blur(${previewBlur}px)` : null}></video>
          {:else}
            <img src={src(settings.bgPath)} alt="" style:object-position={FOCUS_POS[settings.bgFocus]} style:filter={previewBlur ? `blur(${previewBlur}px)` : null} />
          {/if}
          <div class="fill" style:background="rgba(10, 11, 13, {settings.bgDim / 100})"></div>
        {:else}
          <div class="fill mansion"></div>
          <svg viewBox="0 0 520 300" class="castle big"><path d="M10 290h500M40 290V120h34v22h22v-22h22v22h22v-22h34v170M150 290V170h220v120M212 170V86h96v84M200 86l60-66 60 66M240 290v-52a20 20 0 0 1 40 0v52M372 290V120h34v22h22v-22h22v22h22v-22h34v170" /></svg>
        {/if}
      </div>
      <div class="mside"><i class="logo"></i><i></i><i class="on"></i><i></i><i></i><i></i></div>
      <div class="mmain">
        <i class="mtitle"></i>
        <div class="mcards"><i></i><i></i><i></i></div>
        <i class="mwide"></i>
      </div>
      <figcaption class="sub small">Önizleme</figcaption>
    </figure>
  </div>

  {#if settings.bgKind === "color"}
    <div class="field">
      <span class="label">Renk</span>
      <div class="row">
        {#each SWATCHES as c (c)}
          <button class="swatch" style:background={c} aria-label="Renk {c}" aria-pressed={settings.bgColor === c} onclick={() => save("bgColor", c)}></button>
        {/each}
        <label class="row custom">
          <input type="color" value={settings.bgColor} oninput={(e) => save("bgColor", e.currentTarget.value)} />
          <span class="sub small">Özel renk</span>
        </label>
      </div>
    </div>
  {/if}

  {#if settings.bgKind === "file"}
    <div class="field">
      <span class="label">Arka planların <span class="sub small">· tıkla, hemen değişsin</span></span>
      <div class="lib">
        <button class="tile add" onclick={pickFile} title="Resim (JPG, PNG, WebP), GIF ya da video (MP4, WebM)">
          <Icon name="plus" size={22} /><span>Yeni ekle</span>
        </button>
        {#each library as b (b.path)}
          <div class="tile" class:on={current === b.path}>
            <button class="pick" onclick={() => use(b.path)} title={fileName(b.path)}>
              {#if isVideo(b.path)}
                <video src={`${src(b.path)}#t=0.5`} muted playsinline preload="metadata"></video>
              {:else}
                <img src={src(b.path)} alt="" loading="lazy" />
              {/if}
              <span class="tag">{isVideo(b.path) ? "Video" : isGif(b.path) ? "GIF" : "Resim"} · {formatSize(b.size)}</span>
            </button>
            {#if current === b.path}
              <span class="cur"><Icon name="check" size={13} /></span>
            {:else}
              <button class="del" class:armed={armed === b.path} onclick={() => remove(b.path)} title="Sil">
                <Icon name="trash" size={13} />{#if armed === b.path}<span>Sil?</span>{/if}
              </button>
            {/if}
          </div>
        {/each}
      </div>
      <span class="sub small">
        Seçtiğin dosya GMID'in <code>Arkaplanlar</code> klasörüne kopyalanır; orijinalini silsen de bozulmaz. Hareketli
        arka plan için <b>MP4 video</b>, GIF'ten çok daha küçük ve akıcıdır; GIF'e çevirmene gerek yok. GIF ve videolar
        pencere arkada kalınca ya da oyundayken kendiliğinden durur.
      </span>
    </div>

    <div class="sliders">
      <label class="field">
        <span class="label">Bulanıklık <b class="tnum">{settings.bgBlur} px</b></span>
        <input type="range" min="0" max="20" value={settings.bgBlur} oninput={(e) => save("bgBlur", +e.currentTarget.value)} />
      </label>
      <label class="field">
        <span class="label">Karartma <b class="tnum">%{settings.bgDim}</b></span>
        <input type="range" min="0" max="80" value={settings.bgDim} oninput={(e) => save("bgDim", +e.currentTarget.value)} />
      </label>
      <div class="field">
        <span class="label">Pencere daralınca görünen kısım</span>
        <div class="seg">
          {#each FOCI as [f, l] (f)}
            <button aria-pressed={settings.bgFocus === f} onclick={() => save("bgFocus", f)}>{l}</button>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  <div class="divider"></div>

  <div class="field">
    <span class="label">Görünüm kalitesi</span>
    <div class="quals">
      {#each QUALITIES as [q, l, d, icon] (q)}
        <button class="qual" aria-pressed={settings.quality === q} onclick={() => save("quality", q)}>
          <span class="qi"><Icon name={icon} size={18} /></span>
          <b>{l}</b>
          <span class="sub small">{d}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="line">
    <div>
      <b>Açılış animasyonu</b>
      <span class="sub small">Uygulama kapalıyken açıldığında Merkut Studios logosu belirir. Tepsiden açarken oynamaz.</span>
    </div>
    <div class="row">
      <button class="btn ghost" onclick={onreplay}><Icon name="replay" />Şimdi oynat</button>
      <Switch label="Açılış animasyonu" checked={settings.splash} onchange={(v) => save("splash", v)} />
    </div>
  </div>
</section>

<style>
  .panel { display: flex; flex-direction: column; gap: 18px; }
  .field { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .label { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .label b { color: var(--gold); margin-left: 6px; }
  .label .sub { text-transform: none; letter-spacing: 0; font-weight: 500; }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .line > div:first-child { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1 1 260px; }
  .divider { height: 1px; background: var(--line); }

  .top { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 360px); gap: 20px; align-items: start; }
  .kinds { display: flex; flex-direction: column; gap: 8px; }
  .kind { position: relative; display: flex; align-items: center; gap: 12px; padding: 8px 12px 8px 8px; border-radius: 12px; border: 1px solid var(--line-2); background: var(--coal-2); color: inherit; font: inherit; text-align: left; cursor: pointer; }
  .kind:hover { border-color: var(--gold-dim); }
  .kind[aria-pressed="true"] { border-color: var(--gold); background: rgba(227, 194, 112, 0.08); }
  .thumb { width: 72px; height: 44px; border-radius: 8px; overflow: hidden; flex: none; display: grid; place-items: center; background: var(--night); border: 1px solid var(--line); color: var(--dim); position: relative; }
  .thumb i { position: absolute; inset: 0; }
  .thumb img, .thumb video { width: 100%; height: 100%; object-fit: cover; }
  .thumb .castle { width: 90%; }
  .kt { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .kt b { font-size: 14px; }
  .tick { position: absolute; right: 12px; top: 50%; transform: translateY(-50%); width: 22px; height: 22px; border-radius: 50%; display: grid; place-items: center; background: var(--gold); color: #1b1608; }
  .castle { fill: none; stroke: var(--gold); stroke-width: 6; stroke-linecap: round; stroke-linejoin: round; opacity: 0.5; }

  .mock { position: relative; margin: 0; aspect-ratio: 16 / 10; border-radius: 12px; overflow: hidden; border: 1px solid var(--line-2); box-shadow: 0 14px 30px rgba(0, 0, 0, 0.45); display: grid; grid-template-columns: 22% 1fr; }
  .mbg { position: absolute; inset: 0; background: var(--night); }
  .mbg .fill, .mbg img, .mbg video { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .mbg img, .mbg video { transform: scale(1.04); }
  .mansion { background: radial-gradient(300px 150px at 85% -10%, rgba(227, 194, 112, 0.1), transparent 60%), radial-gradient(240px 180px at 100% 110%, rgba(11, 32, 62, 0.7), transparent 60%), var(--night); }
  .castle.big { position: absolute; right: -2%; bottom: -1%; width: 62%; stroke-width: 1.6; opacity: 0.14; }
  .mside, .mmain { position: relative; display: flex; flex-direction: column; }
  .mside { gap: 5px; padding: 8px 6px; background: rgba(18, 20, 23, 0.82); border-right: 1px solid rgba(255, 255, 255, 0.06); }
  .mside i { height: 6px; border-radius: 3px; background: rgba(255, 255, 255, 0.12); }
  .mside i.logo { height: 12px; width: 60%; background: rgba(227, 194, 112, 0.5); margin-bottom: 4px; }
  .mside i.on { background: rgba(227, 194, 112, 0.45); }
  .mmain { gap: 8px; padding: 10px; }
  .mtitle { height: 8px; width: 34%; border-radius: 4px; background: rgba(255, 255, 255, 0.3); }
  .mcards { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; }
  .mcards i, .mwide { border-radius: 7px; background: rgba(28, 31, 36, 0.72); border: 1px solid rgba(255, 255, 255, 0.07); backdrop-filter: blur(6px); }
  .mcards i { height: 46px; }
  .mwide { flex: 1; min-height: 40px; }
  .mock figcaption { position: absolute; right: 8px; bottom: 6px; padding: 1px 7px; border-radius: 6px; background: rgba(0, 0, 0, 0.5); font-size: 10.5px; }

  .swatch { width: 32px; height: 32px; border-radius: 8px; border: 1px solid var(--line-2); }
  .swatch[aria-pressed="true"] { outline: 2px solid var(--gold); outline-offset: 2px; }
  .custom input { width: 40px; height: 32px; padding: 0; border: 1px solid var(--line-2); border-radius: 8px; background: none; }

  .lib { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 10px; }
  .tile { position: relative; aspect-ratio: 16 / 9; border-radius: 10px; overflow: hidden; border: 1px solid var(--line-2); background: var(--coal-2); }
  .tile.on { border-color: var(--gold); box-shadow: 0 0 0 2px rgba(227, 194, 112, 0.35); }
  .tile.add { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px; border-style: dashed; color: var(--muted); font: inherit; font-size: 13px; cursor: pointer; }
  .tile.add:hover { color: var(--gold-glow); border-color: var(--gold-dim); }
  .pick { position: absolute; inset: 0; padding: 0; border: 0; background: none; cursor: pointer; }
  .pick img, .pick video { width: 100%; height: 100%; object-fit: cover; display: block; }
  .tag { position: absolute; left: 6px; bottom: 6px; padding: 1px 6px; border-radius: 6px; background: rgba(0, 0, 0, 0.6); color: #fff; font-size: 10.5px; }
  .cur, .del { position: absolute; top: 6px; right: 6px; height: 22px; min-width: 22px; border-radius: 7px; display: inline-flex; align-items: center; justify-content: center; gap: 4px; }
  .cur { background: var(--gold); color: #1b1608; }
  .del { border: 0; padding: 0 5px; background: rgba(0, 0, 0, 0.6); color: #fff; opacity: 0; transition: opacity 0.15s; font-size: 11px; font-weight: 700; }
  .tile:hover .del, .del.armed { opacity: 1; }
  .del.armed { background: var(--bad); }
  .field code { padding: 1px 6px; }

  .sliders { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 16px 24px; }
  .quals { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; }
  .qual { display: flex; flex-direction: column; align-items: flex-start; gap: 4px; padding: 12px 14px; border-radius: 12px; border: 1px solid var(--line-2); background: var(--coal-2); color: inherit; font: inherit; text-align: left; cursor: pointer; }
  .qual:hover { border-color: var(--gold-dim); }
  .qual[aria-pressed="true"] { border-color: var(--gold); background: rgba(227, 194, 112, 0.08); }
  .qual[aria-pressed="true"] b, .qual[aria-pressed="true"] .qi { color: var(--gold-glow); }
  .qi { color: var(--muted); }
  @media (max-width: 1000px) {
    .top { grid-template-columns: minmax(0, 1fr); }
    .quals { grid-template-columns: minmax(0, 1fr); }
  }
</style>
