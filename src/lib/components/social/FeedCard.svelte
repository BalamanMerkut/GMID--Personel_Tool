<script lang="ts">
  // Akıştaki bir paylaşım: kim, ne yaptı (puanladı / izledi / oynamaya başladı), konu kartı, beğeni ve yorumlar.
  import Icon from "../../Icon.svelte";
  import { notify } from "../../toast.svelte";
  import { istek, konuGoster, oyVer, profilGoster, sosyal, tamAdres, type Olay } from "../../social.svelte";
  import Avatar from "./Avatar.svelte";
  import Cover from "./Cover.svelte";
  import GeceKart from "./GeceKart.svelte";
  import TurnuvaKart from "./TurnuvaKart.svelte";
  import { eylem, kalan, once, parcala, puanYaz, sure, TUR_AD } from "./metin";

  let { o, ondegisti, onsil }: { o: Olay; ondegisti: () => void; onsil: (o: Olay) => void } = $props();
  let taslak = $state("");
  let acik = $state(false);
  /** Silme iki tıkla: ilk tık "Silinsin mi?" olur, birkaç saniye içinde ikinci tık siler. */
  let silOnay = $state(false);
  let silZaman: ReturnType<typeof setTimeout> | undefined;
  let sikayetOnay = $state(false);
  let sikayetZaman: ReturnType<typeof setTimeout> | undefined;
  let sikayetEttim = $state(false);
  const benim = $derived(o.kisi.id === sosyal.ben?.id);
  /** Kendi paylaşımım ya da ben yöneticiyim: silebilirim. */
  const silebilir = $derived(benim || sosyal.yonetici);

  /** Şikâyet de iki tıkla: ilk tık "Şikâyet edilsin mi?", ikinci tık yöneticiye bildirir. */
  async function sikayet() {
    if (!sikayetOnay) {
      sikayetOnay = true;
      clearTimeout(sikayetZaman);
      sikayetZaman = setTimeout(() => (sikayetOnay = false), 4000);
      return;
    }
    clearTimeout(sikayetZaman);
    sikayetOnay = false;
    try {
      await istek("POST", `/api/akis/${o.id}/sikayet`, {});
      sikayetEttim = true;
      notify("Yöneticiye bildirildi, teşekkürler", "flag");
    } catch (e) {
      hata(e);
    }
  }

  const hata = (e: unknown) => notify(String(e instanceof Error ? e.message : e), "info");

  // ---- anket ----
  const anket = $derived(o.veri.anket ?? null);
  const oylar = $derived(anket ? (anket.durum?.oylar ?? anket.secenekler.map(() => 0)) : []);
  const toplamOy = $derived(oylar.reduce((a, b) => a + b, 0));
  const benimOy = $derived(anket?.durum?.benim ?? null);
  const anketBitti = $derived(!!anket && anket.bitis < Date.now());
  /** Oy verdiysem ya da bittiyse sonuçlar görünür. */
  const sonucGoster = $derived(anketBitti || benimOy != null);
  let oyluyor = $state(false);
  function anketKalan(ms: number): string {
    const dk = Math.round(ms / 60000);
    if (dk <= 0) return "bitti";
    if (dk < 60) return `${dk} dk kaldı`;
    if (dk < 48 * 60) return `${Math.round(dk / 60)} sa kaldı`;
    return `${Math.round(dk / 1440)} gün kaldı`;
  }
  async function oyla(i: number) {
    if (!anket || anketBitti || oyluyor || benimOy === i) return;
    oyluyor = true;
    try {
      o.veri.anket = { ...anket, durum: await oyVer(o.id, i) };
    } catch (e) {
      hata(e);
    } finally {
      oyluyor = false;
    }
  }

  async function begen() {
    o.begendim = !o.begendim;
    o.begeni += o.begendim ? 1 : -1;
    try {
      const r = await istek<{ begendim: boolean; begeni: number }>("POST", `/api/akis/${o.id}/begen`);
      o.begendim = r.begendim;
      o.begeni = r.begeni;
    } catch (e) {
      hata(e);
    }
  }

  async function yorumYap() {
    const metin = taslak.trim();
    if (!metin) return;
    taslak = "";
    try {
      await istek("POST", `/api/akis/${o.id}/yorum`, { metin });
      acik = true;
      ondegisti();
    } catch (e) {
      taslak = metin;
      hata(e);
    }
  }

  async function sil() {
    if (!silOnay) {
      silOnay = true;
      clearTimeout(silZaman);
      silZaman = setTimeout(() => (silOnay = false), 4000);
      return;
    }
    clearTimeout(silZaman);
    silOnay = false;
    try {
      await istek("DELETE", `/api/akis/${o.id}`);
      notify(o.tur === "klip" ? "Klip Sosyal'den kaldırıldı" : "Paylaşım kaldırıldı", "check");
      onsil(o);
    } catch (e) {
      hata(e);
    }
  }

  async function yorumSil(id: number) {
    await istek("DELETE", `/api/yorum/${id}`).catch(hata);
    ondegisti();
  }
</script>

{#snippet yazi(m: string)}
  <p class="metin">{#each parcala(m, sosyal.uyeler) as p, i (i)}{#if p.etiket}<button class="etiket" onclick={() => konuGoster(p.t)}>{p.t}</button>{:else if p.anma}<button class="etiket anma" onclick={() => profilGoster(p.anma ?? null)}>{p.t}</button>{:else}{p.t}{/if}{/each}</p>
{/snippet}

<!-- Twitter düzeni: solda profil resmi, sağda ad · zaman, içerik, altta yorum ve beğeni. -->
<article class="tw">
  <button class="kim" onclick={() => profilGoster(o.kisi.id)} aria-label={o.kisi.ad}><Avatar kisi={o.kisi} boy={44} /></button>
  <div class="govde">
    <header>
      <button class="ad" onclick={() => profilGoster(o.kisi.id)}>{o.kisi.ad}</button>
      {#if eylem(o)}<span class="eylem-yazi">{eylem(o)}</span>{/if}
      <span class="nokta">·</span>
      <span class="zaman" title={new Date(o.zaman).toLocaleString("tr-TR")}>{once(o.zaman)}</span>
      <span class="sag-ust">
        {#if sosyal.yonetici && o.sikayet}
          <span class="sikayet-rozet" title="Bu paylaşımı şikâyet eden kişi sayısı"><Icon name="flag" size={13} />{o.sikayet}</span>
        {/if}
        {#if !benim && sosyal.ben && o.kisi.id !== "gmid"}
          <button class="ic sil" class:onay={sikayetOnay} disabled={sikayetEttim} title={sikayetEttim ? "Şikâyet edildi" : sikayetOnay ? "Göndermek için yeniden tıkla" : "Uygunsuz içeriği şikâyet et"} aria-label="Şikâyet et" onclick={sikayet}>
            <Icon name="flag" size={15} />{#if sikayetOnay}<span>Şikâyet edilsin mi?</span>{/if}
          </button>
        {/if}
        {#if silebilir}
          <button class="ic sil" class:onay={silOnay} title={silOnay ? "Silmek için yeniden tıkla" : benim ? "Gönderiyi sil" : "Yönetici olarak sil"} aria-label="Gönderiyi sil" onclick={sil}>
            <Icon name="trash" size={15} />{#if silOnay}<span>Silinsin mi?</span>{/if}
          </button>
        {/if}
      </span>
    </header>

    {#if o.tur === "gece" && o.veri.gece}
      <GeceKart id={o.veri.gece} yedek={{ oyun: o.veri.oyun ?? o.veri.ad, zaman: o.veri.zaman, kontenjan: o.veri.kontenjan, not: o.veri.not, hemen: !!o.veri.hemen }} />
    {:else if o.tur === "turnuva" && o.veri.turnuva}
      <div class="tk"><TurnuvaKart id={o.veri.turnuva} yedek={{ ad: o.veri.ad, oyun: o.veri.oyun ?? "", boyut: o.veri.boyut, durum: o.veri.durum, sampiyon: o.veri.sampiyon }} /></div>
    {:else if o.tur === "ozet"}
      {@const v = o.veri}
      <div class="ozet">
        <div class="ozet-bas">
          <span class="ozet-kicker">Haftalık özet</span>
          {#if v.baslangic && v.bitis}<span class="sub small">{new Date(v.baslangic).toLocaleDateString("tr-TR", { day: "numeric", month: "long" })} – {new Date(v.bitis - 1).toLocaleDateString("tr-TR", { day: "numeric", month: "long" })}</span>{/if}
        </div>
        <div class="ozet-sayilar">
          <span><b class="tnum">{v.aktif ?? 0}</b>aktif üye</span>
          <span><b class="tnum">{v.gonderi ?? 0}</b>gönderi</span>
          <span><b class="tnum">{v.klip_sayisi ?? 0}</b>klip</span>
          <span><b class="tnum">{typeof v.gece === "number" ? v.gece : 0}</b>oyun gecesi</span>
        </div>
        {#if v.oyunlar?.length}
          <div class="ozet-satir"><Icon name="trophy" size={15} /><span>En çok oynanan: {#each v.oyunlar as g, i (g.ad)}{i ? ", " : ""}<button class="ozet-link" onclick={() => konuGoster(g.ad)}>{g.ad}</button> <span class="sub">({g.kisi} kişi)</span>{/each}</span></div>
        {/if}
        {#if v.klip}
          <div class="ozet-satir vurgu"><Icon name="clip" size={15} /><span><b>Haftanın klibi:</b> <button class="ozet-link" onclick={() => profilGoster(v.klip!.kisi.id)}>{v.klip.kisi.ad}</button>{v.klip.ad ? ` · ${v.klip.ad}` : ""} <span class="sub">({v.klip.begeni} beğeni)</span></span></div>
        {/if}
        {#if v.gonderi_en}
          <div class="ozet-satir"><Icon name="heart" size={15} /><span><b>En beğenilen gönderi:</b> <button class="ozet-link" onclick={() => profilGoster(v.gonderi_en!.kisi.id)}>{v.gonderi_en.kisi.ad}</button>{v.gonderi_en.metin ? ` — “${v.gonderi_en.metin.slice(0, 80)}${v.gonderi_en.metin.length > 80 ? "…" : ""}”` : ""}</span></div>
        {/if}
      </div>
    {:else if o.tur === "gonderi"}
      {#if o.veri.metin}{@render yazi(o.veri.metin)}{/if}
      {#if anket}
        <div class="anket" class:sonuc={sonucGoster}>
          {#each anket.secenekler as sec, i (i)}
            {@const yuzde = toplamOy ? Math.round((oylar[i] / toplamOy) * 100) : 0}
            <button class="sec" class:benim={benimOy === i} disabled={anketBitti || oyluyor} onclick={() => oyla(i)} title={anketBitti ? "Anket bitti" : benimOy === i ? "Oyun bu" : "Oy ver"}>
              {#if sonucGoster}<span class="dolgu" style:width="{yuzde}%"></span>{/if}
              <span class="sec-ad">{#if benimOy === i}<Icon name="check" size={13} />{/if}{sec}</span>
              {#if sonucGoster}<span class="yuzde tnum">%{yuzde}</span>{/if}
            </button>
          {/each}
          <span class="anket-bilgi tnum">{toplamOy} oy · {anketBitti ? "Anket bitti" : anketKalan(anket.bitis - Date.now())}{!anketBitti && benimOy != null ? " · fikrini değiştirebilirsin" : ""}</span>
        </div>
      {/if}
      {#if o.veri.resim}<img class="medya" src={tamAdres(o.veri.resim)} alt="" loading="lazy" />{/if}
    {:else if o.tur === "klip"}
      <!-- Paylaşılan klip: 7 gün durur, sonra video da paylaşım da silinir (beğeniler profilde kalır). -->
      {#if o.veri.not}{@render yazi(o.veri.not)}{/if}
      <div class="klip">
        <!-- svelte-ignore a11y_media_has_caption -->
        <video src={tamAdres(o.veri.url)} poster={tamAdres(o.veri.kapak) ?? undefined} controls preload="metadata" playsinline></video>
      </div>
      <div class="klip-alt">
        <b>{o.veri.ad}</b>
        <span class="sub small">{[o.veri.oyun, o.veri.sure ? sure(o.veri.sure) : null].filter(Boolean).join(" · ")}</span>
        <span class="kalan" title="Klipler 7 gün sonra silinir; aldığı beğeniler profilinde kalır"><Icon name="clock" size={13} />{kalan(o.veri.bitis ?? o.zaman + 7 * 86400000)}</span>
      </div>
    {:else}
      {#if o.veri.yorum}{@render yazi(o.veri.yorum)}{/if}
      <div class="konu">
        <Cover url={o.veri.kapak} ad={o.veri.ad} en="58px" kose={8} />
        <div class="konu-yazi">
          <b>{o.veri.ad}</b>
          <span class="sub small">{o.tur === "oyun" ? "oyun" : TUR_AD[o.veri.tur ?? "game"]}{o.veri.yil ? ` · ${o.veri.yil}` : ""}</span>
        </div>
        {#if puanYaz(o.veri.puan)}<span class="puan tnum">{puanYaz(o.veri.puan)}</span>{/if}
      </div>
    {/if}

    <footer>
      <button class="eylem begen-d" class:begendi={o.begendim} onclick={begen} aria-pressed={o.begendim} title="Beğen">
        <span class="hale"><Icon name="heart" size={17} /></span><span class="tnum">{o.begeni || ""}</span>
      </button>
      <button class="eylem yorum-d" class:acik onclick={() => (acik = !acik)} title="Yanıtla">
        <span class="hale"><Icon name="chat" size={17} /></span><span class="tnum">{o.yorumlar.length || ""}</span>
      </button>
    </footer>

    {#if acik || o.yorumlar.length}
      <div class="yorumlar">
        {#each acik ? o.yorumlar : o.yorumlar.slice(-2) as y (y.id)}
          <div class="yorum">
            <button class="kim" onclick={() => profilGoster(y.kisi.id)}><Avatar kisi={y.kisi} boy={28} nokta={false} /></button>
            <div class="yb">
              <span><button class="ad kucuk-ad" onclick={() => profilGoster(y.kisi.id)}>{y.kisi.ad}</button> <span class="zaman">· {once(y.zaman)}</span></span>
              <span class="yorum-metin">{#each parcala(y.metin, sosyal.uyeler) as p, i (i)}{#if p.etiket}<button class="etiket" onclick={() => konuGoster(p.t)}>{p.t}</button>{:else if p.anma}<button class="etiket anma" onclick={() => profilGoster(p.anma ?? null)}>{p.t}</button>{:else}{p.t}{/if}{/each}</span>
            </div>
            {#if y.kisi.id === sosyal.ben?.id || o.kisi.id === sosyal.ben?.id || sosyal.yonetici}
              <button class="ic kucuk" title="Yanıtı sil" aria-label="Yanıtı sil" onclick={() => yorumSil(y.id)}><Icon name="x" size={13} /></button>
            {/if}
          </div>
        {/each}
        {#if !acik && o.yorumlar.length > 2}
          <button class="dahasi" onclick={() => (acik = true)}>Tüm yanıtları göster ({o.yorumlar.length})</button>
        {/if}
        <form class="yaz" onsubmit={(e) => (e.preventDefault(), yorumYap())}>
          {#if sosyal.ben}<Avatar kisi={sosyal.ben} boy={28} nokta={false} />{/if}
          <input bind:value={taslak} placeholder="Yanıtını yaz" maxlength="1000" aria-label="Yanıt" />
          <button class="btn sm yanitla" disabled={!taslak.trim()}>Yanıtla</button>
        </form>
      </div>
    {/if}
  </div>
</article>

<style>
  .tw { display: grid; grid-template-columns: 44px minmax(0, 1fr); gap: 12px; padding: 14px 18px; border-bottom: 1px solid var(--line); transition: background 0.12s; }
  .tw:hover { background: rgba(255, 255, 255, 0.018); }
  .govde { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  header { display: flex; align-items: baseline; gap: 6px; min-width: 0; line-height: 1.3; }
  .kim { border: 0; background: none; padding: 0; cursor: pointer; border-radius: 50%; align-self: start; }
  .ad { border: 0; background: none; padding: 0; font: inherit; font-size: 15px; font-weight: 700; color: var(--text); cursor: pointer; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ad:hover { text-decoration: underline; }
  .eylem-yazi { color: var(--muted); font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .nokta, .zaman { color: var(--dim); font-size: 13.5px; white-space: nowrap; }
  .sag-ust { margin-left: auto; align-self: center; display: inline-flex; align-items: center; gap: 2px; }
  .sikayet-rozet { display: inline-flex; align-items: center; gap: 4px; padding: 2px 8px; border-radius: 999px; background: rgba(229, 72, 77, 0.16); color: #ff8a8d; font-size: 12px; font-weight: 700; }
  .ic:disabled { opacity: 0.35; cursor: default; }
  .metin { margin: 0; font-size: 15px; line-height: 1.5; white-space: pre-wrap; overflow-wrap: anywhere; }
  .etiket { border: 0; background: none; padding: 0; font: inherit; color: var(--gold); cursor: pointer; }
  .etiket.anma { color: #6cb6ff; }
  .etiket:hover { text-decoration: underline; color: var(--gold-glow); }
  .medya { display: block; max-width: 100%; max-height: 520px; border-radius: 16px; border: 1px solid var(--line); object-fit: cover; }
  .klip { border-radius: 16px; overflow: hidden; background: #000; aspect-ratio: 16 / 9; border: 1px solid var(--line); }
  .klip video { width: 100%; height: 100%; display: block; object-fit: contain; background: #000; }
  .klip-alt { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; }
  .klip-alt b { font-size: 14.5px; }
  .kalan { margin-left: auto; display: inline-flex; align-items: center; gap: 5px; font-size: 12px; color: var(--dim); }
  .konu { display: flex; align-items: center; gap: 13px; padding: 10px; border-radius: 16px; border: 1px solid var(--line-2); }
  .konu-yazi { display: flex; flex-direction: column; gap: 3px; min-width: 0; flex: 1; }
  .konu-yazi b { font-size: 15px; }
  .puan { flex: none; min-width: 48px; height: 48px; border-radius: 12px; display: grid; place-items: center; font-size: 19px; font-weight: 800; color: #1a1408; background: linear-gradient(160deg, var(--gold-glow), var(--gold)); }
  footer { display: flex; gap: 40px; margin-left: -8px; }
  .eylem { display: inline-flex; align-items: center; gap: 4px; padding: 0; border: 0; background: none; color: var(--dim); font: inherit; font-size: 13px; cursor: pointer; }
  .hale { display: grid; place-items: center; width: 34px; height: 34px; border-radius: 50%; transition: background 0.12s; }
  .yorum-d:hover, .yorum-d.acik { color: #6cb6ff; }
  .yorum-d:hover .hale { background: rgba(108, 182, 255, 0.12); }
  .begen-d:hover, .begen-d.begendi { color: #ff6b81; }
  .begen-d:hover .hale { background: rgba(255, 107, 129, 0.12); }
  .begen-d.begendi :global(svg) { fill: currentColor; }
  .ic { display: grid; place-items: center; width: 32px; height: 32px; border-radius: 9px; border: 0; background: none; color: var(--dim); cursor: pointer; flex: none; }
  .ic:hover:not(:disabled) { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .ic.kucuk { width: 24px; height: 24px; opacity: 0; }
  .ic.sil { display: inline-flex; align-items: center; gap: 6px; }
  .ic.sil.onay, .ic.sil.onay:hover { width: auto; padding: 0 12px; background: rgba(229, 72, 77, 0.18); color: #ff8a8d; border-radius: 999px; font-size: 12.5px; font-weight: 700; }
  .yorum:hover .ic.kucuk { opacity: 1; }
  .yorumlar { display: flex; flex-direction: column; gap: 10px; padding-top: 4px; }
  .yorum { display: flex; align-items: flex-start; gap: 9px; }
  .yb { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; font-size: 14px; line-height: 1.45; }
  .kucuk-ad { font-size: 14px; }
  .yorum-metin { white-space: pre-wrap; overflow-wrap: anywhere; }
  .dahasi { align-self: flex-start; border: 0; background: none; padding: 0; color: #6cb6ff; font: inherit; font-size: 13.5px; cursor: pointer; }
  .dahasi:hover { text-decoration: underline; }
  .yaz { display: flex; gap: 8px; align-items: center; }
  .yaz input { flex: 1; min-width: 0; background: transparent; border: 0; border-bottom: 1px solid var(--line-2); padding: 8px 2px; color: var(--text); font: inherit; font-size: 14px; outline: none; }
  .yaz input:focus { border-color: var(--gold); }
  .yanitla { border-radius: 999px; padding: 6px 14px; font-size: 13px; }
  .anket { display: flex; flex-direction: column; gap: 6px; margin-top: 8px; }
  .sec { position: relative; overflow: hidden; display: flex; align-items: center; gap: 8px; padding: 9px 12px; border-radius: 10px; border: 1px solid var(--gold-dim); background: rgba(227, 194, 112, 0.04); color: var(--text); font: inherit; font-size: 14px; font-weight: 600; text-align: left; cursor: pointer; }
  .sec:hover:not(:disabled) { background: rgba(227, 194, 112, 0.12); }
  .sec:disabled { cursor: default; }
  .sonuc .sec { border-color: var(--line-2); background: rgba(255, 255, 255, 0.02); }
  .sec.benim { border-color: var(--gold); }
  .dolgu { position: absolute; inset: 0 auto 0 0; background: rgba(227, 194, 112, 0.18); border-radius: inherit; transition: width 0.4s ease; }
  .sec.benim .dolgu { background: rgba(227, 194, 112, 0.32); }
  .sec-ad { position: relative; flex: 1; min-width: 0; display: inline-flex; align-items: center; gap: 6px; overflow-wrap: anywhere; }
  .yuzde { position: relative; font-weight: 800; color: var(--gold-glow); }
  .anket-bilgi { font-size: 12.5px; color: var(--dim); }
  .ozet { display: flex; flex-direction: column; gap: 10px; margin-top: 6px; padding: 14px; border-radius: 16px; border: 1px solid rgba(227, 194, 112, 0.35); background: linear-gradient(135deg, rgba(227, 194, 112, 0.12), rgba(255, 255, 255, 0.02) 65%); }
  .ozet-bas { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; flex-wrap: wrap; }
  .ozet-kicker { font-family: var(--f-display); font-weight: 800; font-size: 16px; color: var(--gold-glow); }
  .ozet-sayilar { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; }
  .ozet-sayilar span { display: flex; flex-direction: column; align-items: center; gap: 2px; padding: 8px 4px; border-radius: 12px; background: rgba(0, 0, 0, 0.25); font-size: 12px; color: var(--muted); text-align: center; }
  .ozet-sayilar b { font-family: var(--f-display); font-size: 22px; color: var(--text); line-height: 1.1; }
  .ozet-satir { display: flex; gap: 8px; align-items: flex-start; font-size: 14px; color: var(--muted); }
  .ozet-satir :global(svg) { flex: none; margin-top: 2px; color: var(--gold); }
  .ozet-satir b { color: var(--text); }
  .ozet-satir > span { display: inline; line-height: 1.5; }
  .ozet-satir .sub { display: inline; }
  .ozet-satir.vurgu { color: var(--text); }
  .ozet .ozet-link { border: 0; background: none; padding: 0; font: inherit; color: var(--gold-glow); font-weight: 700; cursor: pointer; }
  .ozet .ozet-link:hover { text-decoration: underline; }
  .tk { margin-top: 6px; }
</style>
