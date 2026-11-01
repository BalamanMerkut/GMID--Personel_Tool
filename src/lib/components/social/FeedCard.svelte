<script lang="ts">
  // Akıştaki bir paylaşım: kim, ne yaptı (puanladı / izledi / oynamaya başladı), konu kartı, beğeni ve yorumlar.
  import Icon from "../../Icon.svelte";
  import { notify } from "../../toast.svelte";
  import { istek, profilGoster, sosyal, type Olay } from "../../social.svelte";
  import Avatar from "./Avatar.svelte";
  import Cover from "./Cover.svelte";
  import { eylem, once, puanYaz, TUR_AD } from "./metin";

  let { o, ondegisti, onsil }: { o: Olay; ondegisti: () => void; onsil: (o: Olay) => void } = $props();
  let taslak = $state("");
  let acik = $state(false);

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

  async function yorumSil(id: number) {
    await istek("DELETE", `/api/yorum/${id}`).catch(hata);
    ondegisti();
  }
</script>

<article class="olay">
  <header>
    <button class="kim" onclick={() => profilGoster(o.kisi.id)}><Avatar kisi={o.kisi} boy={40} /></button>
    <div class="baslik">
      <button class="ad" onclick={() => profilGoster(o.kisi.id)}>{o.kisi.ad}</button>
      <span class="sub">{eylem(o)}</span>
      <span class="zaman" title={new Date(o.zaman).toLocaleString("tr-TR")}>{once(o.zaman)}</span>
    </div>
    {#if o.kisi.id === sosyal.ben?.id}
      <button class="ic" title="Paylaşımı kaldır" aria-label="Paylaşımı kaldır" onclick={() => onsil(o)}><Icon name="trash" size={15} /></button>
    {/if}
  </header>
  <div class="konu">
    <Cover url={o.veri.kapak} ad={o.veri.ad} en="58px" kose={8} />
    <div class="konu-yazi">
      <b>{o.veri.ad}</b>
      <span class="sub small">{o.tur === "oyun" ? "oyun" : TUR_AD[o.veri.tur ?? "game"]}{o.veri.yil ? ` · ${o.veri.yil}` : ""}</span>
      {#if o.veri.yorum}<q>{o.veri.yorum}</q>{/if}
    </div>
    {#if puanYaz(o.veri.puan)}<span class="puan tnum">{puanYaz(o.veri.puan)}</span>{/if}
  </div>
  <footer>
    <button class="eylem" class:begendi={o.begendim} onclick={begen} aria-pressed={o.begendim} title="Beğen">
      <Icon name="heart" size={16} /><span class="tnum">{o.begeni || ""}</span>
    </button>
    <button class="eylem" onclick={() => (acik = !acik)} title="Yorum yap">
      <Icon name="chat" size={16} /><span class="tnum">{o.yorumlar.length || ""}</span>
    </button>
  </footer>
  {#if acik || o.yorumlar.length}
    <div class="yorumlar">
      {#each acik ? o.yorumlar : o.yorumlar.slice(-2) as y (y.id)}
        <div class="yorum">
          <button class="kim" onclick={() => profilGoster(y.kisi.id)}><Avatar kisi={y.kisi} boy={26} nokta={false} /></button>
          <div class="yb"><b>{y.kisi.ad}</b> {y.metin} <span class="zaman">{once(y.zaman)}</span></div>
          {#if y.kisi.id === sosyal.ben?.id || o.kisi.id === sosyal.ben?.id}
            <button class="ic kucuk" title="Yorumu sil" aria-label="Yorumu sil" onclick={() => yorumSil(y.id)}><Icon name="x" size={13} /></button>
          {/if}
        </div>
      {/each}
      {#if !acik && o.yorumlar.length > 2}
        <button class="dahasi" onclick={() => (acik = true)}>Tüm yorumlar ({o.yorumlar.length})</button>
      {/if}
      <form class="yaz" onsubmit={(e) => (e.preventDefault(), yorumYap())}>
        <input bind:value={taslak} placeholder="Yorum yaz…" maxlength="1000" aria-label="Yorum" />
        <button class="ic" disabled={!taslak.trim()} aria-label="Gönder"><Icon name="send" size={16} /></button>
      </form>
    </div>
  {/if}
</article>

<style>
  .olay { display: flex; flex-direction: column; gap: 10px; padding: 14px 16px; border-radius: 16px; border: 1px solid var(--line); background: linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0.015)); }
  header { display: flex; align-items: center; gap: 11px; }
  .kim { border: 0; background: none; padding: 0; cursor: pointer; border-radius: 50%; }
  .baslik { display: flex; flex-wrap: wrap; align-items: baseline; gap: 4px 7px; flex: 1; min-width: 0; }
  .ad { border: 0; background: none; padding: 0; font: inherit; font-weight: 700; color: var(--text); cursor: pointer; }
  .ad:hover { color: var(--gold-glow); }
  .zaman { font-size: 12px; color: var(--dim); }
  .konu { display: flex; align-items: center; gap: 13px; padding: 10px; border-radius: 12px; background: rgba(0, 0, 0, 0.18); }
  .konu-yazi { display: flex; flex-direction: column; gap: 3px; min-width: 0; flex: 1; }
  .konu-yazi b { font-size: 15px; }
  q { color: var(--muted); font-size: 13.5px; font-style: italic; }
  q::before { content: "“"; } q::after { content: "”"; }
  .puan { flex: none; min-width: 48px; height: 48px; border-radius: 12px; display: grid; place-items: center; font-size: 19px; font-weight: 800; color: #1a1408; background: linear-gradient(160deg, var(--gold-glow), var(--gold)); }
  footer { display: flex; gap: 6px; }
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
  .yaz input { flex: 1; min-width: 0; background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 999px; padding: 8px 14px; color: var(--text); font: inherit; font-size: 13.5px; outline: none; }
  .yaz input:focus { border-color: rgba(227, 194, 112, 0.5); }
</style>
