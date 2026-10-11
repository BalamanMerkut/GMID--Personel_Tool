<script lang="ts">
  import { api, inTauri, isVideo } from "../api";
  import Switch from "../components/Switch.svelte";
  import Icon from "../Icon.svelte";
  import { save, settings, type BgFocus, type BgKind, type Quality } from "../settings.svelte";
  import { notify } from "../toast.svelte";

  let { onreplay }: { onreplay: () => void } = $props();

  const KINDS: [BgKind, string][] = [
    ["default", "Malikâne"],
    ["color", "Düz renk"],
    ["file", "Resim, GIF ya da video"],
  ];
  const FOCI: [BgFocus, string][] = [
    ["center", "Orta"],
    ["top", "Üst"],
    ["bottom", "Alt"],
    ["left", "Sol"],
    ["right", "Sağ"],
  ];
  const QUALITIES: [Quality, string, string][] = [
    ["auto", "Otomatik", "Bilgisayarın hareket azaltma ayarına göre kendisi seçer."],
    ["high", "Yüksek", "Bulanık cam paneller ve hareketli arka plan açık."],
    ["light", "Hafif", "Bulanıklık ve hareket kapalı; en az kaynak kullanır."],
  ];
  const SWATCHES = ["#16181c", "#0b203e", "#1d1a12", "#1f1430", "#0f2422"];

  const fileName = $derived(settings.bgPath.split(/[\\/]/).pop() ?? "");
  const preview = $derived(settings.bgPath ? api.fileSrc(settings.bgPath) : "");

  async function pickFile() {
    try {
      const path = await api.pickBackground();
      if (!path) {
        if (!inTauri) notify("Dosya seçmek masaüstü uygulamasında çalışır", "info");
        return;
      }
      await save("bgPath", path);
      await save("bgKind", "file");
      notify("Arka plan değişti", "image");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  async function setKind(kind: BgKind) {
    if (kind === "file" && !settings.bgPath) return pickFile();
    await save("bgKind", kind);
  }
</script>

  <section class="panel">
    <h3 class="h2"><Icon name="palette" />Görünüm</h3>

    <div class="field">
      <span class="label">Arka plan</span>
      <div class="seg">
        {#each KINDS as [k, l] (k)}
          <button aria-pressed={settings.bgKind === k} onclick={() => setKind(k)}>{l}</button>
        {/each}
      </div>
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
      <div class="filebox">
        <div class="preview">
          {#if preview && isVideo(settings.bgPath)}
            <video src={preview} muted playsinline preload="metadata"></video>
          {:else if preview}
            <img src={preview} alt="Seçili arka plan" />
          {/if}
        </div>
        <div class="fileinfo">
          <b class="ellipsis" title={fileName}>{fileName || "Dosya seçilmedi"}</b>
          <span class="sub small">Dosya <code>Arkaplanlar</code> klasörüne kopyalandı; orijinali silsen de bozulmaz.</span>
          <button class="btn ghost" onclick={pickFile}><Icon name="image" />Başka dosya seç</button>
        </div>
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
          <span class="label">Pencere daralınca ortada kalacak kısım</span>
          <div class="seg">
            {#each FOCI as [f, l] (f)}
              <button aria-pressed={settings.bgFocus === f} onclick={() => save("bgFocus", f)}>{l}</button>
            {/each}
          </div>
        </div>
      </div>
      <p class="sub small">GIF ve videolar pencere arkada kaldığında ya da küçültüldüğünde kendiliğinden durur.</p>
    {/if}

    <div class="divider"></div>

    <div class="field">
      <span class="label">Görünüm kalitesi</span>
      <div class="seg">
        {#each QUALITIES as [q, l] (q)}
          <button aria-pressed={settings.quality === q} onclick={() => save("quality", q)}>{l}</button>
        {/each}
      </div>
      <span class="sub small">{QUALITIES.find(([q]) => q === settings.quality)?.[2]}</span>
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
  .panel { display: flex; flex-direction: column; gap: 16px; }
  .field { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .label { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .label b { color: var(--gold); margin-left: 6px; }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .line > div:first-child { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1 1 260px; }
  .divider { height: 1px; background: var(--line); }
  .swatch { width: 32px; height: 32px; border-radius: 8px; border: 1px solid var(--line-2); }
  .swatch[aria-pressed="true"] { outline: 2px solid var(--gold); outline-offset: 2px; }
  .custom input { width: 40px; height: 32px; padding: 0; border: 1px solid var(--line-2); border-radius: 8px; background: none; }
  .filebox { display: grid; grid-template-columns: minmax(0, 260px) minmax(0, 1fr); gap: 16px; align-items: center; }
  .preview { aspect-ratio: 16 / 9; max-width: 100%; border-radius: 10px; overflow: hidden; background: var(--coal-2); border: 1px solid var(--line); }
  .preview img, .preview video { width: 100%; height: 100%; object-fit: cover; display: block; }
  .fileinfo { display: flex; flex-direction: column; gap: 8px; align-items: flex-start; min-width: 0; }
  .ellipsis { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fileinfo code { padding: 1px 6px; }
  .sliders { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 16px 24px; }
  @media (max-width: 1000px) {
    .filebox { grid-template-columns: minmax(0, 1fr); }
  }
</style>
