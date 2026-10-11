<script lang="ts">
  // Kütüphanem: Steam ve Epic'ten içe aktarılan, elle eklenen oyunlar. Açık oyun 5 sn içinde yakalanır,
  // gerçek oynama süresi tutulur (Steam'in tuttuğu süre ayrıca görünür).
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import RatingCard from "../components/RatingCard.svelte";
  import Icon from "../Icon.svelte";
  import { fmtScore, scoreTone } from "../review";
  import { getJson, setJson } from "../settings.svelte";
  import { ago, playtime } from "../time";
  import { notify } from "../toast.svelte";
  import type { FoundGame, Game, GameSuggestion, PlaySession, Review, RunningApp } from "../types";

  type Source = "all" | "steam" | "epic" | "riot" | "xbox" | "other" | "manual";
  type Sort = "recent" | "played" | "name" | "platform";
  const SOURCES: [Source, string][] = [["all", "Tümü"], ["steam", "Steam"], ["epic", "Epic"], ["riot", "Riot"], ["xbox", "Xbox"], ["other", "Diğer"], ["manual", "Elle eklenen"]];
  const SORTS: [Sort, string][] = [["recent", "Son oynanan"], ["played", "En çok oynanan"], ["name", "Ada göre"], ["platform", "Steam süresi"]];

  let games: Game[] = $state([]);
  let loaded = $state(false);
  let source: Source = $state("all");
  let sort: Sort = $state(getJson<Sort>("library.sort", "recent"));
  let search = $state("");
  let showHidden = $state(false);
  let now = $state(Date.now());

  // Pencereler
  let scanning = $state(false);
  let found: FoundGame[] | null = $state(null);
  let picked: string[] = $state([]);
  let adding = $state(false);
  let apps: RunningApp[] = $state([]);
  let manualName = $state("");
  let manualExe = $state("");
  let detail: Game | null = $state(null);
  let detailSessions: PlaySession[] = $state([]);
  let armedRemove = $state(false);
  let reviews: Review[] = $state([]);
  let rating: Game | null = $state(null);
  let suggestions: GameSuggestion[] = $state([]);
  let suggestNames: Record<string, string> = $state({});

  async function load() {
    const [g, r] = await Promise.all([api.games.list().catch(() => []), api.reviews.list().catch(() => [])]);
    // Yalnızca arşivde olanlar (puanlanmış ama kurulu değil) burada görünmez.
    games = g.filter((x) => x.inLibrary);
    reviews = r;
    suggestions = await api.games.suggestions().catch(() => []);
    loadedAt = Date.now();
    loaded = true;
    if (detail) detail = games.find((g) => g.id === detail?.id) ?? null;
  }

  onMount(() => {
    void load();
    const stop = on("games-changed", () => void load());
    const stopReviews = on("reviews-changed", () => void load());
    const stopSuggest = on("games-suggestions", () => void load());
    const timer = setInterval(() => (now = Date.now()), 30_000);
    return () => {
      clearInterval(timer);
      void stop.then((f) => f());
      void stopReviews.then((f) => f());
      void stopSuggest.then((f) => f());
    };
  });

  const key = (f: { source: string; sourceId: string }) => `${f.source}:${f.sourceId}`;
  const visible = $derived(games.filter((g) => showHidden || !g.hidden));
  const counts = $derived(Object.fromEntries(SOURCES.map(([s]) => [s, visible.filter((g) => s === "all" || g.source === s).length])));
  const shown = $derived.by(() => {
    const q = search.trim().toLocaleLowerCase("tr-TR");
    const list = visible.filter(
      (g) => (source === "all" || g.source === source) && (!q || `${g.name} ${g.developer ?? ""}`.toLocaleLowerCase("tr-TR").includes(q)),
    );
    const by: Record<Sort, (a: Game, b: Game) => number> = {
      recent: (a, b) => (b.runningSince ? 1 : 0) - (a.runningSince ? 1 : 0) || (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0),
      played: (a, b) => b.playedSeconds - a.playedSeconds || b.platformMinutes - a.platformMinutes,
      name: (a, b) => a.name.localeCompare(b.name, "tr"),
      platform: (a, b) => b.platformMinutes - a.platformMinutes,
    };
    return [...list].sort(by[sort]);
  });
  const running = $derived(games.filter((g) => g.runningSince));
  const reviewOf = (g: Game) => reviews.find((r) => r.subjectId === g.id) ?? null;
  const hiddenCount = $derived(games.filter((g) => g.hidden).length);

  const cover = (g: { cover: string | null }) => (!g.cover ? null : /^(https?:|data:)/.test(g.cover) ? g.cover : api.fileSrc(g.cover));
  /** Açık oyunun süresi ekranda da artsın (liste yüklendiğinden beri geçen). */
  let loadedAt = Date.now();
  const live = (g: Game) => g.playedSeconds + (g.runningSince ? Math.max(0, (now - loadedAt) / 1000) : 0);
  const sinceText = (ms: number) => playtime((now - ms) / 1000);

  async function answer(sg: GameSuggestion, action: "add" | "never") {
    const name = suggestNames[sg.path]?.trim() || sg.name;
    await act(api.games.answerSuggestion(sg.path, action, name), action === "add" ? `${name} kütüphaneye eklendi` : "Bir daha sorulmayacak");
    await load();
  }

  async function scan() {
    scanning = true;
    try {
      found = await api.games.scan();
      picked = found.filter((f) => !f.already).map(key);
      if (!found.length) notify("Steam, Epic ya da Riot'ta kurulu oyun bulunamadı. Oyunu elle ekleyebilirsin.", "info");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      scanning = false;
    }
  }

  async function importPicked() {
    if (!found) return;
    const list = found.filter((f) => picked.includes(key(f)));
    try {
      const n = await api.games.import(list);
      notify(`${n} oyun kütüphaneye eklendi · kapaklar iniyor`, "check");
      found = null;
      await load();
    } catch (e) {
      notify(String(e), "info");
    }
  }

  async function openAdd() {
    adding = true;
    manualName = "";
    manualExe = "";
    apps = await api.games.runningApps().catch(() => []);
  }

  async function pickExe() {
    const p = await api.games.pickExe();
    if (p) {
      manualExe = p;
      if (!manualName) manualName = p.split(/[\\/]/).pop()?.replace(/\.exe$/i, "") ?? "";
    }
  }

  async function addManual() {
    if (!manualExe) return notify("Önce oyunun .exe dosyasını seç", "info");
    try {
      await api.games.addManual(manualName, manualExe);
      notify(`${manualName || "Oyun"} eklendi`, "check");
      adding = false;
      await load();
    } catch (e) {
      notify(String(e), "info");
    }
  }

  async function openDetail(g: Game) {
    detail = g;
    armedRemove = false;
    detailSessions = (await api.games.sessions(0).catch(() => [] as PlaySession[])).filter((s) => s.gameId === g.id).slice(0, 12);
  }

  async function act(p: Promise<unknown>, ok?: string) {
    try {
      await p;
      if (ok) notify(ok, "check");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  async function removeGame(g: Game) {
    if (!armedRemove) {
      armedRemove = true;
      return;
    }
    await act(api.games.remove(g.id), `${g.name} kütüphaneden kaldırıldı`);
    detail = null;
    await load();
  }

  const sourceLabel = (s: string) => ({ steam: "Steam", epic: "Epic", riot: "Riot", xbox: "Xbox", other: "Diğer" })[s] ?? "Elle eklendi";
  const fmtDate = (ms: number) =>
    new Date(ms).toLocaleString("tr-TR", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
</script>

<div class="lib">
  {#each running as g (g.id)}
    <section class="panel now">
      {#if cover(g)}<img class="ncover" src={cover(g)} alt="" />{/if}
      <div class="ntx">
        <span class="live"><span class="dot"></span>Şu an oynuyorsun</span>
        <b>{g.name}</b>
        <span class="sub small">Açık: {sinceText(g.runningSince ?? now)} · süre tutuluyor</span>
      </div>
      <button class="btn ghost" onclick={() => openDetail(g)}>Ayrıntılar</button>
    </section>
  {/each}

  {#if suggestions.length}
    <section class="panel sugg">
      <div>
        <b>Algılanan oyunlar</b>
        <span class="sub small">Açtığın ama kütüphanende olmayan programlar. Adını düzeltip ekleyebilirsin; oyun değilse bir daha sorulmaz.</span>
      </div>
      {#each suggestions as sg (sg.path)}
        <div class="sg">
          <input class="sgname" value={suggestNames[sg.path] ?? sg.name} oninput={(e) => (suggestNames[sg.path] = e.currentTarget.value)} aria-label="Oyunun adı" />
          <span class="sub small sgpath" title={sg.path}>{sg.exe} · {sg.reason === "discord" ? "bilinen oyun" : "tam ekran çalıştı"} · {ago(sg.seenAt, now)}</span>
          <button class="btn ghost sm" onclick={() => answer(sg, "never")}>Oyun değil</button>
          <button class="btn sm" onclick={() => answer(sg, "add")}><Icon name="plus" size={15} />Ekle</button>
        </div>
      {/each}
    </section>
  {/if}

  <section class="panel">
    <div class="spread">
      <div class="seg">
        {#each SOURCES as [s, label] (s)}
          {#if s === "all" || counts[s]}
            <button aria-pressed={source === s} onclick={() => (source = s)}>{label} <span class="n">{counts[s]}</span></button>
          {/if}
        {/each}
      </div>
      <div class="row">
        <input class="search" type="search" placeholder="Oyun ara…" bind:value={search} />
        <select bind:value={sort} onchange={() => void setJson("library.sort", sort)} aria-label="Sırala">
          {#each SORTS as [s, label] (s)}<option value={s}>{label}</option>{/each}
        </select>
        <button class="btn ghost" onclick={openAdd}><Icon name="plus" size={17} />Oyun ekle</button>
        <button class="btn" onclick={scan} disabled={scanning}>
          <Icon name="search" size={17} />{scanning ? "Taranıyor…" : "Oyunları tara"}
        </button>
      </div>
    </div>

    {#if loaded && !games.length}
      <div class="empty">
        <Icon name="library" size={40} />
        <h3>Kütüphanen boş</h3>
        <p class="sub">
          <b>Oyunları tara</b> Steam, Epic ve Riot'ta (League of Legends, VALORANT) kurulu oyunlarını bulur (Steam'deki
          oynama sürelerinle birlikte). Battle.net gibi başka yerlerdeki oyunları <b>Oyun ekle</b> ile ekleyebilirsin: oyun açıkken listede çıkar.
        </p>
        <button class="btn" onclick={scan} disabled={scanning}><Icon name="search" size={17} />Oyunları tara</button>
      </div>
    {:else}
      <div class="grid">
        {#each shown as g (g.id)}
          <button class="card" class:hid={g.hidden} class:on={!!g.runningSince} onclick={() => openDetail(g)} title={g.name}>
            <div class="cv">
              {#if cover(g)}<img src={cover(g)} alt="" loading="lazy" />{:else}<span class="ph">{g.name}</span>{/if}
              {#if g.runningSince}<span class="badge"><span class="dot"></span>Oynuyor</span>{/if}
              <span class="src">{sourceLabel(g.source)}</span>
              {#if reviewOf(g)}{@const r = reviewOf(g)!}<span class={`score tone-${scoreTone(r.overall)}`}>{r.overall == null ? "İstiyorum" : fmtScore(r.overall)}</span>{/if}
            </div>
            <b class="nm">{g.name}</b>
            <span class="sub small tnum">
              {g.playedSeconds ? playtime(live(g)) : "Henüz oynanmadı"}{#if g.platformMinutes}{` · Steam ${playtime(g.platformMinutes * 60)}`}{/if}
            </span>
            <span class="sub small">{g.lastPlayed ? ago(g.lastPlayed, now) : " "}</span>
          </button>
        {:else}
          <p class="sub">Bu süzgeçte oyun yok.</p>
        {/each}
      </div>
      {#if hiddenCount}
        <button class="link" onclick={() => (showHidden = !showHidden)}>
          {showHidden ? "Gizlenenleri sakla" : `Gizlenen ${hiddenCount} oyunu göster`}
        </button>
      {/if}
    {/if}
  </section>
</div>

{#if found}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (found = null)}>
    <div class="dlg panel" role="dialog" aria-label="Bulunan oyunlar">
      <div class="spread">
        <h3 class="h2"><Icon name="search" />Bulunan oyunlar</h3>
        <button class="x" onclick={() => (found = null)} title="Kapat"><Icon name="x" size={18} /></button>
      </div>
      <p class="sub small">İstemediklerinin işaretini kaldır. Kütüphanede olanlar yeniden eklenirse kurulum bilgisi ve Steam süresi güncellenir.</p>
      <div class="row">
        <button class="link" onclick={() => (picked = found?.map(key) ?? [])}>Tümünü seç</button>
        <button class="link" onclick={() => (picked = [])}>Hiçbiri</button>
        <span class="sub small">{picked.length} / {found.length} seçili</span>
      </div>
      <div class="flist">
        {#each found as f (key(f))}
          {@const sel = picked.includes(key(f))}
          <label class="fi" class:off={!sel}>
            <input type="checkbox" checked={sel} onchange={() => (picked = sel ? picked.filter((x) => x !== key(f)) : [...picked, key(f)])} />
            <span class="ftx"><b>{f.name}</b><span class="sub small">{sourceLabel(f.source)}{f.installDir ? ` · ${f.installDir}` : ""}</span></span>
            <span class="sub small tnum">{f.platformMinutes ? `Steam ${playtime(f.platformMinutes * 60)}` : ""}</span>
            {#if f.already}<span class="chip">Kütüphanede</span>{/if}
          </label>
        {/each}
      </div>
      <div class="foot">
        <button class="btn ghost" onclick={() => (found = null)}>Vazgeç</button>
        <button class="btn" disabled={!picked.length} onclick={importPicked}><Icon name="check" size={17} />{picked.length} oyunu ekle</button>
      </div>
    </div>
  </div>
{/if}

{#if adding}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (adding = false)}>
    <div class="dlg panel" role="dialog" aria-label="Oyun ekle">
      <div class="spread">
        <h3 class="h2"><Icon name="plus" />Oyun ekle</h3>
        <button class="x" onclick={() => (adding = false)} title="Kapat"><Icon name="x" size={18} /></button>
      </div>
      <p class="sub small">Oyun şu an açıksa aşağıda listelenir; tıkla, adını kontrol et, ekle. Açık değilse .exe dosyasını seç.</p>
      {#if apps.length}
        <div class="flist short">
          {#each apps as a (a.path)}
            <button class="app" class:on={manualExe === a.path} onclick={() => ((manualExe = a.path), (manualName = a.title))}>
              <b>{a.title}</b><span class="sub small">{a.name}</span>
            </button>
          {/each}
        </div>
      {:else}
        <p class="sub small">Şu an açık bir uygulama bulunamadı.</p>
      {/if}
      <div class="field">
        <span>Oyunun adı</span>
        <input bind:value={manualName} placeholder="Valorant" />
      </div>
      <div class="row">
        <button class="btn ghost" onclick={pickExe}><Icon name="folder" size={16} />.exe seç…</button>
        <span class="sub small path">{manualExe || "Dosya seçilmedi"}</span>
      </div>
      <div class="foot">
        <button class="btn ghost" onclick={() => (adding = false)}>Vazgeç</button>
        <button class="btn" disabled={!manualExe} onclick={addManual}><Icon name="check" size={17} />Ekle</button>
      </div>
    </div>
  </div>
{/if}

{#if detail}
  {@const g = detail}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (detail = null)}>
    <div class="dlg panel det" role="dialog" aria-label={g.name}>
      <div class="dhead">
        {#if cover(g)}<img class="dcover" src={cover(g)} alt="" />{/if}
        <div class="dtx">
          <div class="spread">
            <h3>{g.name}</h3>
            <button class="x" onclick={() => (detail = null)} title="Kapat"><Icon name="x" size={18} /></button>
          </div>
          <span class="sub small">{[g.developer, g.year, sourceLabel(g.source)].filter(Boolean).join(" · ")}</span>
          {#if g.genres.length}<div class="tags">{#each g.genres as t (t)}<span class="chip">{t}</span>{/each}</div>{/if}
          {#if g.description}<p class="desc small">{g.description}</p>{/if}
          <div class="row">
            {#if g.launch}<button class="btn" onclick={() => act(api.games.launch(g.id))}><Icon name="play" size={17} />Oyna</button>{/if}
            {#if reviewOf(g)}
              {@const r = reviewOf(g)!}
              <button class="btn ghost sm" onclick={() => (rating = g)} title="Puanını düzenle">⭐ <b class={`tone-${scoreTone(r.overall)}`}>{r.overall == null ? "İstek listende" : fmtScore(r.overall)}</b> · Düzenle</button>
            {:else}
              <button class="btn ghost sm" onclick={() => (rating = g)}><Icon name="star" size={15} />Puanla</button>
            {/if}
            <button class="btn ghost sm" onclick={() => act(api.games.reveal(g.id))}><Icon name="folder" size={15} />Klasör</button>
            <button class="btn ghost sm" onclick={() => act(api.games.refreshMeta(g.id), "Bilgiler yenileniyor")}><Icon name="refresh" size={15} />Bilgileri yenile</button>
            <button class="btn ghost sm" onclick={() => act(api.games.pickCover(g.id))} title="Kendi resmini kapak yap"><Icon name="image" size={15} />Kapak seç</button>
          </div>
        </div>
      </div>
      <div class="tiles">
        <div class="tile"><span class="sub small">Gerçek oynama</span><b class="tnum">{playtime(g.playedSeconds)}</b><span class="sub small">GMID'in ölçtüğü</span></div>
        <div class="tile"><span class="sub small">Steam'deki süre</span><b class="tnum">{g.platformMinutes ? playtime(g.platformMinutes * 60) : "—"}</b><span class="sub small">kart programlarıyla şişebilir</span></div>
        <div class="tile"><span class="sub small">Oturum</span><b class="tnum">{g.sessions}</b><span class="sub small">{g.lastPlayed ? `son: ${ago(g.lastPlayed, now)}` : "henüz yok"}</span></div>
      </div>
      {#if detailSessions.length}
        <div class="sess">
          <span class="sub small up">Son oturumlar</span>
          {#each detailSessions as s (s.id)}
            <div class="srow"><span class="tnum">{fmtDate(s.startedAt)}</span><span class="sub tnum">{playtime(s.seconds)}</span></div>
          {/each}
        </div>
      {/if}
      <div class="foot">
        <button class="link" onclick={() => act(api.games.setHidden(g.id, !g.hidden), g.hidden ? "Kütüphanede gösteriliyor" : "Gizlendi; süresi tutulmaz")}>
          {g.hidden ? "Göster ve izle" : "Gizle (süre tutma)"}
        </button>
        <button class="link danger" onclick={() => removeGame(g)}>
          {armedRemove ? (reviewOf(g) ? "Emin misin? Puanı arşivde kalır" : "Emin misin? Oturumlar da silinir") : "Kütüphaneden kaldır"}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if rating}
  <RatingCard game={rating} review={reviewOf(rating)} onclose={() => (rating = null)} onsaved={() => void load()} ondeleted={() => void load()} />
{/if}

<style>
  .lib { display: flex; flex-direction: column; gap: 16px; max-width: 1500px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 14px; }
  .now { flex-direction: row; align-items: center; gap: 16px; border-color: rgba(92, 192, 138, 0.5); background: linear-gradient(90deg, rgba(92, 192, 138, 0.1), transparent); }
  .ncover { width: 54px; height: 81px; object-fit: cover; border-radius: 8px; }
  .ntx { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .ntx b { font-family: var(--f-display); font-size: 18px; }
  .live { display: inline-flex; align-items: center; gap: 6px; color: var(--good); font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--good); box-shadow: 0 0 8px rgba(92, 192, 138, 0.8); animation: blink 1.4s ease-in-out infinite; }
  @keyframes blink { 50% { opacity: 0.4; } }
  .sugg { border-color: rgba(53, 160, 162, 0.5); background: linear-gradient(90deg, rgba(53, 160, 162, 0.08), transparent); gap: 10px; }
  .sugg > div:first-child { display: flex; flex-direction: column; gap: 2px; }
  .sg { display: grid; grid-template-columns: minmax(160px, 260px) minmax(0, 1fr) auto auto; gap: 10px; align-items: center; }
  .sgname { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 9px; padding: 6px 10px; color: var(--text); font-weight: 600; }
  .sgpath { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .seg .n { color: var(--dim); margin-left: 3px; }
  .search { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; width: 200px; color: var(--text); }
  select { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; color: var(--text); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 18px 14px; }
  .card { display: flex; flex-direction: column; gap: 3px; text-align: left; border: 0; background: none; padding: 0; color: var(--text); min-width: 0; }
  .cv { position: relative; aspect-ratio: 2 / 3; border-radius: 12px; overflow: hidden; background: linear-gradient(160deg, var(--coal), var(--coal-2)); border: 1px solid var(--line); transition: transform 0.15s, border-color 0.15s, box-shadow 0.15s; }
  .card:hover .cv { transform: translateY(-3px); border-color: var(--gold-dim); box-shadow: 0 12px 26px rgba(0, 0, 0, 0.4); }
  .card.on .cv { border-color: var(--good); box-shadow: 0 0 18px rgba(92, 192, 138, 0.3); }
  .card.hid { opacity: 0.45; }
  .cv img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .ph { position: absolute; inset: 0; display: grid; place-items: center; padding: 12px; text-align: center; font-family: var(--f-display); font-weight: 700; color: var(--muted); }
  .badge { position: absolute; left: 8px; top: 8px; display: inline-flex; align-items: center; gap: 5px; background: rgba(12, 14, 17, 0.88); color: var(--good); font-size: 11px; font-weight: 700; padding: 3px 8px; border-radius: 999px; }
  .src { position: absolute; right: 6px; bottom: 6px; background: rgba(12, 14, 17, 0.8); color: var(--muted); font-size: 10.5px; font-weight: 600; padding: 1px 6px; border-radius: 5px; }
  .score { position: absolute; top: 8px; right: 8px; padding: 2px 7px; border-radius: 8px; background: rgba(12, 14, 17, 0.9); font-family: var(--f-display); font-weight: 800; font-size: 14px; border: 1px solid currentColor; }
  .tone-gold { color: var(--gold-glow); }
  .tone-good { color: var(--good); }
  .tone-mid { color: #e0a84a; }
  .tone-low { color: var(--bad); }
  .tone-none { color: var(--teal); }
  .nm { font-weight: 600; font-size: 14px; margin-top: 6px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 10px; text-align: center; padding: 40px 20px; color: var(--muted); }
  .empty h3 { margin: 0; color: var(--text); font-family: var(--f-display); }
  .empty p { max-width: 560px; margin: 0; }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; align-self: flex-start; }
  .link:hover { color: var(--neon); }
  .link.danger { color: var(--dim); }
  .link.danger:hover { color: var(--bad); }

  .back { position: fixed; inset: 0; z-index: 60; background: rgba(0, 0, 0, 0.55); display: grid; place-items: center; padding: 24px; }
  .dlg { width: min(680px, 100%); max-height: min(760px, 100%); overflow: hidden; background: var(--night-2); box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6); }
  .x { border: 0; background: none; color: var(--muted); padding: 6px; border-radius: 8px; display: grid; }
  .x:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .flist { overflow: auto; display: flex; flex-direction: column; gap: 2px; border: 1px solid var(--line); border-radius: 12px; padding: 6px; min-height: 80px; }
  .flist.short { max-height: 220px; }
  .fi { display: grid; grid-template-columns: auto minmax(0, 1fr) auto auto; gap: 10px; align-items: center; padding: 7px 8px; border-radius: 9px; cursor: pointer; }
  .fi:hover { background: rgba(255, 255, 255, 0.04); }
  .fi.off { opacity: 0.5; }
  .fi input { accent-color: var(--gold); width: 16px; height: 16px; }
  .ftx { display: flex; flex-direction: column; min-width: 0; }
  .ftx .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .app { display: flex; justify-content: space-between; gap: 10px; text-align: left; border: 0; background: transparent; color: var(--text); padding: 8px 10px; border-radius: 9px; }
  .app:hover { background: rgba(255, 255, 255, 0.05); }
  .app.on { background: rgba(227, 194, 112, 0.12); box-shadow: inset 3px 0 0 var(--gold); }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field > span { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .field input { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 10px; padding: 9px 12px; color: var(--text); }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 420px; }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; }
  .det { overflow: auto; }
  .dhead { display: flex; gap: 18px; }
  .dcover { width: 140px; aspect-ratio: 2 / 3; object-fit: cover; border-radius: 12px; flex: none; }
  .dtx { flex: 1; display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .dtx h3 { margin: 0; font-family: var(--f-display); font-size: 22px; }
  .tags { display: flex; flex-wrap: wrap; gap: 6px; }
  .desc { margin: 0; color: var(--muted); }
  .sm { padding: 6px 11px; font-size: 13px; }
  .tiles { display: grid; grid-template-columns: repeat(3, 1fr); gap: 10px; }
  .tile { display: flex; flex-direction: column; gap: 2px; padding: 12px 14px; border-radius: 12px; background: rgba(255, 255, 255, 0.03); border: 1px solid var(--line); }
  .tile b { font-family: var(--f-display); font-size: 22px; color: var(--gold-glow); }
  .sess { display: flex; flex-direction: column; gap: 4px; }
  .up { text-transform: uppercase; letter-spacing: 0.06em; font-weight: 700; }
  .srow { display: flex; justify-content: space-between; padding: 6px 10px; border-radius: 8px; background: rgba(255, 255, 255, 0.025); font-size: 13.5px; }
  @media (max-width: 700px) {
    .tiles { grid-template-columns: 1fr; }
    .dhead { flex-direction: column; }
  }
</style>
