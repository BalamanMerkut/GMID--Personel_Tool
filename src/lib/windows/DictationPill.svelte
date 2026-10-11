<script lang="ts">
  // Sesle yazma göstergesi (ekranın alt ortası, tıklamaları geçirir): yazı yok, sadece durum.
  //  • dinliyor: kırmızı nokta, sesle oynayan dalga, süre
  //  • yazıya döküyor: üç altın nokta
  //  • hata: kısa bir uyarı işareti (ayrıntı sağ üstteki göstergede)
  import { onMount } from "svelte";
  import { call } from "../api";
  import { on } from "../bus";

  type View = { state: "listening" | "processing" | "error" | "hidden"; secs: number };
  const BARS = 9;

  let view: View = $state({ state: "hidden", secs: 0 });
  let level = $state(0);
  let bars: number[] = $state(Array(BARS).fill(0.08));

  // Güvenlik ağı: dinlerken her saniye, diğer durumlarda bir kez haber gelir. Haber kesilirse (uygulama
  // gizlemeyi kaçırırsa) gösterge kendini boşaltır; pencere zaten tıklamaları geçirir.
  let lastSeen = Date.now();
  const STALE: Record<View["state"], number> = { listening: 3500, processing: 125_000, error: 2500, hidden: Infinity };

  onMount(() => {
    const set = (v: View) => {
      view = v;
      lastSeen = Date.now();
    };
    void call<View>("dictation_view_state").then(set);
    const watchdog = setInterval(() => {
      if (view.state !== "hidden" && Date.now() - lastSeen > STALE[view.state]) view = { state: "hidden", secs: 0 };
    }, 500);
    const stops = [
      on<View>("dikte", set),
      on<number>("dikte-level", (l) => (level = l)),
    ];
    // Dalga: yeni seviye ortadan girer, kenarlara doğru söner.
    const timer = setInterval(() => {
      const target = view.state === "listening" ? Math.min(1, Math.pow(level, 0.6) * 1.4) : 0.08;
      const next = [...bars];
      for (let i = 0; i < BARS; i++) {
        const shape = 1 - Math.abs(i - (BARS - 1) / 2) / ((BARS - 1) / 2) * 0.55;
        const jitter = 0.75 + Math.random() * 0.5;
        const want = Math.max(0.08, target * shape * jitter);
        next[i] = next[i] + (want - next[i]) * 0.45;
      }
      bars = next;
    }, 60);
    return () => {
      clearInterval(timer);
      clearInterval(watchdog);
      stops.forEach((s) => void s.then((f) => f()));
    };
  });

  const time = $derived(`${Math.floor(view.secs / 60)}:${String(view.secs % 60).padStart(2, "0")}`);
</script>

{#if view.state !== "hidden"}
  <div class="pill" class:proc={view.state === "processing"} class:err={view.state === "error"}>
    {#if view.state === "listening"}
      <span class="rec"></span>
      <span class="wave" aria-hidden="true">
        {#each bars as b, i (i)}<i style:height="{Math.round(b * 17) + 3}px"></i>{/each}
      </span>
      <span class="t">{time}</span>
    {:else if view.state === "processing"}
      <span class="dots" aria-label="Yazıya dökülüyor"><i></i><i></i><i></i></span>
    {:else}
      <span class="bang">!</span>
    {/if}
  </div>
{/if}

<style>
  :global(html), :global(body) { background: transparent !important; overflow: hidden; }
  .pill {
    position: fixed; left: 50%; top: 50%; transform: translate(-50%, -50%);
    /* İnce ama okunur: 34 px boy, ince altın çizgi. */
    height: 34px; min-width: 52px; padding: 0 12px; border-radius: 999px;
    display: flex; align-items: center; justify-content: center; gap: 9px;
    background: rgba(12, 14, 17, 0.92); border: 1px solid rgba(227, 194, 112, 0.38);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.4), 0 0 12px rgba(227, 194, 112, 0.08);
    animation: pop 0.16s ease-out;
  }
  .pill.err { border-color: rgba(224, 106, 106, 0.8); }
  .rec { width: 7px; height: 7px; border-radius: 50%; background: #e06a6a; box-shadow: 0 0 10px rgba(224, 106, 106, 0.9); animation: blink 1.2s ease-in-out infinite; }
  .wave { display: flex; align-items: center; gap: 2px; height: 22px; }
  .wave i { display: block; width: 3px; border-radius: 4px; background: linear-gradient(var(--gold-glow), var(--gold)); transition: height 0.06s linear; }
  .t { font-size: 11.5px; font-weight: 700; color: var(--muted); font-variant-numeric: tabular-nums; min-width: 28px; }
  .dots { display: flex; gap: 5px; }
  .dots i { width: 6px; height: 6px; border-radius: 50%; background: var(--gold); animation: bounce 0.9s ease-in-out infinite; }
  .dots i:nth-child(2) { animation-delay: 0.15s; }
  .dots i:nth-child(3) { animation-delay: 0.3s; }
  .bang { width: 16px; height: 16px; font-size: 11px; border-radius: 50%; display: grid; place-items: center; background: #e06a6a; color: #fff; font-weight: 800; }
  @keyframes pop { from { opacity: 0; transform: translate(-50%, -40%) scale(0.92); } }
  @keyframes blink { 50% { opacity: 0.35; } }
  @keyframes bounce { 0%, 100% { transform: translateY(0); opacity: 0.5; } 50% { transform: translateY(-4px); opacity: 1; } }
</style>
