<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import Switch from "../components/Switch.svelte";
  import type { GameInfo, Server, ServerInput, ServerStatus, ServerWatch } from "../types";

  /** Ekran açıkken sunucular bu aralıkla yeniden sorulur. */
  const POLL_MS = 30_000;
  const LIST_URL_KEY = "servers.listUrl";

  const MODES: Record<string, string> = {
    dm: "Herkes tek", ffa: "Herkes tek", tdm: "Takım ölüm maçı", war: "Takım ölüm maçı", sd: "Arama & İmha",
    ctf: "Bayrak kapmaca", hq: "Karargâh", sab: "Sabotaj", dom: "Hakimiyet", koth: "Tepenin hâkimi", re: "Kurtarma",
  };
  /** Logo yoksa kutucuktaki rozetin rengi. */
  const COLORS: Record<string, string> = {
    cod2: "#c9a35a", cod4: "#8fa35a", coduo: "#b07a4a", cod1: "#a08a6a", minecraft: "#5ca04a",
    "minecraft-bedrock": "#4a9a7a", cs16: "#d08a3a", cs2: "#d0a03a", source: "#6a8ac0", valheim: "#6a90a0",
    samp: "#c05a5a", mta: "#a05ac0",
  };

  type Filter = "all" | "community" | "mine" | "online";

  let games: GameInfo[] = $state([]);
  let servers: Server[] = $state([]);
  let statuses: Record<string, ServerStatus> = $state({});
  let filter: Filter = $state("all");
  let busy = $state(false);
  let cooldown = $state(false);
  let syncError: string | null = $state(null);
  let listUrl = $state("");
  let defaultUrl = $state("");
  let open: Record<string, "how" | "players" | "notify" | null> = $state({});
  let watch: Record<string, ServerWatch> = $state({});

  function setWatch(id: string, rule: ServerWatch) {
    if (rule.online || rule.minPlayers) watch = { ...watch, [id]: rule };
    else {
      const { [id]: _, ...rest } = watch;
      watch = rest;
    }
    void api.serverWatch.set(id, rule).catch((e) => notify(String(e), "info"));
  }
  let now = $state(Date.now());

  // ekle / düzenle
  let form: ServerInput | null = $state(null);
  let saving = $state(false);
  // "oyunun .exe'sini göster" penceresi
  let needExe: { server: Server; game: GameInfo } | null = $state(null);
  // topluluk listesi adresi
  let editingUrl = $state(false);
  let urlDraft = $state("");

  const gameOf = (id: string) => games.find((g) => g.id === id);
  const counts = $derived({
    all: servers.length,
    community: servers.filter((s) => s.remote).length,
    mine: servers.filter((s) => !s.remote).length,
    online: servers.filter((s) => statuses[s.id]?.state === "online").length,
  });
  const shown = $derived.by(() => {
    if (filter === "community") return servers.filter((s) => s.remote);
    if (filter === "mine") return servers.filter((s) => !s.remote);
    if (filter === "online")
      return servers
        .filter((s) => statuses[s.id]?.state === "online")
        .sort((a, b) => (statuses[b.id]?.info?.players ?? 0) - (statuses[a.id]?.info?.players ?? 0));
    return servers;
  });
  const playing = $derived(
    servers.reduce((n, s) => n + (statuses[s.id]?.state === "online" ? statuses[s.id]?.info?.players ?? 0 : 0), 0),
  );
  const lastCheck = $derived(Math.max(0, ...Object.values(statuses).map((s) => s.checkedAt)));

  function mergeStatuses(list: ServerStatus[]) {
    const next = { ...statuses };
    for (const s of list) next[s.id] = s;
    statuses = next;
  }

  async function query(ids: string[] = []) {
    if (busy && !ids.length) return;
    if (!ids.length) busy = true;
    try {
      mergeStatuses(await api.servers.query(ids));
    } catch (e) {
      console.error(e);
    } finally {
      if (!ids.length) busy = false;
    }
  }

  async function sync(force: boolean) {
    try {
      const r = await api.servers.sync(force);
      servers = r.servers;
      syncError = r.error;
      listUrl = r.url;
      defaultUrl = r.defaultUrl;
    } catch (e) {
      syncError = String(e);
    }
  }

  async function refresh() {
    cooldown = true;
    setTimeout(() => (cooldown = false), 5000);
    busy = true;
    await sync(true);
    busy = false;
    await query();
  }

  onMount(() => {
    let alive = true;
    (async () => {
      [games, servers] = await Promise.all([api.servers.games(), api.servers.list()]);
      watch = await api.serverWatch.rules().catch(() => ({}));
      mergeStatuses(await api.servers.cached());
      busy = true;
      await sync(false);
      busy = false;
      if (alive) await query();
    })();
    const poll = setInterval(() => {
      now = Date.now();
      if (document.visibilityState === "visible") void query();
    }, POLL_MS);
    return () => {
      alive = false;
      clearInterval(poll);
    };
  });

  // ---- yardımcılar ----

  function prettyMap(map: string): string {
    if (!map) return "";
    const m = map.replace(/^mp_/i, "");
    return m === map ? map : m.split("_").map((w) => w.charAt(0).toLocaleUpperCase("tr-TR") + w.slice(1)).join(" ");
  }

  function modeLabel(mode: string, game: string): string {
    if (game.startsWith("cod")) return MODES[mode.toLowerCase()] ?? mode;
    return mode;
  }

  /** Steam sunucuları "mod" yerine çoğu zaman oyunun adını yazar; tekrar etmesin. */
  function sameAsGame(mode: string, g: GameInfo | undefined): boolean {
    const m = mode.toLocaleLowerCase("tr-TR");
    return !!g && (m === g.id || g.name.toLocaleLowerCase("tr-TR").includes(m));
  }

  function fill(text: string, s: Server): string {
    return text.replaceAll("{host}", s.host).replaceAll("{port}", String(s.port));
  }

  function pingClass(ms: number): string {
    return ms < 60 ? "good" : ms < 120 ? "ok" : "slow";
  }

  function toggle(id: string, what: "how" | "players" | "notify") {
    open = { ...open, [id]: open[id] === what ? null : what };
  }

  async function copy(text: string, extra = "") {
    try {
      await navigator.clipboard.writeText(text);
      notify(`Kopyalandı: ${text}${extra}`, "copy");
    } catch {
      notify(`Adres: ${text}`, "info");
    }
  }

  async function join(s: Server) {
    try {
      const r = await api.servers.join(s.id);
      if (r.kind === "copy") {
        const g = gameOf(s.game);
        await copy(r.address, g ? ` · ${g.name}'ı aç ve sunucu ekle'ye yapıştır` : "");
      } else {
        notify(`${gameOf(s.game)?.name ?? "Oyun"} açılıyor…`, "play");
      }
    } catch (e) {
      const msg = String(e);
      if (msg.startsWith("EXE_NEEDED:")) {
        const game = gameOf(msg.slice(11));
        if (game) needExe = { server: s, game };
      } else {
        notify(msg, "info");
      }
    }
  }

  async function pickExe() {
    if (!needExe) return;
    const { server, game } = needExe;
    const path = await api.servers.pickExe(game.name, game.exe ?? ".exe");
    if (!path) return;
    try {
      await api.servers.setExe(game.id, path);
      needExe = null;
      await join(server);
    } catch (e) {
      notify(String(e), "info");
    }
  }

  // ---- ekle / düzenle ----

  function openAdd() {
    form = { id: null, name: "", game: "cod2", host: "", port: null, queryPort: null, logo: null, description: "", connectNotes: "", password: "" };
  }

  function openEdit(s: Server) {
    form = {
      id: s.id, name: s.name, game: s.game, host: s.host, port: s.port, queryPort: s.queryPort, logo: s.logo,
      description: s.description, connectNotes: s.connectNotes, password: s.password,
    };
  }

  async function pickLogo() {
    if (!form) return;
    try {
      const path = await api.servers.pickLogo();
      if (path) form.logo = path;
      else if (!("__TAURI_INTERNALS__" in window)) notify("Logo seçimi uygulamada çalışır.", "info");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  /** Sunucunun verdiği satır yapıştırılınca (connect 1.2.3.4:27015; password x · steam://connect/…) alanlar dolar. */
  function onHostInput() {
    if (!form) return;
    const t = form.host.trim();
    let m = t.match(/steam:\/\/connect\/([^/\s]+)(?:\/([^\s]+))?/i);
    if (!m) m = t.match(/^\/?connect\s+([^;\s]+)\s*(?:;\s*password\s+"?([^";]+)"?)?/i);
    if (!m) return;
    form.host = m[1];
    if (m[2]) form.password = decodeURIComponent(m[2].trim());
    const steam = ["cs2", "cs16", "source"];
    if (/steam:|connect/i.test(t) && !steam.includes(form.game) && !form.game.startsWith("cod")) form.game = "cs2";
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (!form) return;
    saving = true;
    try {
      const port = form.port ? Number(form.port) : null;
      const s = await api.servers.save({ ...form, port });
      notify(form.id ? "Sunucu güncellendi" : "Sunucu eklendi", "check");
      form = null;
      servers = await api.servers.list();
      await query([s.id]);
    } catch (err) {
      notify(String(err), "info");
    } finally {
      saving = false;
    }
  }

  /** Silme iki tıklamayla: ilk tıklama düğmeyi "Sil?" yapar. */
  let armed: string | null = $state(null);
  let disarm: ReturnType<typeof setTimeout> | undefined;

  async function remove(s: Server) {
    if (armed !== s.id) {
      armed = s.id;
      clearTimeout(disarm);
      disarm = setTimeout(() => (armed = null), 3000);
      return;
    }
    armed = null;
    try {
      await api.servers.remove(s.id);
      servers = servers.filter((x) => x.id !== s.id);
      notify("Sunucu silindi", "trash");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  // ---- topluluk listesi ----

  async function saveUrl(url: string) {
    await api.setSetting(LIST_URL_KEY, url.trim());
    editingUrl = false;
    busy = true;
    await sync(true);
    busy = false;
    await query();
    if (!syncError) notify(url.trim() ? "Topluluk listesi güncellendi" : "Topluluk listesi kapatıldı", "check");
  }
</script>

<div class="srv">
  <div class="top">
    <div class="summary">
      <p class="sub">
        {#if counts.online}
          Şu an <b class="tnum">{playing}</b> kişi oyunda · <b class="tnum">{counts.online}/{counts.all}</b> sunucu açık
        {:else if busy}
          Sunucular kontrol ediliyor…
        {:else}
          GMID topluluğunun ve senin eklediğin sunucular.
        {/if}
      </p>
      {#if lastCheck}<span class="sub small">Son kontrol {ago(lastCheck, now)} · her 30 saniyede yenilenir</span>{/if}
    </div>
    <div class="row">
      <button class="btn ghost" onclick={refresh} disabled={busy || cooldown}>
        <span class:spin={busy}><Icon name="refresh" size={18} /></span>Yenile
      </button>
      <button class="btn" onclick={openAdd}><Icon name="plus" size={18} />Sunucu ekle</button>
    </div>
  </div>

  <div class="seg" role="group" aria-label="Süzgeç">
    <button aria-pressed={filter === "all"} onclick={() => (filter = "all")}>Tümü <span class="n">{counts.all}</span></button>
    <button aria-pressed={filter === "community"} onclick={() => (filter = "community")}>GMID <span class="n">{counts.community}</span></button>
    <button aria-pressed={filter === "mine"} onclick={() => (filter = "mine")}>Benim eklediklerim <span class="n">{counts.mine}</span></button>
    <button aria-pressed={filter === "online"} onclick={() => (filter = "online")}>Şu an açık <span class="n">{counts.online}</span></button>
  </div>

  {#if syncError}
    <div class="warn"><Icon name="info" size={18} />Topluluk listesi güncellenemedi: {syncError}. Eldeki liste gösteriliyor.</div>
  {/if}

  <div class="grid">
    {#each shown as s (s.id)}
      {@const g = gameOf(s.game)}
      {@const st = statuses[s.id]}
      {@const info = st?.state === "online" ? st.info : null}
      {@const state = st?.state ?? "checking"}
      <article class="tile" class:off={state === "offline"}>
        <header>
          <div class="logo" style:--c={COLORS[s.game] ?? "#7a8088"}>
            {#if s.logo}
              <img src={api.fileSrc(s.logo)} alt="" />
            {:else if info?.favicon}
              <img src={info.favicon} alt="" class="pixel" />
            {:else}
              <span>{g?.short ?? "?"}</span>
            {/if}
          </div>
          <div class="title">
            <b title={s.name}>{s.name}</b>
            <span class="sub small">{g?.name ?? s.game}{#if s.remote}<span class="gmid">GMID</span>{/if}</span>
          </div>
          <span class="pill {state}" class:stale={st?.stale} title={st?.stale ? "Son kontrolde yanıt gelmedi; bir önceki bilgi gösteriliyor" : ""}>
            {state === "online" ? "Açık" : state === "offline" ? "Kapalı" : state === "unknown" ? "Bilinmiyor" : "Bakılıyor"}
          </span>
        </header>

        {#if s.description}<p class="desc">{s.description}</p>{/if}

        <div class="stats">
          {#if info}
            <div class="players">
              <Icon name="users" size={17} />
              <span class="tnum"><b>{info.players}</b> / {info.maxPlayers || "?"}</span>
              <div class="bar"><i style:width="{info.maxPlayers ? Math.min(100, (info.players / info.maxPlayers) * 100) : 0}%"></i></div>
              {#if info.password}<span class="lock" title="Şifreli sunucu"><Icon name="lock" size={15} /></span>{/if}
            </div>
            <div class="meta">
              {#if info.map}<span>{prettyMap(info.map)}</span>{/if}
              {#if info.mode && !sameAsGame(info.mode, g)}<span>{modeLabel(info.mode, s.game)}</span>{/if}
              {#if info.version}<span>{info.version}</span>{/if}
              {#if info.pingMs}<span class="ping {pingClass(info.pingMs)} tnum">{info.pingMs} ms</span>{/if}
            </div>
          {:else if state === "offline"}
            <span class="sub small">{st?.error ?? "Yanıt vermedi"} · sunucu kapalı olabilir</span>
          {:else if state === "unknown"}
            <span class="sub small">Bu oyunun sunucuları sorgulanamıyor; adres ve bağlanma bilgisi aşağıda.</span>
          {:else}
            <span class="sub small">Kontrol ediliyor…</span>
          {/if}
        </div>

        <div class="addr">
          <code>{s.host}:{s.port}</code>
          <button class="mini" title="Adresi kopyala" onclick={() => copy(`${s.host}:${s.port}`)}><Icon name="copy" size={16} /></button>
          {#if !s.remote}
            <button class="mini" title="Düzenle" onclick={() => openEdit(s)}><Icon name="edit" size={16} /></button>
            <button class="mini" class:armed={armed === s.id} title="Sil" onclick={() => remove(s)}>
              <Icon name="trash" size={16} />{#if armed === s.id}<span>Sil?</span>{/if}
            </button>
          {/if}
        </div>

        <div class="acts">
          <button class="btn" onclick={() => join(s)} disabled={state === "offline" && !st?.stale && g?.join === "copy"}>
            <Icon name={g?.join === "copy" ? "copy" : "play"} size={17} />{g?.join === "copy" ? "Adresi kopyala" : "Oyna"}
          </button>
          <button class="btn ghost sm" aria-expanded={open[s.id] === "how"} onclick={() => toggle(s.id, "how")}>Nasıl bağlanırım?</button>
          {#if state !== "unknown"}
            <button class="btn ghost sm bell" class:on={!!watch[s.id]} title="Bildirim: açılınca ya da dolunca haber ver" aria-expanded={open[s.id] === "notify"} onclick={() => toggle(s.id, "notify")}>
              <Icon name="bell" size={16} />
            </button>
          {/if}
          {#if info?.playerList.length}
            <button class="btn ghost sm" title="Oyuncular" aria-expanded={open[s.id] === "players"} onclick={() => toggle(s.id, "players")}>
              <Icon name="users" size={16} /><span class="tnum">{info.playerList.length}</span>
            </button>
          {/if}
        </div>

        {#if open[s.id] === "notify"}
          {@const w = watch[s.id] ?? { online: false, minPlayers: null }}
          <div class="drawer notify">
            <div class="nl">
              <span>Sunucu açılınca haber ver</span>
              <Switch label="Açılınca haber ver" checked={w.online} onchange={(v) => setWatch(s.id, { ...w, online: v })} />
            </div>
            <div class="nl">
              <span>Şu kadar kişi olunca haber ver</span>
              <select value={w.minPlayers ?? 0} onchange={(e) => setWatch(s.id, { ...w, minPlayers: Number(e.currentTarget.value) || null })} aria-label="Kişi sayısı">
                <option value={0}>Kapalı</option>
                {#each [2, 3, 4, 5, 6, 8, 10, 12, 16, 20] as n (n)}<option value={n}>en az {n} kişi</option>{/each}
              </select>
            </div>
            <p class="sub small">2 dakikada bir bakılır; aynı şey için tekrar tekrar bildirim gelmez. Oyun Modu sessizken gelmez.</p>
          </div>
        {:else if open[s.id] === "how"}
          <div class="drawer">
            <p>{fill(g?.howTo ?? "Adres: {host}:{port}", s)}</p>
            {#if g?.join === "exe"}
              <p class="sub small">Oyna'ya ilk bastığında oyunun yerini bir kez göstermen gerekebilir; sonra tek tıkla girer.</p>
            {/if}
            {#if s.connectNotes}<p class="note"><Icon name="info" size={15} />{s.connectNotes}</p>{/if}
          </div>
        {:else if open[s.id] === "players" && info}
          <div class="drawer">
            <ul class="plist">
              {#each info.playerList as p, i (i)}
                <li><span class="pn">{p.name}</span>
                  {#if p.score !== null}<span class="tnum">{p.score}</span>{/if}
                  {#if p.ping}<span class="tnum dim">{p.ping} ms</span>{/if}
                </li>
              {/each}
            </ul>
            {#if info.playerList.length < info.players}
              <p class="sub small">Sunucu sadece {info.playerList.length} kişinin adını paylaşıyor.</p>
            {/if}
          </div>
        {/if}
      </article>
    {:else}
      <div class="panel empty">
        <Icon name="server" size={28} />
        {#if filter === "mine"}
          <p>Henüz sunucu eklemedin. Arkadaşının sunucusunu ya da kendi sunucunu <b>Sunucu ekle</b> ile ekleyebilirsin.</p>
        {:else if filter === "online"}
          <p>Şu an açık sunucu görünmüyor.</p>
        {:else if filter === "community"}
          <p>Topluluk listesi şimdilik boş. GMID sunucuları eklendikçe burada kendiliğinden görünecek.</p>
        {:else}
          <p>Burada GMID topluluğunun sunucuları görünecek. Kendi bildiğin bir sunucuyu da <b>Sunucu ekle</b> ile ekleyebilirsin.</p>
        {/if}
      </div>
    {/each}
  </div>

  <section class="panel source">
    <div class="spread">
      <h3 class="h2"><Icon name="link" />Topluluk listesi</h3>
      {#if !editingUrl}
        <button class="link" onclick={() => { urlDraft = listUrl; editingUrl = true; }}>Değiştir</button>
      {/if}
    </div>
    <p class="sub small">
      GMID sunucuları tek bir listeden gelir; liste güncellenince herkesin uygulamasında birkaç dakika içinde değişir.
      Listeyi yönetmek için GitHub'daki <b>topluluk/sunucular.json</b> dosyasını düzenlemek yeterli.
    </p>
    {#if editingUrl}
      <div class="row">
        <input type="url" class="url" bind:value={urlDraft} placeholder="https://…/sunucular.json" aria-label="Liste adresi" />
        <button class="btn sm" onclick={() => saveUrl(urlDraft)}>Kaydet</button>
        <button class="btn ghost sm" onclick={() => saveUrl(defaultUrl)}>Varsayılan</button>
        <button class="btn ghost sm" onclick={() => (editingUrl = false)}>Vazgeç</button>
      </div>
      <p class="sub small">Adresi boş bırakıp kaydedersen topluluk listesi kapanır; sadece kendi eklediklerin kalır.</p>
    {:else}
      <div class="path">{listUrl || "Kapalı"}</div>
    {/if}
  </section>
</div>

{#if form}
  <div class="modal" role="presentation" onclick={(e) => e.target === e.currentTarget && (form = null)}>
    <form class="panel dialog" onsubmit={save}>
      <h3 class="h2"><Icon name="server" />{form.id ? "Sunucuyu düzenle" : "Sunucu ekle"}</h3>
      <label class="field">
        <span>Oyun</span>
        <select bind:value={form.game}>
          {#each games as g (g.id)}<option value={g.id}>{g.name}</option>{/each}
        </select>
      </label>
      <div class="two">
        <label class="field grow">
          <span>Adres (IP ya da alan adı)</span>
          <input type="text" bind:value={form.host} oninput={onHostInput} placeholder="ör. 185.12.34.56:27015" required />
        </label>
        <label class="field port">
          <span>Port</span>
          <input type="number" min="1" max="65535" bind:value={form.port} placeholder={String(gameOf(form.game)?.defaultPort || "")} />
        </label>
      </div>
      <p class="sub small tip">
        <Icon name="info" size={14} />Sunucunun sitesinde/Discord'unda verdiği <code>connect 1.2.3.4:27015; password abc</code> ya da
        <code>steam://connect/…</code> satırını olduğu gibi adres kutusuna yapıştırabilirsin; adres ve şifre kendiliğinden ayrılır.
      </p>
      <label class="field">
        <span>Şifre (varsa)</span>
        <input type="text" maxlength="64" bind:value={form.password} placeholder="Şifresiz sunucuda boş bırak" autocomplete="off" />
      </label>
      <label class="field">
        <span>Ad</span>
        <input type="text" maxlength="80" bind:value={form.name} placeholder="ör. Kuzenlerle CoD2" />
      </label>
      <label class="field">
        <span>Açıklama (isteğe bağlı)</span>
        <textarea rows="2" maxlength="300" bind:value={form.description}></textarea>
      </label>
      <label class="field">
        <span>Not (isteğe bağlı)</span>
        <input type="text" maxlength="200" bind:value={form.connectNotes} />
      </label>
      <div class="field">
        <span>Logo (isteğe bağlı)</span>
        <div class="row">
          <div class="logo sm" style:--c={COLORS[form.game] ?? "#7a8088"}>
            {#if form.logo}<img src={api.fileSrc(form.logo)} alt="" />{:else}<span>{gameOf(form.game)?.short}</span>{/if}
          </div>
          <button type="button" class="btn ghost sm" onclick={pickLogo}><Icon name="image" size={16} />Resim seç</button>
          {#if form.logo}<button type="button" class="link" onclick={() => form && (form.logo = null)}>Kaldır</button>{/if}
        </div>
      </div>
      <div class="row end">
        <button type="button" class="btn ghost" onclick={() => (form = null)}>Vazgeç</button>
        <button class="btn" type="submit" disabled={saving}><Icon name="check" size={18} />Kaydet</button>
      </div>
    </form>
  </div>
{/if}

{#if needExe}
  <div class="modal" role="presentation" onclick={(e) => e.target === e.currentTarget && (needExe = null)}>
    <div class="panel dialog">
      <h3 class="h2"><Icon name="play" />{needExe.game.name} nerede?</h3>
      <p>
        Oyna'nın oyunu açıp seni doğrudan sunucuya sokabilmesi için <b>{needExe.game.exe}</b> dosyasını bir kez göster.
        Bir dahakine sormaz.
      </p>
      <p class="sub small">Steam'den kurduysan genelde şu klasördedir:</p>
      <div class="path">C:\Program Files (x86)\Steam\steamapps\common\{needExe.game.name}</div>
      <div class="row end">
        <button class="btn ghost" onclick={() => { const s = needExe!.server; needExe = null; void copy(`${s.host}:${s.port}`); }}>Sadece adresi kopyala</button>
        <button class="btn" onclick={pickExe}><Icon name="folder" size={18} />Dosyayı seç</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .tip { margin: -4px 0 0; line-height: 1.6; }
  .tip :global(svg) { vertical-align: -2px; margin-right: 4px; }
  .tip code { font-size: 11.5px; white-space: nowrap; }
  .srv { display: flex; flex-direction: column; gap: 16px; max-width: 1500px; margin: 0 auto; }
  .top { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; flex-wrap: wrap; }
  .summary { display: flex; flex-direction: column; gap: 2px; }
  .summary b { color: var(--gold); }
  .seg .n { color: var(--dim); margin-left: 4px; font-variant-numeric: tabular-nums; }
  .warn { display: flex; gap: 10px; align-items: center; padding: 10px 14px; border-radius: 12px; border: 1px solid rgba(224, 106, 106, 0.4); background: rgba(224, 106, 106, 0.08); color: var(--text); font-size: 13px; }
  .spin { display: inline-grid; animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 340px), 1fr)); gap: 16px; align-items: start; }
  .tile {
    display: flex; flex-direction: column; gap: 12px; min-width: 0;
    background: rgba(34, 37, 42, 0.78); border: 1px solid var(--line); border-radius: var(--r-lg); padding: 16px;
    transition: border-color 0.15s;
  }
  .tile:hover { border-color: var(--line-2); }
  .tile.off { opacity: 0.78; }
  header { display: flex; gap: 12px; align-items: center; min-width: 0; }
  .logo {
    width: 52px; height: 52px; flex: none; border-radius: 12px; overflow: hidden;
    display: grid; place-items: center;
    background: color-mix(in srgb, var(--c) 18%, var(--coal-2)); border: 1px solid color-mix(in srgb, var(--c) 45%, transparent);
  }
  .logo.sm { width: 40px; height: 40px; border-radius: 10px; }
  .logo span { font-family: var(--f-display); font-weight: 800; font-size: 14px; color: var(--c); letter-spacing: 0.02em; }
  .logo img { width: 100%; height: 100%; object-fit: cover; }
  .logo img.pixel { image-rendering: pixelated; }
  .title { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .title b { font-family: var(--f-display); font-size: 15px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .gmid { margin-left: 8px; font-size: 10px; font-weight: 700; letter-spacing: 0.08em; color: var(--gold); border: 1px solid var(--gold-dim); border-radius: 999px; padding: 0 6px; }
  .pill { flex: none; font-size: 11.5px; font-weight: 700; padding: 3px 10px; border-radius: 999px; border: 1px solid var(--line-2); color: var(--muted); display: inline-flex; align-items: center; gap: 6px; }
  .pill::before { content: ""; width: 7px; height: 7px; border-radius: 50%; background: currentColor; }
  .pill.online { color: var(--good); border-color: rgba(92, 192, 138, 0.45); background: rgba(92, 192, 138, 0.08); }
  .pill.online.stale { color: #c8b25a; border-color: rgba(200, 178, 90, 0.4); }
  .pill.offline { color: var(--bad); border-color: rgba(224, 106, 106, 0.4); }
  .pill.checking::before { animation: blink 1s ease-in-out infinite; }
  @keyframes blink { 50% { opacity: 0.2; } }
  .desc { margin: 0; color: var(--muted); font-size: 13px; }
  .stats { display: flex; flex-direction: column; gap: 6px; min-height: 44px; justify-content: center; }
  .players { display: flex; align-items: center; gap: 8px; color: var(--muted); }
  .players b { color: var(--text); font-size: 16px; }
  .bar { flex: 1; height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.06); overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--teal), var(--neon)); }
  .lock { color: var(--gold); display: inline-grid; }
  .meta { display: flex; flex-wrap: wrap; gap: 4px 12px; font-size: 12.5px; color: var(--muted); }
  .ping.good { color: var(--good); }
  .ping.ok { color: #c8b25a; }
  .ping.slow { color: var(--bad); }
  .addr { display: flex; align-items: center; gap: 6px; }
  .addr code { flex: 1; min-width: 0; padding: 5px 10px; }
  .acts { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .sm { padding: 6px 11px; font-size: 13px; }
  .mini { border: 0; background: transparent; color: var(--dim); padding: 6px; border-radius: 6px; display: grid; place-items: center; }
  .mini:hover { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .mini.armed { display: inline-flex; gap: 4px; align-items: center; color: var(--bad); background: rgba(224, 106, 106, 0.12); font-size: 12px; font-weight: 700; }
  .drawer { border-top: 1px solid var(--line); padding-top: 10px; display: flex; flex-direction: column; gap: 8px; font-size: 13px; }
  .drawer p { margin: 0; }
  .notify .nl { display: flex; justify-content: space-between; align-items: center; gap: 10px; }
  .notify select { background: var(--coal-2); border: 1px solid var(--line); border-radius: 8px; padding: 5px 8px; color: var(--text); }
  .bell.on { border-color: var(--gold); color: var(--gold-glow); }
  .note { display: flex; gap: 8px; align-items: flex-start; color: var(--gold-glow); }
  .plist { list-style: none; margin: 0; padding: 0; max-height: 220px; overflow: auto; display: flex; flex-direction: column; gap: 2px; }
  .plist li { display: flex; gap: 12px; padding: 4px 8px; border-radius: 6px; }
  .plist li:nth-child(odd) { background: rgba(255, 255, 255, 0.03); }
  .pn { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dim { color: var(--dim); }
  .empty { grid-column: 1 / -1; display: flex; gap: 14px; align-items: center; color: var(--muted); }
  .empty p { margin: 0; }
  .source { display: flex; flex-direction: column; gap: 10px; }
  .url { flex: 1; min-width: 240px; }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; }
  .link:hover { color: var(--neon); }

  .modal { position: fixed; inset: 0; z-index: 20; background: rgba(8, 9, 11, 0.6); display: grid; place-items: center; padding: 20px; }
  .dialog { width: min(520px, 100%); max-height: calc(100vh - 40px); overflow: auto; display: flex; flex-direction: column; gap: 12px; background: var(--night-2); }
  .dialog p { margin: 0; }
  .field { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .field > span { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .two { display: flex; gap: 10px; }
  .field.grow { flex: 1; }
  .field.port { width: 110px; }
  input[type="text"], input[type="number"], input[type="url"], textarea, select {
    background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 8px 10px;
  }
  textarea { resize: vertical; }
  .end { justify-content: flex-end; }
</style>
