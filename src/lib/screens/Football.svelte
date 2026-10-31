<script lang="ts">
  // Futbol: solda ligler (Türkiye · Avrupa kupaları · Avrupa ligleri), sağda seçilen ligin maçları, puan durumu ve
  // gol/asist krallığı. Takıma tıklayınca takım sayfası açılır: kadro (mevkiye göre), son maçlar ve fikstür.
  // Canlı maç varken skorlar 30 sn'de bir, yoksa 5 dk'da bir kendiliğinden yenilenir.
  import { onDestroy, onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import Switch from "../components/Switch.svelte";
  import { FRESH, peek, store } from "../footballCache";
  import { getJson, setJson } from "../settings.svelte";
  import type { FootballFav, FootballGroup, FootballLeaders, FootballMatch, FootballTeam, FootballTeamPage } from "../types";

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
  /** Seçili hafta: bu haftadan kaç hafta sonra (geçmiş: eksi). */
  let week = $state(0);
  /** Hafta içinde tek bir güne süzgeç (YYYYMMDD) ya da hepsi. */
  let dayF = $state<string | null>(null);
  let liveOnly = $state(false);

  let matches = $state<FootballMatch[]>([]);
  let groups = $state<FootballGroup[]>([]);
  let leaders = $state<FootballLeaders | null>(null);
  let teamPage = $state<FootballTeamPage | null>(null);
  /** Açık takım sayfası (yükleniyorken de). */
  let teamOpen = $state<{ league: string; team: FootballTeam } | null>(null);
  let teamTab = $state<"squad" | "games">("squad");
  let loading = $state(false);
  /** Ekranda önbellekteki veri var, arkada tazeleniyor. */
  let refreshing = $state(false);
  /** "Yenile" düğmesi: önbellek taze olsa da sor. */
  let force = false;
  let error = $state("");
  let updated = $state<Date | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  /** Favori takımlar: maçları başlayınca, gol olunca ve bitince bildirilir; oyun içi panelde skoru görünür. */
  let favs = $state(getJson<FootballFav[]>("football.favs", []));
  let notify = $state(getJson<boolean>("football.notify", true));
  let inPanel = $state(getJson<boolean>("panel.football", true));
  const isFav = (id: string) => !!id && favs.some((f) => f.id === id);
  async function toggleFav(lg: string, t: FootballTeam) {
    if (!t.id) return;
    favs = isFav(t.id) ? favs.filter((f) => f.id !== t.id) : [...favs, { league: lg, id: t.id, name: t.name, logo: t.logo }];
    await setJson("football.favs", $state.snapshot(favs));
    void api.football.refreshLive().catch(() => {});
  }
  function setNotify(v: boolean) {
    notify = v;
    void setJson("football.notify", v);
  }
  function setInPanel(v: boolean) {
    inPanel = v;
    void setJson("panel.football", v);
  }

  const WEEKDAY = new Intl.DateTimeFormat("tr-TR", { weekday: "short" });
  const DAYHEAD = new Intl.DateTimeFormat("tr-TR", { weekday: "long", day: "numeric", month: "long" });
  const SHORTDATE = new Intl.DateTimeFormat("tr-TR", { day: "numeric", month: "long" });
  const TIME = new Intl.DateTimeFormat("tr-TR", { hour: "2-digit", minute: "2-digit" });
  const DATE = new Intl.DateTimeFormat("tr-TR", { day: "numeric", month: "short", weekday: "short" });

  function dayAt(off: number): Date {
    const d = new Date();
    d.setHours(12, 0, 0, 0);
    d.setDate(d.getDate() + off);
    return d;
  }
  const ymd = (d: Date) => `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}`;
  /** Seçili haftanın pazartesisi (öğlen). */
  function monday(w: number): Date {
    const d = dayAt(0);
    d.setDate(d.getDate() - ((d.getDay() + 6) % 7) + w * 7);
    return d;
  }
  const plus = (d: Date, n: number) => {
    const x = new Date(d);
    x.setDate(x.getDate() + n);
    return x;
  };
  const localKey = (iso: string) => ymd(new Date(iso));
  const weekTitle = $derived.by(() => {
    const a = monday(week);
    const b = plus(a, 6);
    const span = a.getMonth() === b.getMonth() ? `${a.getDate()}–${SHORTDATE.format(b)}` : `${SHORTDATE.format(a)} – ${SHORTDATE.format(b)}`;
    const name = week === 0 ? "Bu hafta" : week === -1 ? "Geçen hafta" : week === 1 ? "Gelecek hafta" : "";
    return [name, span] as const;
  });
  function dayName(key: string): string {
    const today = ymd(dayAt(0));
    if (key === today) return "Bugün";
    if (key === ymd(dayAt(1))) return "Yarın";
    if (key === ymd(dayAt(-1))) return "Dün";
    const d = new Date(+key.slice(0, 4), +key.slice(4, 6) - 1, +key.slice(6, 8), 12);
    return DAYHEAD.format(d);
  }
  const chipDay = (key: string) => {
    const d = new Date(+key.slice(0, 4), +key.slice(4, 6) - 1, +key.slice(6, 8), 12);
    return [WEEKDAY.format(d), `${d.getDate()}.${String(d.getMonth() + 1).padStart(2, "0")}`];
  };
  /** Kenar menüde olmayan, takım sayfasında görünen kupalar. */
  const CUPS: Record<string, string> = {
    "tur.cup": "Türkiye Kupası",
    "eng.fa": "FA Cup",
    "eng.league_cup": "Lig Kupası",
    "esp.copa_del_rey": "Kral Kupası",
    "ita.coppa_italia": "İtalya Kupası",
    "ger.dfb_pokal": "Almanya Kupası",
    "fra.coupe_de_france": "Fransa Kupası",
    "ned.cup": "Hollanda Kupası",
    "por.taca.portugal": "Portekiz Kupası",
    "uefa.super_cup": "UEFA Süper Kupa",
  };
  const leagueName = (id: string) => (id === "all" ? "Tüm maçlar" : (leagues.find(([s]) => s === id)?.[1] ?? CUPS[id] ?? id));

  /**
   * Önbellekte varsa hemen gösterir; tazeyse (süresi `fresh` ms) hiç sormaz, değilse arkada sorar ve yalnız
   * değiştiyse ekrana yansıtır. Önbellekte yoksa iskelet (yükleniyor) görünür.
   */
  async function guard<T>(key: string, fresh: number | ((v: T) => number), work: () => Promise<T>, apply: (v: T) => void, quiet = false, blank?: () => void) {
    const my = ++seq;
    const hit = peek<T>(key);
    const forced = force;
    force = false;
    if (hit) {
      apply(hit.data);
      error = "";
      loading = false;
      updated = new Date(Date.now() - hit.age);
      const ttl = typeof fresh === "function" ? fresh(hit.data) : fresh;
      if (hit.age < ttl && !forced) {
        refreshing = false;
        return;
      }
      refreshing = true;
    } else if (!quiet) {
      // Elde yoksa başka lig/haftanın eski verisi görünmesin: boşalt, iskelet dönsün.
      blank?.();
      loading = true;
    } else refreshing = true;
    try {
      const v = await work();
      if (my !== seq) return;
      if (store(key, v) || !hit) apply(v);
      error = "";
      updated = new Date();
    } catch (e) {
      // Elde veri varsa onu göstermeye devam et (internet gidip gelse de ekran boşalmasın).
      if (my === seq && !hit) error = String(e);
    } finally {
      if (my === seq) {
        loading = false;
        refreshing = false;
      }
    }
  }

  function loadMatches(quiet = false) {
    const mon = monday(week);
    const keys = new Set(Array.from({ length: 7 }, (_, i) => ymd(plus(mon, i))));
    // Yerel gün, sitenin gün sınırıyla tam örtüşmeyebilir: bir gün öncesi de istenir, yerel haftaya göre süzülür.
    const want = league === "all" ? leagues.map(([s]) => s) : [league];
    const from = ymd(plus(mon, -1));
    const to = ymd(plus(mon, 6));
    const w = week;
    return guard(
      `w:${want.join(",")}:${from}`,
      (list: FootballMatch[]) =>
        w < 0 ? FRESH.past : w > 0 ? FRESH.future : list.some((m) => m.state === "in") ? FRESH.live : FRESH.today,
      () => api.football.week(want, from, to),
      (list) => (matches = list.filter((m) => keys.has(localKey(m.date)))),
      quiet,
      () => (matches = []),
    ).then(schedule);
  }
  const loadTable = () => guard(`t:${league}`, FRESH.table, () => api.football.standings(league), (g) => (groups = g), false, () => (groups = []));
  const loadLeaders = () => guard(`l:${league}`, FRESH.leaders, () => api.football.leaders(league), (l) => (leaders = l), false, () => (leaders = null));

  /** Canlı maç varken 30 sn, bugünse 5 dk; diğer günlerde kendiliğinden yenilenmez. */
  function schedule() {
    clearTimeout(timer);
    if (tab !== "matches" || teamOpen) return;
    const live = matches.some((m) => m.state === "in");
    const wait = live ? 30_000 : week === 0 ? 300_000 : 0;
    if (wait)
      timer = setTimeout(() => {
        force = true;
        void loadMatches(true);
      }, wait);
  }

  function reload() {
    if (teamOpen) return void openTeam(teamOpen.league, teamOpen.team);
    if (tab === "matches" || league === "all") void loadMatches();
    else if (tab === "table") void loadTable();
    else void loadLeaders();
  }

  onMount(async () => {
    leagues = await api.football.leagues().catch(() => []);
    // Panelden × ile kapatılmış olabilir: anahtarın gerçek hâli.
    void api.football.live().then((r) => (inPanel = r.show)).catch(() => {});
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
  function setWeek(w: number) {
    week = w;
    dayF = null;
    liveOnly = false;
    void loadMatches();
  }
  function openTeam(lg: string, team: FootballTeam) {
    if (!team.id) return;
    clearTimeout(timer);
    teamOpen = { league: lg, team };
    teamPage = null;
    teamTab = "squad";
    void guard(
      `tm:${lg}:${team.id}`,
      FRESH.team,
      () => api.football.team(lg, team.id),
      (p) => {
        teamPage = p;
        // Kupa maçından açıldıysa takımın asıl ligi (favori ve başlık için).
        if (teamOpen && p.league !== teamOpen.league) teamOpen = { ...teamOpen, league: p.league };
      },
    );
  }
  function closeTeam() {
    teamOpen = null;
    error = "";
    reload();
  }

  const shown = $derived(
    matches.filter((m) => (!liveOnly || m.state === "in") && (!dayF || localKey(m.date) === dayF)),
  );
  /** Haftanın maçlı günleri ve maç sayıları. */
  const dayCounts = $derived.by(() => {
    const c = new Map<string, number>();
    for (const m of matches) c.set(localKey(m.date), (c.get(localKey(m.date)) ?? 0) + 1);
    return [...c].sort(([a], [b]) => a.localeCompare(b));
  });
  const perDay = $derived(new Map(dayCounts));
  /** Seçili haftanın yedi günü (maçsız günler de görünsün, takvim gibi). */
  const weekDays = $derived(Array.from({ length: 7 }, (_, i) => ymd(plus(monday(week), i))));
  const todayKey = ymd(dayAt(0));
  const liveCount = $derived(matches.filter((m) => m.state === "in").length);
  /** Gün gün, her günde lig lig. */
  const byDay = $derived.by(() => {
    const order = leagues.map(([s]) => s);
    const days: [string, [string, FootballMatch[]][]][] = [];
    for (const m of shown) {
      const k = localKey(m.date);
      let d = days.find(([x]) => x === k);
      if (!d) days.push((d = [k, []]));
      let g = d[1].find(([l]) => l === m.league);
      if (!g) d[1].push((g = [m.league, []]));
      g[1].push(m);
    }
    for (const [, gs] of days) gs.sort(([a], [b]) => order.indexOf(a) - order.indexOf(b));
    return days.sort(([a], [b]) => a.localeCompare(b));
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
    <div class="cat">Favorilerim</div>
    {#each favs as f (f.id)}
      <button class="lg-btn" class:on={teamOpen?.team.id === f.id} onclick={() => openTeam(f.league, { id: f.id, name: f.name, short: f.name, logo: f.logo })}>
        {#if f.logo}<img class="flogo" src={f.logo} alt="" />{:else}<span class="star">★</span>{/if}<span class="fname">{f.name}</span>
      </button>
    {:else}
      <p class="sub small hint">Takım sayfasındaki ☆ ile ekle; maçı başlayınca, gol olunca ve bitince haber veririm.</p>
    {/each}
    <label class="opt"><span>Gol bildirimi</span><Switch checked={notify} label="Gol bildirimi" onchange={setNotify} /></label>
    <label class="opt"><span>Oyun içi panelde skor</span><Switch checked={inPanel} label="Oyun içi panelde skor" onchange={setInPanel} /></label>
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
          <button class="btn ghost sm favb" class:on={isFav(teamOpen.team.id)} onclick={() => teamOpen && toggleFav(teamOpen.league, { ...teamOpen.team, name: tp?.team.name || teamOpen.team.name, logo: tp?.team.logo || teamOpen.team.logo })}>
            <span class="star">{isFav(teamOpen.team.id) ? "★" : "☆"}</span>{isFav(teamOpen.team.id) ? "Favoride" : "Favorilere ekle"}
          </button>
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
          {@render skel("team")}
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
          <!-- En son oynanan en üstte, aşağı indikçe eskiler; gelecek maçlar en altta. Lig, Avrupa ve kupa maçları bir arada. -->
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
          {#if tp.fixtures.length}
            <div class="lgh">Sıradaki maçlar</div>
            {#each tp.fixtures as m (m.id)}{@render game(m, tp.team.id)}{/each}
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
            <button class="btn ghost sm" class:spin={loading || refreshing} onclick={() => ((force = true), reload())} disabled={loading || refreshing} title="Yenile"><Icon name="refresh" size={15} />{loading || refreshing ? "Güncelleniyor…" : "Yenile"}</button>
          </div>
        </div>
      </section>

      {#if tab === "matches"}
        <section class="panel">
          <div class="wk">
            <div class="wk-top">
              <div class="wsel">
                <button class="wnav" onclick={() => setWeek(week - 1)} title="Önceki hafta" aria-label="Önceki hafta"><Icon name="back" size={15} /></button>
                <div class="wtitle">
                  <b>{weekTitle[0] || "Hafta"}</b>
                  <span class="tnum">{weekTitle[1]}</span>
                </div>
                <button class="wnav" onclick={() => setWeek(week + 1)} title="Sonraki hafta" aria-label="Sonraki hafta"><span class="flip"><Icon name="back" size={15} /></span></button>
              </div>
              {#if week !== 0}<button class="wback" onclick={() => setWeek(0)}><Icon name="replay" size={13} />Bu haftaya dön</button>{/if}
              {#if liveCount}
                <button class="live-f" class:on={liveOnly} onclick={() => (liveOnly = !liveOnly)} aria-pressed={liveOnly}><span class="dot"></span>Canlı<b class="tnum">{liveCount}</b></button>
              {/if}
            </div>
            {#if matches.length}
              <div class="strip" role="group" aria-label="Gün">
                <button class="dc all" class:on={!dayF} onclick={() => (dayF = null)}><span class="wd">Tümü</span><b class="tnum">{matches.length}</b><span class="n">maç</span></button>
                {#each weekDays as k (k)}
                  {@const n = perDay.get(k) ?? 0}
                  {@const [wd] = chipDay(k)}
                  <button class="dc" class:on={dayF === k} class:today={k === todayKey} disabled={!n} onclick={() => (dayF = dayF === k ? null : k)} title={dayName(k)}>
                    <span class="wd">{k === todayKey ? "Bugün" : wd}</span>
                    <b class="tnum">{+k.slice(6, 8)}</b>
                    <span class="n tnum">{n ? `${n} maç` : "–"}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
          {#if error}
            {@render failed("Maçlar alınamadı")}
          {:else if loading && !matches.length}
            {@render skel("matches")}
          {:else if !shown.length}
            <p class="sub empty">{liveOnly ? "Şu an oynanan maç yok." : "Bu hafta maç yok."}</p>
          {:else}
            {#each byDay as [k, gs] (k)}
              <div class="dayh" class:today={k === ymd(dayAt(0))}>
                <span>{dayName(k)}</span>
                <i class="tnum">{gs.reduce((n, [, l]) => n + l.length, 0)} maç</i>
              </div>
              {#each gs as [lg, list] (lg)}
                <div class="lg">
                  {#if league === "all"}<button class="lgh link" onclick={() => setLeague(lg)}>{leagueName(lg)} <span class="cnt">{list.length}</span><Icon name="next" size={12} /></button>{/if}
                  <div class="cards">
                    {#each list as m (m.id)}{@render matchRow(m)}{/each}
                  </div>
                </div>
              {/each}
            {/each}
          {/if}
          {#if updated}<p class="sub small upd">Son güncelleme {TIME.format(updated)}{liveCount ? " · canlı maçlar 30 sn'de bir yenilenir" : ""} · takıma tıkla: kadro ve maçları</p>{/if}
        </section>
      {:else if tab === "table"}
        <section class="panel">
          {#if error}
            {@render failed("Puan durumu alınamadı")}
          {:else if loading && !groups.length}
            {@render skel("table")}
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
            {@render skel("table")}
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
  {@const live = m.state === "in"}
  {@const fav = isFav(m.home.team.id) || isFav(m.away.team.id)}
  <article class="mc" class:live class:fav class:post={m.state === "post"}>
    <div class="mc-top">
      {#if live}
        <span class="pill live"><span class="dot"></span>{m.status || "Canlı"}</span>
      {:else if m.home.score === null}
        <span class="pill">{m.status && m.status !== TIME.format(new Date(m.date)) ? m.status : TIME.format(new Date(m.date))}</span>
      {:else}
        <span class="pill done">{m.status || "MS"}</span>
      {/if}
      {#if fav}<span class="favtag" title="Favori takımın"><Icon name="star" size={12} /></span>{/if}
    </div>
    {@render side(m, m.home, m.away)}
    {@render side(m, m.away, m.home)}
    {#if m.events.length}
      <div class="mc-ev">
        {#each m.events as e, i (i)}
          {@const t = e.side === "home" ? m.home.team : m.away.team}
          <span class:red={e.kind === "red"} title={t.name}>
            {#if t.logo}<img src={t.logo} alt="" loading="lazy" />{/if}
            <i>{evIcon(e.kind)}</i>{e.player || "?"}{evNote(e.kind)}{#if e.minute}<b class="tnum">{e.minute}</b>{/if}
          </span>
        {/each}
      </div>
    {/if}
  </article>
{/snippet}

{#snippet side(m: FootballMatch, s: FootballMatch["home"], o: FootballMatch["home"])}
  {@const lose = m.state === "post" && s.score !== null && o.score !== null && s.score < o.score}
  <div class="mc-row" class:win={s.winner || (m.state === "post" && s.score !== null && o.score !== null && s.score > o.score)} class:lose>
    <button class="mc-team" onclick={() => openTeam(m.league, s.team)} title="{s.team.name} · kadro ve maçlar">
      {#if s.team.logo}<img src={s.team.logo} alt="" loading="lazy" />{:else}<span class="ph">{initials(s.team.name)}</span>{/if}
      <span class="nm">{s.team.name}</span>
      {#if isFav(s.team.id)}<Icon name="star" size={11} />{/if}
    </button>
    <span class="mc-score tnum">{s.score ?? ""}</span>
  </div>
{/snippet}

{#snippet skel(kind: "matches" | "table" | "team")}
  <div class="skel" aria-label="Yükleniyor" aria-busy="true">
    {#if kind === "matches"}
      <div class="sk-line w30"></div>
      <div class="cards">
        {#each Array(6) as _, i (i)}
          <div class="mc sk">
            <div class="sk-line w20"></div>
            <div class="sk-team"><span class="sk-dot"></span><span class="sk-line w60"></span><span class="sk-line w8"></span></div>
            <div class="sk-team"><span class="sk-dot"></span><span class="sk-line w50"></span><span class="sk-line w8"></span></div>
          </div>
        {/each}
      </div>
    {:else if kind === "table"}
      {#each Array(10) as _, i (i)}
        <div class="sk-team row"><span class="sk-line w4"></span><span class="sk-dot"></span><span class="sk-line w40"></span><span class="sk-line w20 r"></span></div>
      {/each}
    {:else}
      <div class="squad">
        {#each Array(9) as _, i (i)}
          <div class="pl sk"><span class="sk-dot big"></span><span class="sk-col"><span class="sk-line w70"></span><span class="sk-line w40"></span></span></div>
        {/each}
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
    <span class="gend">
      {#if m.league !== teamOpen?.league}<span class="lgtag sub small" title={leagueName(m.league)}>{leagueName(m.league)}</span>{/if}
      {#if o}<span class="res {o}">{o}</span>{/if}
    </span>
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
  .hint { margin: 2px 8px 6px; line-height: 1.45; }
  .flogo { width: 18px; height: 18px; object-fit: contain; flex: none; }
  .fname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .star { color: var(--gold-glow); font-size: 15px; line-height: 1; }
  .opt { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 6px 8px 6px 10px; font-size: 13px; color: var(--muted); cursor: pointer; }
  .favb { margin-left: auto; display: inline-flex; align-items: center; gap: 6px; }
  .favb.on { border-color: var(--gold); color: var(--gold-glow); background: rgba(227, 194, 112, 0.12); }
  h2 { margin: 0; font-family: var(--f-display); font-size: 22px; }
  .sm { padding: 6px 11px; font-size: 13px; }
  .seg button:disabled { opacity: 0.4; cursor: default; }
  .dayh { margin-top: 14px; padding-bottom: 6px; border-bottom: 1px solid var(--line); font-weight: 700; font-size: 14px; color: var(--text); text-transform: capitalize; }
  .dayh.today { color: var(--gold-glow); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: #ff5a5a; animation: pulse 1.6s infinite; flex: none; }
  @keyframes pulse { 0% { box-shadow: 0 0 0 0 rgba(255, 90, 90, 0.55); } 70% { box-shadow: 0 0 0 7px rgba(255, 90, 90, 0); } 100% { box-shadow: 0 0 0 0 rgba(255, 90, 90, 0); } }
  .lg { display: flex; flex-direction: column; gap: 4px; }
  .lgh { margin-top: 8px; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--dim); display: flex; align-items: center; gap: 6px; }
  .lgh.link { background: none; border: 0; padding: 0; cursor: pointer; font-family: inherit; align-self: flex-start; }
  .lgh.link:hover { color: var(--gold); }
  .tm { display: flex; align-items: center; gap: 10px; min-width: 0; background: none; border: 0; padding: 4px 6px; border-radius: 8px; color: inherit; font: inherit; cursor: pointer; }
  .tm:hover { background: rgba(255, 255, 255, 0.05); }
  .tm:hover .nm { color: var(--gold-glow); }
  .tm.home { justify-content: flex-end; text-align: right; }
  .tm img, .tm .ph { width: 24px; height: 24px; object-fit: contain; flex: none; }
  .ph { display: grid; place-items: center; border-radius: 50%; background: rgba(255, 255, 255, 0.07); font-size: 9.5px; font-weight: 700; color: var(--muted); }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
  .tm.win .nm { font-weight: 700; }
  .score { justify-self: center; display: inline-flex; align-items: center; gap: 6px; min-width: 64px; justify-content: center; padding: 4px 10px; border-radius: 8px; background: rgba(0, 0, 0, 0.3); font-size: 17px; font-weight: 700; }
  .score i { font-style: normal; color: var(--dim); font-weight: 400; }
  .vs { color: var(--dim); font-weight: 400; }
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
  .lgtag { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; }
  .gend { display: flex; align-items: center; justify-content: flex-end; gap: 6px; min-width: 0; }
  .game { display: grid; grid-template-columns: 110px minmax(0, 1fr) 64px minmax(0, 1fr) 160px; align-items: center; gap: 8px; padding: 6px 10px; border-radius: 10px; background: rgba(255, 255, 255, 0.025); }
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
    .tr { grid-template-columns: 28px minmax(0, 1fr) 28px 28px 28px 28px 0 0 36px; }
    .tr > :nth-child(7), .tr > :nth-child(8) { display: none; }
    .game { grid-template-columns: 80px minmax(0, 1fr) 54px minmax(0, 1fr) 34px; }
  }
  /* ---- hafta seçici ---- */
  .wk { display: flex; flex-direction: column; gap: 10px; }
  .wk-top { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .wsel { display: inline-flex; align-items: center; gap: 2px; padding: 3px; border-radius: 12px; border: 1px solid var(--line); background: rgba(0, 0, 0, 0.18); }
  .wnav { display: grid; place-items: center; width: 32px; height: 32px; border-radius: 9px; border: 0; background: none; color: var(--muted); cursor: pointer; transition: background 0.15s, color 0.15s; }
  .wnav:hover { background: rgba(255, 255, 255, 0.07); color: var(--gold-glow); }
  .flip { display: grid; transform: scaleX(-1); }
  .wtitle { display: flex; align-items: baseline; gap: 8px; padding: 0 10px; min-width: 190px; justify-content: center; }
  .wtitle b { font-family: var(--f-display); font-size: 15px; font-weight: 700; color: var(--text); }
  .wtitle span { font-size: 12.5px; color: var(--dim); }
  .wback { display: inline-flex; align-items: center; gap: 6px; padding: 7px 12px; border-radius: 10px; border: 1px solid var(--line); background: none; color: var(--muted); font: inherit; font-size: 12.5px; cursor: pointer; }
  .wback:hover { color: var(--gold-glow); border-color: rgba(227, 194, 112, 0.4); }
  .live-f { margin-left: auto; display: inline-flex; align-items: center; gap: 8px; padding: 6px 8px 6px 12px; border-radius: 999px; border: 1px solid rgba(224, 106, 106, 0.4); background: rgba(224, 106, 106, 0.07); color: #ffb4b4; font: inherit; font-size: 12.5px; font-weight: 600; cursor: pointer; }
  .live-f b { min-width: 20px; padding: 1px 6px; border-radius: 999px; background: rgba(224, 106, 106, 0.25); color: #ffe1e1; font-size: 11.5px; text-align: center; }
  .live-f:hover, .live-f.on { background: rgba(224, 106, 106, 0.2); border-color: rgba(224, 106, 106, 0.65); }
  .strip { display: grid; grid-template-columns: repeat(8, minmax(0, 1fr)); gap: 4px; padding: 4px; border-radius: 14px; background: rgba(0, 0, 0, 0.18); border: 1px solid var(--line); }
  .dc { position: relative; display: flex; flex-direction: column; align-items: center; gap: 1px; padding: 7px 4px 8px; border-radius: 10px; border: 1px solid transparent; background: none; color: var(--muted); font: inherit; cursor: pointer; transition: background 0.15s, color 0.15s; }
  .dc .wd { font-size: 10.5px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--dim); }
  .dc b { font-size: 18px; font-weight: 700; line-height: 1.2; color: var(--text); }
  .dc .n { font-size: 11px; color: var(--dim); }
  .dc:not(:disabled):hover { background: rgba(255, 255, 255, 0.05); }
  .dc:disabled { cursor: default; opacity: 0.35; }
  .dc.today .wd { color: var(--gold); }
  .dc.today::after { content: ""; position: absolute; top: 5px; right: 7px; width: 5px; height: 5px; border-radius: 50%; background: var(--gold); }
  .dc.on { background: rgba(227, 194, 112, 0.13); border-color: rgba(227, 194, 112, 0.45); }
  .dc.on b, .dc.on .wd { color: var(--gold-glow); }
  .dc.on .n { color: var(--gold); }
  .dc.all b { color: var(--gold-glow); }
  @media (max-width: 640px) { .strip { grid-template-columns: repeat(4, minmax(0, 1fr)); } .wtitle { min-width: 0; } }
  /* ---- maç kartları ---- */
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(290px, 1fr)); gap: 10px; }
  .mc { position: relative; display: flex; flex-direction: column; gap: 4px; padding: 10px 12px 11px; border-radius: 14px; border: 1px solid var(--line); background: linear-gradient(180deg, rgba(255, 255, 255, 0.035), rgba(255, 255, 255, 0.015)); transition: border-color 0.15s, transform 0.15s, background 0.15s; }
  .mc:hover { border-color: var(--line-2); transform: translateY(-1px); }
  .mc.live { border-color: rgba(224, 106, 106, 0.45); background: linear-gradient(180deg, rgba(224, 106, 106, 0.08), rgba(255, 255, 255, 0.015)); }
  .mc.fav:not(.live) { border-color: rgba(227, 194, 112, 0.35); }
  .mc-top { display: flex; align-items: center; justify-content: space-between; margin-bottom: 2px; }
  .pill { display: inline-flex; align-items: center; gap: 6px; padding: 2px 9px; border-radius: 999px; font-size: 11.5px; font-weight: 700; background: rgba(255, 255, 255, 0.06); color: var(--muted); }
  .pill.done { color: var(--dim); }
  .pill.live { background: rgba(224, 106, 106, 0.16); color: #ff9a9a; }
  .pill .dot { width: 7px; height: 7px; border-radius: 50%; background: #ff5f5f; animation: pulse 1.4s ease-in-out infinite; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  .favtag { color: var(--gold); display: grid; place-items: center; }
  .mc-row { display: flex; align-items: center; gap: 8px; }
  .mc-team { flex: 1; min-width: 0; display: flex; align-items: center; gap: 10px; border: 0; background: none; padding: 4px 4px; margin-left: -4px; border-radius: 8px; color: inherit; font: inherit; text-align: left; cursor: pointer; }
  .mc-team:hover { background: rgba(255, 255, 255, 0.05); }
  .mc-team:hover .nm { color: var(--gold-glow); }
  .mc-team img, .mc-team .ph { width: 22px; height: 22px; object-fit: contain; flex: none; }
  .mc-team .ph { display: grid; place-items: center; border-radius: 50%; background: var(--coal); font-size: 10px; font-weight: 700; color: var(--muted); }
  .mc-team .nm { font-size: 14px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mc-team :global(svg) { color: var(--gold); flex: none; }
  .mc-score { min-width: 26px; text-align: right; font-size: 17px; font-weight: 800; }
  .mc-row.lose .nm, .mc-row.lose .mc-score { color: var(--dim); font-weight: 500; }
  .mc-row.win .nm { font-weight: 700; }
  .mc.live .mc-score { color: #ffd2d2; }
  .mc-ev { display: flex; flex-wrap: wrap; gap: 4px 10px; margin-top: 4px; padding-top: 7px; border-top: 1px dashed rgba(255, 255, 255, 0.08); font-size: 11.5px; color: var(--muted); }
  .mc-ev span { display: inline-flex; align-items: center; gap: 4px; white-space: nowrap; }
  .mc-ev img { width: 13px; height: 13px; object-fit: contain; }
  .mc-ev i { font-style: normal; font-size: 10px; }
  .mc-ev b { color: var(--text); font-weight: 600; margin-left: 2px; }
  .mc-ev .red { color: #ff8a8a; }
  .mc-ev .red i { color: #ff5f5f; }
  .cnt { font-size: 10.5px; color: var(--dim); font-weight: 600; }
  .dayh { display: flex; align-items: baseline; justify-content: space-between; }
  .dayh i { font-style: normal; font-size: 11.5px; font-weight: 600; color: var(--dim); }
  .btn.spin :global(svg) { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* ---- yüklenirken iskelet: içi dolup boşalan ışık ---- */
  .skel { display: flex; flex-direction: column; gap: 10px; }
  .sk { pointer-events: none; }
  .sk-team { display: flex; align-items: center; gap: 10px; padding: 4px 0; }
  .sk-team.row { padding: 8px 4px; border-bottom: 1px solid rgba(255, 255, 255, 0.04); }
  .sk-col { display: flex; flex-direction: column; gap: 6px; flex: 1; }
  .sk-line, .sk-dot {
    display: block; height: 11px; border-radius: 6px;
    background: linear-gradient(90deg, rgba(255, 255, 255, 0.05) 0%, rgba(255, 255, 255, 0.12) 40%, rgba(255, 255, 255, 0.05) 80%);
    background-size: 220% 100%; animation: shimmer 1.3s ease-in-out infinite;
  }
  .sk-dot { width: 22px; height: 22px; border-radius: 50%; flex: none; }
  .sk-dot.big { width: 38px; height: 38px; }
  .w4 { width: 4%; } .w8 { width: 8%; margin-left: auto; } .w20 { width: 20%; } .w30 { width: 30%; } .w40 { width: 40%; }
  .w50 { width: 50%; } .w60 { width: 60%; } .w70 { width: 70%; }
  .sk-line.r { margin-left: auto; }
  @keyframes shimmer { from { background-position: 120% 0; } to { background-position: -120% 0; } }
  @media (prefers-reduced-motion: reduce) { .sk-line, .sk-dot, .pill .dot { animation: none; } }
</style>
