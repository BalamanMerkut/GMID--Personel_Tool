<script lang="ts">
  // Paylaşılan kapak/afiş (sunucudaki ya da Steam'deki adres); açılamazsa baş harfler.
  import { tamAdres } from "../../social.svelte";
  let {
    url,
    ad,
    en = "100%",
    oran = "2 / 3",
    kose = 10,
  }: { url: string | null | undefined; ad: string; en?: string; oran?: string; kose?: number } = $props();
  const tam = $derived(tamAdres(url));
  let kirik = $state(false);
  $effect(() => {
    void tam;
    kirik = false;
  });
  const bas = $derived(ad.split(/\s+/).map((p) => p[0] ?? "").join("").slice(0, 2).toUpperCase());
</script>

{#if tam && !kirik}
  <img class="kapak" src={tam} alt="" loading="lazy" onerror={() => (kirik = true)} style:width={en} style:aspect-ratio={oran} style:border-radius="{kose}px" />
{:else}
  <span class="kapak yok" style:width={en} style:aspect-ratio={oran} style:border-radius="{kose}px">{bas}</span>
{/if}

<style>
  .kapak { display: block; flex: none; object-fit: cover; background: #1d2026; }
  .yok { display: grid; place-items: center; background: linear-gradient(135deg, #2c3038, #1d2026); color: var(--dim); font-weight: 800; }
</style>
