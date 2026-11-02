<script lang="ts">
  // Kişinin Discord resmi (yoksa baş harfleri) ve çevrim içi noktası (yeşil: çevrim içi, mor: oyunda).
  import gmidLogo from "../../../assets/gmid-logo.webp";
  import { sosyal, type Kisi } from "../../social.svelte";
  let { kisi, boy = 40, nokta = true }: { kisi: Kisi; boy?: number; nokta?: boolean } = $props();
  const bas = $derived(kisi.ad.split(/\s+/).map((p) => p[0] ?? "").join("").slice(0, 2).toUpperCase());
  const c = $derived(sosyal.cevrimici[kisi.id]);
  let kirik = $state(false);
</script>

<span class="av" style:width="{boy}px" style:height="{boy}px">
  {#if kisi.id === "gmid"}<img src={gmidLogo} alt="GMID" />{:else if kisi.avatar && !kirik}<img src={kisi.avatar} alt="" loading="lazy" onerror={() => (kirik = true)} />{:else}<span class="bas" style:font-size="{boy * 0.36}px">{bas}</span>{/if}
  {#if nokta && c}<i class:oyunda={!!c.oyun} title={c.oyun ? `${c.oyun} oynuyor` : "Çevrim içi"}></i>{/if}
</span>

<style>
  .av { position: relative; display: inline-block; flex: none; border-radius: 50%; vertical-align: middle; }
  img, .bas { width: 100%; height: 100%; border-radius: 50%; object-fit: cover; display: grid; place-items: center; }
  .bas { background: linear-gradient(135deg, #3a3f4a, #262a31); color: var(--gold); font-weight: 800; }
  i { position: absolute; right: 0; bottom: 0; width: 28%; height: 28%; min-width: 9px; min-height: 9px; border-radius: 50%; background: var(--good); box-shadow: 0 0 0 2.5px #1b1d22; }
  i.oyunda { background: #8b7cf6; }
</style>
