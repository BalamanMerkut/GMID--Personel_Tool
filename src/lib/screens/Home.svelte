<script lang="ts">
  import { formatSize, type AppInfo } from "../api";
  import Icon from "../Icon.svelte";
  import { SCREENS } from "../nav";
  import { api } from "../api";

  let { info, onnavigate }: { info: AppInfo | null; onnavigate: (id: string) => void } = $props();

  const now = new Date();
  const hour = now.getHours();
  const greeting = hour < 6 ? "İyi geceler" : hour < 12 ? "Günaydın" : hour < 18 ? "İyi günler" : "İyi akşamlar";
  const today = now.toLocaleDateString("tr-TR", { weekday: "long", day: "numeric", month: "long" });

  const PHASES = [
    { n: 1, title: "İskelet", note: "Tema, açılış, menü, Kontrol Odası, tepsi, yedekler" },
    { n: 2, title: "Günlük araçlar" },
    { n: 3, title: "Oyun" },
    { n: 4, title: "Oyun içi deneyim" },
    { n: 5, title: "Video İndirici" },
    { n: 6, title: "Film & Dizi, Kurulum" },
  ];
  const CURRENT_PHASE = 1;
  const screensOf = (n: number) => SCREENS.filter((s) => s.phase === n);
  const lastBackup = $derived(info?.backups[0]);
</script>

<div class="home">
  <div class="hello">
    <div>
      <p class="sub date">{today}</p>
      <h2>{greeting}, <em>malikâne sakini</em>.</h2>
    </div>
    <button class="btn" onclick={() => onnavigate("settings")}><Icon name="palette" />Görünümü kişiselleştir</button>
  </div>

  <div class="grid">
    <section class="panel roadmap">
      <h3 class="h2"><Icon name="castle" />Malikânenin odaları</h3>
      <p class="sub small">Odalar faz faz açılıyor. Henüz açılmamış bir odaya tıklarsan içinde ne olacağını görürsün.</p>
      <ol>
        {#each PHASES as p (p.n)}
          <li class:done={p.n < CURRENT_PHASE} class:now={p.n === CURRENT_PHASE}>
            <span class="num tnum">{p.n}</span>
            <div class="body">
              <b>{p.title}{#if p.n === CURRENT_PHASE}<span class="chip here">Şu an</span>{/if}</b>
              {#if p.note}
                <span class="sub small">{p.note}</span>
              {:else}
                <div class="links">
                  {#each screensOf(p.n) as s (s.id)}
                    <button onclick={() => onnavigate(s.id)}><Icon name={s.icon} size={15} />{s.label}</button>
                  {/each}
                </div>
              {/if}
            </div>
          </li>
        {/each}
      </ol>
    </section>

    <div class="side">
      <section class="panel">
        <h3 class="h2"><Icon name="folder" />Veri klasörün</h3>
        <p class="sub small">GMID Tool'un ürettiği her şey burada durur. Format atarken bu klasörü yedeklemen yeterli.</p>
        <div class="path">{info?.dataDir ?? "…"}</div>
        <div class="spread">
          <span class="sub small tnum">Veritabanı {info ? formatSize(info.dbSize) : "…"}</span>
          <button class="btn ghost" onclick={() => api.openDataFolder()}><Icon name="folder" />Klasörü aç</button>
        </div>
      </section>
      <section class="panel">
        <h3 class="h2"><Icon name="shield" />Son yedek</h3>
        {#if lastBackup}
          <p class="sub tnum">{lastBackup.created} · {formatSize(lastBackup.size)}</p>
        {:else}
          <p class="sub">Henüz yedek yok; ilk yedek birazdan kendiliğinden alınır.</p>
        {/if}
        <p class="sub small">Her gün ilk açılışta otomatik yedek alınır, son 7 yedek saklanır.</p>
      </section>
      <section class="panel">
        <h3 class="h2"><Icon name="info" />Bilmekte fayda var</h3>
        <ul class="tips">
          <li>Pencereyi kapatınca GMID Tool <b>tepside</b> (saatin yanında) çalışmaya devam eder. Simgeye tıklayınca geri gelir.</li>
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
  .side { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  .side .panel, .roadmap { display: flex; flex-direction: column; gap: 10px; }
  ol { list-style: none; margin: 6px 0 0; padding: 0; display: flex; flex-direction: column; }
  ol > li { display: grid; grid-template-columns: 32px minmax(0, 1fr); gap: 14px; padding: 12px 0; border-top: 1px solid var(--line); }
  ol > li:first-child { border-top: 0; }
  .num {
    width: 32px; height: 32px; border-radius: 50%; display: grid; place-items: center;
    font-family: var(--f-display); font-weight: 800; font-size: 14px;
    border: 1px solid var(--line-2); color: var(--muted);
  }
  li.now .num { border-color: var(--gold); color: #1b1608; background: var(--gold); box-shadow: 0 0 16px rgba(227, 194, 112, 0.35); }
  .body { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .body b { font-family: var(--f-display); font-size: 15px; display: flex; align-items: center; gap: 8px; }
  .here { color: var(--gold); border-color: rgba(227, 194, 112, 0.4); font-family: var(--f-body); }
  .links { display: flex; flex-wrap: wrap; gap: 6px; }
  .links button {
    display: inline-flex; align-items: center; gap: 6px; font-size: 12.5px;
    border: 1px solid var(--line); background: rgba(255, 255, 255, 0.03); color: var(--muted);
    border-radius: 999px; padding: 4px 10px;
  }
  .links button:hover { color: var(--text); border-color: var(--gold-dim); }
  .tips { margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 8px; color: var(--muted); }
  .tips b { color: var(--text); font-weight: 600; }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; }
  .link:hover { color: var(--neon); }
  @media (max-width: 1100px) { .grid { grid-template-columns: minmax(0, 1fr); } }
</style>
