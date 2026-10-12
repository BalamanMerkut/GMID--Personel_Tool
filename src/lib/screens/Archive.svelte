<script lang="ts">
  // Oyun Arşivi: puanladığın oyunlar. Alışveriş sitesi düzeni: solda kartlar, sağda filtre kutusu
  // (tür ve etiket, puan, durum, tavsiye, mağaza; her seçeneğin yanında kaç oyun kalacağı), "Bana bir oyun seç".
  import { onMount, untrack } from "svelte";
  import { focus, takeFocus } from "../focus.svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import RatingCard from "../components/RatingCard.svelte";
  import Icon from "../Icon.svelte";
  import { QUESTIONS, RECOMMEND, STATUS, fmtScore, recommendOf, scoreTone, spoilerParts, statusLabel } from "../review";
  import { getJson, setJson } from "../settings.svelte";
  import { ago, playtime } from "../time";
  import { notify } from "../toast.svelte";
  import type { Game, Review, StoreHit } from "../types";

  type Sort = "score" | "recent" | "played" | "year" | "rated";
  const SORTS: [Sort, string][] = [
    ["score", "Puana göre"],
    ["recent", "Son oynanan"],
    ["played", "Oynama süresi"],
    ["year", "Çıkış yılı"],
    ["rated", "Son puanlanan"],
  ];
  const SCORES = [7, 8, 9];
  const STORES: [string, string][] = [["steam", "Steam"], ["epic", "Epic"], ["riot", "Riot"], ["other", "Diğer"]];
  const storeOf = (g: Game) => (g.source === "steam" || g.source === "epic" || g.source === "riot" ? g.source : "other");

  interface Item {
    g: Game;
    r: Review;
  }

  let games: Game[] = $state([]);
  let reviews: Review[] = $state([]);
  let loaded = $state(false);
  let now = $state(Date.now());

  // Filtreler
  let search = $state("");
  let tags: string[] = $state([]);
  let minScore = $state(0);
  let statuses: string[] = $state([]);
  let recs: string[] = $state([]);
  let stores: string[] = $state([]);
  let allTags = $state(false);
  let drawer = $state(false);
  let sort: Sort = $state(getJson<Sort>("archive.sort", "score"));

  // Pencereler
  let rating: { game: Game; review: Review | null } | null = $state(null);
  let detail: Item | null = $state(null);
  let revealed: number[] = $state([]);
  let adding = $state(false);
  let addTab: "library" | "steam" = $state("library");
  let addSearch = $state("");
  let hits: StoreHit[] = $state([]);
  let searching = $state(false);
  let pick: Item | null = $state(null);
  let rolling = $state(false);

  async function load() {
    const [g, r] = await Promise.all([api.games.list().catch(() => []), api.reviews.list().catch(() => [])]);
    games = g;
    reviews = r;
    loaded = true;
    if (detail) {
      const d = items.find((i) => i.r.id === detail?.r.id);
      detail = d ?? null;
    }
  }

  onMount(() => {
    void load();
    const stops = [on("games-changed", () => void load()), on("reviews-changed", () => void load())];
    const timer = setInterval(() => (now = Date.now()), 60_000);
    return () => {
      clearInterval(timer);
      for (const s of stops) void s.then((f) => f());
    };
  });

  const byId = $derived(new Map(games.map((g) => [g.id, g])));
  const items: Item[] = $derived(reviews.flatMap((r) => (byId.has(r.subjectId) ? [{ g: byId.get(r.subjectId)!, r }] : [])));

  // Aramadan gelen istek: o oyunun puan kartını göster.
  $effect(() => {
    void focus.n;
    if (!loaded) return;
    const id = untrack(() => takeFocus("game"));
    const it = id ? untrack(() => items.find((x) => x.g.id === id)) : null;
    if (it) detail = it;
  });
  const reviewed = $derived(new Set(reviews.map((r) => r.subjectId)));
  const labelsOf = (g: Game) => [...new Set([...g.genres, ...g.tags])];

  type Group = "search" | "tags" | "score" | "status" | "rec" | "store";
  function passes(i: Item, skip?: Group): boolean {
    const q = search.trim().toLocaleLowerCase("tr-TR");
    if (skip !== "search" && q && !`${i.g.name} ${i.g.developer ?? ""} ${i.r.oneLiner ?? ""}`.toLocaleLowerCase("tr-TR").includes(q)) return false;
    if (skip !== "tags" && tags.length) {
      const l = labelsOf(i.g);
      if (!tags.every((t) => l.includes(t))) return false;
    }
    if (skip !== "score" && minScore && (i.r.overall ?? 0) < minScore) return false;
    if (skip !== "status" && statuses.length && !statuses.includes(i.r.status ?? "")) return false;
    if (skip !== "rec" && recs.length && !recs.includes(i.r.recommend ?? "")) return false;
    if (skip !== "store" && stores.length && !stores.includes(storeOf(i.g))) return false;
    return true;
  }

  const playedOf = (g: Game) => Math.max(g.playedSeconds, g.platformMinutes * 60);
  const shown = $derived.by(() => {
    const list = items.filter((i) => passes(i));
    const by: Record<Sort, (a: Item, b: Item) => number> = {
      score: (a, b) => (b.r.overall ?? -1) - (a.r.overall ?? -1) || a.g.name.localeCompare(b.g.name, "tr"),
      recent: (a, b) => (b.g.lastPlayed ?? 0) - (a.g.lastPlayed ?? 0),
      played: (a, b) => playedOf(b.g) - playedOf(a.g),
      year: (a, b) => (b.g.year ?? 0) - (a.g.year ?? 0),
      rated: (a, b) => b.r.updatedAt - a.r.updatedAt,
    };
    return list.sort(by[sort]);
  });

  // Her seçeneğin yanında: seçersen kaç oyun kalır.
  const tagCounts = $derived.by(() => {
    const base = items.filter((i) => passes(i, "tags"));
    const all = new Map<string, number>();
    for (const i of items) for (const t of labelsOf(i.g)) all.set(t, all.get(t) ?? 0);
    for (const i of base) {
      const l = labelsOf(i.g);
      if (!tags.every((t) => l.includes(t))) continue;
      for (const t of l) all.set(t, (all.get(t) ?? 0) + 1);
    }
    // Sıra: arşivde en çok geçenler önce (filtre değişince yer değiştirmesin).
    const freq = new Map<string, number>();
    for (const i of items) for (const t of labelsOf(i.g)) freq.set(t, (freq.get(t) ?? 0) + 1);
    return [...all.entries()].sort((a, b) => (freq.get(b[0]) ?? 0) - (freq.get(a[0]) ?? 0) || a[0].localeCompare(b[0], "tr"));
  });
  const tagList = $derived(allTags ? tagCounts : tagCounts.filter(([t], i) => i < 8 || tags.includes(t)));
  const countFor = (skip: Group, test: (i: Item) => boolean) => items.filter((i) => passes(i, skip) && test(i)).length;
  const active = $derived(tags.length + (minScore ? 1 : 0) + statuses.length + recs.length + stores.length + (search.trim() ? 1 : 0));

  function clearAll() {
    tags = [];
    minScore = 0;
    statuses = [];
    recs = [];
    stores = [];
    search = "";
  }
  const toggle = (list: string[], v: string) => (list.includes(v) ? list.filter((x) => x !== v) : [...list, v]);

  // Puanlanmayı bekleyenler: son 30 günde oynanan, puanı olmayan kütüphane oyunları.
  const waiting = $derived(
    games
      .filter((g) => g.inLibrary && !g.hidden && !reviewed.has(g.id) && g.lastPlayed && now - g.lastPlayed < 30 * 86_400_000)
      .sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0))
      .slice(0, 6),
  );
  const unratedLibrary = $derived(
    games
      .filter((g) => g.inLibrary && !g.hidden && !reviewed.has(g.id))
      .filter((g) => !addSearch.trim() || g.name.toLocaleLowerCase("tr-TR").includes(addSearch.trim().toLocaleLowerCase("tr-TR")))
      .sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0) || a.name.localeCompare(b.name, "tr")),
  );

  const cover = (g: { cover: string | null }) => (!g.cover ? null : /^(https?:|data:)/.test(g.cover) ? g.cover : api.fileSrc(g.cover));

  // ---- Oyun ekleme ----
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  function onAddSearch() {
    if (addTab !== "steam") return;
    clearTimeout(searchTimer);
    const term = addSearch.trim();
    if (term.length < 2) {
      hits = [];
      return;
    }
    searchTimer = setTimeout(async () => {
      searching = true;
      try {
        hits = await api.games.storeSearch(term);
      } catch (e) {
        hits = [];
        notify(`Steam'e ulaşılamadı: ${e}`, "info");
      } finally {
        searching = false;
      }
    }, 400);
  }

  function openAdd() {
    adding = true;
    addSearch = "";
    hits = [];
    addTab = games.some((g) => g.inLibrary && !reviewed.has(g.id)) ? "library" : "steam";
  }

  async function addFromStore(appid: number | null, name: string) {
    try {
      const id = await api.games.addArchive(appid, name);
      await load();
      const g = games.find((x) => x.id === id);
      adding = false;
      if (g) rating = { game: g, review: reviews.find((r) => r.subjectId === g.id) ?? null };
    } catch (e) {
      notify(String(e), "info");
    }
  }

  // ---- Bana bir oyun seç ----
  function roll() {
    const pool = shown.filter((i) => i.r.status !== "dropped" && i.r.recommend !== "no");
    const list = pool.length ? pool : shown;
    if (!list.length) return notify("Bu filtrede oyun yok; birkaç filtreyi kaldır", "info");
    rolling = true;
    let step = 0;
    const steps = Math.min(12, list.length * 3);
    const spin = () => {
      const others = list.length > 1 ? list.filter((i) => i !== pick) : list;
      pick = others[Math.floor(Math.random() * others.length)];
      step++;
      if (step < steps) setTimeout(spin, 50 + step * 12);
      else rolling = false;
    };
    spin();
  }

  async function launch(g: Game) {
    try {
      await api.games.launch(g.id);
    } catch (e) {
      notify(String(e), "info");
    }
  }

  function openDetail(i: Item) {
    detail = i;
    revealed = [];
  }

  const fmtDay = (ms: number) => new Date(ms).toLocaleDateString("tr-TR", { day: "numeric", month: "long", year: "numeric" });
</script>

<div class="arc">
  {#if waiting.length}
    <section class="panel wait">
      <div class="wtx">
        <b>Son oynadıkların puanını bekliyor</b>
        <span class="sub small">5 kısa soru, yarım dakika. Sonra Keşfet'te bulursun.</span>
      </div>
      <div class="wlist">
        {#each waiting as g (g.id)}
          <button class="wg" onclick={() => (rating = { game: g, review: null })} title={`${g.name} puanla`}>
            {#if cover(g)}<img src={cover(g)} alt="" />{:else}<span class="wph">{g.name.slice(0, 2)}</span>{/if}
            <span class="small">{g.name}</span>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <div class="layout">
    <section class="panel main">
      <div class="spread bar">
        <div class="row">
          <h2 class="h2"><Icon name="star" />Oyun Arşivi</h2>
          <span class="sub tnum">{shown.length} oyun</span>
        </div>
        <div class="row">
          <input class="search" type="search" placeholder="Ad, geliştirici, yorum…" bind:value={search} />
          <select bind:value={sort} onchange={() => void setJson("archive.sort", sort)} aria-label="Sırala">
            {#each SORTS as [s, label] (s)}<option value={s}>{label}</option>{/each}
          </select>
          <button class="btn ghost filt" onclick={() => (drawer = true)}><Icon name="list" size={16} />Filtrele{active ? ` · ${active}` : ""}</button>
          <button class="btn ghost" onclick={roll} disabled={!shown.length}>🎲 Bana bir oyun seç</button>
          <button class="btn" onclick={openAdd}><Icon name="plus" size={17} />Oyun puanla</button>
        </div>
      </div>

      {#if active}
        <div class="active">
          {#if search.trim()}<button class="ac" onclick={() => (search = "")}>“{search.trim()}” <Icon name="x" size={12} /></button>{/if}
          {#each tags as t (t)}<button class="ac" onclick={() => (tags = toggle(tags, t))}>{t} <Icon name="x" size={12} /></button>{/each}
          {#if minScore}<button class="ac" onclick={() => (minScore = 0)}>Puan {minScore}+ <Icon name="x" size={12} /></button>{/if}
          {#each statuses as s (s)}<button class="ac" onclick={() => (statuses = toggle(statuses, s))}>{statusLabel(s as never)} <Icon name="x" size={12} /></button>{/each}
          {#each recs as r (r)}<button class="ac" onclick={() => (recs = toggle(recs, r))}>{recommendOf(r as never)?.label} <Icon name="x" size={12} /></button>{/each}
          {#each stores as s (s)}<button class="ac" onclick={() => (stores = toggle(stores, s))}>{STORES.find(([k]) => k === s)?.[1]} <Icon name="x" size={12} /></button>{/each}
          <button class="link" onclick={clearAll}>Temizle</button>
        </div>
      {/if}

      {#if loaded && !items.length}
        <div class="empty">
          <Icon name="star" size={40} />
          <h3>Arşivin boş</h3>
          <p class="sub">
            Oynadığın bir oyunu <b>5 kısa soruyla</b> puanla: oynanış, hikâye, keyif, görsellik, performans. Genel puan
            kendiliğinden hesaplanır. Kütüphanende olmayan (konsolda oynadığın, sildiğin) oyunları Steam'den aratıp ekleyebilirsin.
          </p>
          <button class="btn" onclick={openAdd}><Icon name="plus" size={17} />İlk oyununu puanla</button>
        </div>
      {:else}
        <div class="grid">
          {#each shown as i (i.r.id)}
            {@const rec = recommendOf(i.r.recommend)}
            <button class="card" class:picked={pick === i && !rolling} onclick={() => openDetail(i)} title={i.g.name}>
              <div class="cv">
                {#if cover(i.g)}<img src={cover(i.g)} alt="" loading="lazy" />{:else}<span class="ph">{i.g.name}</span>{/if}
                <span class={`score tone-${scoreTone(i.r.overall)}`}>{i.r.overall == null ? "İstiyorum" : fmtScore(i.r.overall)}</span>
                {#if rec}<span class="rec" title={rec.label}>{rec.icon}</span>{/if}
                {#if i.r.status}<span class="st">{statusLabel(i.r.status)}</span>{/if}
              </div>
              <b class="nm">{i.g.name}</b>
              {#if i.r.oneLiner}<span class="one">“{i.r.oneLiner}”</span>{/if}
              <span class="sub small tnum">{[i.g.year, playedOf(i.g) ? playtime(playedOf(i.g)) : null].filter(Boolean).join(" · ")}</span>
            </button>
          {:else}
            <div class="none">
              <p class="sub">Bu filtrelere uyan oyun yok.</p>
              <button class="link" onclick={clearAll}>Filtreleri temizle</button>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    {#if drawer}<div class="shade" role="presentation" onclick={() => (drawer = false)}></div>{/if}
    <aside class="panel filters" class:open={drawer} aria-label="Filtreler">
      <div class="spread">
        <h3 class="fh">Filtrele</h3>
        {#if active}<button class="link" onclick={clearAll}>Temizle</button>{/if}
      </div>

      <div class="grp">
        <span class="gl">Tür ve etiket</span>
        {#each tagList as [t, n] (t)}
          {@const on = tags.includes(t)}
          <label class="opt" class:dim={!n && !on}>
            <input type="checkbox" checked={on} onchange={() => (tags = toggle(tags, t))} />
            <span>{t}</span><span class="cnt tnum">{n}</span>
          </label>
        {:else}
          <span class="sub small">Puanladıkça türler burada çıkar.</span>
        {/each}
        {#if tagCounts.length > 8}
          <button class="link sm" onclick={() => (allTags = !allTags)}>{allTags ? "Daha az göster" : `Tümünü göster (${tagCounts.length})`}</button>
        {/if}
      </div>

      <div class="grp">
        <span class="gl">Puan</span>
        {#each SCORES as s (s)}
          {@const n = countFor("score", (i) => (i.r.overall ?? 0) >= s)}
          <label class="opt" class:dim={!n && minScore !== s}>
            <input type="radio" name="score" checked={minScore === s} onclick={() => (minScore = minScore === s ? 0 : s)} />
            <span>{s}+</span><span class="cnt tnum">{n}</span>
          </label>
        {/each}
      </div>

      <div class="grp">
        <span class="gl">Durum</span>
        {#each STATUS as s (s.id)}
          {@const n = countFor("status", (i) => i.r.status === s.id)}
          <label class="opt" class:dim={!n && !statuses.includes(s.id)}>
            <input type="checkbox" checked={statuses.includes(s.id)} onchange={() => (statuses = toggle(statuses, s.id))} />
            <span>{s.label}</span><span class="cnt tnum">{n}</span>
          </label>
        {/each}
      </div>

      <div class="grp">
        <span class="gl">Tavsiye</span>
        {#each RECOMMEND as r (r.id)}
          {@const n = countFor("rec", (i) => i.r.recommend === r.id)}
          <label class="opt" class:dim={!n && !recs.includes(r.id)}>
            <input type="checkbox" checked={recs.includes(r.id)} onchange={() => (recs = toggle(recs, r.id))} />
            <span>{r.icon} {r.label}</span><span class="cnt tnum">{n}</span>
          </label>
        {/each}
      </div>

      <div class="grp">
        <span class="gl">Mağaza</span>
        {#each STORES as [k, label] (k)}
          {@const n = countFor("store", (i) => storeOf(i.g) === k)}
          <label class="opt" class:dim={!n && !stores.includes(k)}>
            <input type="checkbox" checked={stores.includes(k)} onchange={() => (stores = toggle(stores, k))} />
            <span>{label}</span><span class="cnt tnum">{n}</span>
          </label>
        {/each}
      </div>

      <button class="btn show" onclick={() => (drawer = false)}>{shown.length} oyunu göster</button>
    </aside>
  </div>
</div>

{#if pick}
  {@const p = pick}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (pick = null)}>
    <div class="dlg panel pickd" role="dialog" aria-label="Bana bir oyun seç">
      <span class="sub small up">🎲 {rolling ? "Seçiliyor…" : "Bugün bunu oyna"}</span>
      <div class="pcv" class:spin={rolling}>
        {#if cover(p.g)}<img src={cover(p.g)} alt="" />{:else}<span class="ph">{p.g.name}</span>{/if}
      </div>
      <h3>{p.g.name}</h3>
      {#if !rolling}
        {#if p.r.overall != null}<span class={`pscore tone-${scoreTone(p.r.overall)}`}>{fmtScore(p.r.overall)} / 10</span>{:else}<span class="pscore tone-none">İstek listende</span>{/if}
        {#if p.r.oneLiner}<span class="one">“{p.r.oneLiner}”</span>{/if}
      {/if}
      <div class="row center">
        <button class="btn ghost" onclick={roll} disabled={rolling}><Icon name="shuffle" size={16} />Başka seç</button>
        <button class="btn ghost" disabled={rolling} onclick={() => { const it = p; openDetail(it); pick = null; }}>Ayrıntılar</button>
        {#if p.g.inLibrary && p.g.launch}<button class="btn" disabled={rolling} onclick={() => launch(p.g)}><Icon name="play" size={16} />Oyna</button>{/if}
      </div>
    </div>
  </div>
{/if}

{#if detail}
  {@const d = detail}
  {@const rec = recommendOf(d.r.recommend)}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (detail = null)}>
    <div class="dlg panel det" role="dialog" aria-label={d.g.name}>
      <div class="dhead">
        {#if cover(d.g)}<img class="dcover" src={cover(d.g)} alt="" />{/if}
        <div class="dtx">
          <div class="spread">
            <h3>{d.g.name}</h3>
            <button class="x" onclick={() => (detail = null)} title="Kapat"><Icon name="x" size={18} /></button>
          </div>
          <span class="sub small">{[d.g.developer, d.g.year, d.g.inLibrary ? "Kütüphanede" : "Kütüphanede değil"].filter(Boolean).join(" · ")}</span>
          {#if labelsOf(d.g).length}<div class="tags">{#each labelsOf(d.g) as t (t)}<span class="chip">{t}</span>{/each}</div>{/if}
          <div class="big">
            {#if d.r.overall != null}<b class={`tone-${scoreTone(d.r.overall)}`}>{fmtScore(d.r.overall)}</b><span class="sub">/ 10</span>{/if}
            {#if rec}<span class="chip">{rec.icon} {rec.label}</span>{/if}
            {#if d.r.status}<span class="chip">{statusLabel(d.r.status)}</span>{/if}
          </div>
          {#if d.r.oneLiner}<span class="one big1">“{d.r.oneLiner}”</span>{/if}
        </div>
      </div>

      {#if d.r.overall != null}
      <div class="bars">
        {#each QUESTIONS as q (q.key)}
          {@const v = d.r.scores[q.key]}
          <div class="br">
            <span class="small">{q.icon} {q.label}</span>
            <span class="track"><span class="fill" style={`width:${v == null ? 0 : v * 10}%`}></span></span>
            <span class="tnum bv">{v ?? "Yok"}</span>
          </div>
        {/each}
      </div>
      {/if}

      <div class="meta small sub">
        {#if playedOf(d.g)}<span>Oynama: <b>{playtime(playedOf(d.g))}</b></span>{/if}
        {#if d.g.lastPlayed}<span>Son oynama: {ago(d.g.lastPlayed, now)}</span>{/if}
        {#if d.r.finishedAt}<span>Bitirme: {fmtDay(d.r.finishedAt)}</span>{/if}
        <span>Puan: {fmtDay(d.r.updatedAt)}</span>
      </div>

      {#if d.r.notes}
        <div class="notes">
          {#each spoilerParts(d.r.notes) as part, k (k)}
            {#if part.spoiler && !revealed.includes(k)}
              <button class="spoil" onclick={() => (revealed = [...revealed, k])} title="Spoiler: göstermek için tıkla">{part.text}</button>
            {:else}
              <span class:sp={part.spoiler}>{part.text}</span>
            {/if}
          {/each}
        </div>
      {/if}

      <div class="foot">
        <span></span>
        <div class="row">
          {#if d.g.inLibrary && d.g.launch}<button class="btn ghost" onclick={() => launch(d.g)}><Icon name="play" size={16} />Oyna</button>{/if}
          <button class="btn" onclick={() => (rating = { game: d.g, review: d.r })}><Icon name="edit" size={16} />Düzenle</button>
        </div>
      </div>
    </div>
  </div>
{/if}

{#if adding}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (adding = false)}>
    <div class="dlg panel" role="dialog" aria-label="Oyun puanla">
      <div class="spread">
        <h3 class="h2"><Icon name="star" />Hangi oyunu puanlıyorsun?</h3>
        <button class="x" onclick={() => (adding = false)} title="Kapat"><Icon name="x" size={18} /></button>
      </div>
      <div class="seg">
        <button aria-pressed={addTab === "library"} onclick={() => (addTab = "library")}>Kütüphanemden</button>
        <button aria-pressed={addTab === "steam"} onclick={() => ((addTab = "steam"), onAddSearch())}>Steam'de ara</button>
      </div>
      <input class="search wide" type="search" placeholder={addTab === "library" ? "Kütüphanede ara…" : "Oyunun adı (ör. Hollow Knight)"} bind:value={addSearch} oninput={onAddSearch} />
      <div class="flist">
        {#if addTab === "library"}
          {#each unratedLibrary as g (g.id)}
            <button class="hit" onclick={() => ((adding = false), (rating = { game: g, review: null }))}>
              {#if cover(g)}<img src={cover(g)} alt="" />{:else}<span class="hph"></span>{/if}
              <span class="htx"><b>{g.name}</b><span class="sub small">{g.lastPlayed ? `son oynama ${ago(g.lastPlayed, now)}` : "henüz oynanmadı"}</span></span>
            </button>
          {:else}
            <p class="sub small pad">Kütüphanende puanlanmamış oyun kalmadı. Başka bir oyun için “Steam'de ara”.</p>
          {/each}
        {:else}
          {#if searching}<p class="sub small pad">Aranıyor…</p>{/if}
          {#each hits as h (h.appid)}
            {@const have = games.find((g) => g.steamAppid === h.appid)}
            <button class="hit" onclick={() => (have ? ((adding = false), (rating = { game: have, review: reviews.find((r) => r.subjectId === have.id) ?? null })) : addFromStore(h.appid, h.name))}>
              {#if h.image}<img class="wide" src={h.image} alt="" />{:else}<span class="hph wide"></span>{/if}
              <span class="htx"><b>{h.name}</b><span class="sub small">{have ? (reviewed.has(have.id) ? "Puanlı · düzenle" : "Kütüphanende") : "Steam"}</span></span>
            </button>
          {/each}
          {#if addSearch.trim().length >= 2 && !searching}
            <button class="hit" onclick={() => addFromStore(null, addSearch.trim())}>
              <span class="hph"><Icon name="plus" size={16} /></span>
              <span class="htx"><b>“{addSearch.trim()}” adıyla ekle</b><span class="sub small">Steam'de yoksa (konsol, Battle.net…): bilgiler aranır, bulunamazsa yalnızca adıyla</span></span>
            </button>
          {:else}
            {#if !hits.length && !searching}<p class="sub small pad">Kütüphanende olmayan bir oyun mu? Adını yaz: kapak ve türler Steam'den gelir.</p>{/if}
          {/if}
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if rating}
  <RatingCard game={rating.game} review={rating.review} onclose={() => (rating = null)} onsaved={() => void load()} ondeleted={() => ((detail = null), void load())} />
{/if}

<style>
  .arc { display: flex; flex-direction: column; gap: 16px; max-width: 1600px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 14px; }
  .wait { flex-direction: row; align-items: center; gap: 18px; border-color: var(--gold-dim); background: linear-gradient(90deg, rgba(227, 194, 112, 0.08), transparent); flex-wrap: wrap; }
  .wtx { display: flex; flex-direction: column; gap: 2px; min-width: 220px; }
  .wtx b { font-family: var(--f-display); font-size: 16px; }
  .wlist { display: flex; gap: 10px; flex-wrap: wrap; flex: 1; }
  .wg { display: flex; align-items: center; gap: 8px; border: 1px solid var(--line); background: rgba(255, 255, 255, 0.03); color: var(--text); border-radius: 10px; padding: 4px 10px 4px 4px; max-width: 220px; }
  .wg:hover { border-color: var(--gold); }
  .wg img, .wph { width: 28px; height: 42px; object-fit: cover; border-radius: 6px; flex: none; background: var(--coal-2); display: grid; place-items: center; font-size: 11px; color: var(--muted); }
  .wg .small { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }

  .layout { display: grid; grid-template-columns: minmax(0, 1fr) 270px; gap: 16px; align-items: start; }
  .bar { flex-wrap: wrap; row-gap: 10px; }
  .bar h2 { margin: 0; }
  .search { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; width: 200px; color: var(--text); }
  .search.wide { width: 100%; }
  select { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; color: var(--text); }
  .filt { display: none; }
  .active { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .ac { display: inline-flex; align-items: center; gap: 5px; border: 1px solid var(--gold-dim); background: rgba(227, 194, 112, 0.12); color: var(--gold-glow); border-radius: 999px; padding: 4px 10px; font-size: 13px; font-weight: 600; }
  .ac:hover { border-color: var(--gold); }

  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 20px 14px; }
  .card { display: flex; flex-direction: column; gap: 3px; text-align: left; border: 0; background: none; padding: 0; color: var(--text); min-width: 0; }
  .cv { position: relative; aspect-ratio: 2 / 3; border-radius: 12px; overflow: hidden; background: linear-gradient(160deg, var(--coal), var(--coal-2)); border: 1px solid var(--line); transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s; }
  .card:hover .cv { transform: translateY(-3px); border-color: var(--gold-dim); box-shadow: 0 12px 26px rgba(0, 0, 0, 0.4); }
  .card.picked .cv { border-color: var(--gold); box-shadow: 0 0 0 2px var(--gold), 0 0 24px rgba(227, 194, 112, 0.35); }
  .cv img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .ph { position: absolute; inset: 0; display: grid; place-items: center; padding: 12px; text-align: center; font-family: var(--f-display); font-weight: 700; color: var(--muted); }
  .score { position: absolute; top: 8px; right: 8px; min-width: 38px; text-align: center; padding: 3px 8px; border-radius: 10px; background: rgba(12, 14, 17, 0.9); font-family: var(--f-display); font-weight: 800; font-size: 17px; border: 1px solid currentColor; }
  .rec { position: absolute; left: 8px; bottom: 8px; font-size: 18px; filter: drop-shadow(0 1px 3px rgba(0, 0, 0, 0.8)); }
  .st { position: absolute; right: 6px; bottom: 6px; background: rgba(12, 14, 17, 0.82); color: var(--muted); font-size: 10.5px; font-weight: 600; padding: 1px 7px; border-radius: 5px; }
  .nm { font-weight: 600; font-size: 14px; margin-top: 6px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .one { font-style: italic; color: var(--muted); font-size: 13px; line-height: 1.35; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .tone-gold { color: var(--gold-glow); }
  .tone-good { color: var(--good); }
  .tone-mid { color: #e0a84a; }
  .tone-low { color: var(--bad); }
  .tone-none { color: var(--teal); }
  .none { grid-column: 1 / -1; display: flex; flex-direction: column; gap: 6px; padding: 24px 0; }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center; padding: 40px 20px; color: var(--muted); }
  .empty h3 { margin: 0; color: var(--text); font-family: var(--f-display); }
  .empty p { max-width: 580px; margin: 0; }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; align-self: flex-start; }
  .link:hover { color: var(--neon); }
  .link.sm { font-size: 13px; }

  .filters { position: sticky; top: 0; gap: 16px; max-height: calc(100vh - 140px); overflow: auto; }
  .fh { margin: 0; font-family: var(--f-display); font-size: 17px; }
  .grp { display: flex; flex-direction: column; gap: 2px; }
  .gl { font-size: 11.5px; font-weight: 700; letter-spacing: 0.07em; text-transform: uppercase; color: var(--dim); margin-bottom: 4px; }
  .opt { display: grid; grid-template-columns: 18px minmax(0, 1fr) auto; gap: 8px; align-items: center; padding: 4px 6px; border-radius: 8px; cursor: pointer; font-size: 14px; }
  .opt:hover { background: rgba(255, 255, 255, 0.04); }
  .opt input { accent-color: var(--gold); width: 15px; height: 15px; margin: 0; }
  .opt.dim { opacity: 0.38; }
  .cnt { color: var(--dim); font-size: 12.5px; }
  .show { display: none; }
  .shade { display: none; }

  .back { position: fixed; inset: 0; z-index: 60; background: rgba(0, 0, 0, 0.55); display: grid; place-items: center; padding: 24px; }
  .dlg { width: min(640px, 100%); max-height: min(780px, 100%); overflow: hidden; background: var(--night-2); box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6); }
  .x { border: 0; background: none; color: var(--muted); padding: 6px; border-radius: 8px; display: grid; }
  .x:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .up { text-transform: uppercase; letter-spacing: 0.08em; font-weight: 700; }
  .pickd { width: min(380px, 100%); align-items: center; text-align: center; }
  .pickd h3 { margin: 0; font-family: var(--f-display); font-size: 22px; }
  .pcv { position: relative; width: 190px; aspect-ratio: 2 / 3; border-radius: 14px; overflow: hidden; border: 1px solid var(--gold-dim); box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5); background: var(--coal-2); }
  .pcv img { width: 100%; height: 100%; object-fit: cover; }
  .pcv.spin { filter: blur(1px) brightness(0.85); }
  .pscore { font-family: var(--f-display); font-weight: 800; font-size: 20px; }
  .center { justify-content: center; flex-wrap: wrap; }

  .det { overflow: auto; width: min(720px, 100%); }
  .dhead { display: flex; gap: 18px; }
  .dcover { width: 140px; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 12px; flex: none; }
  .dtx { flex: 1; display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .dtx h3 { margin: 0; font-family: var(--f-display); font-size: 22px; }
  .tags { display: flex; flex-wrap: wrap; gap: 6px; }
  .big { display: flex; align-items: baseline; gap: 8px; flex-wrap: wrap; }
  .big b { font-family: var(--f-display); font-size: 40px; line-height: 1; }
  .big1 { font-size: 15px; -webkit-line-clamp: 4; line-clamp: 4; }
  .bars { display: flex; flex-direction: column; gap: 6px; padding: 12px 14px; border-radius: 12px; background: rgba(255, 255, 255, 0.025); border: 1px solid var(--line); }
  .br { display: grid; grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr) 36px; gap: 12px; align-items: center; }
  .track { height: 8px; border-radius: 4px; background: rgba(255, 255, 255, 0.07); overflow: hidden; }
  .fill { display: block; height: 100%; border-radius: 4px; background: var(--gold); }
  .bv { text-align: right; font-weight: 700; color: var(--gold-glow); }
  .meta { display: flex; flex-wrap: wrap; gap: 6px 16px; }
  .meta b { color: var(--text); }
  .notes { white-space: pre-wrap; line-height: 1.55; padding: 12px 14px; border-radius: 12px; background: rgba(255, 255, 255, 0.025); border: 1px solid var(--line); }
  .spoil { border: 0; padding: 0 4px; border-radius: 4px; background: var(--coal-2); color: transparent; text-shadow: 0 0 8px rgba(255, 255, 255, 0.5); cursor: pointer; font: inherit; }
  .sp { background: rgba(255, 255, 255, 0.06); border-radius: 4px; padding: 0 4px; }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; }

  .flist { overflow: auto; display: flex; flex-direction: column; gap: 2px; border: 1px solid var(--line); border-radius: 12px; padding: 6px; min-height: 120px; max-height: 420px; }
  .hit { display: flex; align-items: center; gap: 12px; text-align: left; border: 0; background: transparent; color: var(--text); padding: 6px 8px; border-radius: 9px; }
  .hit:hover { background: rgba(255, 255, 255, 0.05); }
  .hit img, .hph { width: 30px; height: 45px; object-fit: cover; border-radius: 6px; flex: none; background: var(--coal-2); display: grid; place-items: center; color: var(--muted); }
  .hit img.wide, .hph.wide { width: 92px; height: 34px; }
  .htx { display: flex; flex-direction: column; min-width: 0; }
  .htx b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pad { padding: 10px; margin: 0; }

  @media (max-width: 1000px) {
    .layout { grid-template-columns: 1fr; }
    .filt { display: inline-flex; }
    .filters { position: fixed; top: 0; right: 0; bottom: 0; z-index: 65; width: min(320px, 88vw); max-height: none; border-radius: 16px 0 0 16px; transform: translateX(105%); transition: transform 0.2s ease; background: var(--night-2); }
    .filters.open { transform: none; box-shadow: -20px 0 60px rgba(0, 0, 0, 0.5); }
    .show { display: inline-flex; justify-content: center; position: sticky; bottom: 0; }
    .shade { display: block; position: fixed; inset: 0; z-index: 64; background: rgba(0, 0, 0, 0.45); }
  }
  @media (max-width: 640px) {
    .dhead { flex-direction: column; }
  }
</style>
