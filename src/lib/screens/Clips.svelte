<script lang="ts">
  // Kliplerim: NVIDIA, Xbox Game Bar, AMD, OBS, Steam, Medal kayıtları oyuna göre. Dosyalar yerinde kalır.
  // Üzerine gelince sessiz önizleme, tıklayınca büyük görünüm, çift tıklayınca varsayılan programda açılır.
  import { onMount, untrack } from "svelte";
  import { api } from "../api";
  import { dragOut } from "../dragout";
  import { focus, takeFocus } from "../focus.svelte";
  import { on } from "../bus";
  import ClipTrim from "../components/ClipTrim.svelte";
  import Icon from "../Icon.svelte";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { Clip, ClipRoot, Game } from "../types";

  interface Props {
    onnavigate: (route: string) => void;
  }
  let { onnavigate }: Props = $props();

  type Kind = "all" | "clip" | "shot" | "fav";
  const KINDS: [Kind, string][] = [["all", "Tümü"], ["clip", "Klipler"], ["shot", "Ekran görüntüleri"], ["fav", "Favoriler"]];
  const SOURCES: Record<Clip["source"], string> = {
    nvidia: "NVIDIA", xbox: "Xbox", amd: "AMD", obs: "OBS", steam: "Steam", medal: "Medal", folder: "Klasör",
  };
  const OTHER = "__other";

  let clips: Clip[] = $state([]);
  let games: Game[] = $state([]);
  let roots: ClipRoot[] = $state([]);
  let loaded = $state(false);
  let kind: Kind = $state("all");
  let game = $state("");
  let search = $state("");
  let expanded: string[] = $state([]);
  let now = $state(Date.now());
  let fitting: Record<string, number> = $state({});
  let hover: string | null = $state(null);
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;

  // Büyük görünüm
  let view: Clip | null = $state(null);
  let titleDraft = $state("");
  let armedDelete = $state(false);
  /** Kırp ve hazırla penceresi. */
  let trim: Clip | null = $state(null);
  /** Kartta "Sil?" onayı bekleyen klip. */
  let armed: string | null = $state(null);
  /** Kapaklar: ffmpeg yoksa arka planda iniyor ("ffmpeg") ya da inemedi ("failed"). */
  let thumbs = $state<"" | "ffmpeg" | "failed">("");
  /** Dosyası açılamayan kapaklar (simge gösterilir). */
  let broken = $state<Record<string, true>>({});

  async function load() {
    const [c, g] = await Promise.all([api.clips.list().catch(() => []), api.games.list().catch(() => [])]);
    clips = c;
    games = g;
    loaded = true;
    if (view) view = clips.find((x) => x.id === view?.id) ?? null;
    if (!clips.length) roots = await api.clips.roots().catch(() => []);
  }

  onMount(() => {
    void load();
    const stops = [
      on("clips-changed", () => void load()),
      on<string>("clips-thumbs", (s) => {
        thumbs = s === "ffmpeg" ? "ffmpeg" : s === "failed" ? "failed" : "";
        if (s === "ready") void load();
      }),
      on("games-changed", () => void load()),
      on<{ id: string; progress: number }>("clip-fit", (e) => {
        if (e.progress >= 1) {
          const { [e.id]: _, ...rest } = fitting;
          fitting = rest;
        } else fitting = { ...fitting, [e.id]: e.progress };
      }),
    ];
    const timer = setInterval(() => (now = Date.now()), 60_000);
    return () => {
      clearInterval(timer);
      clearTimeout(hoverTimer);
      for (const s of stops) void s.then((f) => f());
    };
  });

  const byId = $derived(new Map(games.map((g) => [g.id, g])));
  const groupKey = (c: Clip) => (c.gameId && byId.has(c.gameId) ? c.gameId : OTHER);
  const groupName = (k: string) => (k === OTHER ? "Diğer" : (byId.get(k)?.name ?? "Diğer"));
  const label = (c: Clip) => c.title ?? c.name;

  const counts = $derived({
    all: clips.length,
    clip: clips.filter((c) => c.kind === "clip").length,
    shot: clips.filter((c) => c.kind === "shot").length,
    fav: clips.filter((c) => c.favorite).length,
  });
  const filtered = $derived.by(() => {
    const q = search.trim().toLocaleLowerCase("tr-TR");
    return clips.filter(
      (c) =>
        (kind === "all" || (kind === "fav" ? c.favorite : c.kind === kind)) &&
        (!game || groupKey(c) === game) &&
        (!q || `${label(c)} ${c.gameHint ?? ""} ${groupName(groupKey(c))}`.toLocaleLowerCase("tr-TR").includes(q)),
    );
  });
  // Oyun grupları: en yeni klibi olan önce.
  const groups = $derived.by(() => {
    const m = new Map<string, Clip[]>();
    for (const c of filtered) {
      const k = groupKey(c);
      if (!m.has(k)) m.set(k, []);
      m.get(k)!.push(c);
    }
    return [...m.entries()].sort((a, b) => (a[0] === OTHER ? 1 : b[0] === OTHER ? -1 : b[1][0].createdAt - a[1][0].createdAt));
  });
  const gameOptions = $derived.by(() => {
    const m = new Map<string, number>();
    for (const c of clips) m.set(groupKey(c), (m.get(groupKey(c)) ?? 0) + 1);
    return [...m.entries()].sort((a, b) => groupName(a[0]).localeCompare(groupName(b[0]), "tr"));
  });

  const PER_GROUP = 8;
  const src = (p: string) => api.fileSrc(p);
  const cover = (g: Game | undefined) => (!g?.cover ? null : /^(https?:|data:)/.test(g.cover) ? g.cover : api.fileSrc(g.cover));
  const dur = (s: number | null) => (s == null ? "" : `${Math.floor(s / 60)}:${String(Math.round(s % 60)).padStart(2, "0")}`);
  const mb = (b: number) => (b >= 1e9 ? `${(b / 1e9).toFixed(1)} GB` : `${Math.max(0.1, b / 1e6).toFixed(b >= 1e8 ? 0 : 1)} MB`).replace(".", ",");
  const fmtDate = (ms: number) => new Date(ms).toLocaleString("tr-TR", { day: "numeric", month: "long", year: "numeric", hour: "2-digit", minute: "2-digit" });

  function enter(c: Clip) {
    clearTimeout(hoverTimer);
    if (c.kind !== "clip") return;
    hoverTimer = setTimeout(() => (hover = c.id), 350);
  }
  function leave() {
    clearTimeout(hoverTimer);
    hover = null;
  }

  async function act(p: Promise<unknown>, ok?: string) {
    try {
      await p;
      if (ok) notify(ok, "check");
      return true;
    } catch (e) {
      notify(String(e), "info");
      return false;
    }
  }

  /** Tek tık: son seçilen (yoksa çözünürlüğü koruyan) ayarla hazırla, panoya koy. */
  async function fit(c: Clip) {
    if (c.id in fitting) return;
    fitting = { ...fitting, [c.id]: 0 };
    try {
      const r = await api.clips.fitDiscord(c.id);
      notify(`Discord'a hazır · ${(r.size / 1e6).toLocaleString("tr-TR", { maximumFractionDigits: 1 })} MB${r.height ? ` · ${r.height}p` : ""} · panoya kopyalandı, Discord'da Ctrl+V`, "check");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      const { [c.id]: _, ...rest } = fitting;
      fitting = rest;
    }
  }

  // Aramadan gelen istek: o klibi büyük görünümde aç.
  $effect(() => {
    void focus.n;
    if (!loaded) return;
    const id = untrack(() => takeFocus("clip"));
    const c = id ? untrack(() => clips.find((x) => x.id === id)) : null;
    if (c) openView(c);
  });

  function openView(c: Clip) {
    view = c;
    titleDraft = label(c);
    armedDelete = false;
  }

  async function saveTitle(c: Clip) {
    if (titleDraft.trim() === label(c)) return;
    await act(api.clips.rename(c.id, titleDraft.trim() === c.name ? "" : titleDraft));
  }

  async function remove(c: Clip) {
    if (!armedDelete) {
      armedDelete = true;
      return;
    }
    if (await act(api.clips.remove(c.id), "Geri Dönüşüm Kutusu'na gönderildi")) {
      view = null;
      await load();
    }
  }

  /** Karttan silme: ilk tık "Sil?" sorar (3 sn), ikinci tık Geri Dönüşüm Kutusu'na gönderir. */
  async function removeQuick(c: Clip) {
    if (armed !== c.id) {
      armed = c.id;
      setTimeout(() => armed === c.id && (armed = null), 3000);
      return;
    }
    armed = null;
    if (await act(api.clips.remove(c.id), "Geri Dönüşüm Kutusu'na gönderildi")) await load();
  }

  function step(d: number) {
    if (!view) return;
    const i = filtered.findIndex((c) => c.id === view?.id);
    const next = filtered[i + d];
    if (next) openView(next);
  }

  function onkey(e: KeyboardEvent) {
    if (!view || (e.target as HTMLElement)?.tagName === "INPUT") return;
    if (e.key === "Escape") view = null;
    else if (e.key === "ArrowRight") step(1);
    else if (e.key === "ArrowLeft") step(-1);
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="clips">
  <section class="panel">
    <div class="spread bar">
      <div class="seg">
        {#each KINDS as [k, l] (k)}
          <button aria-pressed={kind === k} onclick={() => (kind = k)}>{l} <span class="n">{counts[k]}</span></button>
        {/each}
      </div>
      <div class="row">
        <select bind:value={game} aria-label="Oyun">
          <option value="">Tüm oyunlar</option>
          {#each gameOptions as [k, n] (k)}<option value={k}>{groupName(k)} ({n})</option>{/each}
        </select>
        <input class="search" type="search" placeholder="Klip ara…" bind:value={search} />
        <button class="btn ghost" onclick={() => onnavigate("settings/oyunlar")} title="İzlenen klasörler"><Icon name="folder" size={16} />Klasörler</button>
      </div>
    </div>

    {#if thumbs}
      <div class="tnote" class:bad={thumbs === "failed"}>
        <Icon name={thumbs === "failed" ? "info" : "download"} size={16} />
        {#if thumbs === "ffmpeg"}
          <span>Kapaklar hazırlanıyor: videolardan kare almak için gereken ffmpeg bir kez indiriliyor (~90 MB).</span>
        {:else}
          <span>Kapaklar için ffmpeg indirilemedi. Video İndirici → Araçlar'dan kurunca kapaklar kendiliğinden gelir.</span>
          <button class="btn ghost sm" onclick={() => onnavigate("videos")}>Video İndirici</button>
        {/if}
      </div>
    {/if}

    {#if loaded && !clips.length}
      <div class="empty">
        <Icon name="clip" size={40} />
        <h3>Henüz klip yok</h3>
        <p class="sub">
          NVIDIA'da <b>Alt+F10</b>, Xbox Game Bar'da <b>Win+Alt+G</b> ya da Steam'de <b>F12</b> ile kaydettiğin klipler ve ekran
          görüntüleri burada, oyuna göre görünür. Dosyalar taşınmaz; GMID yalnızca bulur ve gösterir.
        </p>
        {#if roots.length}
          <div class="roots">
            {#each roots as r (r.path)}
              <span class="sub small" class:ok={r.exists}>{r.exists ? "✓" : "—"} {r.label}: <code>{r.path}</code></span>
            {/each}
          </div>
        {/if}
        <button class="btn" onclick={() => onnavigate("settings/oyunlar")}><Icon name="plus" size={16} />Başka bir klasör ekle</button>
      </div>
    {:else}
      {#each groups as [k, list] (k)}
        {@const g = byId.get(k)}
        {@const open = !!game || expanded.includes(k)}
        <div class="group">
          <div class="ghead">
            {#if cover(g)}<img class="gcv" src={cover(g)} alt="" />{:else}<span class="gcv ph"><Icon name="clip" size={16} /></span>{/if}
            <b>{groupName(k)}</b>
            <span class="sub small tnum">{list.length} · {mb(list.reduce((t, c) => t + c.size, 0))}</span>
          </div>
          <div class="grid">
            {#each open ? list : list.slice(0, PER_GROUP) as c (c.id)}
              <div
                class="card"
                role="button"
                tabindex="0"
                onmouseenter={() => enter(c)}
                onmouseleave={leave}
                use:dragOut={() => c.path}
                onclick={() => openView(c)}
                ondblclick={() => act(api.clips.open(c.id))}
                onkeydown={(e) => e.key === "Enter" && openView(c)}
                title={`${label(c)} · çift tıkla aç · sürükleyip Discord'a ya da bir klasöre bırakabilirsin`}
              >
                <div class="th">
                  {#if c.thumb && !broken[c.id]}<img src={src(c.thumb)} alt="" loading="lazy" onerror={() => (broken = { ...broken, [c.id]: true })} />{:else if c.kind === "shot" && !broken[c.id]}<img src={src(c.path)} alt="" loading="lazy" onerror={() => (broken = { ...broken, [c.id]: true })} />{:else if c.kind === "clip" && !broken[c.id]}<!-- Kapak henüz yoksa (klip yeni yazıldı) videonun bir karesi gösterilir. --><video class="still" src={`${src(c.path)}#t=1`} preload="metadata" muted playsinline onerror={() => (broken = { ...broken, [c.id]: true })}></video>{:else}<span class="noth"><Icon name="play" size={22} /></span>{/if}
                  {#if hover === c.id}<video class="prev" src={src(c.path)} muted autoplay loop playsinline></video>{/if}
                  <span class="srcb">{SOURCES[c.source]}</span>
                  {#if c.kind === "clip" && c.duration}<span class="dur tnum">{dur(c.duration)}</span>{/if}
                  <button class="fav" class:on={c.favorite} onclick={(e) => (e.stopPropagation(), act(api.clips.setFavorite(c.id, !c.favorite)))} title={c.favorite ? "Favoriden çıkar" : "Favori"}>{c.favorite ? "★" : "☆"}</button>
                  {#if c.kind === "clip"}
                    <span class="dcs">
                      <button class="dc" onclick={(e) => (e.stopPropagation(), fit(c))} title="Tek tık: çözünürlük korunarak küçültülür, panoya kopyalanır (Discord'da Ctrl+V)">Discord'a sığdır</button>
                      <button class="dc more" onclick={(e) => (e.stopPropagation(), (trim = c))} title="Seçenekler: boyut, kırpma, önizleme"><Icon name="gear" size={13} /></button>
                    </span>
                  {/if}
                  {#if c.id in fitting}
                    <div class="fit"><span>Hazırlanıyor %{Math.round(fitting[c.id] * 100)}</span><span class="pb"><span style={`width:${fitting[c.id] * 100}%`}></span></span></div>
                  {/if}
                </div>
                <b class="nm">{label(c)}</b>
                <div class="crow">
                  <span class="sub small tnum">{ago(c.createdAt, now)} · {mb(c.size)}</span>
                  <span class="cacts">
                    <button class="mini" title="Klasörde göster" onclick={(e) => (e.stopPropagation(), act(api.clips.reveal(c.id)))}><Icon name="folder" size={16} /></button>
                    <button class="mini" class:armed={armed === c.id} title="Sil (Geri Dönüşüm Kutusu'na)" onclick={(e) => (e.stopPropagation(), removeQuick(c))}>
                      <Icon name="trash" size={16} />{#if armed === c.id}<span>Sil?</span>{/if}
                    </button>
                  </span>
                </div>
              </div>
            {/each}
          </div>
          {#if !game && list.length > PER_GROUP}
            <button class="link" onclick={() => (expanded = open ? expanded.filter((x) => x !== k) : [...expanded, k])}>
              {open ? "Daha az göster" : `Tümünü göster (${list.length})`}
            </button>
          {/if}
        </div>
      {:else}
        <p class="sub">Bu süzgeçte klip yok.</p>
      {/each}
    {/if}
  </section>
</div>

{#if view}
  {@const c = view}
  {@const i = filtered.findIndex((x) => x.id === c.id)}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (view = null)}>
    <div class="dlg panel" role="dialog" aria-label={label(c)}>
      <div class="spread">
        <input class="title" bind:value={titleDraft} onblur={() => saveTitle(c)} onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()} aria-label="Ad" />
        <button class="x" onclick={() => (view = null)} title="Kapat"><Icon name="x" size={18} /></button>
      </div>
      <div class="stage">
        {#key c.id}
          {#if c.kind === "clip"}
            <!-- svelte-ignore a11y_media_has_caption -->
            <video src={src(c.path)} controls autoplay poster={c.thumb ? src(c.thumb) : undefined}></video>
          {:else}
            <img src={src(c.path)} alt={label(c)} />
          {/if}
        {/key}
        {#if i > 0}<button class="nav prev" onclick={() => step(-1)} title="Önceki (←)"><Icon name="prev" size={20} /></button>{/if}
        {#if i >= 0 && i < filtered.length - 1}<button class="nav next" onclick={() => step(1)} title="Sonraki (→)"><Icon name="next" size={20} /></button>{/if}
      </div>
      <div class="meta">
        <label class="gsel">
          <span class="sub small">Oyun</span>
          <select value={c.gameId ?? ""} onchange={(e) => act(api.clips.setGame(c.id, e.currentTarget.value || null), "Oyun değiştirildi")}>
            <option value="">— Oyunsuz{c.gameHint ? ` (${c.gameHint})` : ""}</option>
            {#each games.filter((g) => g.inLibrary || g.id === c.gameId) as g (g.id)}<option value={g.id}>{g.name}</option>{/each}
          </select>
        </label>
        <span class="sub small">
          {[SOURCES[c.source], fmtDate(c.createdAt), c.width ? `${c.width}×${c.height}` : null, c.duration ? dur(c.duration) : null, mb(c.size)].filter(Boolean).join(" · ")}
        </span>
      </div>
      <div class="foot">
        <div class="row">
          {#if c.kind === "clip"}
            <button class="btn sm" disabled={c.id in fitting} onclick={() => ((trim = c), (view = null))}>
              <Icon name="clip" size={15} />{c.id in fitting ? `Hazırlanıyor %${Math.round(fitting[c.id] * 100)}` : "Kırp ve Discord'a hazırla"}
            </button>
          {/if}
          <button class="btn ghost sm favb" class:on={c.favorite} onclick={() => act(api.clips.setFavorite(c.id, !c.favorite))}>
            <Icon name="star" size={15} />{c.favorite ? "Favoride" : "Favori"}
          </button>
        </div>
        <div class="tools">
          <button class="ib" onclick={() => act(api.clips.open(c.id))} title="Windows'un oynatıcısında aç"><Icon name="play" size={17} /><span>Oynatıcıda aç</span></button>
          <button class="ib" onclick={() => act(api.clips.reveal(c.id))} title="Dosyanın durduğu klasörü aç"><Icon name="folder" size={17} /><span>Klasörü aç</span></button>
          <i class="sep"></i>
          <button class="ib del" class:armed={armedDelete} onclick={() => remove(c)} title="Sil (Geri Dönüşüm Kutusu'na gider)">
            <Icon name="trash" size={17} />{#if armedDelete}<span>Emin misin? Tekrar bas</span>{/if}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

{#if trim}
  <ClipTrim
    clip={trim}
    onclose={() => {
      const { [trim!.id]: _, ...rest } = fitting;
      fitting = rest;
      trim = null;
    }}
  />
{/if}

<style>
  .tnote { display: flex; align-items: center; gap: 10px; padding: 10px 14px; margin-bottom: 12px; border-radius: 12px; font-size: 13px; color: var(--muted); border: 1px solid var(--line-2); background: rgba(255, 255, 255, 0.03); }
  .tnote.bad { border-color: rgba(224, 106, 106, 0.5); }
  .tnote span { flex: 1; }
  .clips { display: flex; flex-direction: column; gap: 16px; max-width: 1600px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 16px; }
  .bar { flex-wrap: wrap; row-gap: 10px; }
  .seg .n { color: var(--dim); margin-left: 3px; }
  .search { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; width: 180px; color: var(--text); }
  select { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; color: var(--text); max-width: 240px; }

  .group { display: flex; flex-direction: column; gap: 10px; }
  .ghead { display: flex; align-items: center; gap: 10px; }
  .ghead b { font-family: var(--f-display); font-size: 16px; }
  .gcv { width: 24px; height: 36px; object-fit: cover; border-radius: 5px; flex: none; }
  .gcv.ph { display: grid; place-items: center; background: var(--coal-2); color: var(--muted); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 16px 14px; }
  .card { display: flex; flex-direction: column; gap: 3px; min-width: 0; cursor: pointer; outline: none; }
  .th { position: relative; aspect-ratio: 16 / 9; border-radius: 12px; overflow: hidden; background: var(--coal-2); border: 1px solid var(--line); transition: border-color 0.15s, transform 0.15s, box-shadow 0.15s; }
  .card:hover .th, .card:focus-visible .th { border-color: var(--gold-dim); transform: translateY(-2px); box-shadow: 0 10px 24px rgba(0, 0, 0, 0.4); }
  .th img, .prev, .still { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; display: block; }
  .prev { background: #000; }
  .noth { position: absolute; inset: 0; display: grid; place-items: center; color: var(--muted); }
  .srcb { position: absolute; left: 6px; top: 6px; background: rgba(12, 14, 17, 0.82); color: var(--muted); font-size: 10.5px; font-weight: 700; padding: 1px 7px; border-radius: 5px; }
  .dur { position: absolute; right: 6px; bottom: 6px; background: rgba(12, 14, 17, 0.85); color: var(--text); font-size: 12px; font-weight: 700; padding: 1px 7px; border-radius: 6px; }
  .fav { position: absolute; right: 6px; top: 4px; border: 0; background: none; color: rgba(255, 255, 255, 0.8); font-size: 20px; line-height: 1; padding: 2px; opacity: 0; transition: opacity 0.12s; text-shadow: 0 1px 4px rgba(0, 0, 0, 0.8); }
  .fav.on { color: var(--gold-glow); opacity: 1; }
  .dcs { position: absolute; left: 6px; bottom: 6px; display: flex; gap: 2px; opacity: 0; transition: opacity 0.12s; }
  .dc { border: 0; border-radius: 7px; padding: 4px 9px; font-size: 12px; font-weight: 700; background: #5865f2; color: #fff; }
  .dc:first-child { border-radius: 7px 2px 2px 7px; }
  .dc.more { border-radius: 2px 7px 7px 2px; padding: 4px 7px; display: grid; place-items: center; }
  .dc:hover { background: #4752c4; }
  .card:hover .fav, .card:hover .dcs, .card:focus-within .dcs { opacity: 1; }
  .fit { position: absolute; inset: 0; display: flex; flex-direction: column; justify-content: center; align-items: center; gap: 8px; background: rgba(8, 10, 12, 0.72); font-size: 13px; font-weight: 700; }
  .pb { width: 70%; height: 5px; border-radius: 3px; background: rgba(255, 255, 255, 0.15); overflow: hidden; }
  .pb span { display: block; height: 100%; background: #5865f2; transition: width 0.3s; }
  .crow { display: flex; justify-content: space-between; align-items: center; gap: 6px; }
  .cacts { display: flex; opacity: 0.5; transition: opacity 0.15s; }
  .card:hover .cacts, .cacts:has(.armed) { opacity: 1; }
  .mini { display: inline-flex; align-items: center; gap: 4px; border: 0; background: transparent; color: var(--muted); padding: 4px 6px; border-radius: 7px; font-size: 12px; font-weight: 700; }
  .mini:hover { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .mini.armed { color: var(--bad); background: rgba(224, 106, 106, 0.12); }
  .nm { font-weight: 600; font-size: 13.5px; margin-top: 5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; align-self: flex-start; }
  .link:hover { color: var(--neon); }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center; padding: 40px 20px; color: var(--muted); }
  .empty h3 { margin: 0; color: var(--text); font-family: var(--f-display); }
  .empty p { max-width: 600px; margin: 0; }
  .roots { display: flex; flex-direction: column; gap: 4px; align-items: flex-start; padding: 10px 14px; border-radius: 12px; background: rgba(255, 255, 255, 0.03); border: 1px solid var(--line); max-width: 100%; }
  .roots .ok { color: var(--muted); }
  .roots code { font-size: 12px; word-break: break-all; }

  .back { position: fixed; inset: 0; z-index: 60; background: rgba(0, 0, 0, 0.7); display: grid; place-items: center; padding: 24px; }
  .dlg { width: min(1100px, 100%); max-height: 100%; overflow: auto; background: var(--night-2); box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6); gap: 12px; }
  .title { flex: 1; min-width: 0; background: transparent; border: 1px solid transparent; border-radius: 8px; padding: 4px 8px; color: var(--text); font-family: var(--f-display); font-size: 19px; font-weight: 700; }
  .title:hover, .title:focus { border-color: var(--line-2); background: var(--coal-2); }
  .x { border: 0; background: none; color: var(--muted); padding: 6px; border-radius: 8px; display: grid; }
  .x:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .stage { position: relative; background: #000; border-radius: 12px; overflow: hidden; display: grid; place-items: center; max-height: 64vh; }
  .stage video, .stage img { width: 100%; max-height: 64vh; object-fit: contain; display: block; }
  .nav { position: absolute; top: 50%; transform: translateY(-50%); width: 40px; height: 40px; border-radius: 50%; border: 0; background: rgba(12, 14, 17, 0.7); color: var(--text); display: grid; place-items: center; opacity: 0.6; }
  .nav:hover { opacity: 1; }
  .nav.prev { left: 10px; }
  .nav.next { right: 10px; }
  .meta { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; }
  .gsel { display: flex; align-items: center; gap: 8px; }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; }
  .favb.on { border-color: var(--gold); color: var(--gold-glow); background: rgba(227, 194, 112, 0.12); }
  .tools { display: flex; align-items: center; gap: 4px; }
  .tools .ib { display: inline-flex; align-items: center; gap: 7px; height: 34px; padding: 0 11px; border-radius: 10px; border: 1px solid transparent; background: none; color: var(--muted); font: inherit; font-size: 13px; cursor: pointer; }
  .tools .ib:hover { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .tools .sep { width: 1px; height: 22px; background: var(--line-2); margin: 0 6px; }
  .tools .del { padding: 0 9px; }
  .tools .del:hover { color: #ff9a9a; background: rgba(224, 106, 106, 0.1); }
  .tools .del.armed { color: #fff; background: var(--bad); border-color: var(--bad); }
  .sm { padding: 6px 11px; font-size: 13px; }
  @media (max-width: 700px) {
    .grid { grid-template-columns: repeat(auto-fill, minmax(170px, 1fr)); }
  }
</style>
