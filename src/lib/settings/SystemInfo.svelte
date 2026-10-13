<script lang="ts">
  // Sistem özellikleri: üstte bilgisayarın özeti ve dört ana parça (işlemci, ekran kartı, bellek, depolama),
  // altta ayrıntılar (sürücü doluluğu, bellek modülleri, ekranlar, çevre birimleri, ağ, anakart).
  // "Discord'a kopyala" ile düz bir özet panoya gider.
  import { onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";
  import type { DriverStatus, Specs } from "../types";

  let specs = $state<Specs | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function load(refresh = false) {
    busy = true;
    error = null;
    try {
      specs = await api.system.info(refresh);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  let driver = $state<DriverStatus | null>(null);
  let driverBusy = $state(false);
  async function loadDriver(refresh = false) {
    driverBusy = true;
    driver = await api.system.driver(refresh).catch(() => driver);
    driverBusy = false;
  }

  onMount(() => {
    void load();
    void loadDriver();
  });

  async function copy() {
    try {
      await navigator.clipboard.writeText(await api.system.text());
      notify("Sistem özellikleri kopyalandı · Discord'da Ctrl+V", "copy");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  // ---- biçimlendirme ----
  const GiB = 1024 ** 3;
  const num = (n: number, d = 1) => n.toLocaleString("tr-TR", { maximumFractionDigits: d });
  /** Disk boyutu: 1,8 TB · 476 GB */
  function size(b: number) {
    if (b >= 1024 * GiB) return `${num(b / 1024 / GiB)} TB`;
    return `${num(b / GiB, b < 10 * GiB ? 1 : 0)} GB`;
  }
  /** Bellek: tam sayı GB (16 GB, 512 MB). */
  const mem = (b: number) => (b < GiB ? `${Math.round(b / 1024 ** 2)} MB` : `${Math.round(b / GiB)} GB`);
  const ramType = (t: number) => ({ 34: "DDR5", 26: "DDR4", 24: "DDR3", 30: "LPDDR4", 35: "LPDDR5" })[t] ?? "";
  function uptime(min: number) {
    const d = Math.floor(min / 1440), h = Math.floor((min % 1440) / 60), m = min % 60;
    if (d) return `${d} gün ${h} sa`;
    if (h) return `${h} sa ${m} dk`;
    return `${m} dk`;
  }
  /** Şirket unvanlarını kısaltır: "ASUSTeK COMPUTER INC." → ASUS. */
  function brand(s: string) {
    const t = s.trim();
    const known: [RegExp, string][] = [
      [/^asustek/i, "ASUS"], [/^micro-star/i, "MSI"], [/^gigabyte/i, "Gigabyte"], [/^asrock/i, "ASRock"],
      [/^hewlett|^hp\b/i, "HP"], [/^lenovo/i, "Lenovo"], [/^dell/i, "Dell"], [/^acer/i, "Acer"], [/^msi\b/i, "MSI"],
    ];
    for (const [re, name] of known) if (re.test(t)) return name;
    return t.replace(/,?\s+(co\.,?\s*ltd\.?|inc\.?|corporation|corp\.?|ltd\.?)$/i, "");
  }
  /** Üreticinin doldurmadığı yer tutucular. */
  const filler = (s: string) => !s.trim() || /system product name|system manufacturer|to be filled|default string|o\.e\.m/i.test(s);
  function date(s: string) {
    const d = new Date(s);
    return isNaN(+d) ? s : d.toLocaleDateString("tr-TR", { day: "numeric", month: "long", year: "numeric" });
  }
  const monitorTitle = (m: Specs["monitors"][number]) =>
    !m.maker || m.name.toLowerCase().startsWith(m.maker.toLowerCase()) ? m.name : `${m.maker} ${m.name}`;
  const diskKind = (d: Specs["disks"][number]) =>
    [d.bus, d.media].filter((x) => x && x !== "Unspecified" && x !== "Unknown").join(" · ");

  // ---- özet ----
  const cpu = $derived(specs?.cpu[0]);
  const gpu = $derived(specs?.gpu[0]);
  const modules = $derived(specs?.ram.modules ?? []);
  const ramLine = $derived.by(() => {
    if (!modules.length) return "";
    const sizes = [...new Set(modules.map((m) => m.size))];
    const layout = sizes.length === 1 ? `${modules.length}×${mem(sizes[0])}` : `${modules.length} modül`;
    const speed = Math.max(...modules.map((m) => m.speed));
    return [layout, speed ? `${speed} MT/s` : ""].filter(Boolean).join(" · ");
  });
  const ramKind = $derived(ramType(modules[0]?.type ?? 0));
  const diskTotal = $derived((specs?.disks ?? []).reduce((a, d) => a + d.size, 0));
  const volTotal = $derived((specs?.volumes ?? []).reduce((a, v) => a + v.size, 0));
  const volFree = $derived((specs?.volumes ?? []).reduce((a, v) => a + v.free, 0));
  const machine = $derived(specs && !filler(specs.os.model) ? `${brand(specs.os.maker)} ${specs.os.model}`.trim() : "");
  const peripherals = $derived([
    ...(specs?.input ?? []).map((d) => ({ ...d, icon: d.kind === "Fare" ? ("mouse" as const) : ("keyboard" as const) })),
    ...(specs?.audio ?? []).map((d) => ({
      kind: d.kind === "Giriş" ? "Mikrofon" : "Ses çıkışı",
      name: d.name,
      icon: d.kind === "Giriş" ? ("mic" as const) : ("headphones" as const),
    })),
    ...(specs?.cameras ?? []).map((d) => ({ kind: "Kamera", name: d.name, icon: "camera" as const })),
    ...(specs?.printers ?? []).map((d) => ({
      kind: d.kind === "Varsayılan" ? "Yazıcı (varsayılan)" : "Yazıcı",
      name: d.name,
      icon: "printer" as const,
    })),
  ]);
  /** "bugün", "3 gün önce" */
  function age(ms: number) {
    if (!ms) return "";
    const days = Math.floor((Date.now() - ms) / 86_400_000);
    return days <= 0 ? "bugün" : days === 1 ? "dün" : `${days} gün önce`;
  }
</script>

<section class="panel hero">
  <span class="pc"><Icon name="monitor" size={30} /></span>
  <div class="who">
    {#if specs}
      <b class="name">{specs.os.computer || "Bu bilgisayar"}</b>
      <span class="sub">
        {specs.os.name.replace("Microsoft ", "")}{specs.os.version ? ` · ${specs.os.version}` : ""}{specs.os.build ? ` (derleme ${specs.os.build})` : ""}
        {#if machine}<span class="dot">·</span>{machine}{/if}
      </span>
      <span class="sub small tnum">
        {#if specs.os.uptimeMin}{uptime(specs.os.uptimeMin)} önce açıldı{/if}{#if specs.readAt}<span class="dot">·</span>bilgiler {age(specs.readAt)} okundu (değiştiyse Yenile){/if}
      </span>
    {:else if busy}
      <b class="name">Aygıtlar okunuyor…</b>
      <span class="sub small">Birkaç saniye sürebilir.</span>
    {:else}
      <b class="name">Sistem özellikleri</b>
      <span class="sub small">Windows'un kendi kayıtlarından okunur; hiçbir yere gönderilmez.</span>
    {/if}
  </div>
  <div class="acts">
    <button class="btn ghost sm" disabled={busy} onclick={() => load(true)} title="Yeniden oku"><Icon name="refresh" size={14} />{busy ? "Okunuyor…" : "Yenile"}</button>
    <button class="btn sm" disabled={busy || !specs} onclick={copy}><Icon name="discord" size={15} />Discord'a kopyala</button>
  </div>
</section>

{#if error}
  <section class="panel"><p class="err">{error}</p></section>
{/if}

{#if specs}
  <div class="tilebox">
    <div class="tiles">
      <div class="tile">
        <span class="label"><Icon name="cpu" size={15} />İşlemci</span>
        <b class="big">{cpu?.name ?? "—"}</b>
        {#if cpu}
          <span class="meta tnum">
            {#if cpu.cores}<span>{cpu.cores} çekirdek</span>{/if}
            {#if cpu.threads}<span>{cpu.threads} iş parçacığı</span>{/if}
            {#if cpu.mhz}<span>{num(cpu.mhz / 1000, 2)} GHz</span>{/if}
          </span>
        {/if}
      </div>
      <div class="tile">
        <span class="label"><Icon name="gpu" size={15} />Ekran kartı</span>
        <b class="big">{gpu?.name ?? "—"}</b>
        {#if gpu}
          <span class="meta tnum">
            {#if gpu.vram}<span>{mem(gpu.vram)} VRAM</span>{/if}
            {#if gpu.driver}<span>sürücü {driver?.vendor === "nvidia" && driver.installed ? driver.installed : gpu.driver}</span>{/if}
          </span>
        {/if}
        {#each specs.gpu.slice(1) as g (g.name)}
          <span class="extra">+ {g.name}{g.vram ? ` · ${mem(g.vram)}` : ""}</span>
        {/each}
        {#if driver?.newer && driver.latest}
          <button class="drv new" onclick={() => api.openUrl(driver!.url)} title="NVIDIA'nın sürücü sayfası açılır">
            <Icon name="download" size={14} />Yeni sürücü {driver.latest} · İndir
          </button>
        {:else if driver?.latest}
          <span class="drv ok"><Icon name="check" size={14} />Sürücü güncel ({driver.installed})</span>
        {:else if driver?.url}
          <button class="drv" onclick={() => api.openUrl(driver!.url)}>
            {driver.vendor === "amd" ? "AMD" : driver.vendor === "intel" ? "Intel" : "NVIDIA"} sürücü sayfası
          </button>
        {/if}
        {#if driver?.error && driver.vendor === "nvidia"}
          <button class="drv" disabled={driverBusy} onclick={() => loadDriver(true)} title={driver.error}>{driverBusy ? "Bakılıyor…" : "Yeni sürücüye bak"}</button>
        {/if}
      </div>
      <div class="tile">
        <span class="label"><Icon name="ram" size={15} />Bellek</span>
        <b class="big tnum">{specs.ram.total ? mem(specs.ram.total) : "—"} {ramKind}</b>
        {#if ramLine}<span class="meta tnum"><span>{ramLine}</span></span>{/if}
      </div>
      <div class="tile">
        <span class="label"><Icon name="disk" size={15} />Depolama</span>
        <b class="big tnum">{diskTotal ? size(diskTotal) : volTotal ? size(volTotal) : "—"}</b>
        <span class="meta tnum">
          {#if specs.disks.length}<span>{specs.disks.length} disk</span>{/if}
          {#if volTotal}<span>{size(volFree)} boş</span>{/if}
        </span>
      </div>
    </div>
  </div>

  <div class="grid">
    {#if specs.volumes.length || specs.disks.length}
      <section class="panel card">
        <h4><Icon name="disk" size={16} />Sürücüler</h4>
        {#each specs.volumes as v (v.letter)}
          {@const used = v.size ? 1 - v.free / v.size : 0}
          <div class="vol">
            <div class="vline">
              <span><b>{v.letter}</b> {v.label}</span>
              <span class="sub small tnum">{size(v.free)} boş / {size(v.size)}</span>
            </div>
            <div class="bar" role="meter" aria-label="{v.letter} doluluk" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(used * 100)}>
              <span class:full={used > 0.9} style:width="{Math.max(2, used * 100)}%"></span>
            </div>
          </div>
        {/each}
        {#if specs.disks.length}
          <ul class="list">
            {#each specs.disks as d, i (i)}
              <li>
                <span class="nm">{d.name}</span>
                <span class="tags">
                  {#if diskKind(d)}<span class="chip">{diskKind(d)}</span>{/if}
                  <span class="sub small tnum">{size(d.size)}</span>
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}

    {#if specs.monitors.length || gpu?.width}
      <section class="panel card">
        <h4><Icon name="monitor" size={16} />Ekranlar</h4>
        <ul class="list">
          {#each specs.monitors as m, i (i)}
            <li>
              <span class="nm">{monitorTitle(m)}</span>
              <span class="tags tnum">
                {#if m.inches}<span class="chip">{num(m.inches)}″</span>{/if}
                {#if m.year}<span class="sub small">{m.year}</span>{/if}
              </span>
            </li>
          {/each}
        </ul>
        {#if gpu?.width}
          <div class="mode tnum">
            <span class="sub small">Ana ekran</span>
            <b>{gpu.width}×{gpu.height}</b>
            {#if gpu.hz}<span class="chip">{gpu.hz} Hz</span>{/if}
          </div>
        {/if}
      </section>
    {/if}

    {#if peripherals.length}
      <section class="panel card">
        <h4><Icon name="keyboard" size={16} />Çevre birimleri</h4>
        <ul class="list">
          {#each peripherals as p, i (i)}
            <li>
              <span class="kind"><Icon name={p.icon} size={15} />{p.kind}</span>
              <span class="nm right">{p.name}</span>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if modules.length}
      <section class="panel card">
        <h4><Icon name="ram" size={16} />Bellek modülleri</h4>
        <ul class="list">
          {#each modules as m, i (i)}
            <li>
              <span class="nm"><b class="tnum">{mem(m.size)}</b> {ramType(m.type)} <span class="sub small">{m.slot}</span></span>
              <span class="tags tnum">
                {#if m.speed}<span class="chip">{m.speed} MT/s</span>{/if}
                {#if m.maker}<span class="sub small">{m.maker}</span>{/if}
              </span>
            </li>
            {#if m.part}<li class="part"><span class="sub small">{m.part}</span></li>{/if}
          {/each}
        </ul>
      </section>
    {/if}

    {#if specs.board.product || specs.board.bios}
      <section class="panel card">
        <h4><Icon name="board" size={16} />Anakart</h4>
        <dl>
          {#if specs.board.product}<dt>Model</dt><dd>{brand(specs.board.maker)} {specs.board.product}</dd>{/if}
          {#if specs.board.bios}<dt>BIOS</dt><dd class="tnum">{specs.board.bios}{specs.board.biosDate ? ` · ${date(specs.board.biosDate)}` : ""}</dd>{/if}
        </dl>
      </section>
    {/if}

    {#if specs.net.length}
      <section class="panel card">
        <h4><Icon name="network" size={16} />Ağ</h4>
        <ul class="list">
          {#each specs.net as n, i (i)}
            <li>
              <span class="nm">{n.desc || n.name}<span class="sub small"> · {n.name}</span></span>
              {#if n.speed}<span class="chip tnum">{n.speed}</span>{/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  </div>
  <p class="sub small foot">Windows'un kendi kayıtlarından okunur; hiçbir yere gönderilmez.</p>
{/if}

<style>
  .hero { display: flex; align-items: center; gap: 16px; flex-wrap: wrap; }
  .pc {
    display: grid; place-items: center; width: 56px; height: 56px; border-radius: 16px; flex: none;
    color: var(--gold); background: rgba(227, 194, 112, 0.1); border: 1px solid rgba(227, 194, 112, 0.25);
  }
  .who { display: flex; flex-direction: column; gap: 3px; flex: 1 1 260px; min-width: 0; }
  .name { font-family: var(--f-display); font-size: 20px; letter-spacing: 0.01em; }
  .dot { margin: 0 6px; color: var(--dim); }
  .acts { display: flex; gap: 8px; flex-wrap: wrap; }
  .err { color: var(--bad); margin: 0; }

  /* Dört kutu: geniş ekranda yan yana, daralınca 2×2, çok darda alt alta (tek başına kalan olmasın). */
  .tilebox { container-type: inline-size; margin-top: 12px; }
  .tiles { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 12px; }
  @container (max-width: 860px) { .tiles { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  @container (max-width: 420px) { .tiles { grid-template-columns: 1fr; } }
  .tile {
    display: flex; flex-direction: column; gap: 6px; padding: 16px; min-width: 0;
    border-radius: var(--r-lg); border: 1px solid var(--line);
    background: linear-gradient(160deg, rgba(227, 194, 112, 0.08), rgba(34, 37, 42, 0.72) 55%);
  }
  .label {
    display: flex; align-items: center; gap: 6px; color: var(--gold);
    font-family: var(--f-display); font-size: 11px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase;
  }
  .big { font-family: var(--f-display); font-size: 17px; line-height: 1.25; overflow-wrap: anywhere; }
  .meta { display: flex; flex-wrap: wrap; gap: 4px 10px; font-size: 12.5px; color: var(--muted); }
  .meta span:not(:last-child)::after { content: "·"; margin-left: 10px; color: var(--dim); }
  .extra { font-size: 12px; color: var(--dim); overflow-wrap: anywhere; }
  .drv {
    align-self: flex-start; display: inline-flex; align-items: center; gap: 6px; margin-top: 2px; padding: 4px 10px; border-radius: 999px;
    font-size: 12px; font-weight: 600; color: var(--muted); background: transparent; border: 1px solid var(--line-2);
  }
  button.drv:hover { color: var(--text); border-color: var(--gold-dim); }
  .drv.ok { color: var(--good); border-color: rgba(92, 192, 138, 0.4); }
  .drv.new { color: #1b1608; background: var(--gold); border-color: var(--gold); }
  .drv.new:hover { background: var(--gold-glow); color: #1b1608; }

  /* Kartların boyu farklı: sütunlara akıt, boşluk kalmasın. */
  .grid { columns: 340px; column-gap: 12px; margin-top: 12px; }
  .card { display: flex; flex-direction: column; gap: 10px; break-inside: avoid; margin-bottom: 12px; }
  h4 { display: flex; align-items: center; gap: 8px; margin: 0; font-size: 14px; color: var(--gold); }

  .vol { display: flex; flex-direction: column; gap: 5px; }
  .vline { display: flex; justify-content: space-between; gap: 10px; font-size: 13px; }
  .bar { height: 8px; border-radius: 99px; background: var(--coal-2); border: 1px solid var(--line); overflow: hidden; }
  .bar span { display: block; height: 100%; border-radius: 99px; background: var(--gold); }
  .bar span.full { background: var(--bad); }

  .list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  .list li {
    display: flex; justify-content: space-between; align-items: center; gap: 10px;
    padding: 7px 0; border-top: 1px solid rgba(68, 74, 81, 0.5); font-size: 13px;
  }
  .list li:first-child { border-top: 0; }
  .list li.part { border-top: 0; padding-top: 0; }
  .nm { min-width: 0; overflow-wrap: anywhere; }
  .nm.right { text-align: right; }
  .tags { display: flex; align-items: center; gap: 8px; flex: none; }
  .kind { display: inline-flex; align-items: center; gap: 6px; color: var(--muted); flex: none; }
  .mode { display: flex; align-items: center; gap: 10px; padding-top: 8px; border-top: 1px solid var(--line); font-size: 14px; }
  dl { margin: 0; display: grid; grid-template-columns: max-content 1fr; gap: 6px 14px; font-size: 13px; }
  dt { color: var(--dim); }
  dd { margin: 0; overflow-wrap: anywhere; }
  .foot { margin-top: 0; color: var(--dim); }
</style>
