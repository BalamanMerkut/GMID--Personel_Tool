<script lang="ts">
  // Fırsatlar: her platformda şu an ücretsiz oyunlar, Epic'in sıradaki ücretsizi ve istek listendeki indirimler.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Switch from "../components/Switch.svelte";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { DealsPrefs, DealsSnapshot, Giveaway } from "../types";

  let { onnavigate }: { onnavigate: (route: string) => void } = $props();

  const STORES: Record<string, string> = {
    epic: "Epic", steam: "Steam", gog: "GOG", playstation: "PlayStation", xbox: "Xbox / Microsoft", ubisoft: "Ubisoft",
    ea: "EA", prime: "Prime Gaming", itch: "itch.io", battlenet: "Battle.net", other: "Diğer", mobile: "Telefon",
  };
  const NOTIFY_STORES = ["epic", "steam", "gog", "playstation", "xbox", "ubisoft", "ea", "prime", "itch", "battlenet", "other"];

  let snap = $state<DealsSnapshot | null>(null);
  let loading = $state(false);
  let store = $state<string | null>(null);
  let showPrefs = $state(false);
  let prefs = $state<DealsPrefs>({ notifyFree: true, stores: [], notifyWish: true, minDiscount: 20, ...getJson<Partial<DealsPrefs>>("deals.prefs", {}) });
  let now = $state(Date.now());

  onMount(() => {
    void api.deals.get().then((s) => {
      snap = s;
      // Hiç bakılmadıysa ya da 3 saatten eskiyse şimdi bak.
      if (!s.fetchedAt || Date.now() - s.fetchedAt > 3 * 3_600_000) void refresh(true);
    });
    const stop = on<DealsSnapshot>("deals", (s) => (snap = s));
    const t = setInterval(() => (now = Date.now()), 60_000);
    return () => {
      clearInterval(t);
      void stop.then((f) => f());
    };
  });

  async function refresh(quiet = false) {
    loading = true;
    try {
      snap = await api.deals.refresh();
      if (!quiet && snap.error) notify(`Bazı kaynaklara ulaşılamadı: ${snap.error}`, "info");
    } catch (e) {
      if (!quiet) notify(String(e), "info");
    } finally {
      loading = false;
    }
  }

  function savePrefs() {
    void setJson("deals.prefs", $state.snapshot(prefs));
  }

  function toggleStore(s: string) {
    const all = prefs.stores.length ? prefs.stores : NOTIFY_STORES;
    prefs.stores = all.includes(s) ? all.filter((x) => x !== s) : [...all, s];
    if (prefs.stores.length === NOTIFY_STORES.length) prefs.stores = [];
    savePrefs();
  }
  const notifies = (s: string) => !prefs.stores.length || prefs.stores.includes(s);

  const counts = $derived.by(() => {
    const c: Record<string, number> = {};
    for (const g of snap?.free ?? []) c[g.store] = (c[g.store] ?? 0) + 1;
    return c;
  });
  const storesHere = $derived(Object.keys(STORES).filter((s) => counts[s]));
  const shown = $derived((snap?.free ?? []).filter((g) => (store ? g.store === store : g.store !== "mobile")));

  function left(g: Giveaway) {
    if (!g.ends) return "Süresi belirsiz";
    const ms = g.ends - now;
    if (ms <= 0) return "Bitti";
    const h = Math.floor(ms / 3_600_000);
    if (h < 1) return `${Math.max(1, Math.round(ms / 60_000))} dk kaldı`;
    if (h < 48) return `${h} saat kaldı`;
    return `${Math.floor(h / 24)} gün kaldı`;
  }
  const urgent = (g: Giveaway) => !!g.ends && g.ends - now < 24 * 3_600_000;
  const date = (ms: number | null) => (ms ? new Date(ms).toLocaleDateString("tr-TR", { day: "numeric", month: "long", weekday: "long" }) : "");
  const open = (url: string) => void api.openUrl(url).catch((e) => notify(String(e), "info"));
</script>

<div class="deals">
  <section class="panel head">
    <div class="htx">
      <h2>Fırsatlar</h2>
      <p class="sub">
        Her platformda şu an <b>ücretsiz</b> oyunlar ve istek listendeki oyunların indirimleri. Yenisi çıkınca Windows bildirimiyle haber
        veririm.
      </p>
      <span class="sub small">
        {snap?.fetchedAt ? `Son bakış ${ago(snap.fetchedAt, now)}` : "Henüz bakılmadı"} · GamerPower, Epic, Steam, CheapShark
      </span>
    </div>
    <div class="acts">
      <button class="btn ghost sm" class:on={showPrefs} onclick={() => (showPrefs = !showPrefs)}><Icon name="bell" size={14} />Bildirimler</button>
      <button class="btn sm" disabled={loading} onclick={() => refresh()}><Icon name="refresh" size={14} />{loading ? "Bakılıyor…" : "Yenile"}</button>
    </div>
  </section>

  {#if showPrefs}
    <section class="panel prefs">
      <div class="line">
        <div><b>Yeni ücretsiz oyun</b><span class="sub small">Aşağıda seçtiğin mağazalarda yeni bir ücretsiz oyun çıkınca</span></div>
        <Switch label="Ücretsiz oyun bildirimi" checked={prefs.notifyFree} onchange={(v) => ((prefs.notifyFree = v), savePrefs())} />
      </div>
      {#if prefs.notifyFree}
        <div class="chips">
          {#each NOTIFY_STORES as s (s)}
            <button class="chip" class:on={notifies(s)} onclick={() => toggleStore(s)}>{notifies(s) ? "✓ " : ""}{STORES[s]}</button>
          {/each}
        </div>
      {/if}
      <div class="line">
        <div><b>İstek listesinde indirim</b><span class="sub small">Oyun Arşivi'nde "İstiyorum" dediğin bir oyun indirime girince</span></div>
        <div class="row">
          <select value={prefs.minDiscount} onchange={(e) => ((prefs.minDiscount = Number(e.currentTarget.value)), savePrefs())} aria-label="En az indirim" disabled={!prefs.notifyWish}>
            {#each [10, 20, 30, 50, 75] as p (p)}<option value={p}>en az %{p}</option>{/each}
          </select>
          <Switch label="İndirim bildirimi" checked={prefs.notifyWish} onchange={(v) => ((prefs.notifyWish = v), savePrefs())} />
        </div>
      </div>
    </section>
  {/if}

  <section class="panel">
    <div class="spread">
      <h3 class="h2"><Icon name="gift" />Şu an ücretsiz <span class="sub small tnum">{shown.length}</span></h3>
      {#if snap?.error}<span class="err small" title={snap.error}>Bazı kaynaklara ulaşılamadı</span>{/if}
    </div>
    {#if storesHere.length > 1}
      <div class="chips">
        <button class="chip" class:on={!store} onclick={() => (store = null)}>Hepsi</button>
        {#each storesHere as s (s)}
          <button class="chip" class:on={store === s} onclick={() => (store = store === s ? null : s)}>{STORES[s]} <span class="tnum">{counts[s]}</span></button>
        {/each}
      </div>
    {/if}
    {#if shown.length}
      <div class="grid">
        {#each shown as g (g.id)}
          <article class="card">
            <button class="img" onclick={() => open(g.url)} title="Mağazada aç">
              {#if g.image}<img src={g.image} alt="" loading="lazy" referrerpolicy="no-referrer" />{:else}<span class="ph">{g.title}</span>{/if}
              <span class="badge {g.store}">{STORES[g.store] ?? g.store}</span>
            </button>
            <div class="body">
              <b class="t" title={g.title}>{g.title}</b>
              <span class="price">{#if g.worth}<s>{g.worth}</s>{/if}<em>Ücretsiz</em></span>
              <div class="spread">
                <span class="small" class:hot={urgent(g)}>{left(g)}</span>
                <button class="btn sm" onclick={() => open(g.url)}><Icon name="download" size={14} />Al</button>
              </div>
            </div>
          </article>
        {/each}
      </div>
    {:else if loading}
      <p class="sub">Bakılıyor…</p>
    {:else}
      <p class="sub">Şu an listede ücretsiz oyun yok{snap?.error ? " (kaynaklara ulaşılamadı; internet bağlantını kontrol et)" : ""}.</p>
    {/if}
  </section>

  {#if snap?.upcoming.length}
    <section class="panel">
      <h3 class="h2"><Icon name="clock" />Yakında ücretsiz</h3>
      <div class="upc">
        {#each snap.upcoming as g (g.id)}
          <button class="up" onclick={() => open(g.url)}>
            {#if g.image}<img src={g.image} alt="" loading="lazy" referrerpolicy="no-referrer" />{/if}
            <span><b>{g.title}</b><span class="sub small">{STORES[g.store]} · {date(g.starts)}</span></span>
          </button>
        {/each}
      </div>
    </section>
  {/if}

  <section class="panel">
    <h3 class="h2"><Icon name="star" />İstek listende indirimde <span class="sub small tnum">{snap?.wishlist.length ?? 0}</span></h3>
    {#if snap?.wishlist.length}
      <div class="wish">
        {#each snap.wishlist as d (d.gameId)}
          <div class="wrow">
            {#if d.cover}<img src={d.cover.startsWith("http") || d.cover.startsWith("data:") ? d.cover : api.fileSrc(d.cover)} alt="" />{:else}<span class="wph"></span>{/if}
            <span class="wn"><b>{d.name}</b><span class="sub small">{d.store}</span></span>
            <span class="off">-%{d.discount}</span>
            <span class="wp">{#if d.regular}<s>{d.regular}</s>{/if}<b>{d.price}</b></span>
            <button class="btn ghost sm" onclick={() => open(d.url)}>Mağazada aç</button>
          </div>
        {/each}
      </div>
    {:else if (snap?.wishCount ?? 0) > 0}
      <p class="sub">İstek listendeki {snap?.wishCount} oyunun hiçbiri şu an indirimde değil. İndirime girince haber veririm.</p>
    {:else}
      <p class="sub">
        Oyun Arşivi'nde bir oyunu <b>İstiyorum</b> olarak işaretle; Steam, GOG, Epic, EA, Ubisoft ya da Humble'da indirime girince haber veririm.
      </p>
      <button class="btn ghost sm fit" onclick={() => onnavigate("archive")}><Icon name="star" size={14} />Oyun Arşivi'ne git</button>
    {/if}
  </section>
</div>

<style>
  .deals { display: flex; flex-direction: column; gap: 16px; max-width: 1400px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .head { flex-direction: row; align-items: center; gap: 20px; flex-wrap: wrap; }
  .htx { flex: 1; display: flex; flex-direction: column; gap: 4px; min-width: 260px; }
  .htx h2 { margin: 0; font-family: var(--f-display); font-size: 26px; }
  .htx p { margin: 0; max-width: 760px; }
  .acts { display: flex; gap: 8px; }
  .acts .on { border-color: var(--gold); color: var(--gold-glow); }
  .spread { display: flex; justify-content: space-between; align-items: center; gap: 10px; }
  .h2 { margin: 0; display: flex; align-items: center; gap: 8px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { cursor: pointer; }
  .chip.on { border-color: var(--gold); color: var(--gold-glow); background: rgba(227, 194, 112, 0.12); }
  .err { color: var(--bad); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 12px; }
  .card { display: flex; flex-direction: column; border-radius: 14px; overflow: hidden; border: 1px solid var(--line); background: rgba(255, 255, 255, 0.02); transition: border-color 0.15s, transform 0.15s; }
  .card:hover { border-color: var(--gold-dim); transform: translateY(-2px); }
  .img { position: relative; border: 0; padding: 0; aspect-ratio: 16 / 9; background: var(--coal-2); display: block; width: 100%; cursor: pointer; }
  .img img { width: 100%; height: 100%; object-fit: cover; display: block; }
  .ph { display: grid; place-items: center; height: 100%; padding: 10px; color: var(--muted); font-weight: 700; }
  .badge { position: absolute; left: 8px; top: 8px; padding: 2px 8px; border-radius: 6px; font-size: 11px; font-weight: 800; background: rgba(0, 0, 0, 0.72); color: #fff; }
  .badge.epic { background: #2a2a2a; }
  .badge.steam { background: #1b2838; color: #66c0f4; }
  .badge.gog { background: #5c2d91; }
  .badge.playstation { background: #0070d1; }
  .badge.xbox { background: #107c10; }
  .badge.prime { background: #00a8e1; color: #062433; }
  .badge.ubisoft { background: #0a3a8a; }
  .badge.ea { background: #ff4747; }
  .body { display: flex; flex-direction: column; gap: 6px; padding: 10px 12px 12px; }
  .t { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .price { display: flex; gap: 8px; align-items: baseline; }
  .price s { color: var(--dim); font-size: 13px; }
  .price em { font-style: normal; font-weight: 800; color: var(--good); }
  .hot { color: var(--gold-glow); font-weight: 700; }
  .upc { display: flex; gap: 10px; flex-wrap: wrap; }
  .up { display: flex; align-items: center; gap: 10px; padding: 8px 12px 8px 8px; border-radius: 12px; border: 1px solid var(--line); background: rgba(255, 255, 255, 0.02); color: var(--text); text-align: left; }
  .up:hover { border-color: var(--gold-dim); }
  .up img { width: 96px; aspect-ratio: 16 / 9; object-fit: cover; border-radius: 8px; }
  .up span { display: flex; flex-direction: column; gap: 2px; }
  .wish { display: flex; flex-direction: column; gap: 6px; }
  .wrow { display: flex; align-items: center; gap: 12px; padding: 8px 10px; border-radius: 12px; background: rgba(255, 255, 255, 0.03); }
  .wrow img, .wph { width: 44px; height: 58px; object-fit: cover; border-radius: 6px; background: var(--coal-2); flex: none; }
  .wn { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .off { font-weight: 800; color: #1b1608; background: var(--good); padding: 3px 8px; border-radius: 6px; }
  .wp { display: flex; flex-direction: column; align-items: flex-end; min-width: 76px; }
  .wp s { color: var(--dim); font-size: 12px; }
  .fit { align-self: flex-start; }
  .prefs .line { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .prefs .line > div:first-child { display: flex; flex-direction: column; gap: 2px; }
  .row { display: flex; align-items: center; gap: 10px; }
  select { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 6px 10px; color: var(--text); }
</style>
