<script lang="ts">
  // Sosyal: üstte malikâneye not bırakma kutusu (yazı, resim, hazır başlangıçlar), ortada tek sütun akış (gönderiler,
  // puanlar, oyuna girişler, klipler; beğeni ve yanıt), sağda şu an oyunda olanlar, gündem (son 7 gün, #etiketler,
  // beğeniler) ve tanıyor olabileceklerin. Bir #etikete ya da gündeme tıklayınca akış o konuya süzülür.
  import { onMount } from "svelte";
  import Icon from "../Icon.svelte";
  import logo from "../../assets/gmid-logo.webp";
  import Avatar from "../components/social/Avatar.svelte";
  import FeedCard from "../components/social/FeedCard.svelte";
  import GeceKart from "../components/social/GeceKart.svelte";
  import GecePlanla from "../components/social/GecePlanla.svelte";
  import TurnuvaKart from "../components/social/TurnuvaKart.svelte";
  import { birlikte, katil, odaAc, odaListesi, type Gizlilik, type Yetki } from "../birlikte.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { turnuvaAc } from "../social.svelte";
  import { arkadasEkle, gonderiPaylas, istek, konuGoster, profilGoster, sosyal, uyelerYukle, type Kisi, type Olay } from "../social.svelte";
  import { nav } from "../player.svelte";
  import { notify } from "../toast.svelte";

  const SINIR = 500;
  const IPUCLARI = ["Malikâneye bir not bırak…", "Bu akşam ne oynuyoruz?", "Son izlediğin film nasıldı?", "Bugün hangi oyunda takıldın?"];
  /** Tek tıkla başlangıç: boş kutuya hazır bir cümle koyar. */
  const HAZIR: { ad: string; ikon: "play" | "film" | "trophy" | "chat"; metin: string }[] = [
    { ad: "Oyun daveti", ikon: "play", metin: "Bu akşam 21:00'de oyun var, kim geliyor? #OyunGecesi" },
    { ad: "Öneri", ikon: "film", metin: "Önerim: " },
    { ad: "Başardım", ikon: "trophy", metin: "Sonunda başardım: " },
    { ad: "Soru", ikon: "chat", metin: "Bir sorum var: " },
  ];
  const ipucu = IPUCLARI[Math.floor(Math.random() * IPUCLARI.length)];
  let alan = $state<HTMLTextAreaElement | null>(null);

  interface GundemKonu {
    ad: string;
    etiket: boolean;
    paylasim: number;
    begeni: number;
  }
  /** Sunucunun gündemi (son 7 gün); eski sunucuda null → sayfadaki olaylardan hesaplanır. */
  let sunucuGundemi = $state<GundemKonu[] | null>(null);

  let olaylar = $state<Olay[]>([]);
  let yukleniyor = $state(false);
  let son = $state(false);
  let yalnizArkadas = $state(getJson<boolean>("social.feedFriends", false));

  // Yazma kutusu
  let taslak = $state("");
  let resim = $state<File | null>(null);
  let onizleme = $state<string | null>(null);
  let gonderiliyor = $state(false);
  let planAcik = $state(false);
  /** Açık form: oyun gecesi mi, oyuncu arıyorum mu. */
  let planHemen = $state(false);
  // Turnuva açma formu.
  let turnuvaFormu = $state(false);
  let tAd = $state("");
  let tOyun = $state("");
  let tBoyut = $state(8);
  let tGonderiliyor = $state(false);
  const turnuvaListesi = $derived(Object.values(sosyal.turnuvalar).filter((t) => t.durum !== "bitti").slice(0, 3));
  async function yeniTurnuva() {
    if (!tAd.trim() || !tOyun.trim() || tGonderiliyor) return;
    tGonderiliyor = true;
    try {
      await turnuvaAc({ ad: tAd.trim(), oyun: tOyun.trim(), boyut: tBoyut });
      notify("Turnuva açıldı · arkadaşlarına haber gitti", "star");
      turnuvaFormu = false;
      tAd = "";
      tOyun = "";
    } catch (e) {
      hata(e);
    } finally {
      tGonderiliyor = false;
    }
  }

  // Dinleme odası açma formu.
  let odaFormu = $state(false);
  let odaAdi = $state("");
  let odaSinir = $state(10);
  let odaGizlilik = $state<Gizlilik>("herkes");
  let odaYetki = $state<Yetki>("herkes");
  async function yeniOda() {
    await odaAc({ ad: odaAdi.trim() || undefined, sinir: odaSinir, gizlilik: odaGizlilik, yetki: odaYetki });
    if (birlikte.odaId) {
      odaFormu = false;
      odaAdi = "";
    }
  }
  const yaklasan = $derived(Object.values(sosyal.geceler).sort((a, b) => a.zaman - b.zaman).slice(0, 4));
  let dosyaSec = $state<HTMLInputElement | null>(null);
  const kalanHarf = $derived(SINIR - [...taslak].length);
  /** Yazının sonunda "@ali" varsa: adı böyle başlayan üyeler (tıklayınca tamamlanır). */
  const anmaSoz = $derived(/(?:^|\s)@([\p{L}\p{N}_]{0,20})$/u.exec(taslak)?.[1] ?? null);
  const anmaOneri = $derived(
    anmaSoz === null
      ? []
      : sosyal.uyeler
          .filter((k) => k.id !== sosyal.ben?.id && k.ad.toLocaleLowerCase("tr").startsWith(anmaSoz.toLocaleLowerCase("tr")))
          .slice(0, 5),
  );
  function anmaKoy(k: Kisi) {
    if (anmaSoz === null) return;
    taslak = `${taslak.slice(0, taslak.length - anmaSoz.length - 1)}@${k.ad} `;
    queueMicrotask(() => alan?.focus());
  }
  // Anket: soru gönderinin yazısı, 2–4 seçenek, süre.
  let anketAcik = $state(false);
  let secenekler = $state<string[]>(["", ""]);
  let anketSaat = $state(24);
  const doluSecenek = $derived(secenekler.map((x) => x.trim()).filter(Boolean));
  const anketHazir = $derived(!anketAcik || (taslak.trim().length > 0 && doluSecenek.length >= 2));
  const gonderilebilir = $derived(!gonderiliyor && kalanHarf >= 0 && (taslak.trim().length > 0 || !!resim) && anketHazir);

  const hata = (e: unknown) => notify(String(e instanceof Error ? e.message : e), "info");

  async function yukle(devam = false) {
    if (!sosyal.anahtar) return;
    yukleniyor = true;
    try {
      const q = new URLSearchParams();
      if (devam && olaylar.length) q.set("once", String(olaylar[olaylar.length - 1].zaman));
      if (yalnizArkadas) q.set("arkadaslar", "1");
      if (sosyal.konu) q.set("konu", sosyal.konu);
      const { olaylar: yeni } = await istek<{ olaylar: Olay[] }>("GET", `/api/akis?${q}`);
      olaylar = devam ? [...olaylar, ...yeni] : yeni;
      son = yeni.length < 30;
    } catch (e) {
      hata(e);
    } finally {
      yukleniyor = false;
    }
  }

  async function gundemYukle() {
    if (!sosyal.anahtar) return;
    try {
      const r = await istek<{ gundem: GundemKonu[] }>("GET", `/api/gundem${yalnizArkadas ? "?arkadaslar=1" : ""}`);
      sunucuGundemi = r.gundem;
    } catch {
      sunucuGundemi = null;
    }
  }

  function hazirKoy(h: (typeof HAZIR)[number]) {
    taslak = taslak.trim() ? `${taslak.trimEnd()} ${h.metin}` : h.metin;
    queueMicrotask(() => {
      alan?.focus();
      alan?.setSelectionRange(taslak.length, taslak.length);
    });
  }

  function resimSecildi(e: Event) {
    const f = (e.currentTarget as HTMLInputElement).files?.[0] ?? null;
    (e.currentTarget as HTMLInputElement).value = "";
    if (!f) return;
    if (!f.type.startsWith("image/")) return notify("Yalnız resim eklenebilir", "info");
    resimKaldir();
    resim = f;
    onizleme = URL.createObjectURL(f);
  }

  function resimKaldir() {
    if (onizleme) URL.revokeObjectURL(onizleme);
    onizleme = null;
    resim = null;
  }

  async function paylas() {
    if (!gonderilebilir) return;
    gonderiliyor = true;
    try {
      await gonderiPaylas(taslak.trim(), resim, anketAcik ? { secenekler: doluSecenek, saat: anketSaat } : null);
      taslak = "";
      resimKaldir();
      anketAcik = false;
      secenekler = ["", ""];
      if (yalnizArkadas) void yukle();
    } catch (e) {
      hata(e);
    } finally {
      gonderiliyor = false;
    }
  }

  function tus(e: KeyboardEvent) {
    // Ctrl+Enter ile gönder (Twitter'daki gibi).
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      void paylas();
    }
  }

  /** Kart sunucudan sildi; listeden de çıkar. */
  function sil(o: Olay) {
    olaylar = olaylar.filter((x) => x.id !== o.id);
  }

  function suzgec(v: boolean) {
    if (yalnizArkadas === v) return;
    yalnizArkadas = v;
    void setJson("social.feedFriends", v);
    void yukle();
    void gundemYukle();
  }

  // Sağ sütun
  const oyunda = $derived(
    Object.entries(sosyal.cevrimici)
      .filter(([, k]) => k.oyun)
      .map(([id, k]) => ({ ...k, id }))
      .slice(0, 6),
  );
  /** Gündem: sunucudan (son 7 gün, #etiketler, beğeniler); yoksa yüklü sayfadaki olaylardan. */
  const gundem = $derived.by((): GundemKonu[] => {
    if (sunucuGundemi) return sunucuGundemi;
    const say = new Map<string, number>();
    for (const o of olaylar) {
      const ad = o.tur === "klip" ? o.veri.oyun : o.tur === "gonderi" ? null : o.veri.ad;
      if (ad) say.set(ad, (say.get(ad) ?? 0) + 1);
    }
    return [...say.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 5)
      .map(([ad, n]) => ({ ad, etiket: false, paylasim: n, begeni: 0 }));
  });
  const oneriler = $derived(
    sosyal.uyeler
      .filter((k) => k.id !== sosyal.ben?.id)
      .filter((k) => ![...sosyal.arkadaslar, ...sosyal.gelen, ...sosyal.giden].some((x) => x.id === k.id))
      .slice(0, 3),
  );

  async function ekle(k: Kisi) {
    try {
      await arkadasEkle(k);
      notify(`${k.ad}: arkadaşlık isteği gönderildi`, "check");
    } catch (e) {
      hata(e);
    }
  }

  // ---------- en üste dön / yeni gönderiler ----------
  let tunel = $state<HTMLElement | null>(null);
  let yeniVar = $state(false);
  /** Akışta aşağı inilmiş mi (üst kısım ekranın dışında)? */
  const asagida = () => !!tunel && tunel.getBoundingClientRect().top < -120;
  function enUste() {
    tunel?.scrollIntoView({ behavior: "smooth", block: "start" });
  }
  function yenileriGoster() {
    yeniVar = false;
    void yukle();
    enUste();
  }
  // Başkasından yeni paylaşım: yukarıdaysan hemen görünür; aşağıdaysan okuduğun yer kaymaz, düğme çıkar.
  let sonYeni = sosyal.yeniGonderi;
  $effect(() => {
    if (sosyal.yeniGonderi !== sonYeni) {
      sonYeni = sosyal.yeniGonderi;
      void gundemYukle();
      if (asagida()) yeniVar = true;
      else void yukle();
    }
  });

  // Akış değişince (biri paylaştı, beğendi, yanıtladı) ilk sayfa sessizce yenilenir.
  let surum = sosyal.akisSurum;
  $effect(() => {
    if (sosyal.akisSurum !== surum) {
      surum = sosyal.akisSurum;
      void yukle();
      void gundemYukle();
    }
  });
  // Konu (#etiket / gündem) değişince akış ona göre yeniden yüklenir.
  let sonKonu = sosyal.konu;
  $effect(() => {
    if (sosyal.konu !== sonKonu) {
      sonKonu = sosyal.konu;
      void yukle();
    }
  });

  onMount(() => {
    sosyal.bildirim = 0;
    void yukle();
    void gundemYukle();
    void uyelerYukle();
    return () => resimKaldir();
  });
</script>

{#if !sosyal.anahtar}
  <section class="panel bos yalniz">
    <Icon name="sparkle" size={30} />
    <b>Topluluğun gönderileri, puanları ve klipleri burada</b>
    <span class="sub">Görmek ve paylaşmak için Topluluk'tan Discord ile giriş yap.</span>
    <button class="btn" onclick={() => nav.go("social")}><Icon name="users" size={16} />Topluluk'a git</button>
  </section>
{:else}
  <div class="duzen">
    <main class="zaman-tuneli" bind:this={tunel}>
      <div class="sekmeler" role="tablist">
        <button class="logo-ust" title="En üste dön" aria-label="En üste dön" onclick={enUste}><img src={logo} alt="" /></button>
        <button role="tab" aria-selected={!yalnizArkadas} onclick={() => suzgec(false)}><span>Herkes</span></button>
        <button role="tab" aria-selected={yalnizArkadas} onclick={() => suzgec(true)}><span>Arkadaşlarım</span></button>
        <button class="yenile" title="Yenile" aria-label="Yenile" disabled={yukleniyor} onclick={() => yukle()}><Icon name="refresh" size={16} /></button>
      </div>
      {#if yeniVar}
        <div class="yeni-yer"><button class="yeni-dugme" onclick={yenileriGoster}><Icon name="arrow" size={14} />Yeni gönderiler</button></div>
      {/if}

      <!-- Malikâneye not: GMID'in kendi kartı (altın çerçeve), hazır başlangıçlar, resim. -->
      <form class="yaz" onsubmit={(e) => (e.preventDefault(), paylas())}>
        <div class="yaz-ust">
          {#if sosyal.ben}<Avatar kisi={sosyal.ben} boy={34} nokta={false} />{/if}
          <span class="yaz-baslik"><b>{sosyal.ben?.ad ?? "Sen"}</b><span>malikâneye yazıyor</span></span>
        </div>
        <div class="alan-yer">
          <textarea bind:this={alan} bind:value={taslak} onkeydown={tus} placeholder={ipucu} title="#etiket yazarsan gündeme girer, @ad yazarsan o kişiye haber gider" rows="2" aria-label="Gönderi" disabled={gonderiliyor}></textarea>
          <!-- Kalan harf yalnız sınıra yaklaşınca, kutunun köşesinde (düğmeler kaymaz). -->
          {#if kalanHarf <= 100}
            <span class="sayac tnum" class:az={kalanHarf <= 40} class:asti={kalanHarf < 0}>{kalanHarf}</span>
          {/if}
        </div>
        {#if anmaOneri.length}
          <div class="anma-oneri" role="listbox" aria-label="Üye öner">
            {#each anmaOneri as k (k.id)}
              <button type="button" class="anma-k" onclick={() => anmaKoy(k)}><Avatar kisi={k} boy={22} nokta={false} />{k.ad}</button>
            {/each}
          </div>
        {/if}
        {#if onizleme}
          <div class="onizleme">
            <img src={onizleme} alt="Eklenen resim" />
            <button type="button" class="kaldir" title="Resmi kaldır" aria-label="Resmi kaldır" onclick={resimKaldir}><Icon name="x" size={15} /></button>
          </div>
        {/if}
        {#if anketAcik}
          <div class="anket-yaz">
            <span class="anket-ipucu">Soruyu yukarıya yaz, seçenekleri buraya:</span>
            {#each secenekler as _, i (i)}
              <div class="secenek">
                <input bind:value={secenekler[i]} maxlength="60" placeholder={`${i + 1}. seçenek${i < 2 ? "" : " (isteğe bağlı)"}`} aria-label={`${i + 1}. seçenek`} />
                {#if secenekler.length > 2}
                  <button type="button" class="kaldir-s" title="Seçeneği kaldır" aria-label="Seçeneği kaldır" onclick={() => (secenekler = secenekler.filter((_, j) => j !== i))}><Icon name="x" size={13} /></button>
                {/if}
              </div>
            {/each}
            <div class="anket-alt">
              {#if secenekler.length < 4}<button type="button" class="btn ghost sm" onclick={() => (secenekler = [...secenekler, ""])}><Icon name="plus" size={14} />Seçenek ekle</button>{/if}
              <span class="bosluk"></span>
              <label class="anket-sure">Süre
                <select bind:value={anketSaat}><option value={1}>1 saat</option><option value={6}>6 saat</option><option value={24}>1 gün</option><option value={72}>3 gün</option><option value={168}>1 hafta</option></select>
              </label>
            </div>
          </div>
        {/if}
        <div class="hazir">
          {#each HAZIR as h (h.ad)}
            <button type="button" class="cip" onclick={() => hazirKoy(h)} disabled={gonderiliyor}><Icon name={h.ikon} size={14} />{h.ad}</button>
          {/each}
        </div>
        <div class="yaz-alt">
          <button type="button" class="btn ghost sm" onclick={() => dosyaSec?.click()} disabled={gonderiliyor}><Icon name="image" size={15} />Resim</button>
          <button type="button" class="btn ghost sm" class:acik-plan={anketAcik} onclick={() => (anketAcik = !anketAcik)} disabled={gonderiliyor}><Icon name="checklist" size={15} />Anket</button>
          <button type="button" class="btn ghost sm" class:acik-plan={planAcik && planHemen} onclick={() => ((planAcik = !(planAcik && planHemen)), (planHemen = true))}><Icon name="users" size={15} />Oyuncu ara</button>
          <button type="button" class="btn ghost sm" class:acik-plan={planAcik && !planHemen} onclick={() => ((planAcik = !(planAcik && !planHemen)), (planHemen = false))}><Icon name="bell" size={15} />Oyun gecesi</button>
          <input bind:this={dosyaSec} type="file" accept="image/*" hidden onchange={resimSecildi} />
          <span class="bosluk"></span>
          <button class="btn paylas" disabled={!gonderilebilir} title="Ctrl+Enter"><Icon name="send" size={15} />{gonderiliyor ? "Paylaşılıyor…" : "Paylaş"}</button>
        </div>
      </form>

      {#if planAcik}
        <div class="plan-yer">{#key planHemen}<GecePlanla hemen={planHemen} onkapat={() => (planAcik = false)} />{/key}</div>
      {/if}

      {#if sosyal.konu}
        <div class="konu-serit">
          <span><b>{sosyal.konu}</b> ile ilgili paylaşımlar</span>
          <button class="btn ghost sm" onclick={() => konuGoster(null)}><Icon name="x" size={14} />Tüm akış</button>
        </div>
      {/if}

      {#if yukleniyor && !olaylar.length}
        {#each Array(3) as _, i (i)}<div class="iskelet"><span class="sk a"></span><div class="sk-sag"><span class="sk s"></span><span class="sk k"></span></div></div>{/each}
      {:else}
        {#each olaylar as o (o.id)}
          <FeedCard {o} ondegisti={() => yukle()} onsil={sil} />
        {:else}
          <div class="bos">
            <Icon name="sparkle" size={28} />
            <b>Akış şimdilik sessiz</b>
            <span class="sub">{yalnizArkadas ? "Arkadaşların bir şey paylaşınca burada görünür." : "İlk gönderiyi sen yaz: bu akşam kim ne oynuyor?"}</span>
          </div>
        {/each}
        {#if olaylar.length && !son}
          <button class="devam" disabled={yukleniyor} onclick={() => yukle(true)}>{yukleniyor ? "Yükleniyor…" : "Daha fazla göster"}</button>
        {/if}
      {/if}
    </main>

    <aside class="yan">
      <section class="kutu">
        <h3>Şu an oyunda</h3>
        {#each oyunda as k (k.id)}
          <button class="satir" onclick={() => profilGoster(k.id)}>
            <Avatar kisi={k} boy={36} />
            <span class="satir-yazi"><b>{k.ad}</b><span class="mor">{k.oyun}</span></span>
          </button>
        {:else}
          <p class="sub small">Şu an kimse oyunda değil.</p>
        {/each}
      </section>

      <section class="kutu">
        <div class="kutu-bas">
          <h3>Dinleme odaları</h3>
          {#if !birlikte.odaId}
            <button class="btn sm takip" onclick={() => (odaFormu = !odaFormu)}><Icon name={odaFormu ? "x" : "plus"} size={14} />{odaFormu ? "Vazgeç" : "Oda aç"}</button>
          {/if}
        </div>
        {#if odaFormu && !birlikte.odaId}
          <form class="oda-ac" onsubmit={(e) => (e.preventDefault(), void yeniOda())}>
            <input bind:value={odaAdi} maxlength="40" placeholder="Oda adı (ör. Akşam Rap)" aria-label="Oda adı" />
            <label class="oda-satir">Kişi sınırı
              <select bind:value={odaSinir}>{#each [2, 3, 4, 5, 6, 8, 10, 15, 20] as n (n)}<option value={n}>{n} kişi</option>{/each}<option value={0}>Sınırsız</option></select>
            </label>
            <label class="oda-satir">Kim görebilir
              <select bind:value={odaGizlilik}><option value="herkes">Herkes</option><option value="arkadaslar">Arkadaşlarım</option></select>
            </label>
            <label class="oda-satir">Kim şarkı ekler
              <select bind:value={odaYetki}><option value="herkes">Odadaki herkes</option><option value="dj">Ben ve DJ'ler</option><option value="sahip">Yalnız ben</option></select>
            </label>
            <button class="btn sm oda-dugme"><Icon name="headphones" size={14} />Odayı aç</button>
          </form>
        {:else}
          <span class="sub small oda-not">İndirmek yok: odadaki herkes aynı şarkıyı ya da videoyu aynı anda dinler.</span>
        {/if}
        {#each odaListesi() as o (o.id)}
          {@const icinde = birlikte.odaId === o.id}
          {@const dolu = o.sinir != null && o.uyeler.length >= o.sinir}
          <div class="satir">
            <button class="satir-sol" onclick={() => (icinde ? (birlikte.panel = true) : profilGoster(o.sahip.id))}>
              <Avatar kisi={o.sahip} boy={36} />
              <span class="satir-yazi">
                <b>{#if o.gizlilik === "arkadaslar"}<Icon name="lock" size={12} /> {/if}{o.ad}</b>
                <span class="sub small">{o.simdiki ? `${o.caliyor ? "♪" : "⏸"} ${o.simdiki.sarki.baslik}` : "Sıra boş"}</span>
                <span class="sub small">{o.sahip.id === sosyal.ben?.id ? "Senin odan" : o.sahip.ad} · {o.uyeler.length}{o.sinir ? `/${o.sinir}` : ""} kişi</span>
              </span>
            </button>
            {#if icinde}
              <button class="btn ghost sm takip" onclick={() => (birlikte.panel = true)}>Odaya git</button>
            {:else if dolu}
              <span class="sub small">Dolu</span>
            {:else}
              <button class="btn sm takip" onclick={() => katil(o.id)}><Icon name="headphones" size={14} />Katıl</button>
            {/if}
          </div>
        {:else}
          {#if !odaFormu}<p class="sub small oda-not">Şu an açık oda yok. İlk odayı sen aç!</p>{/if}
        {/each}
      </section>

      <section class="kutu">
        <div class="kutu-bas">
          <h3>Turnuvalar</h3>
          <button class="btn sm takip" onclick={() => (turnuvaFormu = !turnuvaFormu)}><Icon name={turnuvaFormu ? "x" : "plus"} size={14} />{turnuvaFormu ? "Vazgeç" : "Turnuva aç"}</button>
        </div>
        {#if turnuvaFormu}
          <form class="oda-ac" onsubmit={(e) => (e.preventDefault(), void yeniTurnuva())}>
            <input bind:value={tAd} maxlength="60" placeholder="Turnuva adı (ör. Cuma 1v1 Kupası)" aria-label="Turnuva adı" />
            <input bind:value={tOyun} maxlength="60" placeholder="Oyun (ör. CS2, Valorant)" aria-label="Oyun" />
            <label class="oda-satir">Kişi sayısı
              <select bind:value={tBoyut}>{#each [4, 8, 16, 32] as n (n)}<option value={n}>{n} kişi</option>{/each}</select>
            </label>
            <button class="btn sm oda-dugme" disabled={!tAd.trim() || !tOyun.trim() || tGonderiliyor}><Icon name="trophy" size={14} />Turnuvayı aç</button>
          </form>
        {:else if !turnuvaListesi.length}
          <p class="sub small oda-not">Açık turnuva yok. Bir turnuva aç; akışa kart düşer, herkes katılır, kazanan şampiyon rozeti alır.</p>
        {/if}
        <div class="geceler">
          {#each turnuvaListesi as t (t.id)}<TurnuvaKart id={t.id} kucuk />{/each}
        </div>
      </section>

      {#if yaklasan.length}
        <section class="kutu">
          <h3>Yaklaşan oyun geceleri</h3>
          <div class="geceler">
            {#each yaklasan as g (g.id)}<GeceKart id={g.id} kucuk />{/each}
          </div>
        </section>
      {/if}

      {#if gundem.length}
        <section class="kutu">
          <h3>Gündem</h3>
          {#each gundem as g, i (g.ad)}
            <button class="gundem" class:secili={sosyal.konu === g.ad} onclick={() => konuGoster(sosyal.konu === g.ad ? null : g.ad)}>
              <span class="sub small">{i + 1} · {g.etiket ? "Etiket" : "Toplulukta"}</span>
              <b class:etiketli={g.etiket}>{g.ad}</b>
              <span class="sub small">{g.paylasim} paylaşım{g.begeni ? ` · ${g.begeni} beğeni` : ""}</span>
            </button>
          {/each}
          <span class="sub small dipnot">Son 7 gün · beğeniler de sayılır</span>
        </section>
      {/if}

      {#if oneriler.length}
        <section class="kutu">
          <h3>Tanıyor olabileceklerin</h3>
          {#each oneriler as k (k.id)}
            <div class="satir">
              <button class="satir-sol" onclick={() => profilGoster(k.id)}>
                <Avatar kisi={k} boy={36} />
                <span class="satir-yazi"><b>{k.ad}</b><span class="sub small">GMID üyesi</span></span>
              </button>
              <button class="btn sm takip" onclick={() => ekle(k)}>Ekle</button>
            </div>
          {/each}
        </section>
      {/if}
    </aside>
  </div>
{/if}

<style>
  .duzen { display: grid; grid-template-columns: minmax(0, 620px) 300px; gap: 28px; justify-content: center; align-items: start; }
  .zaman-tuneli { border-left: 1px solid var(--line); border-right: 1px solid var(--line); min-height: 100%; background: rgba(0, 0, 0, 0.12); }
  /* Aşağı inince sekmeler üst çubuğa yapışır: ana alanın üst boşluğu (24px; dar ekranda 20px) kadar yukarı
     çekilir ki arada içerik görünen bir aralık kalmasın. */
  .sekmeler { position: sticky; top: -24px; z-index: 2; display: flex; border-bottom: 1px solid var(--line); background: rgba(20, 22, 26, 0.97); backdrop-filter: blur(12px); }
  @media (max-width: 1100px) { .sekmeler { top: -20px; } }
  .sekmeler [role="tab"] { flex: 1; border: 0; background: none; color: var(--muted); font: inherit; font-weight: 600; font-size: 15px; padding: 0; height: 52px; cursor: pointer; display: grid; place-items: center; }
  .sekmeler [role="tab"]:hover { background: rgba(255, 255, 255, 0.04); }
  .sekmeler [role="tab"] span { position: relative; height: 100%; display: grid; place-items: center; }
  .sekmeler [role="tab"][aria-selected="true"] { color: var(--text); font-weight: 800; }
  .sekmeler [role="tab"][aria-selected="true"] span::after { content: ""; position: absolute; left: 0; right: 0; bottom: 0; height: 4px; border-radius: 999px; background: var(--gold); }
  .logo-ust { flex: none; width: 56px; border: 0; background: none; display: grid; place-items: center; cursor: pointer; }
  .logo-ust img { width: 32px; height: 32px; border-radius: 50%; box-shadow: 0 0 12px rgba(227, 194, 112, 0.25); transition: transform 0.15s; }
  .logo-ust:hover img { transform: scale(1.08); }
  .yeni-yer { position: sticky; top: 40px; z-index: 3; height: 0; display: flex; justify-content: center; overflow: visible; }
  .yeni-dugme { display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px; border-radius: 999px; border: 0; background: var(--gold); color: #1b1608; font: inherit; font-weight: 800; font-size: 13.5px; cursor: pointer; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45); animation: yeni-gel 0.25s ease-out; }
  .yeni-dugme :global(svg) { transform: rotate(-90deg); }
  @keyframes yeni-gel { from { transform: translateY(-8px); opacity: 0; } }
  .yenile { flex: none; width: 52px; border: 0; background: none; color: var(--dim); display: grid; place-items: center; cursor: pointer; }
  .yenile:hover { color: var(--text); background: rgba(255, 255, 255, 0.04); }
  .yaz { margin: 14px 14px 10px; padding: 14px; display: flex; flex-direction: column; gap: 10px; border-radius: 16px; border: 1px solid rgba(227, 194, 112, 0.28); background: linear-gradient(160deg, rgba(227, 194, 112, 0.07), rgba(255, 255, 255, 0.02) 55%); }
  .yaz-ust { display: flex; align-items: center; gap: 10px; }
  .yaz-baslik { display: flex; flex-direction: column; line-height: 1.25; }
  .yaz-baslik b { font-size: 14px; }
  .yaz-baslik span { font-size: 11.5px; letter-spacing: 0.08em; text-transform: uppercase; color: var(--gold-dim); font-weight: 700; }
  textarea { width: 100%; box-sizing: border-box; border: 1px solid var(--line-2); border-radius: 12px; background: var(--coal-2); color: var(--text); font: inherit; font-size: 15px; line-height: 1.5; resize: none; outline: none; padding: 10px 12px; field-sizing: content; min-height: 64px; max-height: 300px; }
  textarea:focus { border-color: var(--gold-dim); }
  textarea::placeholder { color: var(--dim); }
  .onizleme { position: relative; align-self: flex-start; }
  .anket-yaz { display: flex; flex-direction: column; gap: 6px; padding: 10px; border-radius: 12px; border: 1px dashed rgba(227, 194, 112, 0.35); background: rgba(0, 0, 0, 0.18); }
  .anket-ipucu { font-size: 12px; color: var(--dim); }
  .secenek { display: flex; gap: 6px; align-items: center; }
  .secenek input { flex: 1; min-width: 0; background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 9px; padding: 7px 10px; color: var(--text); font: inherit; font-size: 13.5px; outline: none; }
  .secenek input:focus { border-color: var(--gold-dim); }
  .kaldir-s { border: 0; background: none; color: var(--dim); width: 26px; height: 26px; border-radius: 7px; display: grid; place-items: center; cursor: pointer; }
  .kaldir-s:hover { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .anket-alt { display: flex; align-items: center; gap: 8px; }
  .anket-sure { display: inline-flex; align-items: center; gap: 6px; font-size: 12.5px; color: var(--muted); }
  .anket-sure select { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 8px; padding: 4px 8px; color: var(--text); font: inherit; font-size: 12.5px; color-scheme: dark; }
  .onizleme img { display: block; max-width: 100%; max-height: 260px; border-radius: 12px; border: 1px solid var(--line); }
  .kaldir { position: absolute; top: 8px; left: 8px; width: 30px; height: 30px; border-radius: 50%; border: 0; background: rgba(0, 0, 0, 0.7); color: #fff; display: grid; place-items: center; cursor: pointer; }
  .anma-oneri { display: flex; flex-wrap: wrap; gap: 6px; }
  .anma-k { display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px 4px 4px; border-radius: 999px; border: 1px solid var(--line-2); background: var(--coal-2); color: var(--text); font: inherit; font-size: 13px; cursor: pointer; }
  .anma-k:hover { border-color: #6cb6ff; }
  .hazir { display: flex; flex-wrap: wrap; gap: 6px; }
  .cip { display: inline-flex; align-items: center; gap: 6px; padding: 5px 11px; border-radius: 8px; border: 1px solid var(--line-2); background: rgba(255, 255, 255, 0.03); color: var(--muted); font: inherit; font-size: 12.5px; cursor: pointer; }
  .cip:hover:not(:disabled) { border-color: var(--gold-dim); color: var(--gold-glow); }
  .cip :global(svg) { color: var(--gold); }
  .yaz-alt { display: flex; align-items: center; gap: 10px; }
  .bosluk { flex: 1; }
  .alan-yer { position: relative; }
  .sayac { position: absolute; right: 10px; bottom: 8px; font-size: 12px; color: var(--dim); pointer-events: none; padding: 1px 6px; border-radius: 999px; background: rgba(20, 22, 26, 0.85); }
  .sayac.az { color: var(--gold-glow); }
  .sayac.asti { color: #ff6b6b; font-weight: 700; }
  .paylas { display: inline-flex; align-items: center; gap: 7px; }
  .sm { padding: 6px 11px; font-size: 13px; display: inline-flex; align-items: center; gap: 6px; white-space: nowrap; }
  .plan-yer { margin: 0 14px 10px; }
  .acik-plan { border-color: var(--gold-dim); color: var(--gold-glow); }
  .oda-ac { display: flex; flex-direction: column; gap: 8px; padding: 0 12px; }
  .oda-dugme { justify-content: center; }
  .oda-ac input { flex: 1; min-width: 0; background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 10px; padding: 7px 10px; color: var(--text); font: inherit; font-size: 13px; outline: none; }
  .oda-ac input:focus { border-color: var(--gold-dim); }
  .oda-not { padding: 6px 16px 10px; display: block; }
  .kutu-bas { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding-right: 12px; }
  .oda-satir { display: flex; align-items: center; justify-content: space-between; gap: 8px; font-size: 12.5px; color: var(--muted); }
  .oda-satir select { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 8px; padding: 5px 8px; color: var(--text); font: inherit; font-size: 12.5px; }
  .geceler { display: flex; flex-direction: column; gap: 8px; padding: 0 12px 12px; }
  .konu-serit { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin: 0 14px 10px; padding: 8px 8px 8px 14px; border-radius: 12px; background: rgba(227, 194, 112, 0.1); border: 1px solid rgba(227, 194, 112, 0.3); font-size: 14px; }
  .konu-serit b { color: var(--gold-glow); }
  .bos { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; padding: 40px 24px; color: var(--muted); }
  .bos.yalniz { max-width: 620px; margin: 0 auto; }
  .devam { width: 100%; border: 0; border-bottom: 1px solid var(--line); background: none; color: #6cb6ff; font: inherit; font-size: 14px; padding: 16px; cursor: pointer; }
  .devam:hover { background: rgba(255, 255, 255, 0.03); }
  .iskelet { display: grid; grid-template-columns: 44px 1fr; gap: 12px; padding: 14px 18px; border-bottom: 1px solid var(--line); }
  .sk-sag { display: flex; flex-direction: column; gap: 10px; }
  .sk { display: block; background: linear-gradient(90deg, rgba(255, 255, 255, 0.05) 25%, rgba(255, 255, 255, 0.1) 50%, rgba(255, 255, 255, 0.05) 75%); background-size: 200% 100%; animation: parilti 1.4s infinite linear; }
  .sk.a { width: 44px; height: 44px; border-radius: 50%; }
  .sk.s { width: 45%; height: 12px; border-radius: 6px; }
  .sk.k { width: 100%; height: 84px; border-radius: 16px; }
  @keyframes parilti { to { background-position: -200% 0; } }
  .yan { position: sticky; top: 0; display: flex; flex-direction: column; gap: 16px; }
  .kutu { display: flex; flex-direction: column; border-radius: 16px; border: 1px solid var(--line); background: rgba(255, 255, 255, 0.025); overflow: hidden; }
  .kutu h3 { margin: 0; padding: 12px 16px 8px; font-family: var(--f-display); font-size: 17px; font-weight: 800; }
  .kutu > .sub { margin: 0; padding: 4px 16px 14px; }
  .satir { display: flex; align-items: center; gap: 10px; padding: 10px 16px; border: 0; background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; width: 100%; }
  .satir:hover, .gundem:hover { background: rgba(255, 255, 255, 0.04); }
  .satir-sol { flex: 1; min-width: 0; display: flex; align-items: center; gap: 10px; border: 0; background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; padding: 0; }
  .satir-yazi { display: flex; flex-direction: column; min-width: 0; line-height: 1.3; }
  .satir-yazi b, .satir-yazi span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mor { color: #a99cff; font-size: 13px; }
  .gundem { display: flex; flex-direction: column; align-items: flex-start; padding: 10px 16px; gap: 1px; border: 0; background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; width: 100%; }
  .gundem.secili { background: rgba(227, 194, 112, 0.1); }
  .gundem b { font-size: 15px; }
  .gundem b.etiketli { color: var(--gold); }
  .dipnot { padding: 8px 16px 12px; }
  .takip { border-radius: 999px; padding: 6px 16px; font-weight: 800; }
  @media (max-width: 1180px) {
    .duzen { grid-template-columns: minmax(0, 640px); }
    .yan { display: none; }
  }
</style>
