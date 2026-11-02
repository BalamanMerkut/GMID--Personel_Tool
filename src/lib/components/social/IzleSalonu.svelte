<script lang="ts">
  // Birlikte izle (Sosyal → Birlikte izle sekmesi): izleme odaları. Odada değilken açık izleme odaları ve
  // "İzleme odası aç"; odadayken büyük görüntü (herkes aynı saniyede), oynatma, sıra ve odadakiler.
  import { onDestroy, onMount } from "svelte";
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
    katil,
    medyaElemani,
    medyaYedekKutusu,
    odaAc,
    odaKapat,
    odaListesi,
    odaTuru,
    oynatDuraklat,
    sar,
    sesAyarla,
    siradanCikar,
    simdiCal,
    tekrarDegistir,
    yonetir,
    type Gizlilik,
    type Yetki,
  } from "../../birlikte.svelte";
  import { formatDuration } from "../../downloads.svelte";
  import { profilGoster, sosyal } from "../../social.svelte";
  import Avatar from "./Avatar.svelte";

  const oda = $derived.by(() => {
    const o = benimOdam();
    return o && odaTuru(o) === "video" ? o : null;
  });
  /** Bir müzik odasındayken buradan izleme odasına geçilirse eskisinden çıkılır (sunucu da çıkarır). */
  const muzikteyim = $derived(!!benimOdam() && !oda);
  const benId = $derived(sosyal.ben?.id ?? "");
  const sahibim = $derived(!!oda && oda.sahip.id === benId);
  const yonetici = $derived(!!oda && yonetir(oda, benId));
  const ekler = $derived(!!oda && ekleyebilir(oda, benId));
  const odalar = $derived(odaListesi("video"));

  // Salon ekrandayken yüzen oda paneli ve çubuk gizlenir (görüntü burada).
  onMount(() => {
    birlikte.salonAcik = true;
    birlikte.panel = false;
  });
  onDestroy(() => {
    birlikte.salonAcik = false;
    if (tam) void pencereTam(false);
  });

  function yerlestir(kutu: HTMLElement) {
    const m = medyaElemani();
    kutu.appendChild(m);
    return {
      destroy() {
        const caliyordu = !m.paused;
        if (!m.isConnected || m.parentElement === kutu) medyaYedekKutusu().appendChild(m);
        if (caliyordu) setTimeout(() => m.paused && void m.play().catch(() => {}), 60);
      },
    };
  }

  // ---- tam ekran ----
  let tam = $state(false);
  async function pencereTam(on: boolean) {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().setFullscreen(on);
    } catch {
      /* tarayıcıda */
    }
  }
  function tamEkran(on = !tam) {
    tam = on;
    void pencereTam(on);
  }

  // Tek tık: oynat/duraklat (sahip/DJ); çift tık: tam ekran (tek tık beklenir ki ikisi karışmasın).
  let tikZaman: ReturnType<typeof setTimeout> | undefined;
  function tikla(e: MouseEvent) {
    if (!yonetici || (e.target as HTMLElement).closest("button")) return;
    clearTimeout(tikZaman);
    tikZaman = setTimeout(oynatDuraklat, 230);
  }
  function ciftTikla() {
    clearTimeout(tikZaman);
    tamEkran();
  }

  // ---- ekle ----
  let giris = $state("");
  function gonder(e: SubmitEvent) {
    e.preventDefault();
    if (!giris.trim()) return;
    void ekle(giris, true).then(() => (giris = ""));
  }

  // ---- oda aç ----
  let acForm = $state(false);
  let fAd = $state("");
  let fSinir = $state(10);
  let fGizlilik = $state<Gizlilik>("herkes");
  let fYetki = $state<Yetki>("herkes");
  async function yeniOda() {
    await odaAc({ tur: "video", ad: fAd.trim() || undefined, sinir: fSinir, gizlilik: fGizlilik, yetki: fYetki });
    if (birlikte.odaId) {
      acForm = false;
      fAd = "";
    }
  }

  // ---- ayarlar (sahip) ----
  let ayarAcik = $state(false);
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
  let kapatOnay = $state(false);
</script>

<svelte:window
  onkeydown={(e) => {
    if (tam && e.key === "Escape") tamEkran(false);
    // Boşluk: oynat/duraklat (yazı yazarken değil).
    if (e.key === " " && oda && yonetici && !(e.target as HTMLElement).closest("input, textarea, select, button")) {
      e.preventDefault();
      oynatDuraklat();
    }
  }}
/>

<div class="salon" class:tam>
  {#if oda}
    <div class="sahne-kolon">
      <div class="sahne" class:tiklanir={yonetici && !!oda.simdiki} role="presentation" onclick={tikla} ondblclick={ciftTikla}>
        <div class="medya" use:yerlestir></div>
        {#if !oda.simdiki}
          <div class="bos-sahne">
            <Icon name="film" size={44} />
            <b>Sıra boş</b>
            <span class="sub">{ekler ? "Aşağıya bir YouTube bağlantısı ya da video adı yaz." : "Ekleme yetkisi olan birinin video eklemesini bekle."}</span>
          </div>
        {:else if birlikte.hazirlaniyor}
          <div class="bos-sahne yari">
            <span class="donen"></span>
            <b>Video hazırlanıyor…</b>
            <span class="sub">İlk açılışta birkaç saniye sürebilir.</span>
          </div>
        {/if}
        <button class="tam-dugme" title={tam ? "Tam ekrandan çık (Esc)" : "Tam ekran (çift tık)"} aria-label="Tam ekran" onclick={() => tamEkran()}>
          <Icon name={tam ? "shrink" : "expand"} size={18} />
        </button>
      </div>

      <div class="kontrol-kart">
        {#if oda.simdiki}
          <div class="simdi">
            <b title={oda.simdiki.sarki.baslik}>{oda.simdiki.sarki.baslik}</b>
            <span class="sub small">{oda.simdiki.ekleyen.ad} ekledi{oda.caliyor ? "" : " · duraklatıldı"}</span>
          </div>
          <div class="ilerle">
            <span class="tnum">{formatDuration(birlikte.konum) || "0:00"}</span>
            <input type="range" min="0" max={birlikte.sure || oda.simdiki.sarki.sure || 1} step="1" value={birlikte.konum} disabled={!yonetici} onchange={(e) => sar(+e.currentTarget.value)} aria-label="Konum" />
            <span class="tnum">{formatDuration(birlikte.sure || oda.simdiki.sarki.sure) || "–"}</span>
          </div>
        {/if}
        <div class="kontrol">
          {#if yonetici}
            <button class="oynat" title={oda.caliyor ? "Duraklat" : "Oynat"} disabled={!oda.simdiki} onclick={oynatDuraklat}><Icon name={oda.caliyor ? "pause" : "play"} size={20} /></button>
            <button class="ic" title="Sıradakine geç" aria-label="Sıradakine geç" disabled={!oda.simdiki} onclick={atla}><Icon name="next" size={19} /></button>
          {/if}
          <button class="ic" class:acik={oda.karisik} disabled={!yonetici} title={oda.karisik ? "Karışık (kapat)" : "Karıştır"} aria-label="Karıştır" onclick={karistirDegistir}><Icon name="shuffle" size={18} /></button>
          <button class="ic tekrar" class:acik={!!oda.tekrar && oda.tekrar !== "yok"} disabled={!yonetici} title={oda.tekrar === "sarki" ? "Videoyu tekrarla" : oda.tekrar === "liste" ? "Listeyi tekrarla" : "Tekrar kapalı"} aria-label="Tekrar" onclick={tekrarDegistir}>
            <Icon name="repeat" size={18} />{#if oda.tekrar === "sarki"}<span class="bir">1</span>{/if}
          </button>
          <span class="bosluk"></span>
          <Icon name="speaker" size={16} />
          <input class="ses" type="range" min="0" max="1" step="0.05" value={birlikte.ses} oninput={(e) => sesAyarla(+e.currentTarget.value)} aria-label="Ses" />
          <button class="ic" title="Tam ekran" aria-label="Tam ekran" onclick={() => tamEkran()}><Icon name="expand" size={17} /></button>
        </div>
        {#if ekler}
          <form class="ekle" onsubmit={gonder}>
            <Icon name="film" size={16} />
            <input bind:value={giris} placeholder="YouTube bağlantısı, video adı ya da çalma listesi" disabled={!!birlikte.asama} />
            <button class="btn sm" disabled={!!birlikte.asama || !giris.trim()}>{birlikte.asama || "Ekle"}</button>
          </form>
        {/if}
      </div>
    </div>

    <aside class="yan-kolon">
      <div class="oda-bas">
        <span class="oda-ad">
          <b title={oda.ad}>{oda.ad}</b>
          <span class="sub small">{oda.gizlilik === "arkadaslar" ? "Arkadaşlar · " : ""}{oda.uyeler.length}{oda.sinir ? `/${oda.sinir}` : ""} kişi izliyor</span>
        </span>
        {#if sahibim}<button class="ic" title="Oda ayarları" aria-label="Oda ayarları" onclick={ayarlariAc}><Icon name="gear" size={16} /></button>{/if}
      </div>

      {#if ayarAcik && sahibim}
        <div class="ayarlar">
          <label class="genis"><span>Oda adı</span><input bind:value={tAd} maxlength="40" /></label>
          <label><span>Kişi sınırı</span>
            <select bind:value={tSinir}><option value="0">Sınırsız</option>{#each [2, 3, 4, 5, 6, 8, 10, 15, 20] as n (n)}<option value={String(n)}>{n} kişi</option>{/each}</select>
          </label>
          <label><span>Kimler görür</span>
            <select bind:value={tGizlilik}><option value="herkes">Herkes</option><option value="arkadaslar">Arkadaşlarım</option></select>
          </label>
          <label class="genis"><span>Kim video ekler</span>
            <select bind:value={tYetki}><option value="herkes">Odadaki herkes</option><option value="dj">Ben ve DJ'ler</option><option value="sahip">Yalnız ben</option></select>
          </label>
          <div class="genis sag"><button class="btn sm" onclick={kaydet}>Kaydet</button></div>
        </div>
      {/if}

      <span class="ara-baslik">Sıradakiler {oda.sira.length ? `(${oda.sira.length})` : ""}</span>
      <div class="sira">
        {#each oda.sira as o, i (o.id)}
          <div class="oge">
            <span class="no tnum">{i + 1}</span>
            <button class="oge-yazi" disabled={!yonetici} title={yonetici ? "Şimdi oynat" : o.sarki.baslik} onclick={() => simdiCal(o.id)}>
              {#if o.sarki.kapak}<img src={o.sarki.kapak} alt="" loading="lazy" />{:else}<span class="kapak-yok"><Icon name="film" size={14} /></span>{/if}
              <span class="oge-metin"><b>{o.sarki.baslik}</b><span class="sub small">{o.ekleyen.ad}</span></span>
            </button>
            {#if yonetici || o.ekleyen.id === benId}
              <button class="ic kucuk" title="Sıradan çıkar" aria-label="Sıradan çıkar" onclick={() => siradanCikar(o.id)}><Icon name="x" size={13} /></button>
            {/if}
          </div>
        {:else}
          <p class="sub small">Sırada video yok.</p>
        {/each}
      </div>

      <span class="ara-baslik">İzleyenler</span>
      <div class="uyeler">
        {#each oda.uyeler as u (u.id)}
          <div class="uye">
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
        {#if sahibim}
          <button class="btn ghost sm" class:onay={kapatOnay} onclick={() => (kapatOnay ? odaKapat() : ((kapatOnay = true), setTimeout(() => (kapatOnay = false), 4000)))}>
            {kapatOnay ? "Oda kapansın mı?" : "Odayı kapat"}
          </button>
        {/if}
        <span class="bosluk"></span>
        <button class="btn ghost sm" onclick={ayril}>Ayrıl</button>
      </footer>
    </aside>
  {:else}
    <div class="lobi">
      <div class="lobi-bas">
        <div>
          <h2><Icon name="film" size={22} />Birlikte izle</h2>
          <p class="sub">Bir izleme odası aç, YouTube videolarını sıraya ekle; odadaki herkes aynı videoyu aynı saniyede izler. Sahip ve DJ'ler oynatır, duraklatır, ileri sarar.</p>
        </div>
        <button class="btn" onclick={() => (acForm = !acForm)}><Icon name={acForm ? "x" : "plus"} size={16} />{acForm ? "Vazgeç" : "İzleme odası aç"}</button>
      </div>
      {#if muzikteyim}<p class="sub small not">Şu an bir müzik odasındasın; izleme odası açınca ya da birine katılınca oradan çıkarsın.</p>{/if}

      {#if acForm}
        <form class="ac-form" onsubmit={(e) => (e.preventDefault(), void yeniOda())}>
          <label class="genis"><span>Oda adı</span><input bind:value={fAd} maxlength="40" placeholder="ör. Film Gecesi, Maç Özeti" /></label>
          <label><span>Kişi sınırı</span>
            <select bind:value={fSinir}>{#each [2, 3, 4, 5, 6, 8, 10, 15, 20] as n (n)}<option value={n}>{n} kişi</option>{/each}<option value={0}>Sınırsız</option></select>
          </label>
          <label><span>Kimler görür</span>
            <select bind:value={fGizlilik}><option value="herkes">Herkes</option><option value="arkadaslar">Arkadaşlarım</option></select>
          </label>
          <label><span>Kim video ekler</span>
            <select bind:value={fYetki}><option value="herkes">Odadaki herkes</option><option value="dj">Ben ve DJ'ler</option><option value="sahip">Yalnız ben</option></select>
          </label>
          <div class="sag"><button class="btn"><Icon name="film" size={16} />Odayı aç</button></div>
        </form>
      {/if}

      <div class="odalar">
        {#each odalar as o (o.id)}
          {@const dolu = o.sinir != null && o.uyeler.length >= o.sinir}
          <article class="oda-kart">
            <div class="kart-kapak">
              {#if o.simdiki?.sarki.kapak}<img src={o.simdiki.sarki.kapak} alt="" loading="lazy" />{:else}<Icon name="film" size={36} />{/if}
              {#if o.simdiki && o.caliyor}<span class="canli">● Oynatılıyor</span>{/if}
            </div>
            <div class="kart-yazi">
              <b>{#if o.gizlilik === "arkadaslar"}<Icon name="lock" size={12} /> {/if}{o.ad}</b>
              <span class="sub small" title={o.simdiki?.sarki.baslik}>{o.simdiki ? o.simdiki.sarki.baslik : "Sıra boş"}</span>
              <div class="kart-alt">
                <span class="yuzler">{#each o.uyeler.slice(0, 5) as u (u.id)}<span class="yuz"><Avatar kisi={u} boy={22} nokta={false} /></span>{/each}</span>
                <span class="sub small">{o.uyeler.length}{o.sinir ? `/${o.sinir}` : ""}</span>
                <span class="bosluk"></span>
                <button class="btn sm" disabled={dolu} onclick={() => katil(o.id)}>{dolu ? "Dolu" : "Katıl"}</button>
              </div>
            </div>
          </article>
        {:else}
          <div class="bos-liste">
            <Icon name="film" size={30} />
            <span>Şu an açık izleme odası yok. İlk odayı sen aç!</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .salon { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: 18px; align-items: start; padding: 16px; }
  .salon:has(.lobi) { grid-template-columns: minmax(0, 1fr); }
  .sahne-kolon { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
  .sahne { position: relative; aspect-ratio: 16 / 9; border-radius: 16px; overflow: hidden; background: #000; border: 1px solid var(--line); }
  .medya { position: absolute; inset: 0; }
  .sahne.tiklanir { cursor: pointer; }
  .medya :global(video) { width: 100%; height: 100%; object-fit: contain; display: block; }
  .bos-sahne { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; color: var(--muted); background: radial-gradient(circle at 50% 40%, rgba(227, 194, 112, 0.1), #000 70%); }
  .bos-sahne :global(svg) { color: var(--gold-dim); }
  .bos-sahne.yari { background: rgba(0, 0, 0, 0.6); }
  .donen { width: 34px; height: 34px; border-radius: 50%; border: 3px solid rgba(227, 194, 112, 0.25); border-top-color: var(--gold); animation: don 0.9s linear infinite; }
  @keyframes don { to { transform: rotate(360deg); } }
  .tam-dugme { position: absolute; right: 10px; bottom: 10px; width: 38px; height: 38px; border-radius: 10px; border: 0; background: rgba(0, 0, 0, 0.55); color: #fff; display: grid; place-items: center; cursor: pointer; opacity: 0; transition: opacity 0.15s; }
  .sahne:hover .tam-dugme { opacity: 1; }
  .kontrol-kart { display: flex; flex-direction: column; gap: 10px; padding: 14px 16px; border-radius: 16px; border: 1px solid var(--line); background: rgba(26, 28, 33, 0.85); }
  .simdi { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .simdi b { font-family: var(--f-display); font-size: 17px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ilerle { display: flex; align-items: center; gap: 10px; font-size: 12px; color: var(--dim); }
  .ilerle input { flex: 1; accent-color: var(--gold); }
  .kontrol { display: flex; align-items: center; gap: 8px; color: var(--dim); }
  .oynat { width: 44px; height: 44px; border-radius: 50%; border: 0; background: var(--gold); color: #1b1608; display: grid; place-items: center; cursor: pointer; }
  .oynat:disabled { opacity: 0.4; }
  .ic { display: grid; place-items: center; width: 34px; height: 34px; border-radius: 9px; border: 0; background: none; color: var(--dim); cursor: pointer; position: relative; }
  .ic:hover:not(:disabled) { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .ic:disabled { opacity: 0.35; cursor: default; }
  .ic.acik { color: var(--gold); }
  .ic.acik:disabled { opacity: 1; }
  .ic.kucuk { width: 24px; height: 24px; }
  .bir { position: absolute; right: 4px; bottom: 3px; font-size: 9px; font-weight: 900; color: var(--gold); }
  .bosluk { flex: 1; }
  .ses { width: 120px; accent-color: var(--gold); }
  .ekle { display: flex; align-items: center; gap: 8px; padding: 6px 6px 6px 12px; border-radius: 12px; border: 1px solid var(--line-2); background: var(--coal-2); color: var(--dim); }
  .ekle input { flex: 1; min-width: 0; border: 0; background: none; color: var(--text); font: inherit; font-size: 14px; outline: none; }
  .sm { padding: 6px 12px; font-size: 13px; display: inline-flex; align-items: center; gap: 5px; }

  .yan-kolon { display: flex; flex-direction: column; gap: 8px; padding: 14px; border-radius: 16px; border: 1px solid var(--line); background: rgba(26, 28, 33, 0.85); max-height: calc(100vh - 170px); overflow: hidden; }
  .oda-bas { display: flex; align-items: center; gap: 8px; }
  .oda-ad { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .oda-ad b { font-family: var(--f-display); font-size: 16px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ayarlar, .ac-form { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; padding: 10px; border-radius: 12px; background: rgba(0, 0, 0, 0.25); }
  .ac-form { grid-template-columns: repeat(3, minmax(0, 1fr)); padding: 14px; border: 1px solid var(--line); }
  .ayarlar label, .ac-form label { display: flex; flex-direction: column; gap: 4px; font-size: 11.5px; color: var(--dim); }
  .genis { grid-column: 1 / -1; }
  .sag { grid-column: 1 / -1; display: flex; justify-content: flex-end; }
  .ayarlar input, .ayarlar select, .ac-form input, .ac-form select { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 9px; padding: 7px 9px; color: var(--text); font: inherit; font-size: 13px; outline: none; color-scheme: dark; }
  .ara-baslik { font-size: 11px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; color: var(--gold-dim); margin-top: 6px; }
  .sira { display: flex; flex-direction: column; gap: 4px; overflow-y: auto; min-height: 60px; flex: 1; }
  .oge { display: flex; align-items: center; gap: 6px; }
  .no { width: 16px; text-align: right; color: var(--dim); font-size: 12px; flex: none; }
  .oge-yazi { flex: 1; min-width: 0; display: flex; align-items: center; gap: 8px; padding: 4px; border: 0; border-radius: 9px; background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; }
  .oge-yazi:hover:not(:disabled) { background: rgba(227, 194, 112, 0.08); }
  .oge-yazi:disabled { cursor: default; }
  .oge-yazi img, .kapak-yok { width: 64px; height: 36px; border-radius: 6px; object-fit: cover; flex: none; background: #000; display: grid; place-items: center; color: var(--dim); }
  .oge-metin { min-width: 0; display: flex; flex-direction: column; line-height: 1.25; }
  .oge-metin b { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .uyeler { display: flex; flex-direction: column; gap: 2px; max-height: 200px; overflow-y: auto; }
  .uye { display: flex; align-items: center; gap: 6px; padding: 3px 0; }
  .kisi { flex: 1; min-width: 0; display: flex; align-items: center; gap: 8px; border: 0; background: none; color: inherit; font: inherit; font-size: 13px; cursor: pointer; padding: 0; text-align: left; }
  .rozet { font-size: 10.5px; font-weight: 800; padding: 2px 7px; border-radius: 999px; }
  .rozet.sahip { background: rgba(227, 194, 112, 0.18); color: var(--gold-glow); }
  .rozet.dj { background: rgba(108, 182, 255, 0.16); color: #8cc6ff; }
  .mini { border: 1px solid var(--line-2); background: none; color: var(--muted); border-radius: 7px; padding: 2px 7px; font: inherit; font-size: 11.5px; cursor: pointer; white-space: nowrap; }
  .mini.kirmizi:hover { color: #ff8a8d; border-color: rgba(229, 72, 77, 0.5); }
  footer { display: flex; align-items: center; gap: 6px; padding-top: 8px; border-top: 1px solid var(--line); }
  .onay { background: rgba(229, 72, 77, 0.18); color: #ff8a8d; border-color: transparent; }

  .lobi { display: flex; flex-direction: column; gap: 16px; }
  .lobi-bas { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; }
  .lobi-bas h2 { margin: 0 0 4px; display: flex; align-items: center; gap: 10px; font-family: var(--f-display); font-size: 22px; }
  .lobi-bas h2 :global(svg) { color: var(--gold); }
  .lobi-bas p { margin: 0; max-width: 620px; }
  .not { margin: 0; color: var(--gold-glow); }
  .odalar { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 14px; }
  .oda-kart { display: flex; flex-direction: column; border-radius: 16px; overflow: hidden; border: 1px solid var(--line); background: rgba(26, 28, 33, 0.85); }
  .kart-kapak { position: relative; aspect-ratio: 16 / 9; background: radial-gradient(circle at 50% 40%, rgba(227, 194, 112, 0.12), #0a0b0d 70%); display: grid; place-items: center; color: var(--gold-dim); }
  .kart-kapak img { width: 100%; height: 100%; object-fit: cover; }
  .canli { position: absolute; left: 8px; top: 8px; font-size: 11px; font-weight: 800; color: #ff6b6b; background: rgba(0, 0, 0, 0.65); padding: 2px 8px; border-radius: 999px; }
  .kart-yazi { display: flex; flex-direction: column; gap: 4px; padding: 10px 12px 12px; min-width: 0; }
  .kart-yazi > b { font-size: 15px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .kart-yazi > .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .kart-alt { display: flex; align-items: center; gap: 6px; margin-top: 4px; }
  .yuzler { display: inline-flex; }
  .yuz { margin-right: -6px; border-radius: 50%; box-shadow: 0 0 0 2px #1a1c21; }
  .bos-liste { grid-column: 1 / -1; display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 40px; border-radius: 16px; border: 1px dashed var(--line-2); color: var(--muted); }

  /* Tam ekran: yalnız görüntü ve oynatma kartı. */
  .salon.tam { position: fixed; inset: 0; z-index: 300; padding: 0; gap: 0; background: #000; display: block; }
  .tam .sahne-kolon { height: 100vh; gap: 0; }
  .tam .sahne { flex: 1; aspect-ratio: auto; border-radius: 0; border: 0; }
  .tam .kontrol-kart { border-radius: 0; border: 0; background: #0b0c0e; }
  .tam .ekle, .tam .yan-kolon { display: none; }
  .tam .tam-dugme { opacity: 1; }

  @media (max-width: 1100px) {
    .salon { grid-template-columns: minmax(0, 1fr); }
    .yan-kolon { max-height: none; }
  }
</style>
