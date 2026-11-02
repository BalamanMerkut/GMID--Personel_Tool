<script lang="ts">
  // Akıştaki bir paylaşım: kim, ne yaptı (puanladı / izledi / oynamaya başladı), konu kartı, beğeni ve yorumlar.
  import Icon from "../../Icon.svelte";
  import { notify } from "../../toast.svelte";
  import { istek, profilGoster, sosyal, tamAdres, type Olay } from "../../social.svelte";
  import Avatar from "./Avatar.svelte";
  import Cover from "./Cover.svelte";
  import { eylem, kalan, once, puanYaz, sure, TUR_AD } from "./metin";

  let { o, ondegisti, onsil }: { o: Olay; ondegisti: () => void; onsil: (o: Olay) => void } = $props();
  let taslak = $state("");
  let acik = $state(false);
  /** Silme iki tıkla: ilk tık "Silinsin mi?" olur, birkaç saniye içinde ikinci tık siler. */
  let silOnay = $state(false);
  let silZaman: ReturnType<typeof setTimeout> | undefined;

  const hata = (e: unknown) => notify(String(e instanceof Error ? e.message : e), "info");

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

<!-- Twitter düzeni: solda profil resmi, sağda ad · zaman, içerik, altta yorum ve beğeni. -->
<article class="tw">
  <button class="kim" onclick={() => profilGoster(o.kisi.id)} aria-label={o.kisi.ad}><Avatar kisi={o.kisi} boy={44} /></button>
  <div class="govde">
    <header>
      <button class="ad" onclick={() => profilGoster(o.kisi.id)}>{o.kisi.ad}</button>
      {#if eylem(o)}<span class="eylem-yazi">{eylem(o)}</span>{/if}
      <span class="nokta">·</span>
      <span class="zaman" title={new Date(o.zaman).toLocaleString("tr-TR")}>{once(o.zaman)}</span>
      {#if o.kisi.id === sosyal.ben?.id}
        <button class="ic sil" class:onay={silOnay} title={silOnay ? "Silmek için yeniden tıkla" : "Gönderiyi sil"} aria-label="Gönderiyi sil" onclick={sil}>
          <Icon name="trash" size={15} />{#if silOnay}<span>Silinsin mi?</span>{/if}
        </button>
      {/if}
    </header>

    {#if o.tur === "gonderi"}
      {#if o.veri.metin}<p class="metin">{o.veri.metin}</p>{/if}
      {#if o.veri.resim}<img class="medya" src={tamAdres(o.veri.resim)} alt="" loading="lazy" />{/if}
    {:else if o.tur === "klip"}
      <!-- Paylaşılan klip: 7 gün durur, sonra video da paylaşım da silinir (beğeniler profilde kalır). -->
      {#if o.veri.not}<p class="metin">{o.veri.not}</p>{/if}
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
      {#if o.veri.yorum}<p class="metin">{o.veri.yorum}</p>{/if}
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
      <button class="eylem yorum-d" class:acik onclick={() => (acik = !acik)} title="Yanıtla">
        <span class="hale"><Icon name="chat" size={17} /></span><span class="tnum">{o.yorumlar.length || ""}</span>
      </button>
      <button class="eylem begen-d" class:begendi={o.begendim} onclick={begen} aria-pressed={o.begendim} title="Beğen">
        <span class="hale"><Icon name="heart" size={17} /></span><span class="tnum">{o.begeni || ""}</span>
      </button>
    </footer>

    {#if acik || o.yorumlar.length}
      <div class="yorumlar">
        {#each acik ? o.yorumlar : o.yorumlar.slice(-2) as y (y.id)}
          <div class="yorum">
            <button class="kim" onclick={() => profilGoster(y.kisi.id)}><Avatar kisi={y.kisi} boy={28} nokta={false} /></button>
            <div class="yb">
              <span><button class="ad kucuk-ad" onclick={() => profilGoster(y.kisi.id)}>{y.kisi.ad}</button> <span class="zaman">· {once(y.zaman)}</span></span>
              <span class="yorum-metin">{y.metin}</span>
            </div>
            {#if y.kisi.id === sosyal.ben?.id || o.kisi.id === sosyal.ben?.id}
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
  header .sil { margin-left: auto; align-self: center; }
  .metin { margin: 0; font-size: 15px; line-height: 1.5; white-space: pre-wrap; overflow-wrap: anywhere; }
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
</style>
