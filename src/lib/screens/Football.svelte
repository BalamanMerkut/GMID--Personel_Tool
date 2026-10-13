<script lang="ts">
  // Futbol: seçilen günün maçları (saat, canlı dakika, skor, golü atanlar, kırmızı kartlar) ve ligin puan durumu.
  // Canlı maç varken skorlar 30 sn'de bir, yoksa 5 dk'da bir kendiliğinden yenilenir.
  import { onDestroy, onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import type { FootballGroup, FootballMatch } from "../types";

  type Tab = "matches" | "table";
  let tab = $state<Tab>(getJson<Tab>("football.tab", "matches"));
  let leagues = $state<[string, string][]>([]);
  /** Maçlarda "all" = bütün ligler. */
  let league = $state(getJson<string>("football.league", "all"));
  let tableLeague = $state(getJson<string>("football.tableLeague", "tur.1"));
  /** Seçili gün: bugünden kaç gün sonra (−3…+4). */
  let offset = $state(0);
  let liveOnly = $state(false);

  let matches = $state<FootballMatch[]>([]);
  let groups = $state<FootballGroup[]>([]);
  let loading = $state(false);
  let error = $state("");
  let updated = $state<Date | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  const OFFSETS = [-3, -2, -1, 0, 1, 2, 3, 4];
  const WEEKDAY = new Intl.DateTimeFormat("tr-TR", { weekday: "short" });
  const TIME = new Intl.DateTimeFormat("tr-TR", { hour: "2-digit", minute: "2-digit" });

  function dayAt(off: number): Date {
    const d = new Date();
    d.setHours(12, 0, 0, 0);
    d.setDate(d.getDate() + off);
    return d;
  }
  const ymd = (d: Date) => `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}`;
  const sameDay = (iso: string, d: Date) => {
    const x = new Date(iso);
    return x.getFullYear() === d.getFullYear() && x.getMonth() === d.getMonth() && x.getDate() === d.getDate();
  };
  function dayLabel(off: number): [string, string] {
    const d = dayAt(off);
    const name = off === 0 ? "Bugün" : off === -1 ? "Dün" : off === 1 ? "Yarın" : WEEKDAY.format(d);
    return [name, `${d.getDate()}.${String(d.getMonth() + 1).padStart(2, "0")}`];
  }
  const leagueName = (id: string) => leagues.find(([s]) => s === id)?.[1] ?? id;

  async function loadMatches(quiet = false) {
    const my = ++seq;
    const d = dayAt(offset);
    const prev = dayAt(offset - 1);
    if (!quiet) loading = true;
    try {
      // Yerel gün, sitenin gün sınırıyla tam örtüşmeyebilir: bir önceki gün de istenir, yerel güne göre süzülür.
      const want = league === "all" ? leagues.map(([s]) => s) : [league];
      const list = await api.football.matches(want, [ymd(prev), ymd(d)]);
      if (my !== seq) return;
      matches = list.filter((m) => sameDay(m.date, d));
      error = "";
      updated = new Date();
    } catch (e) {
      if (my === seq) error = String(e);
    } finally {
      if (my === seq) loading = false;
      schedule();
    }
  }

  async function loadTable() {
    const my = ++seq;
    loading = true;
    try {
      const g = await api.football.standings(tableLeague);
      if (my !== seq) return;
      groups = g;
      error = "";
      updated = new Date();
    } catch (e) {
      if (my === seq) error = String(e);
    } finally {
      if (my === seq) loading = false;
    }
  }

  /** Canlı maç varken 30 sn, bugünse 5 dk; diğer günlerde kendiliğinden yenilenmez. */
  function schedule() {
    clearTimeout(timer);
    if (tab !== "matches") return;
    const live = matches.some((m) => m.state === "in");
    const wait = live ? 30_000 : offset === 0 ? 300_000 : 0;
    if (wait) timer = setTimeout(() => void loadMatches(true), wait);
  }

  function reload() {
    if (tab === "matches") void loadMatches();
    else void loadTable();
  }

  onMount(async () => {
    leagues = await api.football.leagues().catch(() => []);
    if (league !== "all" && !leagues.some(([s]) => s === league)) league = "all";
    reload();
  });
  onDestroy(() => clearTimeout(timer));

  function setTab(t: Tab) {
    tab = t;
    void setJson("football.tab", t);
    error = "";
    reload();
  }
  function setLeague(l: string) {
    league = l;
    void setJson("football.league", l);
    void loadMatches();
  }
  function setTableLeague(l: string) {
    tableLeague = l;
    void setJson("football.tableLeague", l);
    void loadTable();
  }
  function setDay(o: number) {
    offset = o;
    void loadMatches();
  }

  const shown = $derived(liveOnly ? matches.filter((m) => m.state === "in") : matches);
  const liveCount = $derived(matches.filter((m) => m.state === "in").length);
  /** Lig lig gruplanmış (seçim sırasıyla: önce Süper Lig). */
  const byLeague = $derived.by(() => {
    const order = leagues.map(([s]) => s);
    const out: [string, FootballMatch[]][] = [];
    for (const m of shown) {
      let g = out.find(([l]) => l === m.league);
      if (!g) out.push((g = [m.league, []]));
      g[1].push(m);
    }
    return out.sort(([a], [b]) => order.indexOf(a) - order.indexOf(b));
  });
  /** Puan tablosundaki renklerin anlamı. */
  const legend = $derived.by(() => {
    const seen = new Map<string, string>();
    for (const g of groups) for (const r of g.rows) if (r.note && r.color && !seen.has(r.note)) seen.set(r.note, r.color);
    return [...seen];
  });
  const NOTE_TR: [RegExp, string][] = [
    [/champions league/i, "Şampiyonlar Ligi"],
    [/conference/i, "Konferans Ligi"],
    [/europa/i, "Avrupa Ligi"],
    [/relegation/i, "Küme düşme"],
    [/promotion/i, "Yükselme"],
    [/play-?off/i, "Play-off"],
    [/knockout/i, "Eleme turu"],
    [/eliminated/i, "Elendi"],
  ];
  const noteTr = (n: string) => {
    const hit = NOTE_TR.find(([re]) => re.test(n));
    return hit ? `${hit[1]}${/qualif|play-?off|round/i.test(n) && !/play-?off/i.test(hit[1]) ? " (eleme)" : ""}` : n;
  };
  const sideEvents = (m: FootballMatch, side: "home" | "away") => m.events.filter((e) => e.side === side);
  const evIcon = (k: string) => (k === "red" ? "▮" : "⚽");
  const evNote = (k: string) => (k === "pen" ? " (P)" : k === "own" ? " (K.K.)" : "");
</script>

<div class="fb">
  <section class="panel head">
    <div class="spread">
      <div>
        <h3 class="h2"><Icon name="trophy" />Futbol</h3>
        <p class="sub small">Maçlar, canlı skor ve puan durumu · Kaynak: ESPN</p>
      </div>
      <div class="row">
        <div class="seg" role="tablist">
          <button aria-pressed={tab === "matches"} onclick={() => setTab("matches")}>Maçlar</button>
          <button aria-pressed={tab === "table"} onclick={() => setTab("table")}>Puan durumu</button>
        </div>
        <button class="btn ghost sm" onclick={reload} disabled={loading} title="Yenile"><Icon name="refresh" size={15} />{loading ? "Yükleniyor…" : "Yenile"}</button>
      </div>
    </div>
  </section>

  {#if tab === "matches"}
    <section class="panel">
      <div class="seg leagues" role="group" aria-label="Lig">
        <button aria-pressed={league === "all"} onclick={() => setLeague("all")}>Tüm ligler</button>
        {#each leagues as [id, name] (id)}
          <button aria-pressed={league === id} onclick={() => setLeague(id)}>{name}</button>
        {/each}
      </div>
      <div class="days" role="group" aria-label="Gün">
        {#each OFFSETS as o (o)}
          {@const [n, d] = dayLabel(o)}
          <button class="day" class:on={offset === o} onclick={() => setDay(o)}><b>{n}</b><span class="tnum">{d}</span></button>
        {/each}
        {#if liveCount}
          <button class="live-f" class:on={liveOnly} onclick={() => (liveOnly = !liveOnly)}><span class="dot"></span>Canlı · {liveCount}</button>
        {/if}
      </div>

      {#if error}
        <div class="err"><Icon name="info" size={16} /><span>Maçlar alınamadı: {error}</span><button class="btn sm" onclick={reload}>Tekrar dene</button></div>
      {:else if loading && !matches.length}
        <p class="sub">Maçlar yükleniyor…</p>
      {:else if !shown.length}
        <p class="sub empty">{liveOnly ? "Şu an oynanan maç yok." : "Bu gün seçili ligde maç yok."}</p>
      {:else}
        {#each byLeague as [lg, list] (lg)}
          <div class="lg">
            {#if league === "all"}<div class="lgh">{leagueName(lg)}</div>{/if}
            {#each list as m (m.id)}
              <div class="match" class:live={m.state === "in"}>
                <div class="st tnum">
                  {#if m.state === "in"}<span class="dot"></span>{m.status}
                  {:else if m.state === "post"}<span class="fin">{m.status}</span>
                  {:else}{TIME.format(new Date(m.date))}{/if}
                </div>
                <div class="tm home" class:win={m.home.winner}>
                  <span class="nm" title={m.home.team.name}>{m.home.team.short}</span>
                  {#if m.home.team.logo}<img src={m.home.team.logo} alt="" loading="lazy" />{:else}<span class="ph"></span>{/if}
                </div>
                <div class="score tnum">
                  {#if m.home.score === null}<span class="vs">–</span>{:else}{m.home.score}<i>-</i>{m.away.score}{/if}
                </div>
                <div class="tm away" class:win={m.away.winner}>
                  {#if m.away.team.logo}<img src={m.away.team.logo} alt="" loading="lazy" />{:else}<span class="ph"></span>{/if}
                  <span class="nm" title={m.away.team.name}>{m.away.team.short}</span>
                </div>
                {#if m.events.length}
                  <div class="evs">
                    <div class="ev home">
                      {#each sideEvents(m, "home") as e, i (i)}<span class:red={e.kind === "red"}>{e.player || "?"}{evNote(e.kind)} <b class="tnum">{e.minute}</b> {evIcon(e.kind)}</span>{/each}
                    </div>
                    <div class="ev away">
                      {#each sideEvents(m, "away") as e, i (i)}<span class:red={e.kind === "red"}>{evIcon(e.kind)} <b class="tnum">{e.minute}</b> {e.player || "?"}{evNote(e.kind)}</span>{/each}
                    </div>
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/each}
      {/if}
      {#if updated}<p class="sub small upd">Son güncelleme {TIME.format(updated)}{liveCount ? " · canlı maçlar 30 sn'de bir yenilenir" : ""}</p>{/if}
    </section>
  {:else}
    <section class="panel">
      <div class="seg leagues" role="group" aria-label="Lig">
        {#each leagues as [id, name] (id)}
          <button aria-pressed={tableLeague === id} onclick={() => setTableLeague(id)}>{name}</button>
        {/each}
      </div>
      {#if error}
        <div class="err"><Icon name="info" size={16} /><span>Puan durumu alınamadı: {error}</span><button class="btn sm" onclick={reload}>Tekrar dene</button></div>
      {:else if loading && !groups.length}
        <p class="sub">Puan durumu yükleniyor…</p>
      {:else if !groups.length}
        <p class="sub empty">Bu lig için puan durumu yok (kupa elemeleri olabilir).</p>
      {:else}
        {#each groups as g (g.name)}
          {#if groups.length > 1}<div class="lgh">{g.name}</div>{/if}
          <div class="tbl">
            <div class="tr th sub small">
              <span>#</span><span class="tt">Takım</span><span title="Oynanan">O</span><span title="Galibiyet">G</span><span title="Beraberlik">B</span><span title="Mağlubiyet">M</span><span title="Atılan:Yenilen">A:Y</span><span title="Averaj">AV</span><span title="Puan">P</span>
            </div>
            {#each g.rows as r (r.team.id)}
              <div class="tr" style:--c={r.color || "transparent"} title={r.note ? noteTr(r.note) : undefined}>
                <span class="rk tnum">{r.rank}</span>
                <span class="tt">{#if r.team.logo}<img src={r.team.logo} alt="" loading="lazy" />{/if}<span class="nm">{r.team.name}</span></span>
                <span class="tnum">{r.played}</span><span class="tnum">{r.won}</span><span class="tnum">{r.draw}</span><span class="tnum">{r.lost}</span>
                <span class="tnum">{r.gf}:{r.ga}</span><span class="tnum">{r.gf - r.ga > 0 ? "+" : ""}{r.gf - r.ga}</span>
                <b class="tnum pts">{r.points}</b>
              </div>
            {/each}
          </div>
        {/each}
        {#if legend.length}
          <div class="legend sub small">
            {#each legend as [n, c] (n)}<span><i style:background={c}></i>{noteTr(n)}</span>{/each}
          </div>
        {/if}
      {/if}
    </section>
  {/if}
</div>

<style>
  .fb { display: flex; flex-direction: column; gap: 16px; max-width: 1000px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .head p { margin: 4px 0 0; }
  .sm { padding: 6px 11px; font-size: 13px; }
  .leagues { gap: 6px; }
  .days { display: flex; gap: 6px; flex-wrap: wrap; align-items: stretch; }
  .day { display: flex; flex-direction: column; align-items: center; gap: 2px; min-width: 62px; padding: 7px 10px; border-radius: 12px; border: 1px solid var(--line-2); background: var(--coal-2); color: var(--muted); cursor: pointer; font: inherit; }
  .day b { font-size: 13px; color: var(--text); font-weight: 600; }
  .day span { font-size: 11.5px; }
  .day:hover { border-color: var(--line-3, var(--gold-dim)); }
  .day.on { border-color: var(--gold); background: rgba(227, 194, 112, 0.12); }
  .day.on b { color: var(--gold-glow); }
  .live-f { margin-left: auto; display: inline-flex; align-items: center; gap: 8px; padding: 7px 14px; border-radius: 12px; border: 1px solid rgba(224, 106, 106, 0.5); background: rgba(224, 106, 106, 0.08); color: #ffb4b4; font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; }
  .live-f.on { background: rgba(224, 106, 106, 0.22); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: #ff5a5a; box-shadow: 0 0 0 0 rgba(255, 90, 90, 0.6); animation: pulse 1.6s infinite; flex: none; }
  @keyframes pulse { 0% { box-shadow: 0 0 0 0 rgba(255, 90, 90, 0.55); } 70% { box-shadow: 0 0 0 7px rgba(255, 90, 90, 0); } 100% { box-shadow: 0 0 0 0 rgba(255, 90, 90, 0); } }
  .lg { display: flex; flex-direction: column; gap: 4px; }
  .lgh { margin-top: 8px; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--dim); }
  .match { display: grid; grid-template-columns: 64px minmax(0, 1fr) 76px minmax(0, 1fr); align-items: center; gap: 4px 12px; padding: 10px 12px; border-radius: 12px; background: rgba(255, 255, 255, 0.025); border: 1px solid transparent; }
  .match:hover { background: rgba(255, 255, 255, 0.045); }
  .match.live { border-color: rgba(224, 106, 106, 0.35); }
  .st { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--muted); }
  .match.live .st { color: #ff8a8a; font-weight: 700; }
  .fin { color: var(--dim); font-weight: 600; }
  .tm { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .tm.home { justify-content: flex-end; text-align: right; }
  .tm img, .tm .ph { width: 24px; height: 24px; object-fit: contain; flex: none; }
  .tm .ph { border-radius: 50%; background: rgba(255, 255, 255, 0.06); }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
  .tm.win .nm { font-weight: 700; }
  .score { justify-self: center; display: inline-flex; align-items: center; gap: 6px; min-width: 64px; justify-content: center; padding: 4px 10px; border-radius: 8px; background: rgba(0, 0, 0, 0.3); font-size: 17px; font-weight: 700; }
  .match.live .score { color: #ffd2d2; }
  .score i { font-style: normal; color: var(--dim); font-weight: 400; }
  .vs { color: var(--dim); font-weight: 400; }
  .evs { grid-column: 2 / 5; display: grid; grid-template-columns: minmax(0, 1fr) 76px minmax(0, 1fr); gap: 0 12px; font-size: 12px; color: var(--muted); }
  .ev { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .ev.home { text-align: right; }
  .ev.away { grid-column: 3; }
  .ev span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ev b { color: var(--text); font-weight: 600; }
  .ev .red { color: #ff8a8a; }
  .empty { padding: 18px 4px; }
  .err { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-radius: 10px; background: rgba(224, 106, 106, 0.08); border: 1px solid rgba(224, 106, 106, 0.35); color: #ffc9c9; font-size: 13px; }
  .err span { flex: 1; }
  .upd { margin: 4px 0 0; }
  .tbl { display: flex; flex-direction: column; }
  .tr { display: grid; grid-template-columns: 34px minmax(0, 1fr) repeat(4, 34px) 54px 40px 40px; align-items: center; gap: 6px; padding: 7px 10px; border-radius: 8px; border-left: 3px solid var(--c, transparent); font-size: 13.5px; }
  .tr:not(.th):nth-child(even) { background: rgba(255, 255, 255, 0.025); }
  .tr:not(.th):hover { background: rgba(255, 255, 255, 0.05); }
  .tr > span:not(.tt), .tr > b { text-align: center; }
  .th { border-left-color: transparent; font-weight: 600; }
  .rk { color: var(--muted); }
  .tt { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .tt img { width: 22px; height: 22px; object-fit: contain; flex: none; }
  .pts { color: var(--gold-glow); }
  .legend { display: flex; flex-wrap: wrap; gap: 6px 16px; margin-top: 6px; }
  .legend span { display: inline-flex; align-items: center; gap: 6px; }
  .legend i { width: 10px; height: 10px; border-radius: 3px; }
  @media (max-width: 760px) {
    .match { grid-template-columns: 52px minmax(0, 1fr) 64px minmax(0, 1fr); gap: 4px 8px; }
    .tr { grid-template-columns: 28px minmax(0, 1fr) 28px 28px 28px 28px 0 0 36px; }
    .tr > :nth-child(7), .tr > :nth-child(8) { display: none; }
  }
</style>
