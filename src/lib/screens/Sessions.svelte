<script lang="ts">
  // Oturum Geçmişi: hangi gün hangi oyunu ne kadar oynadın. Günlük sütunlar en çok oynanan 5 oyuna
  // göre bölünür (gerisi "Diğer"); sütunun üstüne gelince ayrıntı.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import { playtime } from "../time";
  import type { Game, PlaySession } from "../types";

  let { onnavigate }: { onnavigate: (id: string) => void } = $props();

  type Range = 7 | 30;
  let range: Range = $state(7);
  let games: Game[] = $state([]);
  let sessions: PlaySession[] = $state([]);
  let loaded = $state(false);
  let hover: number | null = $state(null);

  /** Doğrulanmış kategorik sıra (koyu yüzey): mavi, turuncu, su yeşili, sarı, pembe; "Diğer" gri. */
  const SERIES = ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181"];
  const OTHER = "#6b7078";

  const dayStart = (t: number) => {
    const d = new Date(t);
    d.setHours(0, 0, 0, 0);
    return d.getTime();
  };

  async function load() {
    const since = dayStart(Date.now()) - 29 * 86_400_000;
    [games, sessions] = await Promise.all([
      api.games.list().catch(() => [] as Game[]),
      api.games.sessions(since).catch(() => [] as PlaySession[]),
    ]);
    loaded = true;
  }

  onMount(() => {
    void load();
    const stop = on("games-changed", () => void load());
    return () => void stop.then((f) => f());
  });

  const nameOf = (id: string) => games.find((g) => g.id === id)?.name ?? "Silinmiş oyun";
  const coverOf = (id: string) => {
    const c = games.find((g) => g.id === id)?.cover;
    return !c ? null : /^(https?:|data:)/.test(c) ? c : api.fileSrc(c);
  };

  /** Seçilen aralığın günleri (eskiden yeniye) ve her gündeki oyun süreleri (oturum başladığı güne yazılır). */
  const days = $derived.by(() => {
    const today = dayStart(Date.now());
    const list = Array.from({ length: range }, (_, i) => ({ start: today - (range - 1 - i) * 86_400_000, byGame: new Map<string, number>() }));
    for (const s of sessions) {
      const d = list.find((x) => x.start === dayStart(s.startedAt));
      if (d) d.byGame.set(s.gameId, (d.byGame.get(s.gameId) ?? 0) + s.seconds);
    }
    return list;
  });
  const inRange = $derived(sessions.filter((s) => s.startedAt >= days[0].start));
  const totals = $derived.by(() => {
    const m = new Map<string, number>();
    for (const s of inRange) m.set(s.gameId, (m.get(s.gameId) ?? 0) + s.seconds);
    return [...m.entries()].sort((a, b) => b[1] - a[1]);
  });
  /** Renk oyuna bağlı (sıraya değil): bu aralıktaki en çok oynanan 5 oyun. */
  const top = $derived(totals.slice(0, 5).map(([id]) => id));
  const colorOf = (id: string) => {
    const i = top.indexOf(id);
    return i >= 0 ? SERIES[i] : OTHER;
  };
  const total = $derived(totals.reduce((t, [, s]) => t + s, 0));
  const longest = $derived(inRange.reduce((m, s) => (s.seconds > (m?.seconds ?? 0) ? s : m), null as PlaySession | null));
  const activeDays = $derived(days.filter((d) => [...d.byGame.values()].some((v) => v > 0)).length);

  // Grafik geometrisi
  const W = 760;
  const H = 220;
  const PAD = { l: 44, r: 8, t: 10, b: 26 };
  const maxDay = $derived(Math.max(3600, ...days.map((d) => [...d.byGame.values()].reduce((a, b) => a + b, 0))));
  /** Saat çizgileri: tam saatler, en fazla 4 çizgi. */
  const ticks = $derived.by(() => {
    const hours = Math.ceil(maxDay / 3600);
    const step = Math.max(1, Math.ceil(hours / 4));
    return Array.from({ length: Math.floor(hours / step) + 1 }, (_, i) => i * step * 3600);
  });
  const yMax = $derived(Math.max(maxDay, ticks[ticks.length - 1]));
  const bandW = $derived((W - PAD.l - PAD.r) / range);
  const barW = $derived(Math.min(36, bandW * 0.62));
  const y = (secs: number) => PAD.t + (H - PAD.t - PAD.b) * (1 - secs / yMax);

  /** Bir günün yığılmış parçaları (alttan üste: en çok oynanan altta). */
  function stack(d: { byGame: Map<string, number> }) {
    const order = [...d.byGame.entries()].sort((a, b) => {
      const ia = top.indexOf(a[0]);
      const ib = top.indexOf(b[0]);
      return (ia < 0 ? 99 : ia) - (ib < 0 ? 99 : ib);
    });
    // "Diğer" tek parça.
    const parts: { id: string; secs: number }[] = [];
    let other = 0;
    for (const [id, secs] of order) {
      if (top.includes(id)) parts.push({ id, secs });
      else other += secs;
    }
    if (other) parts.push({ id: "__other", secs: other });
    let acc = 0;
    return parts.map((p) => {
      const y0 = y(acc);
      acc += p.secs;
      return { ...p, y0, y1: y(acc) };
    });
  }

  const dayLabel = (t: number) =>
    range === 7
      ? new Date(t).toLocaleDateString("tr-TR", { weekday: "short" })
      : new Date(t).toLocaleDateString("tr-TR", { day: "numeric", month: "short" });
  const fullDate = (t: number) => new Date(t).toLocaleDateString("tr-TR", { weekday: "long", day: "numeric", month: "long" });
  const time = (t: number) => new Date(t).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });

  /** Liste: güne göre gruplu, yeniden eskiye. */
  const grouped = $derived.by(() => {
    const m = new Map<number, PlaySession[]>();
    for (const s of inRange) {
      const k = dayStart(s.startedAt);
      m.set(k, [...(m.get(k) ?? []), s]);
    }
    return [...m.entries()].sort((a, b) => b[0] - a[0]);
  });
</script>

<div class="ses">
  <div class="spread">
    <div class="seg" role="tablist">
      <button aria-pressed={range === 7} onclick={() => (range = 7)}>Son 7 gün</button>
      <button aria-pressed={range === 30} onclick={() => (range = 30)}>Son 30 gün</button>
    </div>
    <button class="btn ghost" onclick={() => onnavigate("library")}><Icon name="library" size={17} />Kütüphanem</button>
  </div>

  {#if loaded && !sessions.length}
    <section class="panel empty">
      <Icon name="clock" size={40} />
      <h3>Henüz oturum yok</h3>
      <p class="sub">
        Kütüphanendeki bir oyunu açtığında GMID onu birkaç saniye içinde yakalar ve süresini tutar. Önce
        <b>Kütüphanem</b>'de oyunlarını tara.
      </p>
      <button class="btn" onclick={() => onnavigate("library")}><Icon name="library" size={17} />Kütüphaneme git</button>
    </section>
  {:else if loaded}
    <div class="tiles">
      <div class="tile panel">
        <span class="sub small">Toplam oynama</span>
        <b class="big tnum">{playtime(total)}</b>
        <span class="sub small tnum">{activeDays} / {range} gün oynandı</span>
      </div>
      <div class="tile panel">
        <span class="sub small">Günlük ortalama</span>
        <b class="big tnum">{playtime(total / range)}</b>
        <span class="sub small tnum">oynanan günlerde {playtime(total / Math.max(1, activeDays))}</span>
      </div>
      <div class="tile panel">
        <span class="sub small">En çok oynanan</span>
        <b class="big name">{totals[0] ? nameOf(totals[0][0]) : "—"}</b>
        <span class="sub small tnum">{totals[0] ? `%${Math.round((totals[0][1] / Math.max(1, total)) * 100)} · ${playtime(totals[0][1])}` : ""}</span>
      </div>
      <div class="tile panel">
        <span class="sub small">Oturum</span>
        <b class="big tnum">{inRange.length}</b>
        <span class="sub small tnum">{longest ? `en uzunu ${playtime(longest.seconds)} · ${nameOf(longest.gameId)}` : ""}</span>
      </div>
    </div>

    <section class="panel">
      <div class="spread">
        <h3 class="h2"><Icon name="clock" />Günlük oynama</h3>
        <div class="legend">
          {#each top as id (id)}
            <span class="lg"><i style:background={colorOf(id)}></i>{nameOf(id)}</span>
          {/each}
          {#if totals.length > 5}<span class="lg"><i style:background={OTHER}></i>Diğer</span>{/if}
        </div>
      </div>
      <div class="chart">
        <svg viewBox="0 0 {W} {H}" role="img" aria-label="Günlük oynama süresi">
          {#each ticks as t (t)}
            <line x1={PAD.l} x2={W - PAD.r} y1={y(t)} y2={y(t)} class="grid" />
            <text x={PAD.l - 8} y={y(t) + 4} class="axis" text-anchor="end">{t / 3600} sa</text>
          {/each}
          {#each days as d, i (d.start)}
            {@const x = PAD.l + bandW * i + (bandW - barW) / 2}
            {@const parts = stack(d)}
            <g
              role="presentation"
              onpointerenter={() => (hover = i)}
              onpointerleave={() => hover === i && (hover = null)}
            >
              <!-- Geniş, görünmez dokunma alanı -->
              <rect x={PAD.l + bandW * i} y={PAD.t} width={bandW} height={H - PAD.t - PAD.b} fill="transparent" />
              {#if hover === i}
                <rect x={PAD.l + bandW * i + 1} y={PAD.t} width={bandW - 2} height={H - PAD.t - PAD.b} class="hl" rx="6" />
              {/if}
              {#each parts as p, k (p.id)}
                {@const isTop = k === parts.length - 1}
                {@const h = Math.max(0, p.y0 - p.y1 - (isTop ? 0 : 2))}
                {#if h > 0.5}
                  <path
                    d={isTop
                      ? `M${x},${p.y0} V${p.y1 + Math.min(4, h)} Q${x},${p.y1} ${x + Math.min(4, h)},${p.y1} H${x + barW - Math.min(4, h)} Q${x + barW},${p.y1} ${x + barW},${p.y1 + Math.min(4, h)} V${p.y0} Z`
                      : `M${x},${p.y0} V${p.y0 - h} H${x + barW} V${p.y0} Z`}
                    fill={p.id === "__other" ? OTHER : colorOf(p.id)}
                  />
                {/if}
              {/each}
              {#if range === 7 || i % 5 === 4 || i === days.length - 1}
                <text x={PAD.l + bandW * i + bandW / 2} y={H - 8} class="axis" text-anchor="middle">{dayLabel(d.start)}</text>
              {/if}
            </g>
          {/each}
          <line x1={PAD.l} x2={W - PAD.r} y1={y(0)} y2={y(0)} class="base" />
        </svg>
        {#if hover !== null}
          {@const d = days[hover]}
          {@const parts = [...d.byGame.entries()].sort((a, b) => b[1] - a[1])}
          <div class="tip" style:left="{((PAD.l + bandW * hover + bandW / 2) / W) * 100}%">
            <b>{fullDate(d.start)}</b>
            {#if parts.length}
              {#each parts as [id, secs] (id)}
                <span class="trow"><i style:background={colorOf(id)}></i><span class="tn">{nameOf(id)}</span><span class="tnum">{playtime(secs)}</span></span>
              {/each}
              <span class="trow sum"><span class="tn">Toplam</span><span class="tnum">{playtime(parts.reduce((t, [, s]) => t + s, 0))}</span></span>
            {:else}
              <span class="sub small">Oynanmadı</span>
            {/if}
          </div>
        {/if}
      </div>
    </section>

    <section class="panel">
      <h3 class="h2"><Icon name="list" />Oturumlar</h3>
      {#each grouped as [day, list] (day)}
        <div class="day">
          <span class="sub small up">{fullDate(day)} · {playtime(list.reduce((t, s) => t + s.seconds, 0))}</span>
          {#each list as s (s.id)}
            <div class="srow">
              <span class="sw" style:background={colorOf(s.gameId)}></span>
              {#if coverOf(s.gameId)}<img src={coverOf(s.gameId)} alt="" />{:else}<span class="noimg"></span>{/if}
              <b>{nameOf(s.gameId)}</b>
              <span class="sub small tnum">{time(s.startedAt)}–{s.endedAt ? time(s.endedAt) : "şimdi"}</span>
              <span class="tnum dur">{playtime(s.seconds)}</span>
            </div>
          {/each}
        </div>
      {/each}
    </section>
  {/if}
</div>

<style>
  .ses { display: flex; flex-direction: column; gap: 16px; max-width: 1300px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .empty { align-items: center; text-align: center; padding: 40px 20px; color: var(--muted); }
  .empty h3 { margin: 0; color: var(--text); font-family: var(--f-display); }
  .empty p { margin: 0; max-width: 520px; }
  .tiles { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; }
  .tile { gap: 4px; }
  .big { font-family: var(--f-display); font-size: 26px; color: var(--text); }
  .big.name { font-size: 19px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .legend { display: flex; flex-wrap: wrap; gap: 6px 14px; }
  .lg { display: inline-flex; align-items: center; gap: 6px; font-size: 12.5px; color: var(--muted); }
  .lg i { width: 10px; height: 10px; border-radius: 3px; }
  .chart { position: relative; }
  svg { width: 100%; height: auto; display: block; overflow: visible; }
  .grid { stroke: rgba(255, 255, 255, 0.06); stroke-width: 1; }
  .base { stroke: rgba(255, 255, 255, 0.16); stroke-width: 1; }
  .axis { fill: var(--dim); font-size: 11px; font-family: var(--f-body); }
  .hl { fill: rgba(255, 255, 255, 0.04); }
  .tip { position: absolute; top: 0; transform: translateX(-50%); min-width: 190px; padding: 10px 12px; border-radius: 10px; background: rgba(12, 14, 17, 0.96); border: 1px solid var(--line-2); box-shadow: 0 12px 30px rgba(0, 0, 0, 0.5); display: flex; flex-direction: column; gap: 4px; pointer-events: none; font-size: 13px; z-index: 2; }
  .trow { display: grid; grid-template-columns: 10px minmax(0, 1fr) auto; gap: 8px; align-items: center; }
  .trow i { width: 10px; height: 10px; border-radius: 3px; }
  .trow.sum { grid-template-columns: minmax(0, 1fr) auto; border-top: 1px solid var(--line); padding-top: 4px; margin-top: 2px; font-weight: 700; }
  .tn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .day { display: flex; flex-direction: column; gap: 4px; }
  .up { text-transform: uppercase; letter-spacing: 0.06em; font-weight: 700; margin-top: 6px; }
  .srow { display: grid; grid-template-columns: 4px 28px minmax(0, 1fr) auto 96px; gap: 10px; align-items: center; padding: 6px 10px; border-radius: 9px; background: rgba(255, 255, 255, 0.025); }
  .sw { width: 4px; height: 28px; border-radius: 2px; }
  .srow img, .noimg { width: 28px; height: 42px; object-fit: cover; border-radius: 5px; background: var(--coal-2); }
  .srow b { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dur { text-align: right; font-weight: 600; white-space: nowrap; }
  @media (max-width: 900px) {
    .tiles { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  }
</style>
