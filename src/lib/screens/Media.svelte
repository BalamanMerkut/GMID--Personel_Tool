<script lang="ts">
  // Medya Oynatıcı: iki ayrı bölüm. Müzik (şarkı listesi) ve Videolar (kartlar: klipler, filmler…).
  // Çalma, ekran değişse de sürer (alttaki mini oynatıcı).
  import { api, formatSize, inTauri } from "../api";
  import { downloads, formatDuration, siteLabel } from "../downloads.svelte";
  import Icon from "../Icon.svelte";
  import { play, player } from "../player.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { Download } from "../types";

  let { onnavigate }: { onnavigate: (id: string) => void } = $props();

  type Section = "music" | "video";
  type Site = "all" | "YouTube" | "X" | "Instagram" | "other";
  const SITES: [Site, string][] = [
    ["all", "Tümü"],
    ["YouTube", "YouTube"],
    ["X", "X"],
    ["Instagram", "Instagram"],
    ["other", "Diğer siteler"],
  ];

  let section: Section = $state(getJson<Section>("media.section", "music"));
  let site: Site = $state("all");
  let search = $state("");
  let plFilter: string | null = $state(null);
  let armed: string | null = $state(null);
  let hovering: string | null = $state(null);

  const library = $derived(downloads.list.filter((d) => d.status === "done"));
  const music = $derived(library.filter((d) => d.audioOnly));
  const videos = $derived(library.filter((d) => !d.audioOnly));
  const inSite = (d: Download, t: Site): boolean => {
    if (t === "all") return true;
    const s = siteLabel(d.site);
    if (t === "other") return s !== "YouTube" && s !== "X" && s !== "Instagram";
    return s === t;
  };
  const siteCounts = $derived(Object.fromEntries(SITES.map(([t]) => [t, videos.filter((d) => inSite(d, t)).length])));
  const pool = $derived(section === "music" ? music : videos.filter((d) => inSite(d, site)));
  const shown = $derived.by(() => {
    const q = search.trim().toLocaleLowerCase("tr-TR");
    return pool.filter(
      (d) =>
        (!plFilter || d.playlist === plFilter) &&
        (!q || `${d.title} ${d.uploader} ${d.playlist ?? ""}`.toLocaleLowerCase("tr-TR").includes(q)),
    );
  });
  const playlists = $derived([
    ...new Set((section === "music" ? music : videos).map((d) => d.playlist).filter((p): p is string => !!p)),
  ]);
  const sectionSize = $derived((section === "music" ? music : videos).reduce((s, d) => s + (d.size ?? 0), 0));
  const musicTime = $derived(music.reduce((s, d) => s + (d.duration ?? 0), 0));

  function pick(s: Section) {
    section = s;
    plFilter = null;
    void setJson("media.section", s);
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
  const previewable = (d: Download) => !!d.filePath && /\.(mp4|webm)$/i.test(d.filePath);
</script>

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

  {#if player.item}
    <section class="panel now">
      <div class="nthumb">{#if thumbSrc(player.item.thumbnail)}<img src={thumbSrc(player.item.thumbnail)} alt="" />{/if}</div>
      <div class="ntx">
        <span class="sub small">{player.playing ? "Şu an çalıyor" : "Duraklatıldı"}</span>
        <b title={player.item.title}>{player.item.title}</b>
        <span class="sub small">{[player.item.uploader, player.item.playlist].filter(Boolean).join(" · ")}</span>
      </div>
      <button class="btn" onclick={() => (player.open = true)}><Icon name="expand" size={17} />Oynatıcıyı aç</button>
    </section>
  {/if}

  <section class="panel lib">
    <div class="spread">
      <h3 class="h2">
        <Icon name={section === "music" ? "headphones" : "film"} />{section === "music" ? "Şarkılar" : "Videolar"}
        <span class="sub small tnum">{formatSize(sectionSize)}</span>
      </h3>
      <div class="row">
        {#if shown.length}
          <button class="btn sm" onclick={() => play(shown[0], shown)}><Icon name="play" size={16} />Hepsini çal</button>
        {/if}
        <input class="search" type="search" placeholder="Ara…" bind:value={search} aria-label="Ara" />
        <button class="btn ghost sm" onclick={() => api.openDataFolder("Videolar")}><Icon name="folder" size={16} />Klasör</button>
      </div>
    </div>
    {#if section === "video"}
      <div class="seg" role="tablist">
        {#each SITES as [t, label] (t)}
          {#if t === "all" || siteCounts[t]}
            <button aria-pressed={site === t} onclick={() => (site = t)}>{label} <span class="n">{siteCounts[t]}</span></button>
          {/if}
        {/each}
      </div>
    {/if}
    {#if playlists.length}
      <div class="lists">
        <span class="sub small">Listeler:</span>
        <button class="chipbtn" aria-pressed={!plFilter} onclick={() => (plFilter = null)}>Hepsi</button>
        {#each playlists as p (p)}
          <button class="chipbtn" aria-pressed={plFilter === p} onclick={() => (plFilter = plFilter === p ? null : p)}><Icon name="list" size={13} />{p}</button>
        {/each}
      </div>
    {/if}

    {#if section === "music"}
      <div class="tracks" role="list">
        {#each shown as d, i (d.id)}
          {@const on = player.item?.id === d.id}
          <div class="track" class:on role="listitem" ondblclick={() => play(d, shown)}>
            <button class="tn tnum" title="Çal" onclick={() => (on ? (player.open = true) : play(d, shown))}>
              {#if on && player.playing}<span class="eq"><i></i><i></i><i></i></span>{:else}<span class="num">{i + 1}</span><span class="pl"><Icon name="play" size={15} /></span>{/if}
            </button>
            <span class="tthumb">{#if thumbSrc(d.thumbnail)}<img src={thumbSrc(d.thumbnail)} alt="" loading="lazy" />{:else}<Icon name="headphones" size={16} />{/if}</span>
            <span class="ttx">
              <b title={d.title}>{d.title}</b>
              <span class="sub small">{d.uploader}</span>
            </span>
            <span class="sub small tpl">{d.playlist ?? ""}</span>
            <span class="sub small tnum">{formatDuration(d.duration ?? 0)}</span>
            <span class="cacts">
              <button class="mini" title="Klasörde göster" onclick={() => act(api.videos.reveal(d.id))}><Icon name="folder" size={16} /></button>
              <button class="mini" class:armed={armed === d.id} title="Sil" onclick={() => remove(d)}>
                <Icon name="trash" size={16} />{#if armed === d.id}<span>Sil?</span>{/if}
              </button>
            </span>
          </div>
        {:else}
          <p class="sub empty">
            {music.length ? "Aramaya uyan şarkı yok." : "Henüz şarkı yok. Video İndirici'de \"MP3\" ya da bir çalma listesi indir."}
          </p>
        {/each}
      </div>
    {:else}
      <div class="grid">
        {#each shown as d (d.id)}
          <article
            class="card"
            class:on={player.item?.id === d.id}
            ondblclick={() => play(d, shown)}
            onmouseenter={() => (hovering = d.id)}
            onmouseleave={() => hovering === d.id && (hovering = null)}
            title="Çift tıkla: oynat"
          >
            <div class="cthumb">
              {#if hovering === d.id && previewable(d) && inTauri}
                <!-- svelte-ignore a11y_media_has_caption -->
                <video src={api.fileSrc(d.filePath ?? "")} autoplay muted loop playsinline></video>
              {:else if thumbSrc(d.thumbnail)}
                <img src={thumbSrc(d.thumbnail)} alt="" loading="lazy" />
              {:else}
                <span class="ph"><Icon name="film" size={30} /></span>
              {/if}
              <span class="qual">{d.quality}</span>
              {#if d.duration}<span class="dur tnum">{formatDuration(d.duration)}</span>{/if}
              <div class="hover">
                <button class="round" title="Oynat" onclick={() => play(d, shown)}><Icon name="play" size={20} /></button>
              </div>
            </div>
            <div class="cbody">
              <b class="ctitle" title={d.title}>{d.title}</b>
              <span class="sub small">{[d.uploader, siteLabel(d.site)].filter(Boolean).join(" · ")}</span>
              <div class="spread">
                <span class="sub small tnum">{d.size ? formatSize(d.size) : ""}{d.finishedAt ? ` · ${ago(d.finishedAt)}` : ""}</span>
                <span class="cacts">
                  <button class="mini" title="Klasörde göster" onclick={() => act(api.videos.reveal(d.id))}><Icon name="folder" size={16} /></button>
                  <button class="mini" class:armed={armed === d.id} title="Videoyu sil" onclick={() => remove(d)}>
                    <Icon name="trash" size={16} />{#if armed === d.id}<span>Sil?</span>{/if}
                  </button>
                </span>
              </div>
            </div>
          </article>
        {:else}
          <p class="sub empty">{videos.length ? "Bu süzgeçte video yok." : "Henüz video yok. Video İndirici'de bir bağlantı yapıştırarak başla."}</p>
        {/each}
      </div>
    {/if}
  </section>
  {#if !library.length}
    <button class="btn ghost" onclick={() => onnavigate("videos")}><Icon name="download" size={17} />Video İndirici'ye git</button>
  {/if}
</div>

<style>
  .md { display: flex; flex-direction: column; gap: 16px; max-width: 1500px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 10px; }
  .now { flex-direction: row; align-items: center; gap: 14px; border-color: rgba(227, 194, 112, 0.35); }
  .nthumb { width: 96px; aspect-ratio: 16 / 9; border-radius: 9px; overflow: hidden; background: var(--coal-2); flex: none; }
  .nthumb img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .ntx { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .ntx b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: var(--f-display); }
  .cthumb { position: relative; aspect-ratio: 16 / 9; border-radius: 12px; overflow: hidden; background: var(--coal-2); }
  .cthumb img, .cthumb video { width: 100%; height: 100%; object-fit: cover; display: block; }
  .dur { position: absolute; right: 6px; bottom: 6px; background: rgba(0, 0, 0, 0.75); font-size: 11.5px; font-weight: 600; padding: 1px 6px; border-radius: 5px; }
  .lists { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .chipbtn { display: inline-flex; align-items: center; gap: 5px; border: 1px solid var(--line-2); background: var(--coal-2); border-radius: 999px; padding: 4px 10px; font-size: 12px; color: var(--text); }
  .chipbtn:hover { border-color: var(--gold-dim); }
  .chipbtn[aria-pressed="true"] { border-color: var(--gold); color: var(--gold-glow); background: rgba(227, 194, 112, 0.12); }
  .mini { border: 0; background: transparent; color: var(--dim); padding: 6px; border-radius: 7px; display: inline-flex; align-items: center; gap: 4px; }
  .mini:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .mini.armed { color: var(--bad); background: rgba(224, 106, 106, 0.12); font-size: 12px; font-weight: 700; }
  .lib .h2 .sub { font-family: var(--f-body); font-weight: 500; margin-left: 4px; }
  .search { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 6px 10px; width: 180px; }
  .sm { padding: 6px 11px; font-size: 13px; }
  .seg .n { color: var(--dim); margin-left: 3px; font-variant-numeric: tabular-nums; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 220px), 1fr)); gap: 14px; }
  .card { display: flex; flex-direction: column; gap: 8px; min-width: 0; cursor: default; }
  .cthumb { border: 1px solid var(--line); }
  .card:hover .cthumb { border-color: var(--gold-dim); }
  .card.on .cthumb { border-color: var(--gold); box-shadow: 0 0 16px rgba(227, 194, 112, 0.25); }
  .ph { position: absolute; inset: 0; display: grid; place-items: center; color: var(--dim); }
  .qual { position: absolute; left: 6px; top: 6px; background: rgba(18, 20, 23, 0.85); color: var(--gold); font-size: 10.5px; font-weight: 700; padding: 1px 6px; border-radius: 5px; }
  .hover { position: absolute; inset: 0; display: grid; place-items: center; opacity: 0; transition: opacity 0.15s; background: rgba(0, 0, 0, 0.18); }
  .card:hover .hover { opacity: 1; }
  .round { width: 44px; height: 44px; border-radius: 50%; border: 0; background: rgba(227, 194, 112, 0.92); color: #1b1608; display: grid; place-items: center; }
  .cbody { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .ctitle { font-weight: 600; font-size: 13.5px; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .cacts { display: flex; opacity: 0.5; transition: opacity 0.15s; }
  .card:hover .cacts { opacity: 1; }
  .empty { grid-column: 1 / -1; margin: 6px 0; }

  /* Müzik / Videolar */
  .sections { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .sections button { display: flex; align-items: center; gap: 14px; text-align: left; padding: 14px 18px; border-radius: 16px; border: 1px solid var(--line-2); background: rgba(23, 26, 30, 0.7); color: var(--muted); transition: border-color 0.15s, background 0.15s, color 0.15s; }
  .sections button:hover { border-color: var(--gold-dim); color: var(--text); }
  .sections button.on { border-color: var(--gold); color: var(--gold-glow); background: linear-gradient(135deg, rgba(227, 194, 112, 0.14), rgba(227, 194, 112, 0.03)); box-shadow: 0 0 22px rgba(227, 194, 112, 0.1); }
  .sections b { display: block; font-family: var(--f-display); font-size: 17px; color: var(--text); }
  .sections .on b { color: var(--gold-glow); }

  /* Şarkı listesi */
  .tracks { display: flex; flex-direction: column; gap: 2px; }
  .track { display: grid; grid-template-columns: 34px 44px minmax(0, 2fr) minmax(0, 1fr) 64px auto; gap: 12px; align-items: center; padding: 6px 10px; border-radius: 10px; }
  .track:hover { background: rgba(255, 255, 255, 0.04); }
  .track.on { background: rgba(227, 194, 112, 0.1); box-shadow: inset 3px 0 0 var(--gold); }
  .track.on .ttx b { color: var(--gold-glow); }
  .tn { border: 0; background: none; color: var(--dim); font-size: 12.5px; display: grid; place-items: center; height: 32px; }
  .tn .pl { display: none; color: var(--gold); }
  .track:hover .tn .num { display: none; }
  .track:hover .tn .pl { display: grid; }
  .tthumb { width: 44px; height: 44px; border-radius: 8px; overflow: hidden; background: var(--coal-2); display: grid; place-items: center; color: var(--dim); }
  .tthumb img { width: 100%; height: 100%; object-fit: cover; }
  .ttx { display: flex; flex-direction: column; min-width: 0; }
  .ttx b, .ttx .sub, .tpl { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ttx b { font-weight: 600; font-size: 14px; }
  .eq { display: inline-flex; gap: 2px; align-items: flex-end; height: 12px; }
  .eq i { width: 3px; background: var(--gold); border-radius: 1px; animation: eq 0.9s ease-in-out infinite; }
  .eq i:nth-child(2) { animation-delay: 0.2s; }
  .eq i:nth-child(3) { animation-delay: 0.4s; }
  @keyframes eq { 0%, 100% { height: 3px; } 50% { height: 12px; } }
  @media (max-width: 900px) {
    .track { grid-template-columns: 34px 44px minmax(0, 1fr) 56px auto; }
    .tpl { display: none; }
  }
</style>
