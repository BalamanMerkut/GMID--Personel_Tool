<script lang="ts">
  // Topluluk: son olaylar (puanladı / oynuyor), beğeni ve yorum; kişiler ve profilleri; özel mesajlar;
  // neyin paylaşılacağı. Sunucu: GMID sunucusu (Discord ile giriş).
  import { onMount, tick } from "svelte";
  import Icon from "../Icon.svelte";
  import Switch from "../components/Switch.svelte";
  import { notify } from "../toast.svelte";
  import {
    cikis, esitleSimdi, girisYap, istek, mesajlariDinle, paylasimDegistir, sosyal, tamAdres,
    type Kisi, type Konusma, type Mesaj, type Olay, type Profil, type Puan,
  } from "../social.svelte";

  type Sekme = "akis" | "kisiler" | "mesajlar" | "ben";
  let sekme = $state<Sekme>("akis");

  // ---------- yardımcılar ----------
  const SAAT = new Intl.DateTimeFormat("tr-TR", { hour: "2-digit", minute: "2-digit" });
  const GUN = new Intl.DateTimeFormat("tr-TR", { day: "numeric", month: "short" });
  function once(ms: number): string {
    const s = Math.max(0, (Date.now() - ms) / 1000);
    if (s < 60) return "az önce";
    if (s < 3600) return `${Math.floor(s / 60)} dk`;
    if (s < 86400) return `${Math.floor(s / 3600)} sa`;
    if (s < 7 * 86400) return `${Math.floor(s / 86400)} gün`;
    return GUN.format(ms);
  }
  const bas = (ad: string) => ad.split(/\s+/).map((p) => p[0] ?? "").join("").slice(0, 2).toUpperCase();
  const turAd: Record<string, string> = { game: "oyun", movie: "film", series: "dizi" };
  /** "bir diziyi puanladı", "bir oyunu bitirdi"… */
  function eylem(o: Olay): string {
    if (o.tur === "oyun") return "oynamaya başladı";
    const v = o.veri;
    const nesne: Record<string, [string, string]> = { game: ["bir oyunu", "bir oyun"], movie: ["bir filmi", "bir film"], series: ["bir diziyi", "bir dizi"] };
    const [belirli, yalin] = nesne[v.tur ?? "game"] ?? nesne.game;
    switch (v.durum) {
      case "playing": return `${yalin} oynuyor`;
      case "watching": return `${yalin} izliyor`;
      case "finished": case "completed": return `${belirli} bitirdi`;
      case "watched": return `${belirli} izledi`;
      case "dropped": return `${belirli} bıraktı`;
      case "wishlist": case "watchlist": return `${belirli} listesine ekledi`;
      default: return `${belirli} puanladı`;
    }
  }
  const puanYaz = (p: number | null | undefined) => (p == null ? null : p.toLocaleString("tr-TR", { maximumFractionDigits: 1 }));
  const durumOf = (k: Kisi) => sosyal.cevrimici[k.id];
  /** Açılamayan kapaklar (baş harflerle gösterilir). */
  let kirik = $state<Record<string, boolean>>({});

  // ---------- akış ----------
  let olaylar = $state<Olay[]>([]);
  let akisYukleniyor = $state(false);
  let akisSonu = $state(false);
  let akisKisi = $state<string | null>(null);
  let yorumTaslak = $state<Record<number, string>>({});
  let acikYorum = $state<Record<number, boolean>>({});

  async function akisYukle(devam = false) {
    if (!sosyal.anahtar) return;
    akisYukleniyor = true;
    try {
      const q = new URLSearchParams();
      if (devam && olaylar.length) q.set("once", String(olaylar[olaylar.length - 1].zaman));
      if (akisKisi) q.set("kisi", akisKisi);
      const { olaylar: yeni } = await istek<{ olaylar: Olay[] }>("GET", `/api/akis?${q}`);
      olaylar = devam ? [...olaylar, ...yeni] : yeni;
      akisSonu = yeni.length < 30;
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
    } finally {
      akisYukleniyor = false;
    }
  }

  async function begen(o: Olay) {
    // Hemen göster, sunucudan geleni yaz.
    o.begendim = !o.begendim;
    o.begeni += o.begendim ? 1 : -1;
    try {
      const r = await istek<{ begendim: boolean; begeni: number }>("POST", `/api/akis/${o.id}/begen`);
      o.begendim = r.begendim;
      o.begeni = r.begeni;
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
    }
  }

  async function yorumYap(o: Olay) {
    const metin = (yorumTaslak[o.id] ?? "").trim();
    if (!metin) return;
    yorumTaslak[o.id] = "";
    try {
      await istek("POST", `/api/akis/${o.id}/yorum`, { metin });
      acikYorum[o.id] = true;
      await akisYukle();
    } catch (e) {
      yorumTaslak[o.id] = metin;
      notify(String(e instanceof Error ? e.message : e), "info");
    }
  }

  async function yorumSil(id: number) {
    await istek("DELETE", `/api/yorum/${id}`).catch((e) => notify(String(e.message ?? e), "info"));
    await akisYukle();
  }

  async function olaySil(o: Olay) {
    await istek("DELETE", `/api/akis/${o.id}`).catch((e) => notify(String(e.message ?? e), "info"));
    olaylar = olaylar.filter((x) => x.id !== o.id);
  }

  // Akış değişince (biri puanladı, beğendi, yorum yaptı) ilk sayfa sessizce yenilenir.
  let sonSurum = 0;
  $effect(() => {
    const s = sosyal.akisSurum;
    if (s !== sonSurum && (sekme === "akis" || profilId)) {
      sonSurum = s;
      void akisYukle();
    }
  });

  // ---------- kişiler ve profil ----------
  let kisiler = $state<Kisi[]>([]);
  let profilId = $state<string | null>(null);
  let profil = $state<{ kisi: Kisi; cevrimici: boolean; oyun: string | null; profil: Profil | null } | null>(null);
  let profilSekme = $state<"puanlar" | "oyunlar" | "paylasimlar">("puanlar");
  let puanTur = $state<"hepsi" | "game" | "movie" | "series">("hepsi");

  async function kisilerYukle() {
    try {
      kisiler = (await istek<{ kisiler: Kisi[] }>("GET", "/api/kisiler")).kisiler;
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
    }
  }
  const kisiListesi = $derived(
    kisiler
      .map((k) => ({ ...k, ...(sosyal.cevrimici[k.id] ? { cevrimici: true, oyun: sosyal.cevrimici[k.id].oyun } : { cevrimici: false }) }))
      .sort((a, b) => Number(!!b.cevrimici) - Number(!!a.cevrimici) || Number(!!b.oyun) - Number(!!a.oyun) || a.ad.localeCompare(b.ad, "tr")),
  );

  async function profilAc(id: string) {
    profilId = id;
    profil = null;
    profilSekme = "puanlar";
    puanTur = "hepsi";
    akisKisi = id;
    try {
      profil = await istek("GET", `/api/profil/${id}`);
      await akisYukle();
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
      profilId = null;
    }
  }
  function profilKapat() {
    profilId = null;
    profil = null;
    akisKisi = null;
    void akisYukle();
  }
  const profilPuanlari = $derived(
    (profil?.profil?.puanlar ?? [])
      .filter((p) => puanTur === "hepsi" || p.tur === puanTur)
      .slice()
      .sort((a, b) => b.guncelleme - a.guncelleme),
  );
  const profilOyunlari = $derived((profil?.profil?.oyunlar ?? []).slice().sort((a, b) => b.dakika - a.dakika));
  const saat = (dk: number) => (dk >= 60 ? `${Math.round(dk / 60).toLocaleString("tr-TR")} sa` : `${dk} dk`);

  // ---------- mesajlar ----------
  let konusmalar = $state<Konusma[]>([]);
  let konusulan = $state<Kisi | null>(null);
  let mesajlar = $state<Mesaj[]>([]);
  let taslak = $state("");
  let akis: HTMLDivElement | undefined = $state();

  async function konusmalarYukle() {
    try {
      konusmalar = (await istek<{ konusmalar: Konusma[] }>("GET", "/api/konusmalar")).konusmalar;
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
    }
  }

  async function sona() {
    await tick();
    akis?.scrollTo({ top: akis.scrollHeight });
  }

  async function konusmaAc(k: Kisi) {
    sekme = "mesajlar";
    profilId = null;
    konusulan = k;
    sosyal.acikKonusma = k.id;
    mesajlar = [];
    try {
      mesajlar = (await istek<{ mesajlar: Mesaj[] }>("GET", `/api/mesaj/${k.id}`)).mesajlar;
      const { [k.id]: _, ...kalan } = sosyal.okunmamis;
      sosyal.okunmamis = kalan;
      await sona();
      void konusmalarYukle();
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
    }
  }

  async function gonder() {
    const metin = taslak.trim();
    if (!metin || !konusulan) return;
    taslak = "";
    try {
      await istek("POST", `/api/mesaj/${konusulan.id}`, { metin });
    } catch (e) {
      taslak = metin;
      notify(String(e instanceof Error ? e.message : e), "info");
    }
  }

  // ---------- giriş / paylaşım ----------
  async function giris() {
    try {
      await girisYap();
      await Promise.all([akisYukle(), kisilerYukle()]);
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
    }
  }
  let esitleniyor = $state(false);
  async function simdiEsitle() {
    esitleniyor = true;
    try {
      await esitleSimdi();
      notify("Profilin güncellendi", "check");
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
    } finally {
      esitleniyor = false;
    }
  }

  function sekmeyeGec(s: Sekme) {
    sekme = s;
    profilId = null;
    profil = null;
    if (akisKisi) {
      akisKisi = null;
      if (s === "akis") void akisYukle();
    }
    if (s === "kisiler") void kisilerYukle();
    if (s === "mesajlar") void konusmalarYukle();
    if (s === "akis") sosyal.bildirim = 0;
    if (s !== "mesajlar") sosyal.acikKonusma = null;
  }

  onMount(() => {
    sosyal.bildirim = 0;
    if (sosyal.anahtar) void Promise.all([akisYukle(), kisilerYukle()]);
    const birak = mesajlariDinle((m) => {
      const diger = m.kimden === sosyal.ben?.id ? m.kime : m.kimden;
      if (konusulan && diger === konusulan.id && !mesajlar.some((x) => x.id === m.id)) {
        mesajlar = [...mesajlar, m];
        void sona();
        // Açıkken gelen mesaj okundu sayılsın.
        if (m.kimden === konusulan.id) void istek("GET", `/api/mesaj/${konusulan.id}?once=0`).catch(() => {});
      }
      if (sekme === "mesajlar") void konusmalarYukle();
    });
    return () => {
      birak();
      sosyal.acikKonusma = null;
    };
  });
</script>

{#snippet avatar(k: Kisi, boy = 40)}
  <span class="av" style:width="{boy}px" style:height="{boy}px">
    {#if k.avatar}<img src={k.avatar} alt="" loading="lazy" />{:else}<span class="avb" style:font-size="{boy * 0.36}px">{bas(k.ad)}</span>{/if}
    {#if durumOf(k)}<i class="nokta" class:oyunda={!!durumOf(k)?.oyun} title={durumOf(k)?.oyun ? `${durumOf(k)?.oyun} oynuyor` : "Çevrim içi"}></i>{/if}
  </span>
{/snippet}

{#snippet kapak(url: string | null | undefined, ad: string, cls = "kp")}
  {@const tam = tamAdres(url)}
  {#if tam && !kirik[tam]}<img class={cls} src={tam} alt="" loading="lazy" onerror={() => (kirik[tam] = true)} />{:else}<span class="{cls} yok">{bas(ad)}</span>{/if}
{/snippet}

{#snippet olayKart(o: Olay)}
  <article class="olay">
    <header>
      <button class="kim" onclick={() => profilAc(o.kisi.id)}>{@render avatar(o.kisi, 38)}</button>
      <div class="baslik">
        <button class="ad" onclick={() => profilAc(o.kisi.id)}>{o.kisi.ad}</button>
        <span class="sub">{eylem(o)}</span>
        <span class="zaman" title={new Date(o.zaman).toLocaleString("tr-TR")}>{once(o.zaman)}</span>
      </div>
      {#if o.kisi.id === sosyal.ben?.id}
        <button class="ic" title="Paylaşımı kaldır" aria-label="Paylaşımı kaldır" onclick={() => olaySil(o)}><Icon name="trash" size={15} /></button>
      {/if}
    </header>
    <div class="konu">
      {@render kapak(o.veri.kapak, o.veri.ad)}
      <div class="konu-yazi">
        <b>{o.veri.ad}</b>
        <span class="sub small">{o.tur === "oyun" ? "oyun" : turAd[o.veri.tur ?? "game"]}{o.veri.yil ? ` · ${o.veri.yil}` : ""}</span>
        {#if o.veri.yorum}<q>{o.veri.yorum}</q>{/if}
      </div>
      {#if puanYaz(o.veri.puan)}<span class="puan tnum">{puanYaz(o.veri.puan)}</span>{/if}
    </div>
    <footer>
      <button class="eylem" class:begendi={o.begendim} onclick={() => begen(o)} aria-pressed={o.begendim}>
        <Icon name="heart" size={16} /><span class="tnum">{o.begeni || ""}</span>
      </button>
      <button class="eylem" onclick={() => (acikYorum[o.id] = !acikYorum[o.id])}>
        <Icon name="chat" size={16} /><span class="tnum">{o.yorumlar.length || ""}</span>
      </button>
    </footer>
    {#if acikYorum[o.id] || o.yorumlar.length}
      <div class="yorumlar">
        {#each acikYorum[o.id] ? o.yorumlar : o.yorumlar.slice(-2) as y (y.id)}
          <div class="yorum">
            <button class="kim" onclick={() => profilAc(y.kisi.id)}>{@render avatar(y.kisi, 26)}</button>
            <div class="yb"><b>{y.kisi.ad}</b> {y.metin} <span class="zaman">{once(y.zaman)}</span></div>
            {#if y.kisi.id === sosyal.ben?.id || o.kisi.id === sosyal.ben?.id}
              <button class="ic kucuk" title="Yorumu sil" aria-label="Yorumu sil" onclick={() => yorumSil(y.id)}><Icon name="x" size={13} /></button>
            {/if}
          </div>
        {/each}
        {#if !acikYorum[o.id] && o.yorumlar.length > 2}
          <button class="dahasi" onclick={() => (acikYorum[o.id] = true)}>Tüm yorumlar ({o.yorumlar.length})</button>
        {/if}
        <form class="yaz" onsubmit={(e) => (e.preventDefault(), yorumYap(o))}>
          <input bind:value={yorumTaslak[o.id]} placeholder="Yorum yaz…" maxlength="1000" aria-label="Yorum" />
          <button class="ic" disabled={!(yorumTaslak[o.id] ?? "").trim()} aria-label="Gönder"><Icon name="send" size={16} /></button>
        </form>
      </div>
    {/if}
  </article>
{/snippet}

{#snippet puanKart(p: Puan)}
  <div class="pk">
    {@render kapak(p.kapak, p.ad, "pk-kapak")}
    {#if puanYaz(p.puan)}<span class="pk-puan tnum">{puanYaz(p.puan)}</span>{/if}
    <b title={p.ad}>{p.ad}</b>
    <span class="sub small">{turAd[p.tur]}{p.yil ? ` · ${p.yil}` : ""}</span>
    {#if p.yorum}<q title={p.yorum}>{p.yorum}</q>{/if}
  </div>
{/snippet}

<div class="sosyal">
  {#if !sosyal.anahtar}
    <section class="panel karsilama">
      <div class="k-ikon"><Icon name="users" size={34} /></div>
      <h2>GMID Topluluğu</h2>
      <p class="sub">Arkadaşlarının ne oynadığını, hangi diziyi kaç puanladığını gör; beğen, yorum yap, özel mesaj at.</p>
      <ul class="k-liste">
        <li><Icon name="check" size={15} />Discord hesabınla girersin, şifre yok. Yalnız GMID Discord üyeleri girebilir.</li>
        <li><Icon name="check" size={15} />Neyin paylaşılacağını sen seçersin (oyunların, puanların, şu an oynadığın).</li>
        <li><Icon name="check" size={15} />Notların, ayarların, dosyaların hiçbir zaman paylaşılmaz.</li>
      </ul>
      <button class="btn discord" disabled={sosyal.giriliyor} onclick={giris}>
        <Icon name="discord" size={18} />{sosyal.giriliyor ? "Tarayıcıda onay bekleniyor…" : "Discord ile giriş yap"}
      </button>
    </section>
  {:else}
    <nav class="sekmeler" aria-label="Topluluk">
      <button aria-current={sekme === "akis" && !profilId ? "page" : undefined} onclick={() => sekmeyeGec("akis")}><Icon name="sparkle" size={16} />Akış</button>
      <button aria-current={sekme === "kisiler" && !profilId ? "page" : undefined} onclick={() => sekmeyeGec("kisiler")}>
        <Icon name="users" size={16} />Kişiler<span class="sayi tnum">{Object.keys(sosyal.cevrimici).length}</span>
      </button>
      <button aria-current={sekme === "mesajlar" && !profilId ? "page" : undefined} onclick={() => sekmeyeGec("mesajlar")}>
        <Icon name="chat" size={16} />Mesajlar
        {#if Object.values(sosyal.okunmamis).reduce((a, b) => a + b, 0)}<span class="sayi yeni tnum">{Object.values(sosyal.okunmamis).reduce((a, b) => a + b, 0)}</span>{/if}
      </button>
      <button aria-current={sekme === "ben" && !profilId ? "page" : undefined} onclick={() => sekmeyeGec("ben")}><Icon name="user" size={16} />Paylaşım</button>
      <span class="baglanti" class:acik={sosyal.bagli} title={sosyal.bagli ? "Sunucuya bağlı" : "Bağlanıyor…"}><i></i>{sosyal.bagli ? "Bağlı" : "Bağlanıyor"}</span>
    </nav>

    {#if profilId}
      <section class="panel profil">
        <button class="geri" onclick={profilKapat}><Icon name="back" size={15} />Geri</button>
        {#if !profil}
          <div class="iskelet"><span class="sk-av"></span><span class="sk-satir"></span></div>
        {:else}
          <div class="p-ust">
            {@render avatar(profil.kisi, 76)}
            <div class="p-ad">
              <h2>{profil.kisi.ad}</h2>
              <span class="sub">
                {#if durumOf(profil.kisi)?.oyun}<span class="oyunda-yazi">● {durumOf(profil.kisi)?.oyun} oynuyor</span>
                {:else if durumOf(profil.kisi)}<span class="acik-yazi">● Çevrim içi</span>{:else}Çevrim dışı{/if}
              </span>
              <span class="sub small">{profil.profil?.puanlar.length ?? 0} puan · {profil.profil?.oyunlar.length ?? 0} oyun</span>
            </div>
            {#if profil.kisi.id !== sosyal.ben?.id}
              <button class="btn" onclick={() => profil && konusmaAc(profil.kisi)}><Icon name="chat" size={16} />Mesaj gönder</button>
            {/if}
          </div>
          <div class="seg">
            <button aria-pressed={profilSekme === "puanlar"} onclick={() => (profilSekme = "puanlar")}>Puanları</button>
            <button aria-pressed={profilSekme === "oyunlar"} onclick={() => (profilSekme = "oyunlar")}>Oyunları</button>
            <button aria-pressed={profilSekme === "paylasimlar"} onclick={() => (profilSekme = "paylasimlar")}>Paylaşımları</button>
          </div>
          {#if profilSekme === "puanlar"}
            <div class="seg kucuk">
              {#each [["hepsi", "Hepsi"], ["game", "Oyunlar"], ["movie", "Filmler"], ["series", "Diziler"]] as [v, l] (v)}
                <button aria-pressed={puanTur === v} onclick={() => (puanTur = v as typeof puanTur)}>{l}</button>
              {/each}
            </div>
            {#if profilPuanlari.length}
              <div class="pk-izgara">{#each profilPuanlari as p (p.anahtar)}{@render puanKart(p)}{/each}</div>
            {:else}
              <p class="sub bos">{profil.profil ? "Burada henüz puan yok." : "Henüz bir şey paylaşmamış."}</p>
            {/if}
          {:else if profilSekme === "oyunlar"}
            {#if profilOyunlari.length}
              <div class="pk-izgara">
                {#each profilOyunlari as g (g.ad)}
                  <div class="pk">
                    {@render kapak(g.kapak, g.ad, "pk-kapak")}
                    <b title={g.ad}>{g.ad}</b>
                    <span class="sub small tnum">{g.dakika ? saat(g.dakika) : "henüz oynamadı"}{g.son ? ` · ${once(g.son)}` : ""}</span>
                  </div>
                {/each}
              </div>
            {:else}
              <p class="sub bos">Oyun listesini paylaşmıyor.</p>
            {/if}
          {:else}
            <div class="olaylar">
              {#each olaylar as o (o.id)}{@render olayKart(o)}{:else}<p class="sub bos">Henüz paylaşım yok.</p>{/each}
            </div>
          {/if}
        {/if}
      </section>
    {:else if sekme === "akis"}
      <div class="olaylar">
        {#if akisYukleniyor && !olaylar.length}
          {#each Array(3) as _, i (i)}<div class="olay iskelet"><span class="sk-av"></span><span class="sk-satir"></span><span class="sk-kutu"></span></div>{/each}
        {:else}
          {#each olaylar as o (o.id)}{@render olayKart(o)}{:else}
            <section class="panel bos-kart">
              <Icon name="sparkle" size={28} />
              <b>Akış şimdilik sessiz</b>
              <span class="sub">Biri bir oyunu, filmi ya da diziyi puanlayınca ya da bir oyuna girince burada görünür.</span>
            </section>
          {/each}
          {#if olaylar.length && !akisSonu}
            <button class="btn ghost devam" disabled={akisYukleniyor} onclick={() => akisYukle(true)}>{akisYukleniyor ? "Yükleniyor…" : "Daha eski"}</button>
          {/if}
        {/if}
      </div>
    {:else if sekme === "kisiler"}
      <div class="kisiler">
        {#each kisiListesi as k (k.id)}
          <div class="kisi" class:cevrimdisi={!k.cevrimici}>
            <button class="kisi-ana" onclick={() => profilAc(k.id)}>
              {@render avatar(k, 48)}
              <span class="kisi-yazi">
                <b>{k.ad}{k.id === sosyal.ben?.id ? " (sen)" : ""}</b>
                <span class="sub small">{k.oyun ? `${k.oyun} oynuyor` : k.cevrimici ? "Çevrim içi" : k.son ? `Son görülme ${once(k.son)}` : "Çevrim dışı"}</span>
              </span>
            </button>
            {#if k.id !== sosyal.ben?.id}
              <button class="ic" title="Mesaj gönder" aria-label="{k.ad} kişisine mesaj gönder" onclick={() => konusmaAc(k)}><Icon name="chat" size={17} /></button>
            {/if}
          </div>
        {:else}
          <p class="sub bos">Henüz kimse yok.</p>
        {/each}
      </div>
    {:else if sekme === "mesajlar"}
      <section class="panel dm">
        <aside class="dm-liste">
          {#each konusmalar as k (k.kisi.id)}
            <button class="dm-kisi" class:secili={konusulan?.id === k.kisi.id} onclick={() => konusmaAc(k.kisi)}>
              {@render avatar(k.kisi, 38)}
              <span class="dm-yazi">
                <b>{k.kisi.ad}</b>
                <span class="sub small">{k.son.kimden === sosyal.ben?.id ? "Sen: " : ""}{k.son.metin}</span>
              </span>
              {#if sosyal.okunmamis[k.kisi.id]}<span class="sayi yeni tnum">{sosyal.okunmamis[k.kisi.id]}</span>{/if}
            </button>
          {:else}
            <p class="sub small bos">Henüz mesaj yok. Kişiler'den birine yaz.</p>
          {/each}
        </aside>
        <div class="dm-ana">
          {#if konusulan}
            <header class="dm-ust">
              <button class="kim" onclick={() => konusulan && profilAc(konusulan.id)}>{@render avatar(konusulan, 34)}</button>
              <b>{konusulan.ad}</b>
              <span class="sub small">{durumOf(konusulan)?.oyun ? `${durumOf(konusulan)?.oyun} oynuyor` : durumOf(konusulan) ? "Çevrim içi" : ""}</span>
            </header>
            <div class="dm-akis" bind:this={akis}>
              {#each mesajlar as m, i (m.id)}
                {@const benim = m.kimden === sosyal.ben?.id}
                {#if i === 0 || m.zaman - mesajlar[i - 1].zaman > 30 * 60_000}<div class="dm-zaman">{GUN.format(m.zaman)} {SAAT.format(m.zaman)}</div>{/if}
                <div class="balon" class:benim title={SAAT.format(m.zaman)}>{m.metin}</div>
              {:else}
                <p class="sub small bos">İlk mesajı sen yaz.</p>
              {/each}
            </div>
            <form class="dm-yaz" onsubmit={(e) => (e.preventDefault(), gonder())}>
              <input bind:value={taslak} placeholder="{konusulan.ad} kişisine yaz…" maxlength="1000" aria-label="Mesaj" />
              <button class="btn" disabled={!taslak.trim()}><Icon name="send" size={16} />Gönder</button>
            </form>
          {:else}
            <div class="dm-bos"><Icon name="chat" size={30} /><span class="sub">Soldan bir konuşma seç.</span></div>
          {/if}
        </div>
      </section>
    {:else}
      <section class="panel">
        <div class="ben-ust">
          {#if sosyal.ben}{@render avatar(sosyal.ben, 56)}{/if}
          <div>
            <h2>{sosyal.ben?.ad}</h2>
            <span class="sub small">Discord ile giriş yapıldı</span>
          </div>
          <div class="sag">
            {#if sosyal.ben}<button class="btn ghost sm" onclick={() => sosyal.ben && profilAc(sosyal.ben.id)}>Profilimi gör</button>{/if}
            <button class="btn ghost sm" onclick={() => cikis()}>Çıkış yap</button>
          </div>
        </div>
        <h3 class="h2">Ne paylaşılsın?</h3>
        <p class="sub small">Kapalı olanlar sunucuya hiç gönderilmez; kapatınca profilinden de kalkar.</p>
        <div class="line">
          <div><b>Oyun kütüphanem</b><span class="sub small">Oyunların ve ne kadar oynadığın (Kütüphanem'de gizlediklerin hariç)</span></div>
          <Switch label="Oyun kütüphanem" checked={sosyal.paylas.oyunlar} onchange={(v) => paylasimDegistir({ oyunlar: v })} />
        </div>
        <div class="line">
          <div><b>Oyun puanlarım</b><span class="sub small">Puan, tek cümlelik yorum ve durum (oynuyor, bitirdi…); yeni puanlar akışa düşer</span></div>
          <Switch label="Oyun puanlarım" checked={sosyal.paylas.oyunPuan} onchange={(v) => paylasimDegistir({ oyunPuan: v })} />
        </div>
        <div class="line">
          <div><b>Film ve dizi puanlarım</b><span class="sub small">Film & Dizi Arşivi'ndeki puanların ve yorumların</span></div>
          <Switch label="Film ve dizi puanlarım" checked={sosyal.paylas.filmPuan} onchange={(v) => paylasimDegistir({ filmPuan: v })} />
        </div>
        <div class="line">
          <div><b>Şu an oynadığım oyun</b><span class="sub small">Kişiler'de ve profilinde "CS2 oynuyor"; oyuna girince akışa düşer</span></div>
          <Switch label="Şu an oynadığım oyun" checked={sosyal.paylas.simdi} onchange={(v) => paylasimDegistir({ simdi: v })} />
        </div>
        <div class="row">
          <button class="btn ghost" disabled={esitleniyor} onclick={simdiEsitle}><Icon name="refresh" size={16} />{esitleniyor ? "Güncelleniyor…" : "Profilimi şimdi güncelle"}</button>
          <span class="sub small">Puan verdiğinde ve oyun listesi değiştiğinde kendiliğinden güncellenir.</span>
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .sosyal { display: flex; flex-direction: column; gap: 14px; max-width: 920px; margin: 0 auto; }
  .sosyal > .panel { display: flex; flex-direction: column; gap: 12px; }
  .karsilama { align-items: center; text-align: center; padding: 34px 24px; gap: 12px; }
  .karsilama h2 { margin: 0; font-family: var(--f-display); font-size: 24px; }
  .k-ikon { width: 68px; height: 68px; border-radius: 20px; display: grid; place-items: center; color: var(--gold-glow); background: rgba(227, 194, 112, 0.12); border: 1px solid rgba(227, 194, 112, 0.3); }
  .k-liste { list-style: none; padding: 0; margin: 6px 0; display: flex; flex-direction: column; gap: 8px; text-align: left; }
  .k-liste li { display: flex; gap: 8px; align-items: flex-start; color: var(--muted); font-size: 13.5px; }
  .k-liste :global(svg) { color: var(--good); flex: none; margin-top: 2px; }
  .btn.discord { background: #5865f2; border-color: #5865f2; color: #fff; padding: 11px 20px; font-size: 14.5px; }
  .btn.discord:hover:not(:disabled) { background: #4752c4; }

  .sekmeler { display: flex; align-items: center; gap: 4px; padding: 4px; border-radius: 14px; background: rgba(0, 0, 0, 0.2); border: 1px solid var(--line); }
  .sekmeler button { display: inline-flex; align-items: center; gap: 7px; padding: 8px 14px; border-radius: 10px; border: 0; background: none; color: var(--muted); font: inherit; font-size: 13.5px; font-weight: 600; cursor: pointer; }
  .sekmeler button:hover { color: var(--text); background: rgba(255, 255, 255, 0.04); }
  .sekmeler button[aria-current="page"] { color: var(--gold-glow); background: rgba(227, 194, 112, 0.12); }
  .sayi { min-width: 20px; height: 18px; padding: 0 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); color: var(--muted); font-size: 11px; display: inline-grid; place-items: center; }
  .sayi.yeni { background: #e5484d; color: #fff; }
  .baglanti { margin-left: auto; display: inline-flex; align-items: center; gap: 6px; padding: 0 10px; font-size: 12px; color: var(--dim); }
  .baglanti i { width: 7px; height: 7px; border-radius: 50%; background: var(--dim); }
  .baglanti.acik i { background: var(--good); box-shadow: 0 0 0 3px rgba(92, 192, 138, 0.18); }

  .av { position: relative; display: inline-block; flex: none; border-radius: 50%; vertical-align: middle; }
  .av img, .avb { width: 100%; height: 100%; border-radius: 50%; object-fit: cover; display: grid; place-items: center; }
  .avb { background: linear-gradient(135deg, #3a3f4a, #262a31); color: var(--gold); font-weight: 800; }
  .nokta { position: absolute; right: 0; bottom: 0; width: 28%; height: 28%; min-width: 9px; min-height: 9px; border-radius: 50%; background: var(--good); box-shadow: 0 0 0 2.5px var(--coal-1, #1b1d22); }
  .nokta.oyunda { background: #8b7cf6; }
  .kim { border: 0; background: none; padding: 0; cursor: pointer; border-radius: 50%; }

  .olaylar { display: flex; flex-direction: column; gap: 12px; }
  .olay { display: flex; flex-direction: column; gap: 10px; padding: 14px 16px; border-radius: 16px; border: 1px solid var(--line); background: linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0.015)); }
  .olay header { display: flex; align-items: center; gap: 11px; }
  .baslik { display: flex; flex-wrap: wrap; align-items: baseline; gap: 4px 7px; flex: 1; min-width: 0; }
  .ad { border: 0; background: none; padding: 0; font: inherit; font-weight: 700; color: var(--text); cursor: pointer; }
  .ad:hover { color: var(--gold-glow); }
  .zaman { font-size: 12px; color: var(--dim); }
  .konu { display: flex; align-items: center; gap: 13px; padding: 10px; border-radius: 12px; background: rgba(0, 0, 0, 0.18); }
  .kp { width: 54px; height: 76px; border-radius: 8px; object-fit: cover; flex: none; }
  .kp.yok, .pk-kapak.yok { display: grid; place-items: center; background: linear-gradient(135deg, #2c3038, #1d2026); color: var(--dim); font-weight: 800; }
  .konu-yazi { display: flex; flex-direction: column; gap: 3px; min-width: 0; flex: 1; }
  .konu-yazi b { font-size: 15px; }
  q { color: var(--muted); font-size: 13.5px; font-style: italic; }
  q::before { content: "“"; } q::after { content: "”"; }
  .puan { flex: none; min-width: 48px; height: 48px; border-radius: 12px; display: grid; place-items: center; font-size: 19px; font-weight: 800; color: #1a1408; background: linear-gradient(160deg, var(--gold-glow), var(--gold)); }
  .olay footer { display: flex; gap: 6px; }
  .eylem { display: inline-flex; align-items: center; gap: 6px; padding: 6px 11px; border-radius: 999px; border: 1px solid transparent; background: none; color: var(--muted); font: inherit; font-size: 13px; cursor: pointer; }
  .eylem:hover { background: rgba(255, 255, 255, 0.05); color: var(--text); }
  .eylem.begendi { color: #ff6b81; }
  .eylem.begendi :global(svg) { fill: currentColor; }
  .ic { display: grid; place-items: center; width: 32px; height: 32px; border-radius: 9px; border: 0; background: none; color: var(--dim); cursor: pointer; flex: none; }
  .ic:hover:not(:disabled) { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .ic:disabled { opacity: 0.35; cursor: default; }
  .ic.kucuk { width: 24px; height: 24px; opacity: 0; }
  .yorum:hover .ic.kucuk { opacity: 1; }
  .yorumlar { display: flex; flex-direction: column; gap: 8px; padding-top: 8px; border-top: 1px solid var(--line); }
  .yorum { display: flex; align-items: flex-start; gap: 8px; }
  .yb { flex: 1; font-size: 13.5px; line-height: 1.4; padding: 6px 10px; border-radius: 12px; background: rgba(255, 255, 255, 0.04); }
  .yb b { font-weight: 700; margin-right: 3px; }
  .dahasi { align-self: flex-start; border: 0; background: none; padding: 0; color: var(--dim); font: inherit; font-size: 12.5px; cursor: pointer; }
  .dahasi:hover { color: var(--gold); }
  .yaz { display: flex; gap: 6px; align-items: center; }
  .yaz input, .dm-yaz input { flex: 1; min-width: 0; background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 999px; padding: 8px 14px; color: var(--text); font: inherit; font-size: 13.5px; outline: none; }
  .yaz input:focus, .dm-yaz input:focus { border-color: rgba(227, 194, 112, 0.5); }
  .devam { align-self: center; }
  .bos-kart { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; padding: 34px; color: var(--muted); }
  .bos { padding: 16px 4px; }

  .kisiler { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 10px; }
  .kisi { display: flex; align-items: center; gap: 6px; padding: 10px 10px 10px 12px; border-radius: 14px; border: 1px solid var(--line); background: rgba(255, 255, 255, 0.025); }
  .kisi:hover { border-color: var(--line-2); }
  .kisi.cevrimdisi { opacity: 0.6; }
  .kisi-ana { flex: 1; min-width: 0; display: flex; align-items: center; gap: 12px; border: 0; background: none; padding: 0; color: inherit; font: inherit; text-align: left; cursor: pointer; }
  .kisi-yazi { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .kisi-yazi b, .kisi-yazi span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .profil { gap: 14px; }
  .geri { align-self: flex-start; display: inline-flex; align-items: center; gap: 6px; border: 0; background: none; padding: 0; color: var(--muted); font: inherit; font-size: 13px; cursor: pointer; }
  .geri:hover { color: var(--gold); }
  .p-ust { display: flex; align-items: center; gap: 16px; }
  .p-ad { flex: 1; display: flex; flex-direction: column; gap: 3px; }
  .p-ad h2 { margin: 0; font-family: var(--f-display); font-size: 24px; }
  .oyunda-yazi { color: #a99cff; }
  .acik-yazi { color: var(--good); }
  .seg.kucuk button { padding: 4px 10px; font-size: 12.5px; }
  .pk-izgara { display: grid; grid-template-columns: repeat(auto-fill, minmax(132px, 1fr)); gap: 14px; }
  .pk { position: relative; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .pk-kapak { width: 100%; aspect-ratio: 2 / 3; border-radius: 10px; object-fit: cover; box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35); }
  .pk-puan { position: absolute; top: 8px; right: 8px; min-width: 34px; height: 26px; padding: 0 7px; border-radius: 8px; display: grid; place-items: center; font-size: 13.5px; font-weight: 800; color: #1a1408; background: var(--gold-glow); box-shadow: 0 3px 10px rgba(0, 0, 0, 0.4); }
  .pk b { font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pk q { font-size: 12px; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }

  .sosyal > .panel.dm { display: flex; flex-direction: row; gap: 0; padding: 0; height: min(68vh, 640px); overflow: hidden; }
  .dm-liste { width: 270px; flex: none; border-right: 1px solid var(--line); overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 2px; }
  .dm-kisi { display: flex; align-items: center; gap: 10px; padding: 8px; border-radius: 11px; border: 0; background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; }
  .dm-kisi:hover { background: rgba(255, 255, 255, 0.04); }
  .dm-kisi.secili { background: rgba(227, 194, 112, 0.1); }
  .dm-yazi { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .dm-yazi span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dm-ana { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .dm-ust { display: flex; align-items: center; gap: 10px; padding: 12px 16px; border-bottom: 1px solid var(--line); }
  .dm-akis { flex: 1; overflow-y: auto; padding: 14px 16px; display: flex; flex-direction: column; gap: 5px; }
  .dm-zaman { align-self: center; font-size: 11.5px; color: var(--dim); margin: 8px 0 2px; }
  .balon { align-self: flex-start; max-width: 72%; padding: 8px 12px; border-radius: 16px 16px 16px 5px; background: rgba(255, 255, 255, 0.07); font-size: 14px; line-height: 1.4; white-space: pre-wrap; overflow-wrap: anywhere; }
  .balon.benim { align-self: flex-end; border-radius: 16px 16px 5px 16px; background: linear-gradient(160deg, rgba(227, 194, 112, 0.32), rgba(227, 194, 112, 0.2)); color: #fff4d8; }
  .dm-yaz { display: flex; gap: 8px; padding: 12px 14px; border-top: 1px solid var(--line); }
  .dm-bos { flex: 1; display: grid; place-content: center; justify-items: center; gap: 8px; color: var(--dim); }

  .ben-ust { display: flex; align-items: center; gap: 14px; }
  .ben-ust h2 { margin: 0; font-family: var(--f-display); }
  .sag { margin-left: auto; display: flex; gap: 6px; }
  .line { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 10px 0; border-top: 1px solid var(--line); }
  .line > div { display: flex; flex-direction: column; gap: 2px; }
  .row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }

  .iskelet { display: flex; gap: 12px; align-items: center; }
  .olay.iskelet { flex-direction: row; flex-wrap: wrap; }
  .sk-av, .sk-satir, .sk-kutu { background: linear-gradient(90deg, rgba(255, 255, 255, 0.05) 25%, rgba(255, 255, 255, 0.1) 50%, rgba(255, 255, 255, 0.05) 75%); background-size: 200% 100%; animation: parilti 1.4s infinite linear; }
  .sk-av { width: 38px; height: 38px; border-radius: 50%; }
  .sk-satir { width: 45%; height: 12px; border-radius: 6px; }
  .sk-kutu { width: 100%; height: 80px; border-radius: 12px; }
  @keyframes parilti { to { background-position: -200% 0; } }
  @media (max-width: 700px) {
    .dm-liste { width: 92px; }
    .dm-yazi { display: none; }
  }
  @media (prefers-reduced-motion: reduce) { .sk-av, .sk-satir, .sk-kutu { animation: none; } }
</style>
