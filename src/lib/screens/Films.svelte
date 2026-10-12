<script lang="ts">
  // Film & Dizi Arşivi: izlediğin film ve dizileri 5 kısa soruyla puanla (Oyun Arşivi ile aynı düzen).
  // Bilgiler (Türkçe ad, afiş, özet, yönetmen, oyuncular, tür) Wikipedia ve Wikidata'dan kendiliğinden gelir.
  import { onMount, untrack } from "svelte";
  import { focus, takeFocus } from "../focus.svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import RatingCard from "../components/RatingCard.svelte";
  import Icon from "../Icon.svelte";
  import { RECOMMEND, TITLE_QUESTIONS, TITLE_STATUS, fmtScore, recommendLabel, recommendOf, scoreTone, spoilerParts, statusLabel } from "../review";
  import { getJson, setJson } from "../settings.svelte";
  import { notify } from "../toast.svelte";
  import type { Review, Title, TitleHit } from "../types";

  type Sort = "score" | "rated" | "year" | "name";
  const SORTS: [Sort, string][] = [["score", "Puana göre"], ["rated", "Son eklenen"], ["year", "Çıkış yılı"], ["name", "Ada göre"]];
  const SCORES = [7, 8, 9];
  const KINDS: [string, string][] = [["movie", "Film"], ["series", "Dizi"]];

  interface Item {
    t: Title;
    r: Review;
  }

  let titles: Title[] = $state([]);
  let reviews: Review[] = $state([]);
  let loaded = $state(false);

  let search = $state("");
  let genres: string[] = $state([]);
  let kinds: string[] = $state([]);
  let minScore = $state(0);
  let statuses: string[] = $state([]);
  let recs: string[] = $state([]);
  let people: string[] = $state([]);
  let allGenres = $state(false);
  let drawer = $state(false);
  let sort: Sort = $state(getJson<Sort>("films.sort", "score"));

  let rating: { title: Title; review: Review | null; fresh: boolean } | null = $state(null);
  let detail: Item | null = $state(null);
  let revealed: number[] = $state([]);
  let adding = $state(false);
  let addSearch = $state("");
  let hits: TitleHit[] = $state([]);
  let searching = $state(false);
  let manualKind: "movie" | "series" = $state("movie");
  let pick: Item | null = $state(null);
  let rolling = $state(false);

  async function load() {
    const [t, r] = await Promise.all([api.titles.list().catch(() => []), api.reviews.list("title").catch(() => [])]);
    titles = t;
    reviews = r;
    loaded = true;
    if (detail) detail = items.find((i) => i.r.id === detail?.r.id) ?? null;
  }

  onMount(() => {
    void load();
    const stops = [on("titles-changed", () => void load()), on("reviews-changed", () => void load())];
    return () => {
      for (const s of stops) void s.then((f) => f());
    };
  });

  const byId = $derived(new Map(titles.map((t) => [t.id, t])));
  const items: Item[] = $derived(reviews.flatMap((r) => (byId.has(r.subjectId) ? [{ t: byId.get(r.subjectId)!, r }] : [])));

  // Aramadan gelen istek: o film/dizinin ayrıntısını aç.
  $effect(() => {
    void focus.n;
    if (!loaded) return;
    const id = untrack(() => takeFocus("title"));
    const it = id ? untrack(() => items.find((x) => x.t.id === id)) : null;
    if (it) detail = it;
  });

  type Group = "search" | "genre" | "kind" | "score" | "status" | "rec" | "people";
  function passes(i: Item, skip?: Group): boolean {
    const q = search.trim().toLocaleLowerCase("tr-TR");
    if (skip !== "search" && q && !`${i.t.name} ${i.t.original ?? ""} ${i.t.directors.join(" ")} ${i.t.cast.join(" ")} ${i.r.oneLiner ?? ""}`.toLocaleLowerCase("tr-TR").includes(q)) return false;
    if (skip !== "genre" && genres.length && !genres.every((g) => i.t.genres.includes(g))) return false;
    if (skip !== "kind" && kinds.length && !kinds.includes(i.t.kind)) return false;
    if (skip !== "score" && minScore && (i.r.overall ?? 0) < minScore) return false;
    if (skip !== "status" && statuses.length && !statuses.includes(i.r.status ?? "")) return false;
    if (skip !== "rec" && recs.length && !recs.includes(i.r.recommend ?? "")) return false;
    if (skip !== "people" && people.length && !people.every((p) => i.t.directors.includes(p) || i.t.cast.includes(p))) return false;
    return true;
  }

  const shown = $derived.by(() => {
    const list = items.filter((i) => passes(i));
    const by: Record<Sort, (a: Item, b: Item) => number> = {
      score: (a, b) => (b.r.overall ?? -1) - (a.r.overall ?? -1) || a.t.name.localeCompare(b.t.name, "tr"),
      rated: (a, b) => b.r.updatedAt - a.r.updatedAt,
      year: (a, b) => (b.t.year ?? 0) - (a.t.year ?? 0),
      name: (a, b) => a.t.name.localeCompare(b.t.name, "tr"),
    };
    return list.sort(by[sort]);
  });

  const countFor = (skip: Group, test: (i: Item) => boolean) => items.filter((i) => passes(i, skip) && test(i)).length;
  const genreCounts = $derived.by(() => {
    const freq = new Map<string, number>();
    for (const i of items) for (const g of i.t.genres) freq.set(g, (freq.get(g) ?? 0) + 1);
    return [...freq.keys()]
      .sort((a, b) => (freq.get(b) ?? 0) - (freq.get(a) ?? 0) || a.localeCompare(b, "tr"))
      .map((g) => [g, countFor("genre", (i) => [...genres, g].every((x) => i.t.genres.includes(x)))] as [string, number]);
  });
  const genreList = $derived(allGenres ? genreCounts : genreCounts.filter(([g], i) => i < 8 || genres.includes(g)));
  // Yönetmenler: en az iki kayıtta geçenler ("Christopher Nolan filmleri").
  const directorList = $derived.by(() => {
    const freq = new Map<string, number>();
    for (const i of items) for (const d of i.t.directors) freq.set(d, (freq.get(d) ?? 0) + 1);
    return [...freq.entries()].filter(([d, n]) => n >= 2 || people.includes(d)).sort((a, b) => b[1] - a[1]).slice(0, 8).map(([d]) => d);
  });
  const active = $derived(genres.length + kinds.length + (minScore ? 1 : 0) + statuses.length + recs.length + people.length + (search.trim() ? 1 : 0));
  const toggle = (list: string[], v: string) => (list.includes(v) ? list.filter((x) => x !== v) : [...list, v]);
  function clearAll() {
    genres = [];
    kinds = [];
    minScore = 0;
    statuses = [];
    recs = [];
    people = [];
    search = "";
  }

  const src = (p: string | null) => (!p ? null : /^(https?:|data:)/.test(p) ? p : api.fileSrc(p));
  const metaLine = (t: Title) =>
    [t.year, t.kind === "series" ? (t.seasons ? `${t.seasons} sezon` : "Dizi") : t.runtime ? `${Math.floor(t.runtime / 60)} sa ${t.runtime % 60} dk` : "Film"]
      .filter(Boolean)
      .join(" · ");
  const progress = (r: Review) => (r.extra?.season ? `S${r.extra.season}${r.extra.episode ? ` · B${r.extra.episode}` : ""}` : null);

  // ---- ekleme ----
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  function onAddSearch() {
    clearTimeout(searchTimer);
    const term = addSearch.trim();
    if (term.length < 2) {
      hits = [];
      return;
    }
    searchTimer = setTimeout(async () => {
      searching = true;
      try {
        hits = await api.titles.search(term);
      } catch (e) {
        hits = [];
        notify(`Wikipedia'ya ulaşılamadı: ${e}`, "info");
      } finally {
        searching = false;
      }
    }, 450);
  }

  async function addHit(h: TitleHit | null) {
    try {
      const t = await api.titles.add(h, addSearch.trim(), manualKind);
      adding = false;
      const existing = reviews.find((r) => r.subjectId === t.id) ?? null;
      rating = { title: t, review: existing, fresh: !existing };
      await load();
    } catch (e) {
      notify(String(e), "info");
    }
  }

  function roll() {
    const pool = shown.filter((i) => i.r.status === "watchlist" || (i.r.recommend !== "no" && i.r.status !== "dropped"));
    const list = pool.length ? pool : shown;
    if (!list.length) return notify("Bu filtrede film yok", "info");
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

  function openDetail(i: Item) {
    detail = i;
    revealed = [];
  }

  const subjectOf = (t: Title) => ({
    id: t.id,
    name: t.name,
    cover: t.poster,
    line1: [t.genres.slice(0, 4).join(" · "), t.directors[0]].filter(Boolean).join(" — "),
    line2: metaLine(t),
    series: t.kind === "series",
  });
  const fmtDay = (ms: number) => new Date(ms).toLocaleDateString("tr-TR", { day: "numeric", month: "long", year: "numeric" });
</script>

<div class="arc">
  <div class="layout">
    <section class="panel main">
      <div class="spread bar">
        <div class="row">
          <h2 class="h2"><Icon name="film" />Film & Dizi Arşivi</h2>
          <span class="sub tnum">{shown.length} kayıt</span>
        </div>
        <div class="row">
          <input class="search" type="search" placeholder="Ad, yönetmen, oyuncu…" bind:value={search} />
          <select bind:value={sort} onchange={() => void setJson("films.sort", sort)} aria-label="Sırala">
            {#each SORTS as [s, label] (s)}<option value={s}>{label}</option>{/each}
          </select>
          <button class="btn ghost filt" onclick={() => (drawer = true)}><Icon name="list" size={16} />Filtrele{active ? ` · ${active}` : ""}</button>
          <button class="btn ghost" onclick={roll} disabled={!shown.length}>🎲 Bana bir film seç</button>
          <button class="btn" onclick={() => ((adding = true), (addSearch = ""), (hits = []))}><Icon name="plus" size={17} />Film / dizi ekle</button>
        </div>
      </div>

      {#if active}
        <div class="active">
          {#if search.trim()}<button class="ac" onclick={() => (search = "")}>“{search.trim()}” <Icon name="x" size={12} /></button>{/if}
          {#each kinds as k (k)}<button class="ac" onclick={() => (kinds = toggle(kinds, k))}>{KINDS.find(([x]) => x === k)?.[1]} <Icon name="x" size={12} /></button>{/each}
          {#each genres as g (g)}<button class="ac" onclick={() => (genres = toggle(genres, g))}>{g} <Icon name="x" size={12} /></button>{/each}
          {#each people as p (p)}<button class="ac" onclick={() => (people = toggle(people, p))}>{p} <Icon name="x" size={12} /></button>{/each}
          {#if minScore}<button class="ac" onclick={() => (minScore = 0)}>Puan {minScore}+ <Icon name="x" size={12} /></button>{/if}
          {#each statuses as s (s)}<button class="ac" onclick={() => (statuses = toggle(statuses, s))}>{statusLabel(s as never)} <Icon name="x" size={12} /></button>{/each}
          {#each recs as r (r)}<button class="ac" onclick={() => (recs = toggle(recs, r))}>{recommendLabel(r as never, "title")} <Icon name="x" size={12} /></button>{/each}
          <button class="link" onclick={clearAll}>Temizle</button>
        </div>
      {/if}

      {#if loaded && !items.length}
        <div class="empty">
          <Icon name="film" size={40} />
          <h3>Arşivin boş</h3>
          <p class="sub">
            İzlediğin bir filmi ya da diziyi <b>5 kısa soruyla</b> puanla: hikâye, oyunculuk, yönetmenlik, tempo, keyif.
            Afiş, Türkçe özet, yönetmen ve oyuncular kendiliğinden gelir. İzlemek istediklerini de "İzlenecek" olarak ekleyebilirsin.
          </p>
          <button class="btn" onclick={() => ((adding = true), (addSearch = ""), (hits = []))}><Icon name="plus" size={17} />İlk filmini ekle</button>
        </div>
      {:else}
        <div class="grid">
          {#each shown as i (i.r.id)}
            {@const rec = recommendOf(i.r.recommend)}
            <button class="card" class:picked={pick === i && !rolling} onclick={() => openDetail(i)} title={i.t.name}>
              <div class="cv">
                {#if src(i.t.poster)}<img src={src(i.t.poster)} alt="" loading="lazy" />{:else}<span class="ph">{i.t.name}</span>{/if}
                <span class={`score tone-${scoreTone(i.r.overall)}`}>{i.r.overall == null ? "İzlenecek" : fmtScore(i.r.overall)}</span>
                {#if rec}<span class="rec" title={recommendLabel(rec.id, "title")}>{rec.icon}</span>{/if}
                <span class="st">{progress(i.r) ?? (i.t.kind === "series" ? "Dizi" : "Film")}</span>
              </div>
              <b class="nm">{i.t.name}</b>
              {#if i.r.oneLiner}<span class="one">“{i.r.oneLiner}”</span>{/if}
              <span class="sub small tnum">{metaLine(i.t)}</span>
            </button>
          {:else}
            <div class="none">
              <p class="sub">Bu filtrelere uyan kayıt yok.</p>
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
        <span class="gl">Film / dizi</span>
        {#each KINDS as [k, label] (k)}
          {@const n = countFor("kind", (i) => i.t.kind === k)}
          <label class="opt" class:dim={!n && !kinds.includes(k)}>
            <input type="checkbox" checked={kinds.includes(k)} onchange={() => (kinds = toggle(kinds, k))} />
            <span>{label}</span><span class="cnt tnum">{n}</span>
          </label>
        {/each}
      </div>
      <div class="grp">
        <span class="gl">Tür</span>
        {#each genreList as [g, n] (g)}
          <label class="opt" class:dim={!n && !genres.includes(g)}>
            <input type="checkbox" checked={genres.includes(g)} onchange={() => (genres = toggle(genres, g))} />
            <span>{g}</span><span class="cnt tnum">{n}</span>
          </label>
        {:else}
          <span class="sub small">Ekledikçe türler burada çıkar.</span>
        {/each}
        {#if genreCounts.length > 8}
          <button class="link sm" onclick={() => (allGenres = !allGenres)}>{allGenres ? "Daha az göster" : `Tümünü göster (${genreCounts.length})`}</button>
        {/if}
      </div>
      {#if directorList.length}
        <div class="grp">
          <span class="gl">Yönetmen</span>
          {#each directorList as d (d)}
            {@const n = countFor("people", (i) => [...people, d].every((x) => i.t.directors.includes(x) || i.t.cast.includes(x)))}
            <label class="opt" class:dim={!n && !people.includes(d)}>
              <input type="checkbox" checked={people.includes(d)} onchange={() => (people = toggle(people, d))} />
              <span>{d}</span><span class="cnt tnum">{n}</span>
            </label>
          {/each}
        </div>
      {/if}
      <div class="grp">
        <span class="gl">Puan</span>
        {#each SCORES as s (s)}
          {@const n = countFor("score", (i) => (i.r.overall ?? 0) >= s)}
          <label class="opt" class:dim={!n && minScore !== s}>
            <input type="radio" name="fscore" checked={minScore === s} onclick={() => (minScore = minScore === s ? 0 : s)} />
            <span>{s}+</span><span class="cnt tnum">{n}</span>
          </label>
        {/each}
      </div>
      <div class="grp">
        <span class="gl">Durum</span>
        {#each TITLE_STATUS as s (s.id)}
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
            <span>{r.icon} {recommendLabel(r.id, "title")}</span><span class="cnt tnum">{n}</span>
          </label>
        {/each}
      </div>
      <button class="btn show" onclick={() => (drawer = false)}>{shown.length} kaydı göster</button>
    </aside>
  </div>
</div>

{#if pick}
  {@const p = pick}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (pick = null)}>
    <div class="dlg panel pickd" role="dialog" aria-label="Bana bir film seç">
      <span class="sub small up">🎲 {rolling ? "Seçiliyor…" : "Bu akşam bunu izle"}</span>
      <div class="pcv" class:spin={rolling}>
        {#if src(p.t.poster)}<img src={src(p.t.poster)} alt="" />{:else}<span class="ph">{p.t.name}</span>{/if}
      </div>
      <h3>{p.t.name}</h3>
      {#if !rolling}
        <span class="sub small">{metaLine(p.t)}</span>
        {#if p.r.oneLiner}<span class="one">“{p.r.oneLiner}”</span>{/if}
      {/if}
      <div class="row center">
        <button class="btn ghost" onclick={roll} disabled={rolling}><Icon name="shuffle" size={16} />Başka seç</button>
        <button class="btn" disabled={rolling} onclick={() => { const it = p; openDetail(it); pick = null; }}>Ayrıntılar</button>
      </div>
    </div>
  </div>
{/if}

{#if detail}
  {@const d = detail}
  {@const rec = recommendOf(d.r.recommend)}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (detail = null)}>
    <div class="dlg panel det" role="dialog" aria-label={d.t.name}>
      <div class="dhead">
        {#if src(d.t.poster)}<img class="dcover" src={src(d.t.poster)} alt="" />{/if}
        <div class="dtx">
          <div class="spread">
            <h3>{d.t.name}</h3>
            <button class="x" onclick={() => (detail = null)} title="Kapat"><Icon name="x" size={18} /></button>
          </div>
          <span class="sub small">{[d.t.original, metaLine(d.t)].filter(Boolean).join(" · ")}</span>
          {#if d.t.genres.length}<div class="tags">{#each d.t.genres as g (g)}<span class="chip">{g}</span>{/each}</div>{/if}
          <div class="big">
            {#if d.r.overall != null}<b class={`tone-${scoreTone(d.r.overall)}`}>{fmtScore(d.r.overall)}</b><span class="sub">/ 10</span>{/if}
            {#if rec}<span class="chip">{rec.icon} {recommendLabel(rec.id, "title")}</span>{/if}
            {#if d.r.status}<span class="chip">{statusLabel(d.r.status)}</span>{/if}
            {#if progress(d.r)}<span class="chip">Kaldığın yer: {progress(d.r)}</span>{/if}
          </div>
          {#if d.r.oneLiner}<span class="one big1">“{d.r.oneLiner}”</span>{/if}
        </div>
      </div>
      {#if d.t.summary}<p class="summary">{d.t.summary}</p>{/if}
      <div class="meta small sub">
        {#if d.t.directors.length}<span>{d.t.kind === "series" ? "Yaratıcı" : "Yönetmen"}: <b>{d.t.directors.join(", ")}</b></span>{/if}
        {#if d.t.cast.length}<span>Oyuncular: <b>{d.t.cast.slice(0, 5).join(", ")}</b></span>{/if}
        {#if d.r.extra?.where}<span>Nerede: <b>{d.r.extra.where}</b></span>{/if}
        {#if d.r.extra?.with}<span>Kiminle: <b>{d.r.extra.with}</b></span>{/if}
        {#if d.r.finishedAt}<span>İzleme: {fmtDay(d.r.finishedAt)}</span>{/if}
      </div>
      {#if d.r.overall != null}
        <div class="bars">
          {#each TITLE_QUESTIONS as q (q.key)}
            {@const v = d.r.scores[q.key]}
            <div class="br">
              <span class="small">{q.icon} {q.label}</span>
              <span class="track"><span class="fill" style={`width:${v == null ? 0 : v * 10}%`}></span></span>
              <span class="tnum bv">{v ?? "Yok"}</span>
            </div>
          {/each}
        </div>
      {/if}
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
        <button class="link sm" onclick={() => api.titles.pickPoster(d.t.id).then((ok) => ok && notify("Afiş değişti", "image")).catch((e) => notify(String(e), "info"))}>Afiş seç</button>
        <button class="link sm" onclick={() => api.titles.refresh(d.t.id).then(() => notify("Bilgiler yenileniyor", "check"))}>Bilgileri yenile</button>
        <button class="btn" onclick={() => (rating = { title: d.t, review: d.r, fresh: false })}><Icon name="edit" size={16} />Düzenle</button>
      </div>
      <p class="sub small credit">Bilgiler: Wikipedia ve Wikidata · afiş: Wikipedia, iTunes ya da TVmaze</p>
    </div>
  </div>
{/if}

{#if adding}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (adding = false)}>
    <div class="dlg panel" role="dialog" aria-label="Film ya da dizi ekle">
      <div class="spread">
        <h3 class="h2"><Icon name="film" />Hangi film ya da dizi?</h3>
        <button class="x" onclick={() => (adding = false)} title="Kapat"><Icon name="x" size={18} /></button>
      </div>
      <input class="search wide" type="search" placeholder="Adı (Türkçe ya da orijinal): Inception, Breaking Bad…" bind:value={addSearch} oninput={onAddSearch} />
      <div class="flist">
        {#if searching}<p class="sub small pad">Aranıyor…</p>{/if}
        {#each hits as h (h.wikiTitle)}
          <button class="hit" onclick={() => addHit(h)}>
            {#if h.image}<img src={h.image} alt="" />{:else}<span class="hph"><Icon name="film" size={16} /></span>{/if}
            <span class="htx"><b>{h.name}</b><span class="sub small">{h.kind === "series" ? "Dizi" : "Film"} · {h.description}</span></span>
          </button>
        {/each}
        {#if addSearch.trim().length >= 2 && !searching}
          <div class="hit manual">
            <span class="hph"><Icon name="plus" size={16} /></span>
            <span class="htx"><b>“{addSearch.trim()}” adıyla ekle</b><span class="sub small">Bulunamadıysa: bilgisiz, yalnızca adıyla</span></span>
            <select bind:value={manualKind} aria-label="Tür"><option value="movie">Film</option><option value="series">Dizi</option></select>
            <button class="btn ghost sm" onclick={() => addHit(null)}>Ekle</button>
          </div>
        {:else}
          {#if !hits.length && !searching}<p class="sub small pad">Adını yaz; afiş, özet ve oyuncular Wikipedia'dan gelir. Türkçe adla bulunmazsa orijinal adını dene.</p>{/if}
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if rating}
  {@const rt = rating}
  <RatingCard
    kind="title"
    subject={subjectOf(rt.title)}
    review={rt.review}
    onclose={() => (rating = null)}
    onsaved={() => void load()}
    ondeleted={() => ((detail = null), void load())}
    ondismiss={() => rt.fresh && void api.titles.discard(rt.title.id).then(load)}
  />
{/if}

<style>
  .arc { display: flex; flex-direction: column; gap: 16px; max-width: 1600px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 14px; }

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
  .htx { display: flex; flex-direction: column; min-width: 0; }
  .htx b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pad { padding: 10px; margin: 0; }
  .hit.manual { cursor: default; }
  .hit.manual .htx { flex: 1; }
  .hit.manual select { max-width: 90px; }
  .summary { margin: 0; line-height: 1.55; color: var(--muted); }
  .credit { margin: 0; text-align: right; font-size: 11px; }

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
