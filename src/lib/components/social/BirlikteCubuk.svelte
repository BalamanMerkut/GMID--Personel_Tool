<script lang="ts">
  // Birlikte dinleme çubuğu (her ekranda, alt ortada): dinliyorsan kiminle ve ne çaldığı, ses ve "Ayrıl";
  // yayındaysan kaç kişinin dinlediği ve "Bitir".
  import Icon from "../../Icon.svelte";
  import { ayril, birlikte, odalariYukle, sesAyarla, yayinBitir } from "../../birlikte.svelte";
  import { sosyal } from "../../social.svelte";
  import Avatar from "./Avatar.svelte";

  const oda = $derived(birlikte.dinlenen ? birlikte.odalar[birlikte.dinlenen] : null);
  const benimOdam = $derived(sosyal.ben ? birlikte.odalar[sosyal.ben.id] : null);

  // Giriş yapılınca açık odaları bir kez al (sonrası canlı gelir).
  let yuklendi = false;
  $effect(() => {
    if (sosyal.anahtar && sosyal.bagli && !yuklendi) {
      yuklendi = true;
      void odalariYukle();
    }
  });
</script>

{#if oda}
  <div class="cubuk" role="status">
    <Avatar kisi={oda.sahip} boy={30} nokta={false} />
    <span class="yazi">
      <span class="ust"><Icon name="headphones" size={13} />{oda.sahip.ad} ile dinliyorsun</span>
      <b title={oda.sarki.baslik}>{birlikte.hazirlaniyor ? "Şarkı hazırlanıyor…" : oda.sarki.baslik}</b>
    </span>
    {#if !oda.caliyor && !birlikte.hazirlaniyor}<span class="durak">Duraklatıldı</span>{/if}
    <input class="ses" type="range" min="0" max="1" step="0.05" value={birlikte.ses} oninput={(e) => sesAyarla(+e.currentTarget.value)} aria-label="Ses" />
    <button class="btn ghost sm" onclick={ayril}><Icon name="x" size={14} />Ayrıl</button>
  </div>
{:else if birlikte.yayin}
  <div class="cubuk yayin" role="status">
    <span class="canli"></span>
    <span class="yazi">
      <span class="ust"><Icon name="headphones" size={13} />Yayındasın</span>
      <b>{benimOdam?.dinleyiciler.length ? `${benimOdam.dinleyiciler.length} arkadaşın seninle dinliyor` : "Arkadaşların katılabilir"}</b>
    </span>
    {#if benimOdam?.dinleyiciler.length}
      <span class="yuzler">{#each benimOdam.dinleyiciler.slice(0, 5) as k (k.id)}<span class="yuz" title={k.ad}><Avatar kisi={k} boy={24} nokta={false} /></span>{/each}</span>
    {/if}
    <button class="btn ghost sm" onclick={yayinBitir}>Bitir</button>
  </div>
{/if}

<style>
  .cubuk { position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%); z-index: 60; display: flex; align-items: center; gap: 12px; padding: 8px 10px 8px 10px; border-radius: 999px; border: 1px solid rgba(227, 194, 112, 0.35); background: rgba(20, 22, 26, 0.94); backdrop-filter: blur(10px); box-shadow: 0 14px 40px rgba(0, 0, 0, 0.5); max-width: min(640px, calc(100vw - 32px)); }
  .yazi { display: flex; flex-direction: column; min-width: 0; line-height: 1.25; }
  .ust { display: inline-flex; align-items: center; gap: 5px; font-size: 11.5px; color: var(--gold); font-weight: 700; }
  .yazi b { font-size: 13.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 300px; }
  .durak { font-size: 12px; color: var(--dim); }
  .ses { width: 90px; accent-color: var(--gold); }
  .sm { padding: 5px 11px; font-size: 12.5px; display: inline-flex; align-items: center; gap: 5px; border-radius: 999px; }
  .canli { width: 10px; height: 10px; border-radius: 50%; background: var(--good); box-shadow: 0 0 0 4px rgba(80, 200, 120, 0.2); margin-left: 6px; animation: nabiz 1.6s ease-in-out infinite; }
  @keyframes nabiz { 50% { box-shadow: 0 0 0 7px rgba(80, 200, 120, 0.05); } }
  .yuzler { display: inline-flex; }
  .yuz { margin-right: -6px; border-radius: 50%; box-shadow: 0 0 0 2px #14161a; }
</style>
