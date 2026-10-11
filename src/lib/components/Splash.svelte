<script lang="ts">
  import { onMount } from "svelte";
  import logo from "../../assets/merkut-studios.jpg";

  /** `onleave`: kararma başladı (uygulama belirmeye başlar). `ondone`: animasyon bitti. */
  let { onleave, ondone }: { onleave: () => void; ondone: () => void } = $props();

  let leaving = $state(false);
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

  function finish() {
    if (leaving) return;
    leaving = true;
    onleave();
    setTimeout(ondone, 500);
  }

  onMount(() => {
    const timer = setTimeout(finish, reduced ? 600 : 2400);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" || e.key === " " || e.key === "Enter") {
        e.preventDefault();
        finish();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("keydown", onKey);
    };
  });
</script>

<button class="splash" class:leaving onclick={finish} aria-label="Açılışı atla">
  <img src={logo} alt="Merkut Studios" class:still={reduced} />
  <span class="hint">Atlamak için tıkla</span>
</button>

<style>
  .splash {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    place-items: center;
    border: 0;
    padding: 0;
    background: #000;
    cursor: pointer;
    transition: opacity 0.5s ease;
  }
  .splash.leaving { opacity: 0; pointer-events: none; }
  img {
    width: min(44vmin, 340px);
    height: auto;
    opacity: 0;
    animation: logo-in 2.4s cubic-bezier(0.4, 0, 0.2, 1) forwards;
  }
  img.still { animation: none; opacity: 1; }
  .hint {
    position: absolute;
    bottom: 24px;
    color: #5b6166;
    font-size: 12px;
    letter-spacing: 0.06em;
  }
  /* Karanlıktan belir → hafif parıltı → kararmaya başla */
  @keyframes logo-in {
    0% { opacity: 0; transform: scale(0.96); filter: brightness(1); }
    12% { opacity: 0; transform: scale(0.96); }
    48% { opacity: 1; transform: scale(1); filter: brightness(1); }
    62% { filter: brightness(1.22); }
    80% { opacity: 1; transform: scale(1); filter: brightness(1); }
    100% { opacity: 0.85; transform: scale(1.005); }
  }
</style>
