<script lang="ts">
  // Program Kaldırıcı: programın kendi kaldırıcısı, sonra katı kurallarla artık taraması ve geri alınabilir silme.
  import { onDestroy, onMount } from "svelte";
  import { api, formatSize } from "../api";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";
  import type { CleanResult, InstalledProgram, Leftover, LeftoverScan } from "../types";

  let programs = $state<InstalledProgram[]>([]);
  let loading = $state(true);
  let search = $state("");
  let sort = $state<"name" | "size" | "date">("name");
  let showRules = $state(false);

  type Step = "confirm" | "running" | "stuck" | "scanning" | "results" | "cleaning" | "done";
  let target = $state<InstalledProgram | null>(null);
  let step = $state<Step>("confirm");
  let scan = $state<LeftoverScan | null>(null);
  let picked = $state<Record<string, boolean>>({});
  let result = $state<CleanResult | null>(null);
  let error = $state("");
  let showSkipped = $state(false);
  /** Program simgeleri (kimlik → PNG); liste geldikten sonra arka planda dolar. */
  let icons = $state<Record<string, string>>({});
  let poll: ReturnType<typeof setInterval> | undefined;

  async function load() {
    loading = true;
    programs = await api.uninstall.list().catch(() => []);
    loading = false;
    icons = await api.uninstall.icons().catch(() => icons);
  }
  onMount(() => void load());
  onDestroy(() => clearInterval(poll));

  const shown = $derived.by(() => {
    const q = search.trim().toLocaleLowerCase("tr-TR");
    const l = programs.filter((p) => !q || `${p.name} ${p.publisher}`.toLocaleLowerCase("tr-TR").includes(q));
    if (sort === "size") return [...l].sort((a, b) => b.sizeKb - a.sizeKb);
    if (sort === "date") return [...l].sort((a, b) => (b.date || "").localeCompare(a.date || ""));
    return l;
  });
  const fmtDate = (d: string) => (d.length === 8 ? `${d.slice(6)}.${d.slice(4, 6)}.${d.slice(0, 4)}` : "");
  const key = (c: Leftover) => `${c.kind}|${c.path}|${c.value ?? ""}`;

  function open(p: InstalledProgram) {
    target = p;
    step = "confirm";
    scan = null;
    result = null;
    error = "";
    showSkipped = false;
  }
  function close() {
    clearInterval(poll);
    target = null;
    if (result) void load();
  }

  async function start() {
    if (!target) return;
    error = "";
    try {
      await api.uninstall.run(target.id);
    } catch (e) {
      error = String(e);
      return;
    }
    step = "running";
    const id = target.id;
    clearInterval(poll);
    poll = setInterval(async () => {
      const st = await api.uninstall.state(id).catch(() => null);
      if (!st || step !== "running") return;
      if (st.removed) {
        clearInterval(poll);
        void doScan();
      } else if (!st.running) {
        // Kaldırıcı kapandı ama program duruyor: bazı kaldırıcılar kendini kopyalayıp öyle çalışır; biraz daha bekle.
        setTimeout(async () => {
          const again = await api.uninstall.state(id).catch(() => null);
          if (step !== "running") return;
          if (again?.removed) {
            clearInterval(poll);
            void doScan();
          } else if (again && !again.running) step = "stuck";
        }, 6000);
      }
    }, 1500);
  }

  async function doScan() {
    if (!target) return;
    clearInterval(poll);
    step = "scanning";
    error = "";
    try {
      scan = await api.uninstall.scan(target.id);
      picked = Object.fromEntries(scan.items.map((c) => [key(c), c.strong]));
      step = scan.removed ? "results" : "stuck";
    } catch (e) {
      error = String(e);
      step = "stuck";
    }
  }

  const chosen = $derived(scan ? scan.items.filter((c) => picked[key(c)]) : []);
  const chosenSize = $derived(chosen.reduce((s, c) => s + c.size, 0));
  const groups = $derived(
    scan
      ? [
          { title: "Klasörler ve dosyalar", icon: "folder" as const, items: scan.items.filter((c) => c.kind === "dir" || c.kind === "file") },
          { title: "Kayıt defteri", icon: "list" as const, items: scan.items.filter((c) => c.kind === "key" || c.kind === "value") },
        ].filter((g) => g.items.length)
      : [],
  );

  async function clean() {
    if (!target || !chosen.length) return;
    step = "cleaning";
    error = "";
    try {
      result = await api.uninstall.clean(target.id, chosen.map(key));
      step = "done";
      notify(`${result.done.length} artık temizlendi`, "check");
    } catch (e) {
      error = String(e);
      step = "results";
    }
  }
</script>

<div class="un">
  <section class="panel head">
    <div class="spread">
      <div>
        <h3 class="h2"><Icon name="trash" />Program Kaldırıcı</h3>
        <p class="sub small">
          Önce programın kendi kaldırıcısı açılır; bitince geride kalan klasörler ve kayıt defteri girdileri <b>katı kurallarla</b>
          bulunur. Sen seçip onaylarsın: dosyalar <b>Geri Dönüşüm Kutusu</b>'na gider, kayıtlar silinmeden önce <b>yedeklenir</b>.
        </p>
      </div>
      <button class="btn ghost sm" onclick={() => (showRules = !showRules)}><Icon name="shield" size={15} />Nasıl karar veriyoruz?</button>
    </div>
    {#if showRules}
      <ul class="rules">
        <li><b>Ad birebir tutmalı.</b> Klasör ya da anahtarın adı programın adıyla (sürüm ve bit ekleri atılarak) tam aynı olmalı. Benzerlik yetmez: "Discord" ≠ "DiscordPTB".</li>
        <li><b>Yalnız bilinen yerler.</b> Program Files, ProgramData, AppData ve Başlat menüsünün doğrudan alt klasörleri ya da <code>Yayıncı\Program</code>; masaüstünde yalnız kısayol. Daha derine inilmez.</li>
        <li><b>Windows'a asla dokunulmaz.</b> Windows, Microsoft, Common Files, WindowsApps, System32 klasörleri, sürücü kökleri ve ana klasörlerin kendisi hiç aday olamaz.</li>
        <li><b>Başkasına aitse dokunulmaz.</b> İçinde hâlâ kurulu bir programın dosyası, ortak kullanılan bir DLL ya da şu an çalışan bir program varsa atlanır.</li>
        <li><b>Bağlantılar (junction/symlink) atlanır.</b> Hedefi başka bir yerde olabilir.</li>
        <li><b>Genel adlar eşleşmez.</b> "Update", "Common", "Google" gibi birçok programın paylaştığı adlar ve 4 harften kısa adlar.</li>
        <li><b>Kayıt defterinde</b> yalnız <code>Software\Program</code>, <code>Software\Yayıncı\Program</code>, programın kaldırma kaydı ve başlangıçta açılma kaydı; Microsoft, Classes, servisler ve COM kayıtlarına hiç bakılmaz.</li>
        <li><b>Geri alınabilir.</b> Dosyalar Geri Dönüşüm Kutusu'nda, kayıtlar Belgeler › GMID Tool › Yedekler'de <code>.reg</code> olarak (çift tıkla: geri gelir).</li>
      </ul>
    {/if}
  </section>

  <section class="panel">
    <div class="bar">
      <input class="search" type="search" placeholder="Program ara…" bind:value={search} aria-label="Program ara" />
      <select bind:value={sort} aria-label="Sırala">
        <option value="name">Ada göre</option>
        <option value="size">Boyuta göre</option>
        <option value="date">Kurulma tarihine göre</option>
      </select>
      <span class="sub small tnum">{shown.length} program</span>
      <button class="btn ghost sm" onclick={load} disabled={loading}><Icon name="refresh" size={15} />Yenile</button>
    </div>
    {#if loading}
      <p class="sub">Kurulu programlar okunuyor…</p>
    {:else}
      <ul class="plist">
        {#each shown as p (p.id)}
          <li>
            {@render picon(p, 32)}
            <div class="pt">
              <b title={p.name}>{p.name}</b>
              <span class="sub small">{[p.publisher, p.version].filter(Boolean).join(" · ")}</span>
            </div>
            <span class="sub small tnum sz">{p.sizeKb ? formatSize(p.sizeKb * 1024) : ""}</span>
            <span class="sub small tnum dt">{fmtDate(p.date)}</span>
            <button class="btn ghost sm rm" onclick={() => open(p)}><Icon name="trash" size={15} />Kaldır</button>
          </li>
        {:else}
          <p class="sub">Program bulunamadı.</p>
        {/each}
      </ul>
    {/if}
  </section>
</div>

{#snippet picon(p: InstalledProgram, size: number)}
  {#if icons[p.id]}
    <img class="picon" src={icons[p.id]} alt="" width={size} height={size} />
  {:else}
    <span class="picon ph" style:width="{size}px" style:height="{size}px" style:font-size="{size * 0.45}px" aria-hidden="true">{(p.name.trim()[0] ?? "?").toLocaleUpperCase("tr-TR")}</span>
  {/if}
{/snippet}

{#if target}
  <div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && (step === "confirm" || step === "done" || step === "results" || step === "stuck") && close()}>
    <div class="dlg" role="dialog" aria-label="Program kaldır">
      <h3>{@render picon(target, 24)}{target.name}</h3>

      {#if step === "confirm"}
        <p>
          Programın kendi kaldırıcısı açılacak; adımları orada tamamla (Windows izin isteyebilir). Bitince GMID geride
          kalanları tarar ve sana gösterir; sen onaylamadan hiçbir şey silinmez.
        </p>
        <div class="row">
          <button class="btn danger" onclick={start}><Icon name="trash" size={16} />Kaldırıcıyı aç</button>
          <button class="btn ghost" onclick={close}>Vazgeç</button>
        </div>
      {:else if step === "running"}
        <div class="wait"><span class="spin"></span>Kaldırıcı açık; adımları orada tamamla. Bitince artıklar kendiliğinden taranır.</div>
        <button class="link" onclick={doScan}>Kaldırma bitti, artıkları şimdi tara</button>
      {:else if step === "stuck"}
        <p class="warn">
          Program hâlâ kurulu görünüyor (kaldırma iptal edilmiş ya da kaldırıcı hata vermiş olabilir). Kurulu bir programın
          dosyalarına dokunulmaz; önce programı kaldırman gerekiyor.
        </p>
        <div class="row">
          <button class="btn" onclick={start}><Icon name="refresh" size={16} />Kaldırıcıyı yeniden aç</button>
          <button class="btn ghost" onclick={doScan}>Yeniden kontrol et</button>
          <button class="btn ghost" onclick={close}>Kapat</button>
        </div>
      {:else if step === "scanning"}
        <div class="wait"><span class="spin"></span>Artıklar taranıyor…</div>
      {:else if step === "results" && scan}
        {#if !scan.items.length}
          <p class="ok"><Icon name="check" size={16} />Temiz: geride bu programa ait bir şey kalmamış.</p>
        {:else}
          <p class="sub small">Yalnız bu programa ait olduğu kesin olanlar seçili geldi. Emin olmadıklarına bak, istemediğini kaldır.</p>
          {#each groups as g (g.title)}
            <div class="grp">
              <span class="gl"><Icon name={g.icon} size={14} />{g.title}</span>
              {#each g.items as c (key(c))}
                <label class="item" class:weak={!c.strong}>
                  <input type="checkbox" bind:checked={picked[key(c)]} />
                  <span class="it">
                    <span class="ip" title={c.path}>{c.path}{c.value ? ` → ${c.value}` : ""}</span>
                    <span class="sub small">{c.reason}{c.size ? ` · ${formatSize(c.size)}` : ""}</span>
                  </span>
                </label>
              {/each}
            </div>
          {/each}
        {/if}
        {#if scan.skipped.length}
          <button class="link" onclick={() => (showSkipped = !showSkipped)}>
            <Icon name="shield" size={13} />Güvenlik için dokunulmayanlar ({scan.skipped.length})
          </button>
          {#if showSkipped}
            <ul class="skipped">
              {#each scan.skipped as s (s.path)}<li><span class="ip">{s.path}</span><span class="sub small">{s.reason}</span></li>{/each}
            </ul>
          {/if}
        {/if}
        {#if error}<p class="err">{error}</p>{/if}
        <div class="row">
          {#if scan.items.length}
            <button class="btn danger" disabled={!chosen.length} onclick={clean}>
              <Icon name="trash" size={16} />Seçilenleri sil ({chosen.length}{chosenSize ? ` · ${formatSize(chosenSize)}` : ""})
            </button>
          {/if}
          <button class="btn ghost" onclick={close}>{scan.items.length ? "Hiçbirini silme" : "Kapat"}</button>
        </div>
      {:else if step === "cleaning"}
        <div class="wait"><span class="spin"></span>Siliniyor… (Program Files ya da HKLM için Windows izin isteyebilir)</div>
      {:else if step === "done" && result}
        <p class="ok"><Icon name="check" size={16} />{result.done.length} artık temizlendi. Dosyalar Geri Dönüşüm Kutusu'nda, kayıtların yedeği alındı.</p>
        {#if result.failed.length}
          <p class="warn">Silinemeyenler ({result.failed.length}):</p>
          <ul class="skipped">
            {#each result.failed as [p, e] (p)}<li><span class="ip">{p}</span><span class="sub small">{e}</span></li>{/each}
          </ul>
        {/if}
        <div class="row">
          <button class="btn ghost" onclick={() => api.openDataFolder("Yedekler")}><Icon name="folder" size={16} />Kayıt yedekleri</button>
          <button class="btn" onclick={close}>Tamam</button>
        </div>
      {/if}
      {#if error && step !== "results"}<p class="err">{error}</p>{/if}
    </div>
  </div>
{/if}

<style>
  .un { display: flex; flex-direction: column; gap: 16px; max-width: 1200px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .head p { margin: 6px 0 0; max-width: 760px; line-height: 1.5; }
  .rules { margin: 0; padding: 12px 14px 12px 30px; border-radius: 12px; background: rgba(0, 0, 0, 0.2); border: 1px solid var(--line); display: flex; flex-direction: column; gap: 6px; font-size: 13px; color: var(--muted); line-height: 1.5; }
  .rules b { color: var(--text); }
  code { font-size: 12px; background: rgba(255, 255, 255, 0.06); padding: 0 4px; border-radius: 4px; }
  .sm { padding: 6px 11px; font-size: 13px; }
  .bar { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
  .search { flex: 1; min-width: 200px; background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 8px 12px; color: var(--text); }
  .plist { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 2px; }
  .plist li { display: grid; grid-template-columns: 32px minmax(0, 1fr) 90px 90px auto; align-items: center; gap: 12px; padding: 8px 10px; border-radius: 10px; }
  .plist li:hover { background: rgba(255, 255, 255, 0.04); }
  .picon { flex: none; display: block; object-fit: contain; image-rendering: auto; }
  .picon.ph { display: grid; place-items: center; border-radius: 8px; background: rgba(255, 255, 255, 0.06); color: var(--muted); font-weight: 700; }
  .pt { display: flex; flex-direction: column; min-width: 0; }
  .pt b { font-weight: 600; font-size: 14px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pt .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sz, .dt { text-align: right; }
  .rm { opacity: 0.55; transition: opacity 0.12s; }
  .plist li:hover .rm, .rm:focus-visible { opacity: 1; }
  .back { position: fixed; inset: 0; z-index: 50; display: grid; place-items: center; padding: 20px; background: rgba(0, 0, 0, 0.55); }
  .dlg { width: min(760px, 100%); max-height: calc(100vh - 40px); overflow: auto; display: flex; flex-direction: column; gap: 14px; padding: 22px; border-radius: 18px; border: 1px solid var(--line-2); background: var(--coal-2); box-shadow: 0 30px 70px rgba(0, 0, 0, 0.6); }
  .dlg h3 { margin: 0; display: flex; align-items: center; gap: 8px; font-family: var(--f-display); }
  .dlg p { margin: 0; line-height: 1.5; }
  .row { display: flex; flex-wrap: wrap; gap: 8px; }
  .btn.danger { background: rgba(224, 106, 106, 0.16); border-color: rgba(224, 106, 106, 0.6); color: #ffd6d6; }
  .btn.danger:hover:not(:disabled) { background: rgba(224, 106, 106, 0.28); }
  .wait { display: flex; align-items: center; gap: 10px; color: var(--muted); }
  .spin { width: 16px; height: 16px; border-radius: 50%; border: 2px solid var(--line-2); border-top-color: var(--gold); animation: spin 0.8s linear infinite; flex: none; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .link { background: none; border: 0; padding: 0; color: var(--gold); font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; display: inline-flex; align-items: center; gap: 6px; align-self: flex-start; }
  .link:hover { color: var(--gold-glow); }
  .ok { display: flex; align-items: center; gap: 8px; color: var(--good); }
  .warn { color: var(--gold); }
  .err { color: var(--bad); }
  .grp { display: flex; flex-direction: column; gap: 4px; }
  .gl { display: inline-flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--dim); }
  .item { display: flex; align-items: flex-start; gap: 10px; padding: 8px 10px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); cursor: pointer; }
  .item input { margin-top: 3px; accent-color: var(--gold); }
  .item.weak { border: 1px dashed rgba(227, 194, 112, 0.35); }
  .it { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .ip { font-family: ui-monospace, Consolas, monospace; font-size: 12.5px; word-break: break-all; color: var(--text); }
  .skipped { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .skipped li { display: flex; flex-direction: column; gap: 2px; padding: 6px 10px; border-radius: 8px; background: rgba(255, 255, 255, 0.025); }
  @media (max-width: 760px) {
    .plist li { grid-template-columns: 32px minmax(0, 1fr) auto; }
    .sz, .dt { display: none; }
  }
</style>
