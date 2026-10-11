<script lang="ts">
  import { onMount } from "svelte";
  import { api, formatSize, type AppInfo } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import { findScreen, SCREENS } from "../nav";
  import { ago, relative, when } from "../time";
  import type { NoteSummary, Reminder, Server, ServerStatus } from "../types";
  import { downloads, isActive } from "../downloads.svelte";

  let { info, onnavigate }: { info: AppInfo | null; onnavigate: (id: string) => void } = $props();

  const now = new Date();
  const hour = now.getHours();
  const greeting = hour < 6 ? "İyi geceler" : hour < 12 ? "Günaydın" : hour < 18 ? "İyi günler" : "İyi akşamlar";
  const today = now.toLocaleDateString("tr-TR", { weekday: "long", day: "numeric", month: "long" });

  const CURRENT_PHASE = 5;
  /** Sırası atlanan fazlar (sonra yapılacak). */
  const LATER = new Set([3, 4]);
  const PHASES: { n: number; title: string; note?: string; ids?: string[] }[] = [
    { n: 1, title: "İskelet", note: "Tema, açılış, menü, Kontrol Odası, tepsi, yedekler" },
    { n: 2, title: "Günlük araçlar", ids: ["audio", "notes", "reminders", "timer"] },
    { n: 3, title: "Oyun", ids: ["servers", "library", "sessions", "clips", "archive"] },
    { n: 4, title: "Oyun içi deneyim" },
    { n: 5, title: "Video İndirici", ids: ["videos"] },
    { n: 6, title: "Film & Dizi, Kurulum" },
  ];
  const screensOf = (p: (typeof PHASES)[number]) =>
    p.ids ? p.ids.map((id) => findScreen(id)!).filter(Boolean) : SCREENS.filter((s) => s.phase === p.n);
  const lastBackup = $derived(info?.backups[0]);

  let reminders: Reminder[] = $state([]);
  let notes: NoteSummary[] = $state([]);
  let servers: Server[] = $state([]);
  let statuses: Record<string, ServerStatus> = $state({});
  const dlRecent = $derived(
    downloads.list.filter((d) => isActive(d) || (d.status === "done" && d.finishedAt && Date.now() - d.finishedAt < 86_400_000)).slice(0, 4),
  );
  const liveServers = $derived(
    servers
      .filter((s) => s.remote || statuses[s.id]?.state === "online")
      .sort((a, b) => (statuses[b.id]?.info?.players ?? -1) - (statuses[a.id]?.info?.players ?? -1))
      .slice(0, 4),
  );

  async function load() {
    const [r, n] = await Promise.all([api.reminders.list(), api.notes.list(false)]);
    reminders = r.filter((x) => !x.done).slice(0, 3);
    notes = n.slice(0, 3);
  }

  async function loadServers() {
    servers = await api.servers.list();
    // Topluluk listesi ilk açılışta da gelsin (kısa sürede tekrar indirilmez).
    servers = (await api.servers.sync(false)).servers;
    let list = await api.servers.cached();
    // Sunucular ekranına hiç girilmediyse bir kez sor.
    if (!list.length && servers.length) list = await api.servers.query();
    statuses = Object.fromEntries(list.map((s) => [s.id, s]));
  }

  onMount(() => {
    void load();
    void loadServers().catch(console.error);
    const stops = [on("reminders-changed", load), on("notes-changed", load)];
    return () => stops.forEach((s) => void s.then((f) => f()));
  });
</script>

<div class="home">
  <div class="hello">
    <div>
      <p class="sub date">{today}</p>
      <h2>{greeting}, <em>malikâne sakini</em>.</h2>
    </div>
    <div class="row">
      <button class="btn ghost" onclick={() => api.notes.openQuick()}><Icon name="note" />Hızlı not</button>
      <button class="btn" onclick={() => onnavigate("reminders")}><Icon name="bell" />Hatırlatıcı kur</button>
    </div>
  </div>

  <div class="grid">
    <div class="col">
      <section class="panel">
        <div class="spread">
          <h3 class="h2"><Icon name="bell" />Yaklaşan hatırlatıcılar</h3>
          <button class="link" onclick={() => onnavigate("reminders")}>Tümü</button>
        </div>
        {#each reminders as r (r.id)}
          <div class="li" class:hot={r.notified}>
            <span class="t">{r.title}</span>
            <span class="m tnum">{when(r.dueAt)} · {relative(r.dueAt)}</span>
          </div>
        {:else}
          <p class="sub small">Yaklaşan hatırlatıcı yok.</p>
        {/each}
      </section>

      <section class="panel">
        <div class="spread">
          <h3 class="h2"><Icon name="note" />Son notlar</h3>
          <button class="link" onclick={() => onnavigate("notes")}>Tümü</button>
        </div>
        {#each notes as n (n.id)}
          <button class="li note" onclick={() => onnavigate("notes")}>
            <span class="t">{n.title || "Başlıksız not"}</span>
            <span class="m">{ago(n.updatedAt)}</span>
          </button>
        {:else}
          <p class="sub small">Henüz not yok. <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>N</kbd> ile her yerden yazabilirsin.</p>
        {/each}
      </section>

      <section class="panel roadmap">
        <h3 class="h2"><Icon name="castle" />Malikânenin odaları</h3>
        <ol>
          {#each PHASES as p (p.n)}
            <li class:done={p.n < CURRENT_PHASE && !LATER.has(p.n)} class:now={p.n === CURRENT_PHASE}>
              <span class="num tnum">{#if p.n < CURRENT_PHASE && !LATER.has(p.n)}<Icon name="check" size={15} />{:else}{p.n}{/if}</span>
              <div class="body">
                <b>{p.title}{#if p.n === CURRENT_PHASE}<span class="chip here">Yeni açıldı</span>{/if}{#if p.n === 3}<span class="chip">Sunucular açık · gerisi sırada</span>{/if}</b>
                {#if p.note}
                  <span class="sub small">{p.note}</span>
                {:else}
                  <div class="links">
                    {#each screensOf(p) as s (s.id)}
                      <button onclick={() => onnavigate(s.id)}><Icon name={s.icon} size={15} />{s.label}</button>
                    {/each}
                  </div>
                {/if}
              </div>
            </li>
          {/each}
        </ol>
      </section>
    </div>

    <div class="col">
      <section class="panel">
        <div class="spread">
          <h3 class="h2"><Icon name="download" />İndirmeler</h3>
          <button class="link" onclick={() => onnavigate("videos")}>Video İndirici</button>
        </div>
        {#each dlRecent as d (d.id)}
          {@const pct = Math.round((downloads.live[d.id]?.progress ?? d.progress) * 100)}
          <button class="li note" onclick={() => onnavigate("videos")}>
            <span class="t">{d.title}</span>
            <span class="m tnum">{d.status === "done" ? "indi ✓" : d.status === "queued" ? "sırada" : d.status === "error" ? "hata" : d.status === "paused" ? "duraklatıldı" : `%${pct}`}</span>
          </button>
        {:else}
          <p class="sub small">Devam eden indirme yok. Bir video bağlantısını <button class="link inl" onclick={() => onnavigate("videos")}>Video İndirici</button>'ye yapıştır.</p>
        {/each}
      </section>

      {#if servers.length}
        <section class="panel">
          <div class="spread">
            <h3 class="h2"><Icon name="server" />Sunucular</h3>
            <button class="link" onclick={() => onnavigate("servers")}>Tümü</button>
          </div>
          {#each liveServers as s (s.id)}
            {@const st = statuses[s.id]}
            <button class="li note" onclick={() => onnavigate("servers")}>
              <span class="dot {st?.state ?? ''}"></span>
              <span class="t">{s.name}</span>
              <span class="m tnum">
                {#if st?.state === "online" && st.info}{st.info.players}/{st.info.maxPlayers} oyuncu{:else if st?.state === "offline"}kapalı{:else}…{/if}
              </span>
            </button>
          {:else}
            <p class="sub small">Şu an açık sunucu yok.</p>
          {/each}
        </section>
      {/if}

      <section class="panel">
        <h3 class="h2"><Icon name="keyboard" />Her yerden çalışan kısayollar</h3>
        <ul class="keys">
          <li><span>Kulaklık ↔ hoparlör</span><span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>A</kbd></span></li>
          <li><span>Mikrofonu sessize al</span><span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>M</kbd></span></li>
          <li><span>Mikrofon kilidi</span><span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>L</kbd></span></li>
          <li><span>Hızlı not</span><span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>N</kbd></span></li>
        </ul>
        <button class="link" onclick={() => onnavigate("audio")}>Kısayolları değiştir →</button>
      </section>

      <section class="panel">
        <h3 class="h2"><Icon name="folder" />Veri klasörün</h3>
        <div class="path">{info?.dataDir ?? "…"}</div>
        <div class="spread">
          <span class="sub small tnum">
            Veritabanı {info ? formatSize(info.dbSize) : "…"}{#if lastBackup} · son yedek {lastBackup.created}{/if}
          </span>
          <button class="btn ghost" onclick={() => api.openDataFolder()}><Icon name="folder" />Aç</button>
        </div>
      </section>

      <section class="panel">
        <h3 class="h2"><Icon name="info" />Bilmekte fayda var</h3>
        <ul class="tips">
          <li>Pencereyi kapatınca GMID Tool <b>tepside</b> (saatin yanında) çalışmaya devam eder; hatırlatıcılar ve sayaçlar yine çalar.</li>
          <li>Bir notu <b>masaüstüne yapıştırabilirsin</b>: her zaman üstte duran küçük bir not olur.</li>
          <li>Windows açılınca kendiliğinden başlaması için: <button class="link" onclick={() => onnavigate("settings")}>Kontrol Odası → Başlangıç</button></li>
        </ul>
      </section>
    </div>
  </div>
</div>

<style>
  .home { display: flex; flex-direction: column; gap: 20px; max-width: 1400px; margin: 0 auto; }
  .hello { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; flex-wrap: wrap; }
  .date { text-transform: capitalize; }
  h2 { font-family: var(--f-display); font-weight: 800; font-size: clamp(22px, 2.6vw, 30px); margin: 0; text-wrap: balance; }
  h2 em { font-style: normal; color: var(--gold); }
  .grid { display: grid; grid-template-columns: minmax(0, 1.4fr) minmax(300px, 1fr); gap: 16px; align-items: start; }
  .col { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  .panel { display: flex; flex-direction: column; gap: 10px; }
  .li { display: flex; justify-content: space-between; gap: 12px; padding: 9px 12px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); border: 1px solid transparent; text-align: left; width: 100%; }
  .li.note:hover { border-color: var(--gold-dim); }
  .li.hot { border-color: rgba(227, 194, 112, 0.45); background: rgba(227, 194, 112, 0.07); }
  .li .t { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--dim); align-self: center; flex: none; }
  .dot.online { background: var(--good); box-shadow: 0 0 8px rgba(92, 192, 138, 0.6); }
  .dot.offline { background: var(--bad); }
  .li .m { color: var(--muted); font-size: 12px; white-space: nowrap; }
  ol { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  ol > li { display: grid; grid-template-columns: 32px minmax(0, 1fr); gap: 14px; padding: 11px 0; border-top: 1px solid var(--line); }
  ol > li:first-child { border-top: 0; }
  .num { width: 32px; height: 32px; border-radius: 50%; display: grid; place-items: center; font-family: var(--f-display); font-weight: 800; font-size: 14px; border: 1px solid var(--line-2); color: var(--muted); }
  li.done .num { border-color: var(--gold-dim); color: var(--gold); }
  li.now .num { border-color: var(--gold); color: #1b1608; background: var(--gold); box-shadow: 0 0 16px rgba(227, 194, 112, 0.35); }
  .body { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .body b { font-family: var(--f-display); font-size: 15px; display: flex; align-items: center; gap: 8px; }
  .here { color: var(--gold); border-color: rgba(227, 194, 112, 0.4); font-family: var(--f-body); }
  .links { display: flex; flex-wrap: wrap; gap: 6px; }
  .links button { display: inline-flex; align-items: center; gap: 6px; font-size: 12.5px; border: 1px solid var(--line); background: rgba(255, 255, 255, 0.03); color: var(--muted); border-radius: 999px; padding: 4px 10px; }
  .links button:hover { color: var(--text); border-color: var(--gold-dim); }
  li.now .links button { color: var(--text); border-color: var(--gold-dim); }
  .keys { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .keys li { display: flex; justify-content: space-between; align-items: center; gap: 10px; color: var(--muted); }
  .keys li span:last-child { display: flex; gap: 3px; }
  kbd { font-family: var(--f-body); font-size: 11px; font-weight: 600; padding: 1px 6px; border-radius: 5px; background: var(--coal); border: 1px solid var(--line-2); color: var(--text); }
  .tips { margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 8px; color: var(--muted); }
  .tips b { color: var(--text); font-weight: 600; }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; align-self: flex-start; }
  .link:hover { color: var(--neon); }
  .link.inl { display: inline; }
  @media (max-width: 1100px) { .grid { grid-template-columns: minmax(0, 1fr); } }
</style>
