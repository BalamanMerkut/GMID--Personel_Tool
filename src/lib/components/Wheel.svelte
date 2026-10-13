<script lang="ts">
  // Çark: SVG dilimler, tepede ibre. `spin(hedef, süre)` çarkı hedef dilimde durdurur; her dilim geçişinde
  // `ontick` çağrılır (tık sesi), ibre hafifçe sekiyor. Ortadaki göbeğe tıklamak `onhub`'ı çağırır.
  // Bir dilim çıkarılınca (eleme) bir anda kaybolmaz: daralır, komşuları genişler.
  import { untrack } from "svelte";
  import Icon from "../Icon.svelte";
  import { easeOut, fit, inkFor, landing, randFloat, sliceAt, sliceColor, slicePath } from "../fun";

  /** `color` verilirse dilim o renkte (elemede renkler yerinde kalsın diye). */
  type Item = { id: string; name: string; color?: string };
  type Shown = Item & { w: number };
  let {
    items,
    busy = false,
    ontick,
    onhub,
  }: { items: Item[]; busy?: boolean; ontick?: () => void; onhub?: () => void } = $props();

  const C = 200;
  const R = 192;
  let rotation = $state(0);
  let kick = $state(0);

  // Ekrandaki dilimler; `w` dilimin payı (çıkarılan dilim 1 → 0 daralır).
  let shown = $state<Shown[]>([]);
  let settled: Promise<void> = Promise.resolve();

  $effect(() => {
    const next = items.map((it, i) => ({ ...it, color: it.color ?? sliceColor(i, items.length), w: 1 }));
    untrack(() => {
      const prev = shown.filter((s) => s.w > 0);
      const ids = new Set(next.map((n) => n.id));
      const kept = prev.filter((p) => ids.has(p.id));
      const removalOnly =
        prev.length > next.length && kept.length === next.length && kept.every((p, i) => p.id === next[i].id);
      if (!removalOnly || matchMedia("(prefers-reduced-motion: reduce)").matches) {
        shown = next;
        return;
      }
      const gone = new Set(prev.filter((p) => !ids.has(p.id)).map((p) => p.id));
      const t0 = performance.now();
      settled = new Promise((resolve) => {
        const frame = (now: number) => {
          const t = Math.min(1, (now - t0) / 450);
          const w = 1 - (t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2);
          shown = prev.map((p) => (gone.has(p.id) ? { ...p, w } : p));
          if (t < 1) requestAnimationFrame(frame);
          else {
            shown = next;
            resolve();
          }
        };
        requestAnimationFrame(frame);
      });
    });
  });

  const arcs = $derived.by(() => {
    const total = shown.reduce((a, s) => a + s.w, 0) || 1;
    let acc = 0;
    return shown.map((s) => {
      const from = (acc / total) * 360;
      acc += s.w;
      return { ...s, from, to: (acc / total) * 360 };
    });
  });
  const font = $derived(items.length <= 6 ? 19 : items.length <= 10 ? 16 : items.length <= 18 ? 13 : items.length <= 30 ? 10.5 : 8.5);
  const maxChars = $derived(Math.max(4, Math.floor((R - 52) / (font * 0.58))));

  /** Çarkı çevirir; hedef dilimde durunca biter. Hareket azaltma açıksa kısa sürer. */
  export async function spin(target: number, ms: number): Promise<void> {
    await settled;
    const n = items.length;
    if (n === 0) return;
    const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const duration = reduce ? Math.min(ms, 900) : ms;
    const start = rotation;
    const end = landing(start, n, target, reduce ? 1 : Math.max(3, Math.round(ms / 900)), randFloat());
    let last = sliceAt(start, n);
    const t0 = performance.now();
    return new Promise((resolve) => {
      const frame = (now: number) => {
        const t = Math.min(1, (now - t0) / duration);
        rotation = start + (end - start) * easeOut(t);
        const at = sliceAt(rotation, n);
        if (at !== last) {
          last = at;
          kick++;
          ontick?.();
        }
        if (t < 1) requestAnimationFrame(frame);
        else {
          rotation = end % 360;
          resolve();
        }
      };
      requestAnimationFrame(frame);
    });
  }
</script>

<div class="wheel">
  {#key kick}<span class="pointer" class:kicked={kick > 0} aria-hidden="true"></span>{/key}
  <svg viewBox="0 0 400 400" style:transform="rotate({rotation}deg)" role="img" aria-label="Çark: {items.length} dilim">
    <circle cx={C} cy={C} r={R + 4} class="rim" />
    {#if arcs.length === 0}
      <circle cx={C} cy={C} r={R} class="empty" />
    {:else}
      {#each arcs as a (a.id)}
        <path d={slicePath(C, R, a.from, a.to)} fill={a.color} />
        {#if arcs.length > 1}
          <g transform="rotate({(a.from + a.to) / 2} {C} {C})" opacity={a.w}>
            <text
              transform="translate({C} {C}) rotate(-90)"
              x={R - 16}
              y="0"
              text-anchor="end"
              dominant-baseline="central"
              fill={inkFor(a.color!)}
              font-size={font}>{fit(a.name, maxChars)}</text
            >
          </g>
        {/if}
      {/each}
      {#if arcs.length > 1}
        {#each arcs as a (a.id)}
          {@const r = (a.from * Math.PI) / 180}
          <line x1={C} y1={C} x2={C + R * Math.sin(r)} y2={C - R * Math.cos(r)} class="sep" />
        {/each}
      {/if}
    {/if}
  </svg>
  {#if arcs.length === 1}
    <!-- Tek dilim: ad çarkla dönmesin, düz okunsun. -->
    <span class="solo" style:color={inkFor(arcs[0].color!)}>{fit(arcs[0].name, 28)}</span>
  {/if}
  <button class="hub" disabled={busy || items.length === 0} onclick={() => onhub?.()} title="Çevir (Boşluk)">
    <Icon name="wheel" size={26} />
  </button>
</div>

<style>
  .wheel { position: relative; width: 100%; aspect-ratio: 1; max-width: 560px; margin: 0 auto; }
  svg { width: 100%; height: 100%; display: block; filter: drop-shadow(0 14px 30px rgba(0, 0, 0, 0.45)); will-change: transform; }
  .rim { fill: #15171a; stroke: rgba(227, 194, 112, 0.7); stroke-width: 3; }
  .empty { fill: var(--coal-2); }
  .sep { stroke: rgba(18, 20, 23, 0.55); stroke-width: 2; }
  text { font-family: var(--f-display); font-weight: 700; pointer-events: none; }
  .pointer {
    position: absolute; left: 50%; top: -6px; z-index: 2; width: 0; height: 0; transform: translateX(-50%);
    border-left: 16px solid transparent; border-right: 16px solid transparent; border-top: 30px solid var(--gold);
    filter: drop-shadow(0 3px 4px rgba(0, 0, 0, 0.6)); transform-origin: 50% 0;
  }
  .pointer.kicked { animation: kick 0.12s ease-out; }
  @keyframes kick { 40% { transform: translateX(-50%) rotate(-14deg); } }
  .solo {
    position: absolute; left: 50%; top: 28%; transform: translate(-50%, -50%); max-width: 70%; text-align: center;
    font-family: var(--f-display); font-weight: 800; font-size: clamp(18px, 3.4vw, 30px); pointer-events: none;
  }
  .hub {
    position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%);
    width: 17%; aspect-ratio: 1; border-radius: 50%; display: grid; place-items: center;
    background: radial-gradient(circle at 35% 30%, #2b2f35, #121417); color: var(--gold);
    border: 3px solid var(--gold); box-shadow: 0 4px 14px rgba(0, 0, 0, 0.5);
    transition: transform 0.15s, color 0.15s;
  }
  .hub:hover:not(:disabled) { transform: translate(-50%, -50%) scale(1.06); color: var(--gold-glow); }
  .hub:disabled { cursor: default; opacity: 0.85; }
</style>
