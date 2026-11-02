<script lang="ts">
  // Dinleme odası çubuğu (her ekranda, alt ortada): bir odadaysan ve oda paneli kapalıysa odanın adı, çalan şarkı,
  // (sahip/DJ isen) oynat/duraklat, ses, "Odayı aç" ve "Ayrıl".
  import Icon from "../../Icon.svelte";
  import { ayril, birlikte, odalariYukle, oynatDuraklat, sesAyarla, yonetir } from "../../birlikte.svelte";
  import { sosyal } from "../../social.svelte";

  const oda = $derived(birlikte.odaId ? (birlikte.odalar[birlikte.odaId] ?? null) : null);

  // Giriş yapılınca açık odaları al; bağlantı her yeniden kurulduğunda da (kopukken kaçanlar için).
  $effect(() => {
    if (sosyal.anahtar && sosyal.bagli) void odalariYukle();
  });
</script>

{#if oda && !birlikte.panel}
  <div class="cubuk" role="status">
    {#if yonetir(oda) && oda.simdiki}
      <button class="oynat" title={oda.caliyor ? "Duraklat" : "Oynat"} onclick={oynatDuraklat}><Icon name={oda.caliyor ? "pause" : "play"} size={15} /></button>
    {:else}
      <span class="simge"><Icon name="headphones" size={16} /></span>
    {/if}
    <button class="yazi" title="Odayı aç" onclick={() => (birlikte.panel = true)}>
      <span class="ust">{oda.ad} · {oda.uyeler.length} kişi</span>
      <b title={oda.simdiki?.sarki.baslik}>
        {birlikte.hazirlaniyor ? "Hazırlanıyor…" : (oda.simdiki?.sarki.baslik ?? "Sıra boş, bir şarkı ekleyin")}
      </b>
    </button>
    {#if oda.simdiki && !oda.caliyor && !birlikte.hazirlaniyor}<span class="durak">Duraklatıldı</span>{/if}
    <input class="ses" type="range" min="0" max="1" step="0.05" value={birlikte.ses} oninput={(e) => sesAyarla(+e.currentTarget.value)} aria-label="Ses" />
    <button class="btn ghost sm" onclick={() => (birlikte.panel = true)}><Icon name="expand" size={14} />Oda</button>
    <button class="btn ghost sm" onclick={ayril}><Icon name="x" size={14} />Ayrıl</button>
  </div>
{/if}

<style>
  .cubuk { position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%); z-index: 60; display: flex; align-items: center; gap: 12px; padding: 8px 10px; border-radius: 999px; border: 1px solid rgba(227, 194, 112, 0.35); background: rgba(20, 22, 26, 0.94); backdrop-filter: blur(10px); box-shadow: 0 14px 40px rgba(0, 0, 0, 0.5); max-width: min(680px, calc(100vw - 32px)); }
  .yazi { display: flex; flex-direction: column; min-width: 0; line-height: 1.25; background: none; border: 0; color: inherit; text-align: left; cursor: pointer; padding: 0; }
  .ust { font-size: 11.5px; color: var(--gold); font-weight: 700; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 300px; }
  .yazi b { font-size: 13.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 300px; }
  .durak { font-size: 12px; color: var(--dim); }
  .ses { width: 90px; accent-color: var(--gold); }
  .sm { padding: 5px 11px; font-size: 12.5px; display: inline-flex; align-items: center; gap: 5px; border-radius: 999px; }
  .oynat, .simge { flex: none; width: 34px; height: 34px; border-radius: 50%; display: grid; place-items: center; }
  .oynat { border: 0; background: var(--gold); color: #1b1608; cursor: pointer; }
  .simge { background: rgba(227, 194, 112, 0.14); color: var(--gold); }
</style>
