<script lang="ts">
  // Dinleme odası paneli (sağ altta, her ekranda): şu an çalan (videoda görüntü), oynatma, şarkı ekleme, sıra,
  // odadakiler; sahip için ayarlar, DJ yapma, çıkarma, odayı kapatma.
  import Icon from "../../Icon.svelte";
  import {
    at,
    atla,
    ayarla,
    ayril,
    benimOdam,
    birlikte,
    djYap,
    ekle,
    ekleyebilir,
    karistirDegistir,
    medyaElemani,
    medyaYedekKutusu,
    tekrarDegistir,
    odaKapat,
    oynatDuraklat,
    sar,
    sesAyarla,
    siradanCikar,
    simdiCal,
    yonetir,
    type Gizlilik,
    type Yetki,
  } from "../../birlikte.svelte";
  import { formatDuration } from "../../downloads.svelte";
  import { profilGoster, sosyal } from "../../social.svelte";
  import Avatar from "./Avatar.svelte";

  const oda = $derived(benimOdam());
  const benId = $derived(sosyal.ben?.id ?? "");
  const sahibim = $derived(!!oda && oda.sahip.id === benId);
  const yonetici = $derived(!!oda && yonetir(oda, benId));
  const ekler = $derived(!!oda && ekleyebilir(oda, benId));

  let giris = $state("");
  const video = false;
  let ayarAcik = $state(false);
  /** Geniş panel (videoyu büyük izlemek için). */
  let genis = $state(false);
  /** Tam ekran: panel bütün pencereyi kaplar, pencere de tam ekrana geçer. */
  let tam = $state(false);
  async function pencereTam(on: boolean) {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().setFullscreen(on);
    } catch {
      /* tarayıcıda ya da izin yoksa yalnız pencere içinde büyür */
    }
  }
  function tamEkran(on = !tam) {
    tam = on;
    void pencereTam(on);
  }
  // Oda kapanınca / panel küçülünce tam ekrandan çık.
  $effect(() => {
    if (tam && (!birlikte.panel || !birlikte.odaId)) tamEkran(false);
  });
  let kapatOnay = $state(false);

  // Ayar taslağı (açılınca odadakiyle dolar).
  let tAd = $state("");
  let tSinir = $state("0");
  let tGizlilik = $state<Gizlilik>("herkes");
  let tYetki = $state<Yetki>("herkes");
  function ayarlariAc() {
    if (!oda) return;
    tAd = oda.ad;
    tSinir = String(oda.sinir ?? 0);
    tGizlilik = oda.gizlilik;
    tYetki = oda.yetki;
    ayarAcik = !ayarAcik;
  }
  async function kaydet() {
    await ayarla({ ad: tAd, sinir: Number(tSinir), gizlilik: tGizlilik, yetki: tYetki });
    ayarAcik = false;
  }

  function gonder(e: SubmitEvent) {
    e.preventDefault();
    if (!giris.trim()) return;
    void ekle(giris, video).then(() => (giris = ""));
  }

  // Video/ses öğesi panelin içine yerleşir (panel kapanınca da çalmaya devam eder).
  function yerlestir(kutu: HTMLElement) {
    const m = medyaElemani();
    kutu.appendChild(m);
    return {
      destroy() {
        // Silinmez, gizli kutuya taşınır: panel küçülünce müzik durmasın.
        const caliyordu = !m.paused;
        if (!m.isConnected || m.parentElement === kutu) medyaYedekKutusu().appendChild(m);
        // Yine de duraklatıldıysa sürdür.
        if (caliyordu) setTimeout(() => m.paused && void m.play().catch(() => {}), 60);
      },
    };
  }

  const YETKI: Record<Yetki, string> = { sahip: "Yalnız ben", dj: "Ben ve DJ'ler", herkes: "Odadaki herkes" };
  const EKLEYEN: Record<Yetki, string> = { sahip: "yalnız oda sahibi", dj: "sahip ve DJ'ler", herkes: "odadaki herkes" };
  const gorunur = $derived(!!oda?.simdiki?.sarki.video);
</script>

<svelte:window onkeydown={(e) => tam && e.key === "Escape" && tamEkran(false)} />

{#if oda && birlikte.panel && !birlikte.salonAcik}
  <section class="panel-oda" class:genis class:tam aria-label="Dinleme odası">
    <header>
      <span class="baslik">
        <Icon name="headphones" size={16} />
        <b title={oda.ad}>{oda.ad}</b>
        {#if oda.gizlilik === "arkadaslar"}<span class="etiket" title="Yalnız arkadaşlar görür"><Icon name="lock" size={12} /></span>{/if}
        <span class="sub small tnum">{oda.uyeler.length}{oda.sinir ? `/${oda.sinir}` : ""} kişi</span>
      </span>
      {#if sahibim}<button class="ic" title="Oda ayarları" aria-label="Oda ayarları" onclick={ayarlariAc}><Icon name="gear" size={15} /></button>{/if}
      <button class="ic" title={genis ? "Paneli daralt" : "Paneli genişlet (büyük izle)"} aria-label="Genişlet" onclick={() => (genis = !genis)}><Icon name={genis ? "shrink" : "expand"} size={15} /></button>
      <button class="ic" title="Küçült (çalmaya devam eder)" aria-label="Küçült" onclick={() => (birlikte.panel = false)}><Icon name="shrinkdown" size={16} /></button>
    </header>

    {#if ayarAcik && sahibim}
      <div class="ayarlar">
        <label class="genis"><span>Oda adı</span><input bind:value={tAd} maxlength="40" /></label>
        <label><span>Kişi sınırı</span>
          <select bind:value={tSinir}><option value="0">Sınırsız</option>{#each [2, 3, 4, 5, 6, 8, 10, 15, 20] as n (n)}<option value={String(n)}>{n} kişi</option>{/each}</select>
        </label>
        <label><span>Kimler görür</span>
          <select bind:value={tGizlilik}><option value="herkes">Herkes</option><option value="arkadaslar">Arkadaşlarım</option></select>
        </label>
        <label class="genis"><span>Kim şarkı ekler</span>
          <select bind:value={tYetki}>{#each Object.entries(YETKI) as [k, v] (k)}<option value={k}>{v}</option>{/each}</select>
        </label>
        <div class="genis sag"><button class="btn sm" onclick={kaydet}>Kaydet</button></div>
      </div>
    {/if}

    <!-- Şu an çalan -->
    <div class="sahne" class:video={gorunur} role="presentation" ondblclick={() => gorunur && tamEkran()}>
      <div class="medya" use:yerlestir></div>
      {#if gorunur}
        <button class="tam-dugme" title={tam ? "Tam ekrandan çık (Esc)" : "Tam ekran (çift tık)"} aria-label="Tam ekran" onclick={() => tamEkran()}><Icon name={tam ? "shrink" : "expand"} size={16} /></button>
      {/if}
      {#if !gorunur}
        <div class="kapak">
          {#if oda.simdiki?.sarki.kapak}<img src={oda.simdiki.sarki.kapak} alt="" />{:else}<Icon name="headphones" size={44} />{/if}
        </div>
      {/if}
    </div>
    {#if oda.simdiki}
      <div class="simdi">
        <b title={oda.simdiki.sarki.baslik}>{birlikte.hazirlaniyor ? "Hazırlanıyor…" : oda.simdiki.sarki.baslik}</b>
        <span class="sub small">{oda.simdiki.ekleyen.ad} ekledi{oda.caliyor ? "" : " · duraklatıldı"}</span>
        <div class="ilerle">
          <span class="tnum">{formatDuration(birlikte.konum) || "0:00"}</span>
          <input type="range" min="0" max={birlikte.sure || oda.simdiki.sarki.sure || 1} step="1" value={birlikte.konum} disabled={!yonetici} onchange={(e) => sar(+e.currentTarget.value)} aria-label="Konum" />
          <span class="tnum">{formatDuration(birlikte.sure || oda.simdiki.sarki.sure) || "–"}</span>
        </div>
      </div>
    {:else}
      <p class="sub small bos">Şu an bir şey çalmıyor. {ekler ? "Aşağıdan şarkı ya da video ekle." : "Ekleme yetkisi olan birinin eklemesini bekle."}</p>
    {/if}
    <div class="kontrol">
      {#if yonetici}
        <button class="oynat" title={oda.caliyor ? "Duraklat" : "Oynat"} disabled={!oda.simdiki} onclick={oynatDuraklat}><Icon name={oda.caliyor ? "pause" : "play"} size={18} /></button>
        <button class="ic" title="Sıradakine geç" aria-label="Sıradakine geç" disabled={!oda.simdiki} onclick={atla}><Icon name="next" size={18} /></button>
      {/if}
      {#if yonetici || oda.karisik || (oda.tekrar && oda.tekrar !== "yok")}
        <button class="ic" class:acik={oda.karisik} disabled={!yonetici} title={oda.karisik ? "Karışık çalıyor (kapat)" : "Karıştırarak çal"} aria-label="Karıştır" onclick={karistirDegistir}><Icon name="shuffle" size={17} /></button>
        <button class="ic tekrar" class:acik={!!oda.tekrar && oda.tekrar !== "yok"} disabled={!yonetici} title={oda.tekrar === "sarki" ? "Şarkıyı tekrarla (sonraki: listeyi tekrarla)" : oda.tekrar === "liste" ? "Listeyi tekrarla (sonraki: kapalı)" : "Tekrar kapalı (sonraki: şarkıyı tekrarla)"} aria-label="Tekrar" onclick={tekrarDegistir}>
          <Icon name="repeat" size={17} />{#if oda.tekrar === "sarki"}<span class="bir">1</span>{/if}
        </button>
      {/if}
      <span class="bosluk"></span>
      <Icon name="speaker" size={15} />
      <input class="ses" type="range" min="0" max="1" step="0.05" value={birlikte.ses} oninput={(e) => sesAyarla(+e.currentTarget.value)} aria-label="Ses" />
    </div>

    {#if ekler}
      <form class="ekle" onsubmit={gonder}>
        <input bind:value={giris} placeholder={oda.tur === "video" ? "YouTube bağlantısı, video adı ya da listesi" : "Şarkı adı, YouTube bağlantısı ya da listesi"} disabled={!!birlikte.asama} />
        <button class="btn sm" disabled={!!birlikte.asama || !giris.trim()}>{birlikte.asama || "Ekle"}</button>
      </form>
    {/if}

    <div class="liste">
      <span class="ara-baslik">Sıradakiler {oda.sira.length ? `(${oda.sira.length})` : ""}</span>
      {#each oda.sira as o, i (o.id)}
        <div class="satir">
          <span class="no tnum">{i + 1}</span>
          {#if yonetici}
            <button class="satir-yazi cal" title="Şimdi çal" onclick={() => simdiCal(o.id)}>
              <b title={o.sarki.baslik}>{o.sarki.video ? "🎬 " : ""}{o.sarki.baslik}</b><span class="sub small">{o.ekleyen.ad}</span>
              <span class="cal-ikon"><Icon name="play" size={13} /></span>
            </button>
          {:else}
            <span class="satir-yazi"><b title={o.sarki.baslik}>{o.sarki.video ? "🎬 " : ""}{o.sarki.baslik}</b><span class="sub small">{o.ekleyen.ad}</span></span>
          {/if}
          {#if yonetici || o.ekleyen.id === benId}
            <button class="ic kucuk" title="Sıradan çıkar" aria-label="Sıradan çıkar" onclick={() => siradanCikar(o.id)}><Icon name="x" size={13} /></button>
          {/if}
        </div>
      {:else}
        <p class="sub small">Sıra boş.</p>
      {/each}

      <span class="ara-baslik">Odadakiler</span>
      {#each oda.uyeler as u (u.id)}
        <div class="satir">
          <button class="kisi" onclick={() => profilGoster(u.id)}><Avatar kisi={u} boy={26} nokta={false} /><span>{u.ad}</span></button>
          {#if u.id === oda.sahip.id}<span class="rozet sahip">Sahip</span>{:else if oda.djler.includes(u.id)}<span class="rozet dj">DJ</span>{/if}
          {#if sahibim && u.id !== benId}
            <button class="mini" onclick={() => djYap(u.id, !oda.djler.includes(u.id))}>{oda.djler.includes(u.id) ? "DJ'liği al" : "DJ yap"}</button>
            <button class="mini kirmizi" onclick={() => at(u.id)}>Çıkar</button>
          {/if}
        </div>
      {/each}
    </div>

    <footer>
      <span class="sub small">Şarkı ekleyen: {EKLEYEN[oda.yetki]}</span>
      {#if sahibim}
        <button class="btn ghost sm" class:onay={kapatOnay} onclick={() => (kapatOnay ? odaKapat() : ((kapatOnay = true), setTimeout(() => (kapatOnay = false), 4000)))}>
          {kapatOnay ? "Oda kapansın mı?" : "Odayı kapat"}
        </button>
      {/if}
      <button class="btn ghost sm" onclick={ayril}>Ayrıl</button>
    </footer>
  </section>
{/if}

<style>
  .panel-oda { position: fixed; right: 18px; bottom: 76px; z-index: 65; width: 400px; max-height: calc(100vh - 120px); display: flex; flex-direction: column; gap: 10px; padding: 12px; border-radius: 18px; border: 1px solid rgba(227, 194, 112, 0.35); background: #14161a; box-shadow: 0 20px 60px rgba(0, 0, 0, 0.55); overflow: hidden; }
  header { display: flex; align-items: center; gap: 6px; }
  .baslik { flex: 1; min-width: 0; display: inline-flex; align-items: center; gap: 7px; color: var(--gold-glow); }
  .baslik b { color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .etiket { color: var(--dim); display: grid; }
  .ic { display: grid; place-items: center; width: 30px; height: 30px; border-radius: 8px; border: 0; background: none; color: var(--dim); cursor: pointer; }
  .ic:hover:not(:disabled) { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .ic:disabled { opacity: 0.35; }
  .ic.kucuk { width: 24px; height: 24px; }
  .ayarlar { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; padding: 10px; border-radius: 12px; background: rgba(0, 0, 0, 0.25); }
  .ayarlar label { display: flex; flex-direction: column; gap: 4px; font-size: 11.5px; color: var(--dim); }
  .ayarlar .genis { grid-column: 1 / -1; }
  .ayarlar .sag { display: flex; justify-content: flex-end; }
  .ayarlar input, .ayarlar select, .ekle input { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 9px; padding: 7px 9px; color: var(--text); font: inherit; font-size: 13px; outline: none; color-scheme: dark; }
  .sahne { position: relative; border-radius: 12px; overflow: hidden; background: #000; height: 120px; display: grid; place-items: center; }
  .sahne.video { height: auto; aspect-ratio: 16 / 9; }
  .medya { position: absolute; inset: 0; }
  .medya :global(video) { width: 100%; height: 100%; object-fit: contain; display: block; }
  .sahne:not(.video) .medya { visibility: hidden; }
  .kapak { position: relative; width: 100%; height: 100%; display: grid; place-items: center; color: var(--gold-dim); background: linear-gradient(135deg, rgba(227, 194, 112, 0.12), rgba(0, 0, 0, 0.5)); }
  .kapak img { width: 100%; height: 100%; object-fit: cover; opacity: 0.85; }
  .simdi { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .simdi b { font-size: 14.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ilerle { display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--dim); margin-top: 4px; }
  .ilerle input { flex: 1; accent-color: var(--gold); }
  .bos { margin: 0; }
  .ic.acik { color: var(--gold); }
  .ic.acik:disabled { opacity: 1; }
  .ic.tekrar { position: relative; }
  .bir { position: absolute; right: 3px; bottom: 2px; font-size: 9px; font-weight: 900; color: var(--gold); }
  .panel-oda.genis { width: min(760px, calc(100vw - 120px)); }
  .tam-dugme { position: absolute; right: 8px; bottom: 8px; z-index: 2; width: 34px; height: 34px; border-radius: 9px; border: 0; background: rgba(0, 0, 0, 0.55); color: #fff; display: grid; place-items: center; cursor: pointer; opacity: 0; transition: opacity 0.15s; }
  .sahne:hover .tam-dugme, .tam .tam-dugme { opacity: 1; }
  .satir-yazi.cal { position: relative; border: 0; background: none; color: inherit; font: inherit; text-align: left; padding: 3px 26px 3px 6px; margin: -3px 0 -3px -6px; border-radius: 8px; cursor: pointer; }
  .satir-yazi.cal:hover { background: rgba(227, 194, 112, 0.08); }
  .cal-ikon { position: absolute; right: 6px; top: 50%; transform: translateY(-50%); color: var(--gold); opacity: 0; transition: opacity 0.12s; }
  .satir-yazi.cal:hover .cal-ikon { opacity: 1; }
  /* Tam ekran: yalnız görüntü ve oynatma çubuğu. */
  .panel-oda.tam { inset: 0; right: 0; bottom: 0; width: auto; max-height: none; border-radius: 0; border: 0; padding: 0; gap: 0; z-index: 300; background: #000; display: grid; grid-template-rows: minmax(0, 1fr) auto auto; }
  .tam > header, .tam > .ayarlar, .tam > .ekle, .tam > .liste, .tam > footer, .tam > .bos { display: none; }
  .tam .sahne, .tam .sahne.video { height: 100%; aspect-ratio: auto; border-radius: 0; }
  .tam .simdi { padding: 10px 18px 0; background: #0b0c0e; }
  .tam .kontrol { padding: 8px 18px 14px; background: #0b0c0e; }
  .kontrol { display: flex; align-items: center; gap: 8px; color: var(--dim); }
  .oynat { width: 38px; height: 38px; border-radius: 50%; border: 0; background: var(--gold); color: #1b1608; display: grid; place-items: center; cursor: pointer; }
  .oynat:disabled { opacity: 0.4; }
  .bosluk { flex: 1; }
  .ses { width: 110px; accent-color: var(--gold); }
  .ekle { display: flex; gap: 6px; align-items: center; }
  .ekle input:not([type="checkbox"]) { flex: 1; min-width: 0; }
  .liste { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 4px; padding-right: 2px; }
  .ara-baslik { font-size: 11px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; color: var(--gold-dim); margin-top: 6px; }
  .satir { display: flex; align-items: center; gap: 8px; padding: 4px 2px; }
  .no { width: 18px; text-align: right; color: var(--dim); font-size: 12px; }
  .satir-yazi { flex: 1; min-width: 0; display: flex; flex-direction: column; line-height: 1.25; }
  .satir-yazi b { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .kisi { flex: 1; min-width: 0; display: flex; align-items: center; gap: 8px; border: 0; background: none; color: inherit; font: inherit; font-size: 13px; cursor: pointer; padding: 0; text-align: left; }
  .rozet { font-size: 10.5px; font-weight: 800; padding: 2px 7px; border-radius: 999px; }
  .rozet.sahip { background: rgba(227, 194, 112, 0.18); color: var(--gold-glow); }
  .rozet.dj { background: rgba(108, 182, 255, 0.16); color: #8cc6ff; }
  .mini { border: 1px solid var(--line-2); background: none; color: var(--muted); border-radius: 7px; padding: 2px 7px; font: inherit; font-size: 11.5px; cursor: pointer; white-space: nowrap; }
  .mini:hover { color: var(--text); }
  .mini.kirmizi:hover { color: #ff8a8d; border-color: rgba(229, 72, 77, 0.5); }
  footer { display: flex; align-items: center; gap: 6px; padding-top: 8px; border-top: 1px solid var(--line); }
  footer .sub { flex: 1; }
  .sm { padding: 5px 11px; font-size: 12.5px; }
  .onay { background: rgba(229, 72, 77, 0.18); color: #ff8a8d; border-color: transparent; }
</style>
