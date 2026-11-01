<script lang="ts">
  // Steam tarzı profil: kapak arka planı, büyük resim, seviye ve sayılar; vitrinler (favori oyunlar, en çok
  // oynananlar, film ve dizi vitrinleri, ortak oyunlar, son etkinlik).
  import Icon from "../Icon.svelte";
  import Avatar from "../components/social/Avatar.svelte";
  import Cover from "../components/social/Cover.svelte";
  import FeedCard from "../components/social/FeedCard.svelte";
  import { once, puanYaz, saat } from "../components/social/metin";
  import { api } from "../api";
  import { nav } from "../player.svelte";
  import { arkadasEkle, arkadasSil, istek, sohbetAc, sosyal, tamAdres, type Kisi, type Olay, type Profil, type Puan } from "../social.svelte";
  import { notify } from "../toast.svelte";

  interface Yanit {
    kisi: Kisi;
    cevrimici: boolean;
    oyun: string | null;
    profil: Profil | null;
    iliski: Kisi["iliski"];
    arkadas: number;
    /** Başkalarından alınan beğeni ve yorum (süresi dolup silinen kliplerinki dahil). */
    begeni?: number;
    yorum?: number;
  }

  let veri = $state<Yanit | null>(null);
  let olaylar = $state<Olay[]>([]);
  let benimOyunlarim = $state<string[]>([]);
  let hataMetni = $state("");

  const hata = (e: unknown) => notify(String(e instanceof Error ? e.message : e), "info");
  const id = $derived(sosyal.profilId ?? sosyal.ben?.id ?? null);
  const benim = $derived(id === sosyal.ben?.id);

  async function yukle(kim: string) {
    veri = null;
    hataMetni = "";
    try {
      const [p, a] = await Promise.all([
        istek<Yanit>("GET", `/api/profil/${kim}`),
        istek<{ olaylar: Olay[] }>("GET", `/api/akis?kisi=${kim}&sayi=4`),
      ]);
      veri = p;
      olaylar = a.olaylar;
    } catch (e) {
      hataMetni = String(e instanceof Error ? e.message : e);
    }
  }

  $effect(() => {
    if (id && sosyal.anahtar) void yukle(id);
  });
  $effect(() => {
    void api.games.list().then((g) => (benimOyunlarim = g.filter((x) => x.inLibrary).map((x) => x.name.toLocaleLowerCase("tr")))).catch(() => {});
  });

  // Arkadaşlık durumu canlı listeden (istek gönderince anında değişsin).
  const iliski = $derived.by((): Kisi["iliski"] => {
    if (!veri) return null;
    const k = veri.kisi.id;
    if (sosyal.arkadaslar.some((x) => x.id === k)) return "arkadas";
    if (sosyal.gelen.some((x) => x.id === k)) return "gelen";
    if (sosyal.giden.some((x) => x.id === k)) return "giden";
    return null;
  });
  const canli = $derived(veri ? sosyal.cevrimici[veri.kisi.id] : undefined);
  const oyunu = $derived(canli?.oyun ?? null);

  const p = $derived(veri?.profil ?? { oyunlar: [], puanlar: [] });
  const puanli = (tur: Puan["tur"]) =>
    p.puanlar.filter((x) => x.tur === tur && x.puan != null).slice().sort((a, b) => (b.puan ?? 0) - (a.puan ?? 0) || b.guncelleme - a.guncelleme);
  const favOyun = $derived(puanli("game").slice(0, 6));
  const favFilm = $derived(puanli("movie").slice(0, 8));
  const favDizi = $derived(puanli("series").slice(0, 8));
  const enCok = $derived(p.oyunlar.filter((o) => o.dakika > 0).slice().sort((a, b) => b.dakika - a.dakika).slice(0, 5));
  const toplamDk = $derived(p.oyunlar.reduce((t, o) => t + o.dakika, 0));
  const ortak = $derived(veri && !benim ? p.oyunlar.filter((o) => benimOyunlarim.includes(o.ad.toLocaleLowerCase("tr"))).slice(0, 12) : []);
  const ortalama = $derived.by(() => {
    const l = p.puanlar.filter((x) => x.puan != null);
    return l.length ? l.reduce((t, x) => t + (x.puan ?? 0), 0) / l.length : null;
  });
  /** Malikâne seviyesi: oynanan saat ve paylaşılan puanlarla büyür (Steam seviyesi gibi, eğlencesine). */
  const seviye = $derived(Math.max(1, Math.floor(Math.sqrt(toplamDk / 60 / 5 + p.puanlar.length * 2))));
  /** Arka plan: favori ya da en çok oynanan oyunun kapağı. */
  const arkaPlan = $derived(tamAdres(favOyun[0]?.kapak ?? enCok[0]?.kapak ?? favDizi[0]?.kapak ?? favFilm[0]?.kapak ?? null));
  const SAYI = (n: number) => n.toLocaleString("tr-TR");
</script>

{#snippet vitrin(baslik: string, simge: "trophy" | "film" | "library" | "users" | "sparkle" | "clock", alt: string)}
  <header class="v-ust"><Icon name={simge} size={16} /><b>{baslik}</b><span>{alt}</span></header>
{/snippet}

{#snippet afisler(l: Puan[])}
  <div class="afisler">
    {#each l as x (x.anahtar)}
      <div class="afis" title={x.yorum ? `${x.ad}: “${x.yorum}”` : x.ad}>
        <div class="afis-kapak"><Cover url={x.kapak} ad={x.ad} kose={8} />{#if puanYaz(x.puan)}<span class="rozet tnum">{puanYaz(x.puan)}</span>{/if}</div>
        <span class="afis-ad">{x.ad}</span>
      </div>
    {/each}
  </div>
{/snippet}

<div class="profil">
  {#if !sosyal.anahtar}
    <section class="panel bos">
      <Icon name="user" size={30} />
      <b>Profil için Topluluk'a giriş yap</b>
      <button class="btn" onclick={() => nav.go("social")}><Icon name="users" size={16} />Topluluk'a git</button>
    </section>
  {:else if hataMetni}
    <section class="panel bos"><Icon name="info" size={26} /><span>{hataMetni}</span></section>
  {:else if !veri}
    <div class="ust iskelet"><span class="sk-av"></span><span class="sk-satir"></span></div>
  {:else}
    <section class="ust" style:--arka={arkaPlan ? `url("${arkaPlan}")` : "none"}>
      <div class="ust-arka" aria-hidden="true"></div>
      <div class="ust-ic">
        <div class="cerceve"><Avatar kisi={veri.kisi} boy={132} nokta={false} /></div>
        <div class="kimlik">
          <h1>{veri.kisi.ad}</h1>
          <span class="durum" class:oyunda={!!oyunu} class:acik={!!canli}>
            {#if oyunu}<i></i>Şu an {oyunu} oynuyor{:else if canli}<i></i>Çevrim içi{:else}Çevrim dışı{/if}
          </span>
          <div class="dugmeler">
            {#if benim}
              <button class="btn ghost sm" onclick={() => nav.go("settings/topluluk")}><Icon name="gear" size={14} />Paylaşımı düzenle</button>
            {:else if iliski === "arkadas"}
              <button class="btn sm" onclick={() => veri && sohbetAc(veri.kisi)}><Icon name="chat" size={14} />Mesaj gönder</button>
              <button class="btn ghost sm" title="Arkadaşlıktan çıkar" onclick={() => veri && arkadasSil(veri.kisi).catch(hata)}>Arkadaşsınız ✓</button>
            {:else if iliski === "gelen"}
              <button class="btn sm" onclick={() => veri && arkadasEkle(veri.kisi).catch(hata)}><Icon name="check" size={14} />İsteği kabul et</button>
              <button class="btn ghost sm" onclick={() => veri && arkadasSil(veri.kisi).catch(hata)}>Reddet</button>
            {:else if iliski === "giden"}
              <button class="btn ghost sm" title="İsteği geri al" onclick={() => veri && arkadasSil(veri.kisi).catch(hata)}>İstek gönderildi</button>
            {:else}
              <button class="btn sm" onclick={() => veri && arkadasEkle(veri.kisi).catch(hata)}><Icon name="plus" size={14} />Arkadaş ekle</button>
            {/if}
          </div>
        </div>
        <div class="seviye" title="Malikâne seviyesi: oynadığın saat ve paylaştığın puanlarla yükselir">
          <span>Seviye</span>
          <b class="tnum">{seviye}</b>
        </div>
      </div>
    </section>

    <div class="govde">
      <div class="vitrinler">
        {#if oyunu}
          <section class="vitrin simdi">
            <Icon name="bolt" size={18} />
            <div><span class="sub small">Şu an oynuyor</span><b>{oyunu}</b></div>
          </section>
        {/if}

        {#if favOyun.length}
          <section class="vitrin">
            {@render vitrin("Favori oyunlar", "trophy", "en yüksek puanladıkları")}
            <div class="fav-oyunlar">
              {#each favOyun as x, i (x.anahtar)}
                <div class="fav" class:ilk={i === 0} title={x.yorum ? `“${x.yorum}”` : x.ad}>
                  <Cover url={x.kapak} ad={x.ad} kose={10} />
                  <div class="fav-alt"><span>{x.ad}</span><b class="tnum">{puanYaz(x.puan)}</b></div>
                </div>
              {/each}
            </div>
          </section>
        {/if}

        {#if enCok.length}
          <section class="vitrin">
            {@render vitrin("En çok oynananlar", "clock", `toplam ${saat(toplamDk)}`)}
            <div class="enler">
              {#each enCok as o (o.ad)}
                <div class="en">
                  <Cover url={o.kapak} ad={o.ad} en="46px" kose={6} />
                  <div class="en-yazi">
                    <div class="spread"><b>{o.ad}</b><span class="sub small tnum">{saat(o.dakika)}</span></div>
                    <div class="cubuk"><i style:width="{Math.max(4, (o.dakika / enCok[0].dakika) * 100)}%"></i></div>
                    {#if o.son}<span class="sub small">son oynama {once(o.son)}</span>{/if}
                  </div>
                </div>
              {/each}
            </div>
          </section>
        {/if}

        {#if favDizi.length}
          <section class="vitrin">
            {@render vitrin("Dizi vitrini", "film", `${favDizi.length} favori dizi`)}
            {@render afisler(favDizi)}
          </section>
        {/if}

        {#if favFilm.length}
          <section class="vitrin">
            {@render vitrin("Film vitrini", "film", `${favFilm.length} favori film`)}
            {@render afisler(favFilm)}
          </section>
        {/if}

        {#if ortak.length}
          <section class="vitrin">
            {@render vitrin("Ortak oyunlarınız", "users", `${ortak.length} oyun ikinizde de var`)}
            <div class="ortak">
              {#each ortak as o (o.ad)}<div class="ortak-oyun" title={o.ad}><Cover url={o.kapak} ad={o.ad} kose={8} /></div>{/each}
            </div>
          </section>
        {/if}

        {#if olaylar.length}
          <section class="vitrin">
            {@render vitrin("Son etkinlik", "sparkle", "")}
            <div class="olaylar">
              {#each olaylar as o (o.id)}<FeedCard {o} ondegisti={() => id && yukle(id)} onsil={() => id && yukle(id)} />{/each}
            </div>
          </section>
        {/if}

        {#if !p.oyunlar.length && !p.puanlar.length}
          <section class="vitrin bos-v"><span class="sub">{benim ? "Henüz bir şey paylaşmadın. Kontrol Odası → Topluluk'tan neyin görüneceğini seçebilirsin." : "Henüz bir şey paylaşmamış."}</span></section>
        {/if}
      </div>

      <aside class="yan">
        <section class="kutu">
          <b class="kutu-baslik">Malikâne kaydı</b>
          <div class="sayi"><span>Oyun</span><b class="tnum">{SAYI(p.oyunlar.length)}</b></div>
          <div class="sayi"><span>Oynanan saat</span><b class="tnum">{SAYI(Math.round(toplamDk / 60))}</b></div>
          <div class="sayi"><span>Puanlanan oyun</span><b class="tnum">{SAYI(p.puanlar.filter((x) => x.tur === "game").length)}</b></div>
          <div class="sayi"><span>Film</span><b class="tnum">{SAYI(p.puanlar.filter((x) => x.tur === "movie").length)}</b></div>
          <div class="sayi"><span>Dizi</span><b class="tnum">{SAYI(p.puanlar.filter((x) => x.tur === "series").length)}</b></div>
          {#if ortalama != null}<div class="sayi"><span>Ortalama puan</span><b class="tnum altin">{puanYaz(ortalama)}</b></div>{/if}
          <div class="sayi"><span>Arkadaş</span><b class="tnum">{SAYI(veri.arkadas)}</b></div>
          <div class="sayi" title="Paylaşımlarına gelen beğeniler (silinen kliplerinki de sayılır)"><span>Aldığı beğeni</span><b class="tnum kirmizi">{SAYI(veri.begeni ?? 0)}</b></div>
        </section>
        {#if benim && sosyal.arkadaslar.length}
          <section class="kutu">
            <b class="kutu-baslik">Arkadaşlar</b>
            {#each sosyal.arkadaslar.slice(0, 8) as k (k.id)}
              <button class="ark" onclick={() => (sosyal.profilId = k.id)}>
                <Avatar kisi={k} boy={30} />
                <span>{k.ad}</span>
              </button>
            {/each}
          </section>
        {/if}
      </aside>
    </div>
  {/if}
</div>

<style>
  .profil { display: flex; flex-direction: column; gap: 16px; max-width: 1100px; margin: 0 auto; }
  .bos { display: flex; flex-direction: column; align-items: center; gap: 10px; padding: 34px; text-align: center; }
  .ust { position: relative; overflow: hidden; border-radius: 20px; border: 1px solid var(--line-2); min-height: 210px; display: flex; align-items: flex-end; }
  .ust-arka { position: absolute; inset: -30px; background-image: linear-gradient(180deg, rgba(14, 15, 18, 0.35), rgba(14, 15, 18, 0.92)), var(--arka); background-color: #20232a; background-size: cover; background-position: center 30%; filter: blur(14px) saturate(1.2); }
  .ust-ic { position: relative; display: flex; align-items: flex-end; gap: 22px; width: 100%; padding: 26px 28px; }
  .cerceve { padding: 4px; border-radius: 50%; background: linear-gradient(140deg, var(--gold-glow), #6a5424 60%, var(--gold)); box-shadow: 0 10px 30px rgba(0, 0, 0, 0.55); }
  .kimlik { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 6px; }
  h1 { margin: 0; font-family: var(--f-display); font-size: 32px; text-shadow: 0 2px 12px rgba(0, 0, 0, 0.6); }
  .durum { display: inline-flex; align-items: center; gap: 7px; font-size: 13.5px; color: var(--muted); }
  .durum i { width: 8px; height: 8px; border-radius: 50%; background: var(--good); }
  .durum.acik { color: #8fe0b4; }
  .durum.oyunda { color: #b9adff; }
  .durum.oyunda i { background: #8b7cf6; box-shadow: 0 0 0 3px rgba(139, 124, 246, 0.25); }
  .dugmeler { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 6px; }
  .sm { padding: 6px 12px; font-size: 13px; }
  .kirmizi { color: #ff7a7e; }
  .seviye { display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 10px 16px; border-radius: 14px; background: rgba(0, 0, 0, 0.35); border: 1px solid rgba(227, 194, 112, 0.35); }
  .seviye span { font-size: 10.5px; font-weight: 800; letter-spacing: 0.14em; text-transform: uppercase; color: var(--gold); }
  .seviye b { width: 46px; height: 46px; border-radius: 50%; display: grid; place-items: center; font-size: 20px; font-family: var(--f-display); border: 2px solid var(--gold); color: var(--gold-glow); }

  .govde { display: grid; grid-template-columns: minmax(0, 1fr) 260px; gap: 16px; align-items: start; }
  .vitrinler { display: flex; flex-direction: column; gap: 14px; min-width: 0; }
  .vitrin { display: flex; flex-direction: column; gap: 12px; padding: 16px; border-radius: 16px; border: 1px solid var(--line); background: linear-gradient(180deg, rgba(34, 37, 42, 0.85), rgba(26, 28, 33, 0.85)); }
  .v-ust { display: flex; align-items: center; gap: 8px; padding-bottom: 10px; border-bottom: 1px solid var(--line); }
  .v-ust :global(svg) { color: var(--gold); }
  .v-ust b { font-family: var(--f-display); font-size: 15px; letter-spacing: 0.02em; }
  .v-ust span { margin-left: auto; font-size: 12px; color: var(--dim); }
  .simdi { flex-direction: row; align-items: center; gap: 12px; border-color: rgba(139, 124, 246, 0.45); background: linear-gradient(90deg, rgba(139, 124, 246, 0.18), rgba(26, 28, 33, 0.85)); color: #c9c0ff; }
  .simdi div { display: flex; flex-direction: column; }
  .simdi b { font-size: 16px; color: var(--text); }

  .fav-oyunlar { display: grid; grid-template-columns: 1.6fr repeat(5, 1fr); gap: 10px; align-items: end; }
  .fav { position: relative; display: flex; flex-direction: column; min-width: 0; transition: transform 0.15s; }
  .fav:hover { transform: translateY(-3px); }
  .fav-alt { display: flex; align-items: center; gap: 6px; padding-top: 6px; font-size: 12px; }
  .fav-alt span { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted); }
  .fav-alt b { color: var(--gold-glow); }
  .fav.ilk .fav-alt span { color: var(--text); font-weight: 600; font-size: 13.5px; }
  .fav.ilk .fav-alt b { font-size: 16px; }

  .enler { display: flex; flex-direction: column; gap: 10px; }
  .en { display: flex; gap: 12px; align-items: center; }
  .en-yazi { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .en-yazi b { font-size: 13.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cubuk { height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.06); overflow: hidden; }
  .cubuk i { display: block; height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--gold), var(--gold-glow)); }

  .afisler { display: grid; grid-template-columns: repeat(auto-fill, minmax(104px, 1fr)); gap: 12px; }
  .afis { display: flex; flex-direction: column; gap: 5px; min-width: 0; }
  .afis-kapak { position: relative; box-shadow: 0 8px 18px rgba(0, 0, 0, 0.35); border-radius: 8px; transition: transform 0.15s; }
  .afis:hover .afis-kapak { transform: translateY(-3px); }
  .rozet { position: absolute; top: 6px; right: 6px; min-width: 30px; height: 22px; padding: 0 6px; border-radius: 7px; display: grid; place-items: center; font-size: 12px; font-weight: 800; color: #1a1408; background: var(--gold-glow); }
  .afis-ad { font-size: 12px; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .ortak { display: grid; grid-template-columns: repeat(auto-fill, minmax(70px, 1fr)); gap: 8px; }
  .olaylar { display: flex; flex-direction: column; gap: 10px; }
  .bos-v { align-items: center; text-align: center; padding: 26px; }

  .yan { display: flex; flex-direction: column; gap: 14px; position: sticky; top: 0; }
  .kutu { display: flex; flex-direction: column; gap: 6px; padding: 14px 16px; border-radius: 16px; border: 1px solid var(--line); background: rgba(26, 28, 33, 0.85); }
  .kutu-baslik { font-size: 11px; font-weight: 800; letter-spacing: 0.12em; text-transform: uppercase; color: var(--dim); margin-bottom: 4px; }
  .sayi { display: flex; justify-content: space-between; align-items: baseline; font-size: 13.5px; padding: 3px 0; }
  .sayi span { color: var(--muted); }
  .sayi b { font-size: 15px; }
  .altin { color: var(--gold-glow); }
  .ark { display: flex; align-items: center; gap: 10px; padding: 5px 6px; border-radius: 9px; border: 0; background: none; color: var(--text); font: inherit; font-size: 13.5px; text-align: left; cursor: pointer; }
  .ark:hover { background: rgba(255, 255, 255, 0.05); }

  .iskelet { min-height: 210px; gap: 16px; align-items: center; padding: 26px; border: 1px solid var(--line); }
  .sk-av, .sk-satir { background: linear-gradient(90deg, rgba(255, 255, 255, 0.05) 25%, rgba(255, 255, 255, 0.1) 50%, rgba(255, 255, 255, 0.05) 75%); background-size: 200% 100%; animation: parilti 1.4s infinite linear; }
  .sk-av { width: 132px; height: 132px; border-radius: 50%; }
  .sk-satir { width: 240px; height: 20px; border-radius: 8px; }
  @keyframes parilti { to { background-position: -200% 0; } }
  @media (max-width: 900px) {
    .govde { grid-template-columns: 1fr; }
    .yan { position: static; }
    .fav-oyunlar { grid-template-columns: repeat(3, 1fr); }
    .ust-ic { flex-wrap: wrap; }
  }
</style>
