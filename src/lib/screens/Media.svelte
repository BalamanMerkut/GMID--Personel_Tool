<script lang="ts">
  // Medya Oynatıcı: iki ayrı bölüm. Müzik (solda listeler, üstte "Şimdi çalan", şarkılar liste ya da kapak
  // görünümünde) ve Videolar (büyük kapaklı kartlar). Çalma, ekran değişse de sürer (alttaki mini oynatıcı).
  import { onMount } from "svelte";
  import { api, formatSize, inTauri } from "../api";
  import PlaylistEditor from "../components/PlaylistEditor.svelte";
  import { downloads, formatDuration, siteLabel } from "../downloads.svelte";
  import { shuffle } from "../fun";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../icons";
  import { loadStats, stats, toggleFav } from "../library.svelte";
  import { controls, play, player, playQuiet } from "../player.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { Download, UserPlaylist } from "../types";

  let { onnavigate }: { onnavigate: (id: string) => void } = $props();

  type Section = "music" | "video";
  type Site = "all" | "YouTube" | "X" | "Instagram" | "other";
  /** "auto": görünüme göre (listede liste sırası, "En çok dinlenenler"de dinlenme, yoksa eklenme). */
  type Sort = "auto" | "name" | "artist" | "duration" | "plays";
  type Layout = "list" | "covers";
  const SITES: [Site, string][] = [
    ["all", "Tümü"],
    ["YouTube", "YouTube"],
    ["X", "X"],
    ["Instagram", "Instagram"],
    ["other", "Diğer siteler"],
  ];
  /** "Son eklenenler": en yeni bu kadar şarkı. */
  const RECENT = 30;

  let section: Section = $state(getJson<Section>("media.section", "music"));
  /** Müzikte seçili liste: "all" | "favs" | "recent" | "top" | "my:<id>" | "dl:<ad>". */
  let view = $state("all");
  let sort: Sort = $state(getJson<Sort>("media.sort", "auto"));
  let layout: Layout = $state(getJson<Layout>("media.layout", "list"));
  let site: Site = $state("all");
  let vlist: string | null = $state(null);
  let search = $state("");
  let armed: string | null = $state(null);
  let hovering: string | null = $state(null);
  /** Kendi çalma listelerim. */
  let myLists: UserPlaylist[] = $state([]);
  let editing: UserPlaylist | "new" | null = $state(null);
  /** Yeni liste açılırken önceden seçili gelecek şarkı. */
  let seed: string | null = $state(null);
  /** "Listeye ekle" menüsü açık olan şarkı. */
  let addFor: string | null = $state(null);
  let armedList = $state(false);

  async function loadLists() {
    myLists = await api.videos.playlists().catch(() => []);
    if (view.startsWith("my:") && !myLists.some((p) => `my:${p.id}` === view)) view = "all";
  }
  onMount(() => {
    loadStats();
    void loadLists();
  });
  // Şarkı silinince listelerden de düşer: sayılar tazelensin.
  let seenCount = -1;
  $effect(() => {
    const n = library.length;
    if (seenCount !== -1 && n !== seenCount) void loadLists();
    seenCount = n;
  });
  const currentList = $derived(view.startsWith("my:") ? (myLists.find((p) => `my:${p.id}` === view) ?? null) : null);

  async function toggleInList(p: UserPlaylist, songId: string) {
    const items = p.items.includes(songId) ? p.items.filter((x) => x !== songId) : [...p.items, songId];
    try {
      const saved = await api.videos.savePlaylist(p.id, p.name, items);
      myLists = myLists.map((x) => (x.id === saved.id ? saved : x));
      notify(items.length > p.items.length ? `"${p.name}" listesine eklendi` : `"${p.name}" listesinden çıkarıldı`, "check");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  async function deleteList() {
    if (!currentList) return;
    if (!armedList) {
      armedList = true;
      setTimeout(() => (armedList = false), 3000);
      return;
    }
    armedList = false;
    await api.videos.deletePlaylist(currentList.id);
    notify(`"${currentList.name}" silindi`, "trash");
    view = "all";
    await loadLists();
  }

  function onWindowClick(e: MouseEvent) {
    if (addFor && !(e.target as HTMLElement).closest(".addwrap")) addFor = null;
  }

  const library = $derived(downloads.list.filter((d) => d.status === "done"));
  const music = $derived(library.filter((d) => d.audioOnly));
  const videos = $derived(library.filter((d) => !d.audioOnly));
  const added = (d: Download) => d.finishedAt ?? d.createdAt;
  const plays = (d: Download) => stats.plays[d.id] ?? 0;
  const newest = (a: Download, b: Download) => added(b) - added(a);

  const inSite = (d: Download, t: Site): boolean => {
    if (t === "all") return true;
    const s = siteLabel(d.site);
    if (t === "other") return s !== "YouTube" && s !== "X" && s !== "Instagram";
    return s === t;
  };
  const siteCounts = $derived(Object.fromEntries(SITES.map(([t]) => [t, videos.filter((d) => inSite(d, t)).length])));

  const musicLists = $derived([...new Set(music.map((d) => d.playlist).filter((p): p is string => !!p))]);
  const videoLists = $derived([...new Set(videos.map((d) => d.playlist).filter((p): p is string => !!p))]);
  const favSongs = $derived(music.filter((d) => stats.favs.includes(d.id)));
  const topSongs = $derived(
    music
      .filter((d) => plays(d) > 0)
      .sort((a, b) => plays(b) - plays(a))
      .slice(0, RECENT),
  );

  const shelf = $derived<[string, IconName, string, number][]>([
    ["all", "headphones", "Tüm şarkılar", music.length],
    ["favs", "heart", "Favoriler", favSongs.length],
    ["recent", "clock", "Son eklenenler", Math.min(RECENT, music.length)],
    ["top", "trophy", "En çok dinlenenler", topSongs.length],
  ]);

  /** Seçili görünümün şarkıları, kendi sırasıyla. */
  const base = $derived.by((): Download[] => {
    if (section === "video") return videos.filter((d) => inSite(d, site) && (!vlist || d.playlist === vlist));
    if (view === "favs") return favSongs;
    if (view === "recent") return [...music].sort(newest).slice(0, RECENT);
    if (view === "top") return topSongs;
    if (currentList) return currentList.items.flatMap((id) => music.filter((d) => d.id === id));
    if (view.startsWith("dl:")) return music.filter((d) => d.playlist === view.slice(3));
    return music;
  });

  const collator = new Intl.Collator("tr-TR", { sensitivity: "base", numeric: true });
  const shown = $derived.by(() => {
    const q = search.trim().toLocaleLowerCase("tr-TR");
    const list = base.filter((d) => !q || `${d.title} ${d.uploader} ${d.playlist ?? ""}`.toLocaleLowerCase("tr-TR").includes(q));
    const s = section === "video" && (sort === "artist" || sort === "plays") ? "auto" : sort;
    if (s === "name") return list.sort((a, b) => collator.compare(a.title, b.title));
    if (s === "artist") return list.sort((a, b) => collator.compare(a.uploader, b.uploader) || collator.compare(a.title, b.title));
    if (s === "duration") return list.sort((a, b) => (b.duration ?? 0) - (a.duration ?? 0));
    if (s === "plays") return list.sort((a, b) => plays(b) - plays(a) || newest(a, b));
    // Kendi listemde ve "En çok dinlenenler"de kendi sırası; diğerlerinde en yeni üstte.
    return currentList || (section === "music" && view === "top") ? list : list.sort(newest);
  });

  const viewName = $derived.by(() => {
    if (view === "favs") return "Favoriler";
    if (view === "recent") return "Son eklenenler";
    if (view === "top") return "En çok dinlenenler";
    if (currentList) return currentList.name;
    if (view.startsWith("dl:")) return view.slice(3);
    return "Tüm şarkılar";
  });
  const autoLabel = $derived(currentList ? "Liste sırası" : section === "music" && view === "top" ? "Dinlenme" : "Eklenme (yeni)");
  const totalTime = (list: Download[]) => list.reduce((s, d) => s + (d.duration ?? 0), 0);
  const sectionSize = $derived((section === "music" ? music : videos).reduce((s, d) => s + (d.size ?? 0), 0));
  const musicTime = $derived(totalTime(music));

  function pick(s: Section) {
    section = s;
    vlist = null;
    void setJson("media.section", s);
  }
  function pickView(v: string) {
    view = v;
    armedList = false;
  }
  function setSort(s: Sort) {
    sort = s;
    void setJson("media.sort", s);
  }
  function setLayout(l: Layout) {
    layout = l;
    void setJson("media.layout", l);
  }

  /** Şarkı: oynatıcı öne gelmeden çalar (alttaki çubuk + üstteki kart). Video: oynatıcı açılır. */
  function start(d: Download, queue: Download[]) {
    if (d.audioOnly) playQuiet(d, queue);
    else play(d, queue);
  }
  function startAll(mix: boolean) {
    const q = mix ? shuffle(shown) : shown;
    if (q.length) start(q[0], q);
  }
  const isOn = (d: Download) => player.item?.id === d.id;
  function clickSong(d: Download) {
    if (isOn(d)) controls.toggle();
    else start(d, shown);
  }

  async function act(p: Promise<unknown>) {
    try {
      await p;
    } catch (e) {
      notify(String(e), "info");
    }
  }

  function remove(d: Download) {
    if (armed !== d.id) {
      armed = d.id;
      setTimeout(() => armed === d.id && (armed = null), 3000);
      return;
    }
    armed = null;
    void act(api.videos.remove(d.id, true).then(() => notify("Silindi", "trash")));
  }

  const thumbSrc = (t: string | null) => (!t ? null : /^(https?:|data:)/.test(t) ? t : api.fileSrc(t));
  // Açılamayan küçük resim: bir kez onarmayı dene (siteden yeniden ya da videodan kare); olmazsa simge göster.
  let brokenThumbs: Record<string, boolean> = $state({});
  const repairing = new Set<string>();
  async function thumbFailed(d: Download) {
    if (repairing.has(d.id)) {
      brokenThumbs[d.id] = true;
      return;
    }
    repairing.add(d.id);
    const ok = await api.videos.repairThumb(d.id).catch(() => false);
    if (!ok) brokenThumbs[d.id] = true;
  }
  const thumbOf = (d: Download) => (brokenThumbs[d.id] ? null : thumbSrc(d.thumbnail));
  const previewable = (d: Download) => !!d.filePath && /\.(mp4|webm)$/i.test(d.filePath);

  /** Listedeki ayrı kapaklar (çalan yokken üst kartta 4'lü mozaik). */
  const covers = $derived([...new Set(shown.map(thumbOf).filter((t): t is string => !!t))]);
  /** Üst kartın arka planı: çalan şarkının kapağı, yoksa listedeki ilk kapak. */
  const heroThumb = $derived(player.item ? thumbSrc(player.item.thumbnail) : (covers[0] ?? null));
</script>

{#snippet songActions(d: Download)}
  <span class="sacts" class:open={addFor === d.id || armed === d.id}>
    <span class="addwrap">
      <button class="mini" title="Listeye ekle" onclick={() => (addFor = addFor === d.id ? null : d.id)}><Icon name="plus" size={16} /></button>
      {#if addFor === d.id}
        <div class="addmenu">
          <span class="mlabel">Listeye ekle</span>
          {#each myLists as p (p.id)}
            <button class:on={p.items.includes(d.id)} onclick={() => toggleInList(p, d.id)}>
              <span class="ck">{#if p.items.includes(d.id)}<Icon name="check" size={13} />{/if}</span>{p.name}
            </button>
          {/each}
          <button class="new" onclick={() => ((seed = d.id), (addFor = null), (editing = "new"))}><Icon name="plus" size={13} />Yeni liste…</button>
        </div>
      {/if}
    </span>
    <button class="mini" title="Klasörde göster" onclick={() => act(api.videos.reveal(d.id))}><Icon name="folder" size={16} /></button>
    <button class="mini" class:armed={armed === d.id} title="Sil" onclick={() => remove(d)}>
      <Icon name="trash" size={16} />{#if armed === d.id}<span>Sil?</span>{/if}
    </button>
  </span>
{/snippet}

{#snippet fav(d: Download)}
  {@const f = stats.favs.includes(d.id)}
  <button class="fav" class:on={f} title={f ? "Favorilerden çıkar" : "Favorilere ekle"} aria-pressed={f} onclick={() => toggleFav(d.id)}>
    <Icon name="heart" size={16} />
  </button>
{/snippet}

{#snippet eq(running: boolean)}
  <span class="eq" class:still={!running}><i></i><i></i><i></i></span>
{/snippet}

<div class="md">
  <div class="sections" role="tablist">
    <button role="tab" aria-selected={section === "music"} class:on={section === "music"} onclick={() => pick("music")}>
      <Icon name="headphones" size={22} />
      <span><b>Müzik</b><span class="sub small tnum">{music.length} şarkı{musicTime ? ` · ${formatDuration(musicTime)}` : ""}</span></span>
    </button>
    <button role="tab" aria-selected={section === "video"} class:on={section === "video"} onclick={() => pick("video")}>
      <Icon name="film" size={22} />
      <span><b>Videolar</b><span class="sub small tnum">{videos.length} video · klipler, filmler, diziler</span></span>
    </button>
  </div>

  {#if section === "music"}
    <div class="mus">
      <aside class="side" aria-label="Listeler">
        <span class="glabel">Kitaplık</span>
        {#each shelf as [v, icon, label, n] (v)}
          <button class="li" class:on={view === v} onclick={() => pickView(v)}>
            <Icon name={icon} size={16} /><span class="lname">{label}</span><span class="n tnum">{n}</span>
          </button>
        {/each}

        <span class="glabel row2">Listelerim
          <button class="gadd" title="Yeni liste" onclick={() => ((seed = null), (editing = "new"))}><Icon name="plus" size={14} /></button>
        </span>
        {#each myLists as p (p.id)}
          <button class="li" class:on={view === `my:${p.id}`} onclick={() => pickView(`my:${p.id}`)}>
            <Icon name="list" size={16} /><span class="lname">{p.name}</span><span class="n tnum">{p.items.length}</span>
          </button>
        {:else}
          <button class="li ghostli" onclick={() => ((seed = null), (editing = "new"))}><Icon name="plus" size={16} /><span class="lname">Yeni liste</span></button>
        {/each}

        {#if musicLists.length}
          <span class="glabel">İndirilen listeler</span>
          {#each musicLists as p (p)}
            <button class="li" class:on={view === `dl:${p}`} onclick={() => pickView(`dl:${p}`)}>
              <Icon name="download" size={16} /><span class="lname">{p}</span><span class="n tnum">{music.filter((d) => d.playlist === p).length}</span>
            </button>
          {/each}
        {/if}

        <div class="sfoot">
          <span class="sub small tnum">{formatSize(sectionSize)}</span>
          <button class="btn ghost sm" onclick={() => api.openDataFolder("Videolar")}><Icon name="folder" size={15} />Klasör</button>
        </div>
      </aside>

      <div class="main">
        <section class="hero" class:live={!!player.item}>
          {#if heroThumb}<div class="hbg" style:background-image={`url("${heroThumb}")`}></div>{/if}
          <div class="hshade"></div>
          {#if player.item}
            {@const it = player.item}
            <button class="hcover" title="Oynatıcıyı aç" onclick={() => (player.open = true)}>
              {#if thumbSrc(it.thumbnail)}<img src={thumbSrc(it.thumbnail)} alt="" />{:else}<Icon name="headphones" size={40} />{/if}
            </button>
            <div class="htx">
              <span class="kicker">{#if player.playing}{@render eq(true)}Şimdi çalıyor{:else}Duraklatıldı{/if}</span>
              <h2 title={it.title}>{it.title}</h2>
              <span class="sub">{[it.uploader, it.playlist].filter(Boolean).join(" · ")}</span>
              {#key player.line}
                <p class="lyric" class:none={!player.line}>{player.line || " "}</p>
              {/key}
              <div class="hacts">
                <button class="bigplay" title={player.playing ? "Duraklat" : "Oynat"} onclick={() => controls.toggle()}>
                  <Icon name={player.playing ? "pause" : "play"} size={24} />
                </button>
                <button class="btn sm" disabled={!shown.length} onclick={() => startAll(true)}><Icon name="shuffle" size={16} />Karıştırarak çal</button>
                <button class="btn ghost sm" onclick={() => (player.open = true)}><Icon name="expand" size={16} />Oynatıcıyı aç</button>
              </div>
            </div>
          {:else}
            <div class="hcover" class:mosaic={covers.length >= 4} aria-hidden="true">
              {#each covers.length >= 4 ? covers.slice(0, 4) : covers.slice(0, 1) as t (t)}
                <img src={t} alt="" />
              {:else}
                <Icon name="headphones" size={40} />
              {/each}
            </div>
            <div class="htx">
              <span class="kicker">Liste</span>
              <h2>{viewName}</h2>
              <span class="sub tnum">{shown.length} şarkı{totalTime(shown) ? ` · ${formatDuration(totalTime(shown))}` : ""}</span>
              <div class="hacts">
                <button class="bigplay" title="Hepsini çal" disabled={!shown.length} onclick={() => startAll(false)}><Icon name="play" size={24} /></button>
                <button class="btn sm" disabled={!shown.length} onclick={() => startAll(true)}><Icon name="shuffle" size={16} />Karıştırarak çal</button>
              </div>
            </div>
          {/if}
        </section>

        <div class="bar">
          <div class="btitle">
            <b>{viewName}</b>
            <span class="sub small tnum">{shown.length} şarkı</span>
            {#if currentList}
              <button class="btn ghost sm" onclick={() => (editing = currentList)}><Icon name="edit" size={15} />Düzenle</button>
              <button class="btn ghost sm" class:danger={armedList} onclick={deleteList}><Icon name="trash" size={15} />{armedList ? "Emin misin?" : "Sil"}</button>
            {/if}
          </div>
          <div class="row">
            <input class="search" type="search" placeholder="Ara…" bind:value={search} aria-label="Ara" />
            <select value={sort} onchange={(e) => setSort(e.currentTarget.value as Sort)} aria-label="Sırala">
              <option value="auto">{autoLabel}</option>
              <option value="name">Ada göre</option>
              <option value="artist">Sanatçıya göre</option>
              <option value="duration">Süreye göre</option>
              <option value="plays">En çok dinlenen</option>
            </select>
            <div class="toggle" role="group" aria-label="Görünüm">
              <button aria-pressed={layout === "list"} title="Liste" onclick={() => setLayout("list")}><Icon name="list" size={16} /></button>
              <button aria-pressed={layout === "covers"} title="Kapak" onclick={() => setLayout("covers")}><Icon name="grid" size={16} /></button>
            </div>
          </div>
        </div>

        {#if layout === "list"}
          <div class="tracks" role="list">
            {#if shown.length}
              <div class="thead sub small" aria-hidden="true"><span>#</span><span></span><span>Başlık</span><span></span><span class="r"><Icon name="clock" size={14} /></span><span></span></div>
            {/if}
            {#each shown as d, i (d.id)}
              {@const on = isOn(d)}
              <div class="track" class:on role="listitem" ondblclick={() => start(d, shown)}>
                <span class="tn tnum">{#if on}{@render eq(player.playing)}{:else}{i + 1}{/if}</span>
                <button class="tthumb" title={on ? (player.playing ? "Duraklat" : "Oynat") : "Çal"} onclick={() => clickSong(d)}>
                  {#if thumbOf(d)}<img src={thumbOf(d)} alt="" loading="lazy" onerror={() => thumbFailed(d)} />{:else}<Icon name="headphones" size={16} />{/if}
                  <span class="tplay"><Icon name={on && player.playing ? "pause" : "play"} size={18} /></span>
                </button>
                <span class="ttx">
                  <b title={d.title}>{d.title}</b>
                  <span class="sub small">{d.uploader}{#if view === "top" && plays(d)} · {plays(d)} kez{/if}</span>
                </span>
                {@render fav(d)}
                <span class="sub small tnum r">{formatDuration(d.duration ?? 0)}</span>
                {@render songActions(d)}
              </div>
            {:else}
              <p class="sub nothing">{@render emptyText()}</p>
            {/each}
          </div>
        {:else}
          <div class="covers">
            {#each shown as d (d.id)}
              {@const on = isOn(d)}
              <article class="cover" class:on ondblclick={() => start(d, shown)}>
                <div class="cimg">
                  {#if thumbOf(d)}<img src={thumbOf(d)} alt="" loading="lazy" onerror={() => thumbFailed(d)} />{:else}<span class="ph"><Icon name="headphones" size={30} /></span>{/if}
                  {#if on}<span class="badge">{@render eq(player.playing)}</span>{/if}
                  <span class="cfav">{@render fav(d)}</span>
                  <button class="round" title={on && player.playing ? "Duraklat" : "Çal"} onclick={() => clickSong(d)}>
                    <Icon name={on && player.playing ? "pause" : "play"} size={20} />
                  </button>
                </div>
                <b title={d.title}>{d.title}</b>
                <div class="cfoot">
                  <span class="sub small">{d.uploader}</span>
                  {@render songActions(d)}
                </div>
              </article>
            {:else}
              <p class="sub nothing">{@render emptyText()}</p>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {:else}
    <section class="panel lib">
      <div class="spread">
        <h3 class="h2"><Icon name="film" />Videolar <span class="sub small tnum">{formatSize(sectionSize)}</span></h3>
        <div class="row">
          {#if shown.length}
            <button class="btn sm" onclick={() => play(shown[0], shown)}><Icon name="play" size={16} />Hepsini oynat</button>
          {/if}
          <input class="search" type="search" placeholder="Ara…" bind:value={search} aria-label="Ara" />
          <select value={sort === "artist" || sort === "plays" ? "auto" : sort} onchange={(e) => setSort(e.currentTarget.value as Sort)} aria-label="Sırala">
            <option value="auto">Eklenme (yeni)</option>
            <option value="name">Ada göre</option>
            <option value="duration">Süreye göre</option>
          </select>
          <button class="btn ghost sm" onclick={() => api.openDataFolder("Videolar")}><Icon name="folder" size={16} />Klasör</button>
        </div>
      </div>
      <div class="seg" role="tablist">
        {#each SITES as [t, label] (t)}
          {#if t === "all" || siteCounts[t]}
            <button aria-pressed={site === t} onclick={() => (site = t)}>{label} <span class="n">{siteCounts[t]}</span></button>
          {/if}
        {/each}
      </div>
      {#if videoLists.length}
        <div class="lists">
          <span class="sub small">Listeler:</span>
          <button class="chipbtn" aria-pressed={!vlist} onclick={() => (vlist = null)}>Hepsi</button>
          {#each videoLists as p (p)}
            <button class="chipbtn" aria-pressed={vlist === p} onclick={() => (vlist = vlist === p ? null : p)}><Icon name="list" size={13} />{p}</button>
          {/each}
        </div>
      {/if}

      <div class="grid">
        {#each shown as d (d.id)}
          <article
            class="card"
            class:on={isOn(d)}
            ondblclick={() => play(d, shown)}
            onmouseenter={() => (hovering = d.id)}
            onmouseleave={() => hovering === d.id && (hovering = null)}
            title="Çift tıkla: oynat"
          >
            <div class="cthumb">
              {#if hovering === d.id && previewable(d) && inTauri}
                <!-- svelte-ignore a11y_media_has_caption -->
                <video src={api.fileSrc(d.filePath ?? "")} autoplay muted loop playsinline></video>
              {:else if thumbOf(d)}
                <img src={thumbOf(d)} alt="" loading="lazy" onerror={() => thumbFailed(d)} />
              {:else}
                <span class="ph"><Icon name="film" size={34} /></span>
              {/if}
              <span class="qual">{d.quality}</span>
              {#if d.duration}<span class="dur tnum">{formatDuration(d.duration)}</span>{/if}
              <div class="hover">
                <button class="round big" title="Oynat" onclick={() => play(d, shown)}><Icon name="play" size={24} /></button>
              </div>
            </div>
            <div class="cbody">
              <b class="ctitle" title={d.title}>{d.title}</b>
              <div class="spread">
                <span class="sub small">{[d.uploader, siteLabel(d.site)].filter(Boolean).join(" · ")}{d.finishedAt ? ` · ${ago(d.finishedAt)}` : ""}</span>
                <span class="sacts" class:open={armed === d.id}>
                  <button class="mini" title="Klasörde göster" onclick={() => act(api.videos.reveal(d.id))}><Icon name="folder" size={16} /></button>
                  <button class="mini" class:armed={armed === d.id} title="Videoyu sil" onclick={() => remove(d)}>
                    <Icon name="trash" size={16} />{#if armed === d.id}<span>Sil?</span>{/if}
                  </button>
                </span>
              </div>
            </div>
          </article>
        {:else}
          <p class="sub nothing">{videos.length ? "Bu süzgeçte video yok." : "Henüz video yok. Video İndirici'de bir bağlantı yapıştırarak başla."}</p>
        {/each}
      </div>
    </section>
  {/if}
  {#if !library.length}
    <button class="btn ghost" onclick={() => onnavigate("videos")}><Icon name="download" size={17} />Video İndirici'ye git</button>
  {/if}
</div>

{#snippet emptyText()}
  {#if !music.length}
    Henüz şarkı yok. Video İndirici'de "MP3" ya da bir çalma listesi indir.
  {:else if search.trim()}
    Aramaya uyan şarkı yok.
  {:else if view === "favs"}
    Henüz favori yok. Şarkının yanındaki kalbe dokun.
  {:else if view === "top"}
    Henüz dinlenen şarkı yok. Çaldıkça burada sıralanır.
  {:else}
    Bu liste boş. Şarkının yanındaki + ile ekleyebilirsin.
  {/if}
{/snippet}

<svelte:window onclick={onWindowClick} />

{#if editing}
  <PlaylistEditor
    songs={music}
    initial={editing === "new" ? (seed ? { id: "", name: "", createdAt: 0, items: [seed] } : null) : editing}
    onclose={() => (editing = null)}
    onsaved={(p) => {
      editing = null;
      void loadLists().then(() => (view = `my:${p.id}`));
    }}
  />
{/if}

<style>
  .md { display: flex; flex-direction: column; gap: 16px; max-width: 1500px; margin: 0 auto; }
  .sm { padding: 6px 11px; font-size: 13px; }
  .search { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 6px 10px; width: 180px; }
  .btn.danger { color: var(--bad); border-color: rgba(224, 106, 106, 0.5); }
  .nothing { grid-column: 1 / -1; margin: 10px 4px; }
  .r { text-align: right; justify-self: end; }

  /* Müzik / Videolar */
  .sections { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .sections button { display: flex; align-items: center; gap: 14px; text-align: left; padding: 12px 18px; border-radius: 16px; border: 1px solid var(--line-2); background: rgba(23, 26, 30, 0.7); color: var(--muted); transition: border-color 0.15s, background 0.15s, color 0.15s; }
  .sections button:hover { border-color: var(--gold-dim); color: var(--text); }
  .sections button.on { border-color: var(--gold); color: var(--gold-glow); background: linear-gradient(135deg, rgba(227, 194, 112, 0.14), rgba(227, 194, 112, 0.03)); box-shadow: 0 0 22px rgba(227, 194, 112, 0.1); }
  .sections b { display: block; font-family: var(--f-display); font-size: 17px; color: var(--text); }
  .sections .on b { color: var(--gold-glow); }

  /* Müzik: solda listeler, sağda içerik */
  .mus { display: grid; grid-template-columns: 236px minmax(0, 1fr); gap: 16px; align-items: start; }
  .side { position: sticky; top: 0; display: flex; flex-direction: column; gap: 2px; padding: 12px 10px; border-radius: 16px; border: 1px solid var(--line); background: rgba(23, 26, 30, 0.72); max-height: calc(100vh - 140px); overflow: auto; }
  .glabel { display: flex; align-items: center; justify-content: space-between; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--dim); padding: 10px 8px 6px; }
  .glabel:first-child { padding-top: 2px; }
  .gadd { border: 0; background: transparent; color: var(--gold); border-radius: 6px; padding: 2px; display: inline-grid; }
  .gadd:hover { background: rgba(227, 194, 112, 0.14); }
  .li { display: flex; align-items: center; gap: 10px; width: 100%; border: 0; background: transparent; color: var(--muted); text-align: left; padding: 8px 10px; border-radius: 10px; font-size: 13.5px; transition: background 0.12s, color 0.12s; }
  .li:hover { background: rgba(255, 255, 255, 0.05); color: var(--text); }
  .li.on { background: rgba(227, 194, 112, 0.13); color: var(--gold-glow); box-shadow: inset 3px 0 0 var(--gold); }
  .li .lname { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .li .n { color: var(--dim); font-size: 12px; }
  .li.on .n { color: var(--gold-dim); }
  .ghostli { color: var(--gold); }
  .sfoot { display: flex; align-items: center; justify-content: space-between; gap: 8px; margin-top: 10px; padding: 10px 6px 0; border-top: 1px solid var(--line); }
  .main { display: flex; flex-direction: column; gap: 12px; min-width: 0; }

  /* Şimdi çalan */
  .hero { position: relative; overflow: hidden; display: flex; align-items: center; gap: 22px; padding: 22px; border-radius: 18px; border: 1px solid var(--line-2); background: var(--coal-2); min-height: 196px; }
  .hero.live { border-color: rgba(227, 194, 112, 0.4); }
  .hbg { position: absolute; inset: -40px; background-size: cover; background-position: center; filter: blur(38px) saturate(1.3); opacity: 0.55; transform: scale(1.1); }
  .hshade { position: absolute; inset: 0; background: linear-gradient(90deg, rgba(14, 16, 19, 0.82), rgba(14, 16, 19, 0.45)); }
  .hcover { position: relative; flex: none; width: 152px; height: 152px; border-radius: 14px; overflow: hidden; border: 0; padding: 0; background: rgba(0, 0, 0, 0.35); color: var(--dim); display: grid; place-items: center; box-shadow: 0 14px 34px rgba(0, 0, 0, 0.5); }
  .hcover img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .mosaic { grid-template-columns: 1fr 1fr; grid-template-rows: 1fr 1fr; }
  .htx { position: relative; flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .kicker { display: inline-flex; align-items: center; gap: 8px; font-size: 11.5px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--gold); }
  .htx h2 { margin: 0; font-family: var(--f-display); font-size: 26px; line-height: 1.2; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .lyric { margin: 6px 0 0; min-height: 1.4em; font-size: 15px; font-style: italic; color: var(--gold-glow); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; animation: lyricIn 0.35s ease-out; }
  .lyric.none { animation: none; }
  @keyframes lyricIn { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: none; } }
  .hacts { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; margin-top: 10px; }
  .bigplay { width: 52px; height: 52px; border-radius: 50%; border: 0; background: var(--gold); color: #1b1608; display: grid; place-items: center; box-shadow: 0 8px 22px rgba(227, 194, 112, 0.3); transition: transform 0.12s, background 0.12s; }
  .bigplay:hover:not(:disabled) { transform: scale(1.06); background: var(--gold-glow); }
  .bigplay:disabled { opacity: 0.4; }

  /* Araç çubuğu */
  .bar { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; }
  .btitle { display: flex; align-items: baseline; gap: 10px; min-width: 0; }
  .btitle b { font-family: var(--f-display); font-size: 17px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .toggle { display: inline-flex; border: 1px solid var(--line-2); border-radius: 10px; overflow: hidden; }
  .toggle button { border: 0; background: transparent; color: var(--dim); padding: 6px 9px; display: grid; place-items: center; }
  .toggle button:hover { color: var(--text); }
  .toggle button[aria-pressed="true"] { background: rgba(227, 194, 112, 0.14); color: var(--gold-glow); }

  /* Şarkı satırları */
  .tracks { display: flex; flex-direction: column; gap: 2px; }
  .thead, .track { display: grid; grid-template-columns: 30px 46px minmax(0, 1fr) 34px 56px 104px; gap: 12px; align-items: center; }
  .thead { padding: 0 10px 6px; border-bottom: 1px solid var(--line); margin-bottom: 4px; text-transform: uppercase; letter-spacing: 0.06em; font-size: 11px; }
  .thead span:first-child { text-align: center; }
  .track { padding: 6px 10px; border-radius: 10px; transition: background 0.12s; }
  .track:hover { background: rgba(255, 255, 255, 0.045); }
  .track.on { background: rgba(227, 194, 112, 0.1); }
  .track.on .ttx b { color: var(--gold-glow); }
  .tn { color: var(--dim); font-size: 12.5px; text-align: center; display: grid; place-items: center; }
  .tthumb { position: relative; width: 46px; height: 46px; border-radius: 8px; overflow: hidden; border: 0; padding: 0; background: var(--coal-2); display: grid; place-items: center; color: var(--dim); }
  .tthumb img { width: 100%; height: 100%; object-fit: cover; }
  .tplay { position: absolute; inset: 0; display: grid; place-items: center; background: rgba(0, 0, 0, 0.55); color: var(--gold-glow); opacity: 0; transition: opacity 0.12s; }
  .track:hover .tplay, .tthumb:focus-visible .tplay { opacity: 1; }
  .ttx { display: flex; flex-direction: column; min-width: 0; }
  .ttx b, .ttx .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ttx b { font-weight: 600; font-size: 14px; }
  .fav { border: 0; background: transparent; color: var(--dim); padding: 6px; border-radius: 8px; display: grid; place-items: center; opacity: 0; transition: opacity 0.12s, color 0.12s; }
  .fav:hover { color: var(--text); }
  .fav.on { color: var(--gold); opacity: 1; }
  .fav.on :global(svg) { fill: currentColor; }
  .track:hover .fav, .fav:focus-visible { opacity: 1; }
  .sacts { display: flex; justify-content: flex-end; opacity: 0; transition: opacity 0.12s; }
  .track:hover .sacts, .cover:hover .sacts, .card:hover .sacts, .sacts.open, .sacts:focus-within { opacity: 1; }
  .track:has(.addmenu), .cover:has(.addmenu) { position: relative; z-index: 5; }
  .mini { border: 0; background: transparent; color: var(--dim); padding: 6px; border-radius: 7px; display: inline-flex; align-items: center; gap: 4px; }
  .mini:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .mini.armed { color: var(--bad); background: rgba(224, 106, 106, 0.12); font-size: 12px; font-weight: 700; }
  .eq { display: inline-flex; gap: 2px; align-items: flex-end; height: 12px; }
  .eq i { width: 3px; height: 12px; background: var(--gold); border-radius: 1px; animation: eq 0.9s ease-in-out infinite; }
  .eq i:nth-child(2) { animation-delay: 0.2s; }
  .eq i:nth-child(3) { animation-delay: 0.4s; }
  .eq.still i { animation: none; height: 5px; }
  .eq.still i:nth-child(2) { height: 9px; }
  @keyframes eq { 0%, 100% { height: 3px; } 50% { height: 12px; } }

  /* Listeye ekle menüsü */
  .addwrap { position: relative; display: inline-flex; }
  .addmenu { position: absolute; right: 0; top: calc(100% + 6px); z-index: 20; width: 220px; padding: 8px; border-radius: 12px; border: 1px solid var(--line-2); background: var(--coal-2); box-shadow: 0 18px 40px rgba(0, 0, 0, 0.5); display: flex; flex-direction: column; gap: 2px; }
  .addmenu .mlabel { font-size: 11px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--dim); padding: 2px 6px 6px; }
  .addmenu button { display: flex; align-items: center; gap: 8px; border: 0; background: transparent; color: var(--text); text-align: left; padding: 7px 8px; border-radius: 8px; font-size: 13px; }
  .addmenu button:hover { background: rgba(255, 255, 255, 0.06); }
  .addmenu button.on { color: var(--gold); }
  .addmenu .ck { width: 16px; display: inline-grid; }
  .addmenu .new { color: var(--gold); border-top: 1px solid var(--line); border-radius: 0 0 8px 8px; margin-top: 4px; }

  /* Kapak görünümü */
  .covers { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 168px), 1fr)); gap: 16px 14px; }
  .cover { display: flex; flex-direction: column; gap: 6px; min-width: 0; padding: 8px; border-radius: 14px; transition: background 0.12s; }
  .cover:hover { background: rgba(255, 255, 255, 0.045); }
  .cover > b { font-weight: 600; font-size: 13.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cover.on > b { color: var(--gold-glow); }
  .cimg { position: relative; aspect-ratio: 1; border-radius: 12px; overflow: hidden; background: var(--coal-2); border: 1px solid var(--line); }
  .cover.on .cimg { border-color: var(--gold); box-shadow: 0 0 18px rgba(227, 194, 112, 0.25); }
  .cimg img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .cimg .round { position: absolute; right: 8px; bottom: 8px; opacity: 0; transform: translateY(6px); transition: opacity 0.15s, transform 0.15s; box-shadow: 0 6px 16px rgba(0, 0, 0, 0.45); }
  .cover:hover .round, .cover.on .round { opacity: 1; transform: none; }
  .badge { position: absolute; left: 8px; top: 8px; padding: 4px 6px; border-radius: 6px; background: rgba(14, 16, 19, 0.85); display: grid; }
  .cfav { position: absolute; right: 4px; top: 4px; }
  .cfav .fav { background: rgba(14, 16, 19, 0.7); }
  .cover:hover .fav { opacity: 1; }
  .cfoot { display: flex; align-items: center; justify-content: space-between; gap: 4px; min-width: 0; margin-top: -4px; }
  .cfoot .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .cfoot .mini { padding: 4px; }

  /* Videolar */
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .lib .h2 .sub { font-family: var(--f-body); font-weight: 500; margin-left: 4px; }
  .seg .n { color: var(--dim); margin-left: 3px; font-variant-numeric: tabular-nums; }
  .lists { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .chipbtn { display: inline-flex; align-items: center; gap: 5px; border: 1px solid var(--line-2); background: var(--coal-2); border-radius: 999px; padding: 4px 10px; font-size: 12px; color: var(--text); }
  .chipbtn:hover { border-color: var(--gold-dim); }
  .chipbtn[aria-pressed="true"] { border-color: var(--gold); color: var(--gold-glow); background: rgba(227, 194, 112, 0.12); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 290px), 1fr)); gap: 22px 18px; }
  .card { display: flex; flex-direction: column; gap: 10px; min-width: 0; cursor: default; }
  .cthumb { position: relative; aspect-ratio: 16 / 9; border-radius: 14px; overflow: hidden; background: var(--coal-2); border: 1px solid var(--line); transition: border-color 0.15s, transform 0.15s, box-shadow 0.15s; }
  .cthumb img, .cthumb video { width: 100%; height: 100%; object-fit: cover; display: block; }
  .card:hover .cthumb { border-color: var(--gold-dim); transform: translateY(-2px); box-shadow: 0 12px 28px rgba(0, 0, 0, 0.4); }
  .card.on .cthumb { border-color: var(--gold); box-shadow: 0 0 18px rgba(227, 194, 112, 0.25); }
  .ph { position: absolute; inset: 0; display: grid; place-items: center; color: var(--dim); }
  .qual { position: absolute; left: 8px; top: 8px; background: rgba(18, 20, 23, 0.85); color: var(--gold); font-size: 10.5px; font-weight: 700; padding: 1px 6px; border-radius: 5px; }
  .dur { position: absolute; right: 8px; bottom: 8px; background: rgba(0, 0, 0, 0.8); font-size: 12px; font-weight: 700; padding: 2px 7px; border-radius: 6px; }
  .hover { position: absolute; inset: 0; display: grid; place-items: center; opacity: 0; transition: opacity 0.15s; background: rgba(0, 0, 0, 0.2); }
  .card:hover .hover { opacity: 1; }
  .round { width: 44px; height: 44px; border-radius: 50%; border: 0; background: rgba(227, 194, 112, 0.94); color: #1b1608; display: grid; place-items: center; }
  .round.big { width: 56px; height: 56px; }
  .cbody { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .ctitle { font-weight: 600; font-size: 14.5px; line-height: 1.35; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .cbody .spread .sub { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  @media (max-width: 980px) {
    .mus { grid-template-columns: minmax(0, 1fr); }
    .side { position: static; flex-direction: row; flex-wrap: wrap; max-height: none; gap: 4px; }
    .side .glabel, .sfoot { display: none; }
    .li { width: auto; padding: 6px 10px; border: 1px solid var(--line-2); border-radius: 999px; font-size: 12.5px; }
    .li.on { box-shadow: none; border-color: var(--gold); }
    .hcover { width: 112px; height: 112px; }
    .htx h2 { font-size: 21px; }
    .thead, .track { grid-template-columns: 26px 44px minmax(0, 1fr) 30px 48px auto; gap: 8px; }
  }
</style>
