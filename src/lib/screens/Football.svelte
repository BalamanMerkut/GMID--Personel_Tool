<script lang="ts">
  // Futbol: solda ligler (Türkiye · Avrupa kupaları · Avrupa ligleri), sağda seçilen ligin maçları, puan durumu ve
  // gol/asist krallığı. Takıma tıklayınca takım sayfası açılır: kadro (mevkiye göre), son maçlar ve fikstür.
  // Canlı maç varken skorlar 30 sn'de bir, yoksa 5 dk'da bir kendiliğinden yenilenir.
  import { onDestroy, onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import type { FootballGroup, FootballLeaders, FootballMatch, FootballTeam, FootballTeamPage } from "../types";

  type Tab = "matches" | "table" | "leaders";
  const CATS: [string, string[]][] = [
    ["Türkiye", ["tur.1"]],
    ["Avrupa kupaları", ["uefa.champions", "uefa.europa", "uefa.europa.conf"]],
    ["Avrupa ligleri", ["eng.1", "esp.1", "ita.1", "ger.1", "fra.1", "ned.1", "por.1"]],
  ];

  let tab = $state<Tab>(getJson<Tab>("football.tab", "matches"));
  let leagues = $state<[string, string][]>([]);
  /** "all" yalnız maçlarda: bütün ligler. */
  let league = $state(getJson<string>("football.league", "all"));
  let offset = $state(0);
  let liveOnly = $state(false);

  let matches = $state<FootballMatch[]>([]);
  let groups = $state<FootballGroup[]>([]);
  let leaders = $state<FootballLeaders | null>(null);
  let teamPage = $state<FootballTeamPage | null>(null);
  /** Açık takım sayfası (yükleniyorken de). */
  let teamOpen = $state<{ league: string; team: FootballTeam } | null>(null);
  let teamTab = $state<"squad" | "games">("squad");
  let loading = $state(false);
  let error = $state("");
  let updated = $state<Date | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  const OFFSETS = [-3, -2, -1, 0, 1, 2, 3, 4];
  const WEEKDAY = new Intl.DateTimeFormat("tr-TR", { weekday: "short" });
  const TIME = new Intl.DateTimeFormat("tr-TR", { hour: "2-digit", minute: "2-digit" });
  const DATE = new Intl.DateTimeFormat("tr-TR", { day: "numeric", month: "short", weekday: "short" });

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
  const leagueName = (id: string) => (id === "all" ? "Tüm maçlar" : (leagues.find(([s]) => s === id)?.[1] ?? id));

  async function guard<T>(work: () => Promise<T>, apply: (v: T) => void, quiet = false) {
    const my = ++seq;
    if (!quiet) loading = true;
    try {
      const v = await work();
      if (my !== seq) return;
      apply(v);
      error = "";
      updated = new Date();
    } catch (e) {
      if (my === seq) error = String(e);
    } finally {
      if (my === seq) loading = false;
    }
  }

  function loadMatches(quiet = false) {
    const d = dayAt(offset);
    // Yerel gün, sitenin gün sınırıyla tam örtüşmeyebilir: bir önceki gün de istenir, yerel güne göre süzülür.
    const want = league === "all" ? leagues.map(([s]) => s) : [league];
    return guard(
      () => api.football.matches(want, [ymd(dayAt(offset - 1)), ymd(d)]),
      (list) => (matches = list.filter((m) => sameDay(m.date, d))),
      quiet,
    ).then(schedule);
  }
  const loadTable = () => guard(() => api.football.standings(league), (g) => (groups = g));
  const loadLeaders = () => guard(() => api.football.leaders(league), (l) => (leaders = l));

  /** Canlı maç varken 30 sn, bugünse 5 dk; diğer günlerde kendiliğinden yenilenmez. */
  function schedule() {
    clearTimeout(timer);
    if (tab !== "matches" || teamOpen) return;
    const live = matches.some((m) => m.state === "in");
    const wait = live ? 30_000 : offset === 0 ? 300_000 : 0;
    if (wait) timer = setTimeout(() => void loadMatches(true), wait);
  }

  function reload() {
    if (teamOpen) return void openTeam(teamOpen.league, teamOpen.team);
    if (tab === "matches" || league === "all") void loadMatches();
    else if (tab === "table") void loadTable();
    else void loadLeaders();
  }

  onMount(async () => {
    leagues = await api.football.leagues().catch(() => []);
    if (league !== "all" && !leagues.some(([s]) => s === league)) league = "all";
    if (league === "all") tab = "matches";
    reload();
  });
  onDestroy(() => clearTimeout(timer));

  function setTab(t: Tab) {
    tab = t;
    void setJson("football.tab", t);
    teamOpen = null;
    error = "";
    reload();
  }
  function setLeague(l: string) {
    league = l;
    void setJson("football.league", l);
    teamOpen = null;
    error = "";
    if (l === "all" && tab !== "matches") tab = "matches";
    reload();
  }
  function setDay(o: number) {
    offset = o;
    void loadMatches();
  }
  function openTeam(lg: string, team: FootballTeam) {
    if (!team.id) return;
    clearTimeout(timer);
    teamOpen = { league: lg, team };
    teamPage = null;
    teamTab = "squad";
    void guard(() => api.football.team(lg, team.id), (p) => (teamPage = p));
  }
  function closeTeam() {
    teamOpen = null;
    error = "";
    reload();
  }

  const shown = $derived(liveOnly ? matches.filter((m) => m.state === "in") : matches);
  const liveCount = $derived(matches.filter((m) => m.state === "in").length);
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
    return hit ? `${hit[1]}${/qualif|round/i.test(n) ? " (eleme)" : ""}` : n;
  };
  const POS: [string, string][] = [["G", "Kaleciler"], ["D", "Defans"], ["M", "Orta saha"], ["F", "Forvet"], ["", "Diğer"]];
  const NATION: Record<string, string> = {
    Turkey: "Türkiye", Türkiye: "Türkiye", Brazil: "Brezilya", Argentina: "Arjantin", Germany: "Almanya", France: "Fransa", Spain: "İspanya",
    England: "İngiltere", Italy: "İtalya", Netherlands: "Hollanda", Portugal: "Portekiz", Belgium: "Belçika", Nigeria: "Nijerya", Colombia: "Kolombiya",
    Uruguay: "Uruguay", Croatia: "Hırvatistan", Serbia: "Sırbistan", Poland: "Polonya", Morocco: "Fas", Senegal: "Senegal", Ghana: "Gana",
    Denmark: "Danimarka", Sweden: "İsveç", Norway: "Norveç", Austria: "Avusturya", Switzerland: "İsviçre", Greece: "Yunanistan", Japan: "Japonya",
    "South Korea": "Güney Kore", "United States": "ABD", Mexico: "Meksika", Ukraine: "Ukrayna", Scotland: "İskoçya", Wales: "Galler",
    "Bosnia-Herzegovina": "Bosna-Hersek", Albania: "Arnavutluk", Georgia: "Gürcistan", Cameroon: "Kamerun", Egypt: "Mısır", Algeria: "Cezayir",
  };
  const nation = (n: string) => NATION[n] ?? n;
  /** "1st in Turkish Super Lig" → "Ligde 1. sırada". */
  const standingTr = (s: string) => {
    const m = s.match(/^(\d+)(?:st|nd|rd|th)\b/i);
    return m ? `Ligde ${m[1]}. sırada` : s;
  };
  /** Takımın gözünden sonuç: G / B / M. */
  function outcome(m: FootballMatch, teamId: string): "G" | "B" | "M" | "" {
    const mine = m.home.team.id === teamId ? m.home : m.away;
    const other = mine === m.home ? m.away : m.home;
    if (mine.score === null || other.score === null) return "";
    return mine.score > other.score ? "G" : mine.score < other.score ? "M" : "B";
  }
  const sideEvents = (m: FootballMatch, side: "home" | "away") => m.events.filter((e) => e.side === side);
  const evIcon = (k: string) => (k === "red" ? "▮" : "⚽");
  const evNote = (k: string) => (k === "pen" ? " (P)" : k === "own" ? " (K.K.)" : "");
  const initials = (n: string) => n.split(/\s+/).filter(Boolean).slice(0, 2).map((w) => w[0]).join("").toUpperCase();
</script>

<div class="fb">
  <aside class="panel side">
    <div class="brand"><Icon name="trophy" size={18} /><b>Futbol</b></div>
    <button class="lg-btn" class:on={league === "all"} onclick={() => setLeague("all")}><Icon name="clock" size={15} />Tüm maçlar</button>
    {#each CATS as [cat, ids] (cat)}
      {@const avail = ids.filter((id) => leagues.some(([s]) => s === id))}
      {#if avail.length}
        <div class="cat">{cat}</div>
        {#each avail as id (id)}
          <button class="lg-btn" class:on={league === id} onclick={() => setLeague(id)}>{leagueName(id)}</button>
        {/each}
      {/if}
    {/each}
    <p class="sub small src">Kaynak: ESPN</p>
  </aside>

  <main class="main">
    {#if teamOpen}
      {@const tp = teamPage}
      <section class="panel thead">
        <button class="back" onclick={closeTeam}><Icon name="back" size={16} />{leagueName(league)}</button>
        <div class="tinfo">
          {#if (tp?.team.logo || teamOpen.team.logo)}<img class="tlogo" src={tp?.team.logo || teamOpen.team.logo} alt="" />{:else}<span class="tlogo ph">{initials(teamOpen.team.name)}</span>{/if}
          <div>
            <h2>{tp?.team.name || teamOpen.team.name}</h2>
            <p class="sub small">{[leagueName(teamOpen.league), standingTr(tp?.standing ?? ""), tp?.venue].filter(Boolean).join(" · ")}</p>
          </div>
        </div>
        <div class="seg" role="tablist">
          <button aria-pressed={teamTab === "squad"} onclick={() => (teamTab = "squad")}>Kadro{tp ? ` · ${tp.players.length}` : ""}</button>
          <button aria-pressed={teamTab === "games"} onclick={() => (teamTab = "games")}>Maçlar</button>
        </div>
      </section>
      <section class="panel">
        {#if error}
          {@render failed("Takım bilgisi alınamadı")}
        {:else if !tp}
          <p class="sub">Takım yükleniyor…</p>
        {:else if teamTab === "squad"}
          {#if !tp.players.length}
            <p class="sub empty">Bu takımın kadrosu bulunamadı.</p>
          {/if}
          {#each POS as [code, label] (code)}
            {@const ps = tp.players.filter((p) => p.pos === code)}
            {#if ps.length}
              <div class="lgh">{label} <span class="tnum">{ps.length}</span></div>
              <div class="squad">
                {#each ps as p (p.id || p.name)}
                  <div class="pl">
                    {#if p.photo}<img class="face" src={p.photo} alt="" loading="lazy" />{:else}<span class="face no tnum">{p.jersey || initials(p.name)}</span>{/if}
                    <div class="pt">
                      <b title={p.name}>{p.name}</b>
                      <span class="sub small">{[p.jersey ? `#${p.jersey}` : "", p.age ? `${p.age} yaş` : "", nation(p.nation)].filter(Boolean).join(" · ")}</span>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          {/each}
        {:else}
          {#if tp.fixtures.length}
            <div class="lgh">Sıradaki maçlar</div>
            {#each tp.fixtures as m (m.id)}{@render game(m, tp.team.id)}{/each}
          {/if}
          <div class="lgh">Son maçlar</div>
          {#if tp.results.length}
            <div class="form">
              {#each tp.results.slice(0, 5) as m (m.id)}{@const o = outcome(m, tp.team.id)}<span class="res {o}">{o || "–"}</span>{/each}
              <span class="sub small">son 5 maç</span>
            </div>
            {#each tp.results as m (m.id)}{@render game(m, tp.team.id)}{/each}
          {:else}
            <p class="sub empty">Bu sezon oynanan maç bulunamadı.</p>
          {/if}
        {/if}
      </section>
    {:else}
      <section class="panel head">
        <div class="spread">
          <h2>{leagueName(league)}</h2>
          <div class="row">
            <div class="seg" role="tablist">
              <button aria-pressed={tab === "matches"} onclick={() => setTab("matches")}>Maçlar</button>
              <button aria-pressed={tab === "table"} disabled={league === "all"} title={league === "all" ? "Soldan bir lig seç" : undefined} onclick={() => setTab("table")}>Puan durumu</button>
              <button aria-pressed={tab === "leaders"} disabled={league === "all"} title={league === "all" ? "Soldan bir lig seç" : undefined} onclick={() => setTab("leaders")}>Krallık</button>
            </div>
            <button class="btn ghost sm" onclick={reload} disabled={loading} title="Yenile"><Icon name="refresh" size={15} />{loading ? "Yükleniyor…" : "Yenile"}</button>
          </div>
        </div>
      </section>

      {#if tab === "matches"}
        <section class="panel">
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
            {@render failed("Maçlar alınamadı")}
          {:else if loading && !matches.length}
            <p class="sub">Maçlar yükleniyor…</p>
          {:else if !shown.length}
            <p class="sub empty">{liveOnly ? "Şu an oynanan maç yok." : "Bu gün maç yok."}</p>
          {:else}
            {#each byLeague as [lg, list] (lg)}
              <div class="lg">
                {#if league === "all"}<button class="lgh link" onclick={() => setLeague(lg)}>{leagueName(lg)} <Icon name="next" size={12} /></button>{/if}
                {#each list as m (m.id)}{@render matchRow(m)}{/each}
              </div>
            {/each}
          {/if}
          {#if updated}<p class="sub small upd">Son güncelleme {TIME.format(updated)}{liveCount ? " · canlı maçlar 30 sn'de bir yenilenir" : ""} · takıma tıkla: kadro ve maçları</p>{/if}
        </section>
      {:else if tab === "table"}
        <section class="panel">
          {#if error}
            {@render failed("Puan durumu alınamadı")}
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
                  <button class="tr" style:--c={r.color || "transparent"} title={r.note ? noteTr(r.note) : "Kadro ve maçlar"} onclick={() => openTeam(league, r.team)}>
                    <span class="rk tnum">{r.rank}</span>
                    <span class="tt">{#if r.team.logo}<img src={r.team.logo} alt="" loading="lazy" />{/if}<span class="nm">{r.team.name}</span></span>
                    <span class="tnum">{r.played}</span><span class="tnum">{r.won}</span><span class="tnum">{r.draw}</span><span class="tnum">{r.lost}</span>
                    <span class="tnum">{r.gf}:{r.ga}</span><span class="tnum">{r.gf - r.ga > 0 ? "+" : ""}{r.gf - r.ga}</span>
                    <b class="tnum pts">{r.points}</b>
                  </button>
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
      {:else}
        <section class="panel">
          {#if error}
            {@render failed("Krallık listesi alınamadı")}
          {:else if loading && !leaders}
            <p class="sub">Krallık yükleniyor…</p>
          {:else if leaders && !leaders.goals.length && !leaders.assists.length}
            <p class="sub empty">Bu lig için krallık listesi yok.</p>
          {:else if leaders}
            <div class="kings">
              {@render king("Gol krallığı", "gol", leaders.goals)}
              {@render king("Asist krallığı", "asist", leaders.assists)}
            </div>
          {/if}
        </section>
      {/if}
    {/if}
  </main>
</div>

{#snippet failed(what: string)}
  <div class="err"><Icon name="info" size={16} /><span>{what}: {error}</span><button class="btn sm" onclick={reload}>Tekrar dene</button></div>
{/snippet}

{#snippet teamBtn(lg: string, t: FootballTeam, home: boolean, win: boolean)}
  <button class="tm" class:home class:win onclick={() => openTeam(lg, t)} title="{t.name} · kadro ve maçlar">
    {#if home}<span class="nm">{t.short}</span>{/if}
    {#if t.logo}<img src={t.logo} alt="" loading="lazy" />{:else}<span class="ph">{initials(t.name)}</span>{/if}
    {#if !home}<span class="nm">{t.short}</span>{/if}
  </button>
{/snippet}

{#snippet matchRow(m: FootballMatch)}
  <div class="match" class:live={m.state === "in"}>
    <div class="st tnum">
      {#if m.state === "in"}<span class="dot"></span>{m.status}
      {:else if m.state === "post"}<span class="fin">{m.status}</span>
      {:else}{TIME.format(new Date(m.date))}{/if}
    </div>
    {@render teamBtn(m.league, m.home.team, true, m.home.winner)}
    <div class="score tnum">
      {#if m.home.score === null}<span class="vs">–</span>{:else}{m.home.score}<i>-</i>{m.away.score}{/if}
    </div>
    {@render teamBtn(m.league, m.away.team, false, m.away.winner)}
    {#if m.events.length}
      <div class="evs">
        <div class="ev home">
          {#each sideEvents(m, "home") as e, i (i)}<span class:red={e.kind === "red"}>{e.player || "?"}{evNote(e.kind)}{#if e.minute}{" "}<b class="tnum">{e.minute}</b>{/if} {evIcon(e.kind)}</span>{/each}
        </div>
        <div class="ev away">
          {#each sideEvents(m, "away") as e, i (i)}<span class:red={e.kind === "red"}>{evIcon(e.kind)}{#if e.minute}{" "}<b class="tnum">{e.minute}</b>{/if} {e.player || "?"}{evNote(e.kind)}</span>{/each}
        </div>
      </div>
    {/if}
  </div>
{/snippet}

{#snippet game(m: FootballMatch, teamId: string)}
  {@const o = outcome(m, teamId)}
  <div class="game">
    <span class="gd sub small">{DATE.format(new Date(m.date))}{m.state === "pre" ? ` · ${TIME.format(new Date(m.date))}` : ""}</span>
    {@render teamBtn(m.league, m.home.team, true, m.home.winner)}
    <span class="score tnum sm-score">{#if m.home.score === null}<span class="vs">–</span>{:else}{m.home.score}<i>-</i>{m.away.score}{/if}</span>
    {@render teamBtn(m.league, m.away.team, false, m.away.winner)}
    {#if o}<span class="res {o}">{o}</span>{:else if m.league !== teamOpen?.league}<span class="lgtag sub small">{leagueName(m.league)}</span>{:else}<span></span>{/if}
  </div>
{/snippet}

{#snippet king(title: string, unit: string, list: FootballLeaders["goals"])}
  <div class="king">
    <div class="lgh">{title}</div>
    {#each list as l, i (l.name + i)}
      <div class="kr" class:top={i === 0}>
        <span class="kn tnum">{i + 1}</span>
        {#if l.photo}<img class="face" src={l.photo} alt="" loading="lazy" />{:else}<span class="face no">{initials(l.name)}</span>{/if}
        <div class="pt">
          <b>{l.name}</b>
          <button class="tlink sub small" onclick={() => openTeam(league, l.team)}>
            {#if l.team.logo}<img src={l.team.logo} alt="" />{/if}{l.team.short || l.team.name}
          </button>
        </div>
        <span class="kv tnum">{l.value}<small>{unit}</small></span>
      </div>
    {:else}
      <p class="sub small">Henüz veri yok.</p>
    {/each}
  </div>
{/snippet}

<style>
  .fb { display: grid; grid-template-columns: 220px minmax(0, 1fr); gap: 16px; max-width: 1180px; margin: 0 auto; align-items: start; }
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .main { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  .side { position: sticky; top: 0; gap: 2px; padding: 14px 10px; }
  .brand { display: flex; align-items: center; gap: 8px; padding: 2px 8px 10px; font-family: var(--f-display); font-size: 16px; color: var(--gold-glow); }
  .cat { margin: 12px 8px 4px; font-size: 10.5px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; color: var(--dim); }
  .lg-btn { display: flex; align-items: center; gap: 8px; width: 100%; text-align: left; padding: 8px 10px; border-radius: 9px; border: 0; background: none; color: var(--muted); font: inherit; font-size: 13.5px; cursor: pointer; }
  .lg-btn:hover { background: rgba(255, 255, 255, 0.04); color: var(--text); }
  .lg-btn.on { background: rgba(227, 194, 112, 0.12); color: var(--gold-glow); font-weight: 600; }
  .src { margin: 14px 8px 0; }
  h2 { margin: 0; font-family: var(--f-display); font-size: 22px; }
  .sm { padding: 6px 11px; font-size: 13px; }
  .seg button:disabled { opacity: 0.4; cursor: default; }
  .days { display: flex; gap: 6px; flex-wrap: wrap; align-items: stretch; }
  .day { display: flex; flex-direction: column; align-items: center; gap: 2px; min-width: 60px; padding: 7px 10px; border-radius: 12px; border: 1px solid var(--line-2); background: var(--coal-2); color: var(--muted); cursor: pointer; font: inherit; }
  .day b { font-size: 13px; color: var(--text); font-weight: 600; }
  .day span { font-size: 11.5px; }
  .day.on { border-color: var(--gold); background: rgba(227, 194, 112, 0.12); }
  .day.on b { color: var(--gold-glow); }
  .live-f { margin-left: auto; display: inline-flex; align-items: center; gap: 8px; padding: 7px 14px; border-radius: 12px; border: 1px solid rgba(224, 106, 106, 0.5); background: rgba(224, 106, 106, 0.08); color: #ffb4b4; font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; }
  .live-f.on { background: rgba(224, 106, 106, 0.22); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: #ff5a5a; animation: pulse 1.6s infinite; flex: none; }
  @keyframes pulse { 0% { box-shadow: 0 0 0 0 rgba(255, 90, 90, 0.55); } 70% { box-shadow: 0 0 0 7px rgba(255, 90, 90, 0); } 100% { box-shadow: 0 0 0 0 rgba(255, 90, 90, 0); } }
  .lg { display: flex; flex-direction: column; gap: 4px; }
  .lgh { margin-top: 8px; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--dim); display: flex; align-items: center; gap: 6px; }
  .lgh.link { background: none; border: 0; padding: 0; cursor: pointer; font-family: inherit; align-self: flex-start; }
  .lgh.link:hover { color: var(--gold); }
  .match { display: grid; grid-template-columns: 60px minmax(0, 1fr) 76px minmax(0, 1fr); align-items: center; gap: 4px 12px; padding: 10px 12px; border-radius: 12px; background: rgba(255, 255, 255, 0.025); border: 1px solid transparent; }
  .match:hover { background: rgba(255, 255, 255, 0.045); }
  .match.live { border-color: rgba(224, 106, 106, 0.35); }
  .st { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--muted); }
  .match.live .st { color: #ff8a8a; font-weight: 700; }
  .fin { color: var(--dim); font-weight: 600; }
  .tm { display: flex; align-items: center; gap: 10px; min-width: 0; background: none; border: 0; padding: 4px 6px; border-radius: 8px; color: inherit; font: inherit; cursor: pointer; }
  .tm:hover { background: rgba(255, 255, 255, 0.05); }
  .tm:hover .nm { color: var(--gold-glow); }
  .tm.home { justify-content: flex-end; text-align: right; }
  .tm img, .tm .ph { width: 24px; height: 24px; object-fit: contain; flex: none; }
  .ph { display: grid; place-items: center; border-radius: 50%; background: rgba(255, 255, 255, 0.07); font-size: 9.5px; font-weight: 700; color: var(--muted); }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
  .tm.win .nm { font-weight: 700; }
  .score { justify-self: center; display: inline-flex; align-items: center; gap: 6px; min-width: 64px; justify-content: center; padding: 4px 10px; border-radius: 8px; background: rgba(0, 0, 0, 0.3); font-size: 17px; font-weight: 700; }
  .match.live .score { color: #ffd2d2; }
  .score i { font-style: normal; color: var(--dim); font-weight: 400; }
  .vs { color: var(--dim); font-weight: 400; }
  .evs { grid-column: 2 / 5; display: grid; grid-template-columns: minmax(0, 1fr) 76px minmax(0, 1fr); gap: 0 12px; font-size: 12px; color: var(--muted); }
  .ev { display: flex; flex-direction: column; gap: 1px; min-width: 0; padding: 0 6px; }
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
  .tr { display: grid; grid-template-columns: 34px minmax(0, 1fr) repeat(4, 34px) 54px 40px 40px; align-items: center; gap: 6px; padding: 7px 10px; border-radius: 8px; border: 0; border-left: 3px solid var(--c, transparent); font: inherit; font-size: 13.5px; color: inherit; background: none; text-align: left; }
  button.tr { cursor: pointer; }
  .tr:not(.th):nth-child(even) { background: rgba(255, 255, 255, 0.025); }
  button.tr:hover { background: rgba(255, 255, 255, 0.06); }
  button.tr:hover .nm { color: var(--gold-glow); }
  .tr > span:not(.tt), .tr > b { text-align: center; }
  .th { border-left-color: transparent; font-weight: 600; }
  .rk { color: var(--muted); }
  .tt { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .tt img { width: 22px; height: 22px; object-fit: contain; flex: none; }
  .pts { color: var(--gold-glow); }
  .legend { display: flex; flex-wrap: wrap; gap: 6px 16px; margin-top: 6px; }
  .legend span { display: inline-flex; align-items: center; gap: 6px; }
  .legend i { width: 10px; height: 10px; border-radius: 3px; }
  .thead { gap: 14px; }
  .back { align-self: flex-start; display: inline-flex; align-items: center; gap: 6px; background: none; border: 0; padding: 0; color: var(--muted); font: inherit; font-size: 13px; cursor: pointer; }
  .back:hover { color: var(--gold); }
  .tinfo { display: flex; align-items: center; gap: 16px; }
  .tlogo { width: 64px; height: 64px; object-fit: contain; flex: none; }
  .tlogo.ph { border-radius: 50%; display: grid; place-items: center; background: rgba(255, 255, 255, 0.06); font-weight: 700; font-size: 20px; color: var(--muted); }
  .tinfo p { margin: 4px 0 0; }
  .squad { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 6px; }
  .pl { display: flex; align-items: center; gap: 10px; padding: 8px 10px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); min-width: 0; }
  .face { width: 36px; height: 36px; border-radius: 50%; object-fit: cover; flex: none; background: rgba(255, 255, 255, 0.06); }
  .face.no { display: grid; place-items: center; font-weight: 700; font-size: 13px; color: var(--gold); }
  .pt { display: flex; flex-direction: column; min-width: 0; }
  .pt b { font-weight: 600; font-size: 13.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .form { display: flex; align-items: center; gap: 6px; }
  .res { display: inline-grid; place-items: center; min-width: 26px; height: 22px; padding: 0 6px; border-radius: 6px; font-size: 11.5px; font-weight: 700; background: rgba(255, 255, 255, 0.06); color: var(--muted); }
  .res.G { background: rgba(92, 192, 138, 0.18); color: #7fe0a8; }
  .res.B { background: rgba(227, 194, 112, 0.16); color: var(--gold); }
  .res.M { background: rgba(224, 106, 106, 0.18); color: #ff9a9a; }
  .lgtag { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 120px; justify-self: end; }
  .game .res { justify-self: end; }
  .game { display: grid; grid-template-columns: 110px minmax(0, 1fr) 64px minmax(0, 1fr) 120px; align-items: center; gap: 8px; padding: 6px 10px; border-radius: 10px; background: rgba(255, 255, 255, 0.025); }
  .sm-score { min-width: 54px; font-size: 15px; padding: 3px 8px; }
  .kings { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 16px; }
  .king { display: flex; flex-direction: column; gap: 4px; }
  .kr { display: grid; grid-template-columns: 24px 40px minmax(0, 1fr) auto; align-items: center; gap: 10px; padding: 8px 10px; border-radius: 10px; background: rgba(255, 255, 255, 0.025); }
  .kr.top { background: rgba(227, 194, 112, 0.1); border: 1px solid rgba(227, 194, 112, 0.35); }
  .kn { color: var(--muted); font-weight: 700; text-align: center; }
  .kr .face { width: 40px; height: 40px; }
  .tlink { display: inline-flex; align-items: center; gap: 6px; background: none; border: 0; padding: 0; font: inherit; cursor: pointer; align-self: flex-start; }
  .tlink:hover { color: var(--gold); }
  .tlink img { width: 16px; height: 16px; object-fit: contain; }
  .kv { font-size: 20px; font-weight: 800; color: var(--gold-glow); display: inline-flex; align-items: baseline; gap: 4px; }
  .kv small { font-size: 11px; font-weight: 600; color: var(--dim); }
  @media (max-width: 900px) {
    .fb { grid-template-columns: 1fr; }
    .side { position: static; flex-direction: row; flex-wrap: wrap; gap: 6px; padding: 10px; }
    .brand, .cat, .src { display: none; }
    .lg-btn { width: auto; border: 1px solid var(--line-2); border-radius: 999px; padding: 6px 12px; }
    .match { grid-template-columns: 52px minmax(0, 1fr) 64px minmax(0, 1fr); gap: 4px 8px; }
    .tr { grid-template-columns: 28px minmax(0, 1fr) 28px 28px 28px 28px 0 0 36px; }
    .tr > :nth-child(7), .tr > :nth-child(8) { display: none; }
    .game { grid-template-columns: 80px minmax(0, 1fr) 54px minmax(0, 1fr) 34px; }
  }
</style>
