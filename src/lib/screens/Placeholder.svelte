<script lang="ts">
  import Icon from "../Icon.svelte";
  import type { Screen } from "../nav";

  let { screen, onnavigate }: { screen: Screen; onnavigate: (id: string) => void } = $props();
</script>

<section class="soon">
  <div class="orb"><Icon name={screen.icon} size={34} /></div>
  <span class="chip">Faz {screen.phase}'te geliyor</span>
  <h2>{screen.label}</h2>
  <p>{screen.description}</p>
  <button class="btn ghost" onclick={() => onnavigate("home")}><Icon name="castle" />Giriş Salonu'na dön</button>
</section>

<style>
  .soon { display: grid; justify-items: center; text-align: center; gap: 14px; padding: 8vh 16px; }
  .orb {
    width: 84px; height: 84px; border-radius: 50%; display: grid; place-items: center;
    border: 1px solid rgba(227, 194, 112, 0.4); color: var(--gold);
    box-shadow: 0 0 34px rgba(227, 194, 112, 0.16);
    background: rgba(227, 194, 112, 0.05);
  }
  .chip { color: var(--gold); border-color: rgba(227, 194, 112, 0.35); }
  h2 { font-family: var(--f-display); font-size: 26px; margin: 0; text-wrap: balance; }
  p { max-width: 56ch; color: var(--muted); margin: 0 0 6px; }
</style>
