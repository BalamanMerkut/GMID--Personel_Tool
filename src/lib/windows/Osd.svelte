<script lang="ts">
  // Sağ alttan kayarak gelen bildirim kartı (MSN bildirimi gibi). Odak almaz, tıklamalar alttaki pencereye geçer.
  // Pencereyi Rust gösterir/gizler; burası çizer ve süre dolmadan hemen önce kartı dışarı kaydırır.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../icons";
  import type { OsdMessage } from "../types";

  /** Simgeye göre başlık ve vurgu rengi (başlık verilmediyse). */
  const KINDS: Record<string, { label: string; tone: string }> = {
    mic: { label: "Sesle yazma", tone: "gold" },
    micoff: { label: "Sesle yazma", tone: "dim" },
    mute: { label: "Mikrofon", tone: "red" },
    lock: { label: "Mikrofon", tone: "gold" },
    trophy: { label: "Futbol", tone: "green" },
    image: { label: "Ekran görüntüsü", tone: "teal" },
    speaker: { label: "Ses", tone: "teal" },
    headphones: { label: "Ses", tone: "teal" },
    monitor: { label: "Ses", tone: "teal" },
    volumelow: { label: "Ses", tone: "teal" },
    download: { label: "Video İndirici", tone: "teal" },
    clip: { label: "Klipler", tone: "teal" },
    bolt: { label: "Oyun Modu", tone: "gold" },
    book: { label: "Sözlük", tone: "plum" },
    library: { label: "Oyunlar", tone: "gold" },
    play: { label: "Ses Paneli", tone: "gold" },
    install: { label: "Kurulum", tone: "green" },
    power: { label: "Güç", tone: "red" },
    info: { label: "GMID", tone: "gold" },
  };

  let msg = $state<OsdMessage | null>(null);
  let leaving = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function show(m: OsdMessage) {
    msg = m;
    leaving = false;
    clearTimeout(timer);
    // Pencere `ms` sonra gizlenir; kart ondan biraz önce sağa kayarak çıkar.
    timer = setTimeout(() => (leaving = true), Math.max(600, (m.ms || 3600) - 260));
  }

  onMount(() => {
    void api.overlay.takeOsd().then((m) => m && show(m));
    const stop = on<OsdMessage>("osd", show);
    return () => {
      clearTimeout(timer);
      void stop.then((f) => f());
    };
  });

  /** Başlık yoksa: "Ses çıkışı: Kulaklık" → başlık "Ses çıkışı"; değilse simgenin adı. */
  const parts = $derived.by(() => {
    if (!msg) return { title: "", body: "" };
    if (msg.title) return { title: msg.title, body: msg.text };
    const i = msg.text.indexOf(": ");
    if (i > 0 && i <= 24) return { title: msg.text.slice(0, i), body: msg.text.slice(i + 2) };
    return { title: KINDS[msg.icon]?.label ?? "GMID", body: msg.text };
  });
  const tone = $derived(msg ? (KINDS[msg.icon]?.tone ?? (msg.icon === "info" ? "gold" : "teal")) : "teal");
</script>

{#if msg}
  <!-- Aynı türden art arda gelen mesajlar (ses seviyesi, mikrofon) yeniden kaymasın; yalnız yazı değişir. -->
  {#key msg.icon}
    <div class="toast {tone}" class:leaving role="status">
      <span class="ic"><Icon name={msg.icon as IconName} size={20} /></span>
      <div class="txt">
        <div class="top">
          <b>{parts.title}</b>
          <span class="app">GMID</span>
        </div>
        {#if parts.body}<p>{parts.body}</p>{/if}
      </div>
      {#key msg.id}<i class="life" style:animation-duration="{msg.ms || 3600}ms"></i>{/key}
    </div>
  {/key}
{/if}

<style>
  :global(html), :global(body) { background: transparent !important; overflow: hidden; }
  .toast {
    --a: var(--teal);
    position: fixed; right: 14px; bottom: 14px; width: 344px; min-height: 72px;
    display: flex; align-items: center; gap: 13px; padding: 13px 16px 15px 13px; overflow: hidden;
    border-radius: 14px; background: linear-gradient(180deg, rgba(30, 33, 38, 0.97), rgba(21, 23, 27, 0.97));
    border: 1px solid rgba(255, 255, 255, 0.09); color: var(--text);
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.5), 0 2px 6px rgba(0, 0, 0, 0.35);
    animation: enter 0.34s cubic-bezier(0.2, 0.9, 0.25, 1.05) both;
  }
  .toast.leaving { animation: leave 0.26s ease-in both; }
  .gold { --a: var(--gold); }
  .green { --a: var(--good); }
  .teal { --a: var(--neon); }
  .plum { --a: #b48be0; }
  .red { --a: var(--bad); }
  .dim { --a: var(--muted); }
  .ic {
    width: 40px; height: 40px; flex: none; border-radius: 11px; display: grid; place-items: center;
    color: var(--a); background: color-mix(in srgb, var(--a) 16%, transparent);
    border: 1px solid color-mix(in srgb, var(--a) 28%, transparent);
  }
  .txt { min-width: 0; flex: 1; display: flex; flex-direction: column; gap: 2px; }
  .top { display: flex; align-items: baseline; gap: 8px; }
  .top b { font-family: var(--f-display); font-size: 14px; font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .app { margin-left: auto; font-size: 10px; font-weight: 700; letter-spacing: 0.12em; color: var(--dim); }
  p { margin: 0; font-size: 13px; line-height: 1.35; color: var(--muted); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  /* Kalan süre: altta incecik, vurgu renginde azalan çizgi. */
  .life { position: absolute; left: 0; bottom: 0; height: 2px; width: 100%; background: var(--a); opacity: 0.55; transform-origin: left; animation: life linear forwards; }
  @keyframes enter { from { opacity: 0; transform: translateX(40px) scale(0.98); } }
  @keyframes leave { to { opacity: 0; transform: translateX(46px); } }
  @keyframes life { to { transform: scaleX(0); } }
  @media (prefers-reduced-motion: reduce) {
    .toast, .toast.leaving { animation: none; }
  }
</style>
