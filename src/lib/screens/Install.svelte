<script lang="ts">
  // Format Sonrası Kurulum: katalogdan listeni yap, tek tıkla kur ya da USB'ye "Format Kiti" çıkar.
  import { onMount } from "svelte";
  import { api, formatSize } from "../api";
  import { on } from "../bus";
  import Switch from "../components/Switch.svelte";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";
  import type { InstallHit, InstallItem, InstallRun, InstallStep, KitProgress, KitResult } from "../types";

  let catalog: InstallItem[] = $state([]);
  let list: InstallItem[] = $state([]);
  let installed = $state<Set<string> | null>(null);
  let installedError = $state<string | null>(null);
  let run: InstallRun = $state({ running: false, stopping: false, steps: [], started: null, finished: null });

  let query = $state("");
  let category = $state<string | null>(null);
  let hits: InstallHit[] = $state([]);
  let searching = $state(false);
  let searched = $state("");

  // Format Kiti
  let kitDir = $state<string | null>(null);
  let withPrograms = $state(true);
  let withSetup = $state(true);
  let setupPath = $state<string | null>(null);
  let withBackup = $state(true);
  let withVideos = $state(false);
  let withTools = $state(false);
  let backupSize = $state<number | null>(null);
  let kitBusy = $state(false);
  let kitProgress = $state<KitProgress | null>(null);
  let kit = $state<KitResult | null>(null);

  onMount(() => {
    void api.install.catalog().then((c) => (catalog = c));
    void api.install.list().then((l) => (list = l));
    void api.install.state().then((s) => (run = s));
    void api.install.findSetup().then((p) => (setupPath = p));
    void loadInstalled(false);
    const stops = [
      on<InstallRun>("install", (s) => {
        const wasRunning = run.running;
        run = s;
        if (wasRunning && !s.running) void loadInstalled(true);
      }),
      on<KitProgress>("install-kit", (p) => (kitProgress = p)),
    ];
    return () => stops.forEach((s) => void s.then((f) => f()));
  });

  $effect(() => {
    const options = { videos: withVideos, tools: withTools };
    backupSize = null;
    void api.install.backupSize(options).then((n) => (backupSize = n)).catch(() => {});
  });

  async function loadInstalled(refresh: boolean) {
    try {
      installed = new Set(await api.install.installed(refresh));
      installedError = null;
    } catch (e) {
      installedError = String(e);
    }
  }

  const key = (i: { id: string }) => i.id.toLowerCase();
  const inList = (i: { id: string }) => list.some((x) => key(x) === key(i));
  const isInstalled = (i: InstallItem) => i.source !== "web" && !!installed?.has(key(i));

  function save() {
    void api.install.saveList($state.snapshot(list)).catch((e) => notify(String(e), "info"));
  }

  function toggle(item: InstallItem) {
    list = inList(item) ? list.filter((x) => key(x) !== key(item)) : [...list, item];
    save();
  }

  function addHit(h: InstallHit) {
    if (inList(h)) return;
    list = [...list, { id: h.id, name: h.name, source: "winget", category: "Depodan", url: null, note: null }];
    save();
  }

  function move(i: number, d: -1 | 1) {
    const j = i + d;
    if (j < 0 || j >= list.length) return;
    const next = [...list];
    [next[i], next[j]] = [next[j], next[i]];
    list = next;
    save();
  }

  async function searchStore() {
    const q = query.trim();
    if (q.length < 2) return;
    searching = true;
    try {
      hits = await api.install.search(q);
      searched = q;
    } catch (e) {
      notify(String(e), "info");
    } finally {
      searching = false;
    }
  }

  const categories = $derived([...new Set(catalog.map((c) => c.category ?? "Diğer"))]);
  const shown = $derived(
    catalog.filter((c) => {
      const q = query.trim().toLocaleLowerCase("tr");
      if (category && (c.category ?? "Diğer") !== category) return false;
      return !q || c.name.toLocaleLowerCase("tr").includes(q) || c.id.toLowerCase().includes(q);
    }),
  );
  const grouped = $derived(
    categories
      .map((cat) => ({ cat, items: shown.filter((c) => (c.category ?? "Diğer") === cat) }))
      .filter((g) => g.items.length),
  );
  const freshHits = $derived(hits.filter((h) => !catalog.some((c) => key(c) === key(h))));
  const missing = $derived(list.filter((i) => !isInstalled(i)));
  const stepOf = (i: InstallItem): InstallStep | undefined => run.steps.find((s) => key(s) === key(i));
  const doneCount = $derived(run.steps.filter((s) => !["waiting", "running"].includes(s.state)).length);
  const failed = $derived(run.steps.filter((s) => s.state === "failed"));

  async function installNow(items: InstallItem[]) {
    if (!items.length) return;
    try {
      run = await api.install.run($state.snapshot(items));
    } catch (e) {
      notify(String(e), "info");
    }
  }

  const STATE_TEXT: Record<string, string> = {
    waiting: "Sırada",
    running: "Kuruluyor…",
    done: "Kuruldu",
    already: "Zaten kurulu",
    opened: "Sayfa açıldı",
    failed: "Kurulamadı",
    skipped: "Atlandı",
  };

  function initials(name: string) {
    return name.replace(/[^\p{L}\p{N} ]/gu, "").split(" ").filter(Boolean).slice(0, 2).map((w) => w[0]).join("").toUpperCase() || "?";
  }
  function hue(name: string) {
    let h = 0;
    for (const c of name) h = (h * 31 + c.charCodeAt(0)) % 360;
    return h;
  }

  async function pickDir() {
    const d = await api.install.pickFolder();
    if (d) {
      kitDir = d;
      kit = null;
    }
  }

  async function pickSetup() {
    const p = await api.install.pickSetup();
    if (p) setupPath = p;
  }

  async function buildKit() {
    if (!kitDir) return void (await pickDir());
    kitBusy = true;
    kit = null;
    kitProgress = null;
    try {
      kit = await api.install.exportKit({
        dir: kitDir,
        items: withPrograms ? $state.snapshot(list) : [],
        setup: withSetup ? setupPath : null,
        backup: withBackup,
        backupOptions: { videos: withVideos, tools: withTools },
      });
      notify("Format Kiti hazır", "install");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      kitBusy = false;
    }
  }

  const kitPercent = $derived(
    kitProgress && kitProgress.total ? Math.round((kitProgress.done / kitProgress.total) * 100) : 0,
  );
  const kitStage = $derived(
    kitProgress?.stage === "backup" ? `GMID yedekleniyor · %${kitPercent}` : kitProgress?.stage === "setup" ? "Kurulum dosyası kopyalanıyor…" : "Hazırlanıyor…",
  );
  const fileName = (p: string) => p.split(/[\\/]/).pop() ?? p;
</script>

<div class="inst">
  <section class="panel hero">
    <div class="htx">
      <h2>Format Sonrası Kurulum</h2>
      <p class="sub">
        Formattan sonra programları tek tek indirme. Listeni yap; ister şimdi tek tıkla kur, ister USB'ye bir
        <b>Format Kiti</b> çıkar: formattan sonra <b>Kur.bat</b>'a çift tıklarsın, programlar kurulur, GMID de ayarları ve
        geçmişiyle geri gelir.
      </p>
      {#if installedError}
        <p class="warn small"><Icon name="info" size={14} />{installedError}</p>
      {/if}
    </div>
    <div class="stats">
      <div><b class="tnum">{list.length}</b><span class="sub small">listende</span></div>
      <div><b class="tnum">{installed ? list.length - missing.length : "…"}</b><span class="sub small">kurulu</span></div>
    </div>
  </section>

  <div class="grid">
    <section class="panel cat">
      <div class="spread">
        <h3 class="h2"><Icon name="library" />Katalog</h3>
        <button class="btn ghost sm" title="Kurulu programlara yeniden bak" onclick={() => loadInstalled(true)}><Icon name="refresh" size={14} />Yenile</button>
      </div>
      <div class="searchrow">
        <span class="sbox"><Icon name="search" size={15} /><input bind:value={query} placeholder="Program ara…" onkeydown={(e) => e.key === "Enter" && searchStore()} /></span>
        <button class="btn ghost sm" disabled={searching || query.trim().length < 2} onclick={searchStore}>
          <Icon name="search" size={14} />{searching ? "Aranıyor…" : "Tüm depoda ara"}
        </button>
      </div>
      <div class="chips">
        <button class="chip" class:on={!category} onclick={() => (category = null)}>Hepsi</button>
        {#each categories as c (c)}
          <button class="chip" class:on={category === c} onclick={() => (category = category === c ? null : c)}>{c}</button>
        {/each}
      </div>

      {#if searched && query.trim() === searched}
        <div class="group">
          <h4>Microsoft paket deposunda <span class="q">“{searched}”</span></h4>
          {#if freshHits.length}
            <div class="tiles">
              {#each freshHits as h (h.id)}
                <button class="tile" class:sel={inList(h)} onclick={() => addHit(h)} title={h.id}>
                  <span class="av" style:--h={hue(h.name)}>{initials(h.name)}</span>
                  <span class="tx"><b>{h.name}</b><span class="sub small">{h.id} · {h.version}</span></span>
                  <span class="mark"><Icon name={inList(h) ? "check" : "plus"} size={15} /></span>
                </button>
              {/each}
            </div>
          {:else}
            <p class="sub small">Katalogda olmayan bir sonuç yok.</p>
          {/if}
        </div>
      {/if}

      {#each grouped as g (g.cat)}
        <div class="group">
          <h4>{g.cat}</h4>
          <div class="tiles">
            {#each g.items as item (item.id)}
              <button class="tile" class:sel={inList(item)} onclick={() => toggle(item)} title={item.note ?? item.id}>
                <span class="av" style:--h={hue(item.name)}>{initials(item.name)}</span>
                <span class="tx">
                  <b>{item.name}</b>
                  <span class="sub small">
                    {#if isInstalled(item)}<span class="ok">Kurulu</span>{" · "}{/if}{item.source === "msstore" ? "Microsoft Store" : item.source === "web" ? "İndirme sayfası" : (item.note ?? "winget")}
                  </span>
                </span>
                <span class="mark"><Icon name={inList(item) ? "check" : "plus"} size={15} /></span>
              </button>
            {/each}
          </div>
        </div>
      {:else}
        {#if !searched}<p class="sub">Katalogda “{query}” yok. <b>Tüm depoda ara</b> ile Microsoft'un paket deposuna bak.</p>{/if}
      {/each}
    </section>

    <aside class="panel mine">
      <div class="spread">
        <h3 class="h2"><Icon name="install" />Listem</h3>
        <span class="sub small tnum">{list.length} program</span>
      </div>
      {#if run.running || run.finished}
        <div class="runbar">
          <span class="small">
            {#if run.running}{run.stopping ? "Şu anki bitince duracak…" : `Kuruluyor · ${doneCount}/${run.steps.length}`}
            {:else}Bitti · {run.steps.filter((s) => s.state === "done").length} kuruldu{failed.length ? ` · ${failed.length} kurulamadı` : ""}{/if}
          </span>
          <span class="bar"><i style:width="{run.steps.length ? (doneCount / run.steps.length) * 100 : 0}%"></i></span>
        </div>
      {/if}
      <ol class="items">
        {#each list as item, i (item.id)}
          {@const st = stepOf(item)}
          <li class:failed={st?.state === "failed"}>
            <span class="av sm" style:--h={hue(item.name)}>{initials(item.name)}</span>
            <span class="tx">
              <b>{item.name}</b>
              <span class="sub small" title={st?.message ?? undefined}>
                {#if st && (run.running || run.finished)}
                  <span class="st {st.state}">{STATE_TEXT[st.state]}</span>{st.message && st.state !== "already" ? ` · ${st.message}` : ""}
                {:else if isInstalled(item)}
                  <span class="ok">Kurulu</span>
                {:else}
                  {item.source === "web" ? "İndirme sayfası açılır" : item.source === "msstore" ? "Microsoft Store" : "Kurulacak"}
                {/if}
              </span>
              {#if st?.state === "running" && st.progress != null}
                <span class="bar thin"><i style:width="{st.progress * 100}%"></i></span>
              {/if}
            </span>
            {#if !run.running}
              <span class="acts">
                <button title="Yukarı" disabled={i === 0} onclick={() => move(i, -1)}><Icon name="arrow" size={13} /></button>
                <button title="Aşağı" class="down" disabled={i === list.length - 1} onclick={() => move(i, 1)}><Icon name="arrow" size={13} /></button>
                <button title="Listeden çıkar" onclick={() => toggle(item)}><Icon name="x" size={13} /></button>
              </span>
            {/if}
          </li>
        {:else}
          <li class="empty sub small">Soldaki katalogdan program seç.</li>
        {/each}
      </ol>
      {#if run.running}
        <button class="btn ghost" disabled={run.stopping} onclick={() => api.install.stop()}><Icon name="x" size={15} />Durdur</button>
      {:else}
        <button class="btn" disabled={!missing.length} onclick={() => installNow(missing)}>
          <Icon name="download" size={15} />Şimdi kur{missing.length ? ` (${missing.length})` : ""}
        </button>
        {#if failed.length}
          <button class="btn ghost" onclick={() => installNow(list.filter((i) => failed.some((f) => key(f) === key(i))))}><Icon name="replay" size={15} />Kurulamayanları tekrar dene</button>
        {/if}
        <p class="sub small">Sessiz kurulur; bazı programlar için Windows "değişiklik yapılsın mı?" diye sorabilir.</p>
      {/if}
    </aside>
  </div>

  <section class="panel kitp" id="kit">
    <div class="spread">
      <h3 class="h2"><Icon name="shield" />Format Kiti</h3>
      <span class="sub small">GMID kurulu değilken de çalışır</span>
    </div>
    <div class="kitgrid">
      <div class="opts">
        <div class="line">
          <div><b>Nereye?</b><span class="sub small">USB bellek ya da bulut klasörü (Drive, OneDrive). İçine “GMID Format Kiti” klasörü açılır.</span></div>
          <button class="btn ghost sm" onclick={pickDir}><Icon name="folder" size={14} />{kitDir ? "Değiştir" : "Klasör seç"}</button>
        </div>
        {#if kitDir}<div class="path">{kitDir}</div>{/if}
        <div class="line">
          <div><b>Programlar</b><span class="sub small">Listendeki {list.length} program sırayla ve sessizce kurulur.</span></div>
          <Switch label="Programlar" checked={withPrograms} onchange={(v) => (withPrograms = v)} />
        </div>
        <div class="line">
          <div>
            <b>GMID Tool'un kendisi</b>
            <span class="sub small">
              {#if setupPath}{fileName(setupPath)} · <button class="link" onclick={pickSetup}>değiştir</button>
              {:else}Kurulum dosyası İndirilenler'de bulunamadı. <button class="link" onclick={pickSetup}>Seç</button>{/if}
            </span>
          </div>
          <Switch label="GMID kurulumu" checked={withSetup && !!setupPath} disabled={!setupPath} onchange={(v) => (withSetup = v)} />
        </div>
        <div class="line">
          <div>
            <b>GMID yedeği</b>
            <span class="sub small">Ayarlar, kısayollar, notlar, hatırlatıcılar, oyun kütüphanesi, oturum geçmişi, puanlar, çalma listeleri, kapaklar ve arka planlar{backupSize != null ? ` · ~${formatSize(backupSize)}` : ""}</span>
          </div>
          <Switch label="GMID yedeği" checked={withBackup} onchange={(v) => (withBackup = v)} />
        </div>
        {#if withBackup}
          <div class="subopts">
            <label><input type="checkbox" bind:checked={withVideos} />İndirdiğim videolar ve müzikler <span class="sub small">(büyük olabilir)</span></label>
            <label><input type="checkbox" bind:checked={withTools} />Araçlar ve modeller <span class="sub small">(yt-dlp, ffmpeg, Whisper · yeniden indirilebilir)</span></label>
          </div>
        {/if}
      </div>
      <div class="howto">
        <h4>Formattan sonra</h4>
        <ol>
          <li>Kit klasöründeki <b>Kur.bat</b>'a çift tıkla.</li>
          {#if withPrograms && list.length}<li>Programlar sırayla kurulur; bitenler ✅ ile yazılır.</li>{/if}
          {#if withBackup}<li>GMID'in verisi <code>Belgeler\GMID Tool</code> içine geri konur.</li>{/if}
          {#if withSetup && setupPath}<li>GMID Tool kurulur ve formattan önceki haliyle açılır.</li>{/if}
          <li>Bir şey kurulamazsa Kur.bat'ı yeniden çalıştırman yeterli.</li>
        </ol>
        {#if kitBusy}
          <div class="runbar"><span class="small">{kitStage}</span><span class="bar"><i style:width="{kitProgress?.stage === 'backup' ? kitPercent : 8}%"></i></span></div>
        {:else if kit}
          <div class="kitok">
            <Icon name="check" size={18} />
            <span><b>Hazır.</b> {kit.programs} program{kit.setup ? ", GMID kurulumu" : ""}{kit.backupSize != null ? `, yedek ${formatSize(kit.backupSize)}` : ""}</span>
            <button class="btn ghost sm" onclick={() => kit && api.install.openKit(kit.path)}><Icon name="folder" size={14} />Klasörü aç</button>
          </div>
        {/if}
        <button class="btn" disabled={kitBusy || (!withPrograms && !withBackup && !(withSetup && setupPath))} onclick={buildKit}>
          <Icon name="shield" size={15} />{kitDir ? "Format Kiti'ni oluştur" : "Klasör seç ve oluştur"}
        </button>
      </div>
    </div>
  </section>
</div>

<style>
  .inst { display: flex; flex-direction: column; gap: 16px; max-width: 1180px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
  .hero { flex-direction: row; align-items: center; gap: 24px; padding: 22px 24px; }
  .htx { flex: 1; display: flex; flex-direction: column; gap: 6px; }
  .htx h2 { margin: 0; font-family: var(--f-display); font-size: 26px; }
  .htx p { margin: 0; max-width: 720px; }
  .warn { display: flex; align-items: center; gap: 6px; color: var(--gold-glow); }
  .stats { display: flex; gap: 22px; }
  .stats div { display: flex; flex-direction: column; align-items: center; }
  .stats b { font-size: 26px; font-family: var(--f-display); }
  .grid { display: grid; grid-template-columns: minmax(0, 1fr) 340px; gap: 16px; align-items: start; }
  .mine { position: sticky; top: 0; max-height: calc(100vh - 140px); }
  .spread { display: flex; justify-content: space-between; align-items: center; gap: 10px; }
  .h2 { margin: 0; }
  .searchrow { display: flex; gap: 8px; flex-wrap: wrap; }
  .sbox { flex: 1 1 220px; display: flex; align-items: center; gap: 8px; background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 0 10px; color: var(--muted); }
  .sbox input { flex: 1; min-width: 0; background: none; border: 0; padding: 8px 0; color: var(--text); outline: none; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { cursor: pointer; }
  .chip.on { border-color: var(--gold); color: var(--gold-glow); background: rgba(227, 194, 112, 0.12); }
  .group { display: flex; flex-direction: column; gap: 8px; }
  .group h4 { margin: 6px 0 0; font-size: 12px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--dim); }
  .group h4 .q { text-transform: none; letter-spacing: 0; }
  .tiles { display: grid; grid-template-columns: repeat(auto-fill, minmax(210px, 1fr)); gap: 8px; }
  .tile { display: flex; align-items: center; gap: 10px; padding: 9px 10px; border-radius: 12px; border: 1px solid var(--line); background: rgba(255, 255, 255, 0.02); color: var(--text); text-align: left; min-width: 0; transition: border-color 0.12s, background 0.12s; }
  .tile:hover { border-color: var(--line-2); background: rgba(255, 255, 255, 0.04); }
  .tile.sel { border-color: var(--gold-dim); background: rgba(227, 194, 112, 0.08); }
  .av { flex: none; display: grid; place-items: center; width: 34px; height: 34px; border-radius: 10px; font-weight: 800; font-size: 12px; color: #fff; background: hsl(var(--h) 38% 34%); }
  .av.sm { width: 28px; height: 28px; border-radius: 8px; font-size: 11px; }
  .tx { flex: 1; display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .tx b { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tx .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mark { flex: none; display: grid; place-items: center; width: 24px; height: 24px; border-radius: 50%; border: 1px solid var(--line-2); color: var(--muted); }
  .tile.sel .mark { background: var(--gold); border-color: var(--gold); color: #1b1608; }
  .ok { color: var(--good); font-weight: 600; }
  .items { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; overflow-y: auto; min-height: 0; }
  .items li { display: flex; align-items: center; gap: 10px; padding: 7px 8px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); }
  .items li.failed { background: rgba(224, 106, 106, 0.08); }
  .items li.failed .tx .sub { white-space: normal; }
  .items li.empty { justify-content: center; padding: 18px; background: none; }
  .acts { display: flex; gap: 2px; opacity: 0.55; }
  .items li:hover .acts { opacity: 1; }
  .acts button { display: grid; place-items: center; width: 24px; height: 24px; border: 0; border-radius: 6px; background: none; color: var(--muted); }
  .acts button:hover:not(:disabled) { background: rgba(255, 255, 255, 0.06); color: var(--text); }
  .acts button:disabled { opacity: 0.3; }
  .acts button:first-child :global(svg) { transform: rotate(-90deg); }
  .acts button.down :global(svg) { transform: rotate(90deg); }
  .st { font-weight: 600; }
  .st.running { color: var(--gold-glow); }
  .st.done, .st.already { color: var(--good); }
  .st.opened { color: var(--teal); }
  .st.failed { color: var(--bad); }
  .runbar { display: flex; flex-direction: column; gap: 6px; }
  .bar { display: block; height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.07); overflow: hidden; }
  .bar.thin { height: 3px; margin-top: 4px; }
  .bar i { display: block; height: 100%; background: var(--gold); border-radius: inherit; transition: width 0.25s; }
  .kitgrid { display: grid; grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr); gap: 22px; }
  .opts { display: flex; flex-direction: column; gap: 14px; }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 16px; }
  .line > div:first-child { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
  .path { font-family: var(--f-mono, monospace); font-size: 12px; padding: 7px 10px; border-radius: 8px; background: rgba(255, 255, 255, 0.04); word-break: break-all; }
  .subopts { display: flex; flex-direction: column; gap: 8px; padding-left: 14px; border-left: 2px solid var(--line); }
  .subopts label { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .subopts input { accent-color: var(--gold); }
  .link { border: 0; background: none; padding: 0; color: var(--gold-glow); text-decoration: underline; cursor: pointer; font: inherit; }
  .howto { display: flex; flex-direction: column; gap: 12px; padding: 14px 16px; border-radius: 12px; background: rgba(255, 255, 255, 0.03); border: 1px solid var(--line); }
  .howto h4 { margin: 0; }
  .howto ol { margin: 0; padding-left: 20px; display: flex; flex-direction: column; gap: 4px; color: var(--muted); }
  .kitok { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; color: var(--good); }
  .kitok span { flex: 1; color: var(--text); }
  @media (max-width: 980px) {
    .grid, .kitgrid { grid-template-columns: 1fr; }
    .mine { position: static; max-height: none; }
  }
  @media (max-width: 640px) {
    .hero { flex-direction: column; align-items: stretch; }
  }
</style>
