<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, inTauri } from "../api";
  import { focus, takeFocus } from "../focus.svelte";
  import { on } from "../bus";
  import NoteEditor from "../components/NoteEditor.svelte";
  import Icon from "../Icon.svelte";
  import { colorOf, fontOf, NOTE_COLORS, NOTE_FONTS } from "../notestyle";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { Note, NoteSummary } from "../types";

  let { onnavigate }: { onnavigate?: (route: string) => void } = $props();

  type Filter = "all" | "pinned" | "lecture" | "todo";
  let trash = $state(false);
  let list: NoteSummary[] = $state([]);
  let trashCount = $state(0);
  let query = $state("");
  let filter = $state<Filter>("all");
  let current = $state<Note | null>(null);
  let saveState: "" | "saving" | "saved" = $state("");
  let confirmDelete: string | null = $state(null);
  let editor: ReturnType<typeof NoteEditor> | undefined = $state();
  let newMenu = $state(false);
  let styleMenu = $state(false);

  const FILTERS: { id: Filter; label: string; icon: "note" | "pin" | "mic" | "checklist"; test: (n: NoteSummary) => boolean }[] = [
    { id: "all", label: "Tümü", icon: "note", test: () => true },
    { id: "pinned", label: "Sabit", icon: "pin", test: (n) => n.pinned },
    { id: "lecture", label: "Dersler", icon: "mic", test: (n) => n.kind === "lecture" },
    { id: "todo", label: "Yapılacak", icon: "checklist", test: (n) => n.tasksTotal > 0 },
  ];
  const count = (f: (typeof FILTERS)[number]) => list.filter(f.test).length;

  const filtered = $derived.by(() => {
    const q = query.trim().toLocaleLowerCase("tr");
    const test = FILTERS.find((f) => f.id === filter)?.test ?? (() => true);
    return list.filter((n) => (trash || test(n)) && (!q || `${n.title} ${n.preview}`.toLocaleLowerCase("tr").includes(q)));
  });

  /** Liste tarihe göre bölümlenir: Sabitlenenler, Bugün, Dün, Bu hafta, Bu ay, Daha eski. */
  const sections = $derived.by(() => {
    const day = 86_400_000;
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    const today = start.getTime();
    const out: { label: string; notes: NoteSummary[] }[] = [];
    const put = (label: string, n: NoteSummary) => {
      const s = out.find((x) => x.label === label);
      if (s) s.notes.push(n);
      else out.push({ label, notes: [n] });
    };
    for (const n of filtered) {
      if (n.pinned && !trash) put("Sabitlenenler", n);
      else if (n.updatedAt >= today) put("Bugün", n);
      else if (n.updatedAt >= today - day) put("Dün", n);
      else if (n.updatedAt >= today - 6 * day) put("Bu hafta", n);
      else if (n.updatedAt >= today - 30 * day) put("Bu ay", n);
      else put("Daha eski", n);
    }
    return out;
  });

  async function refresh() {
    list = await api.notes.list(trash);
    if (!trash) trashCount = (await api.notes.list(true).catch(() => [])).length;
  }

  async function open(id: string) {
    await flush();
    current = await api.notes.get(id);
    confirmDelete = null;
    styleMenu = false;
  }

  // Aramadan gelen istek: o notu aç.
  let focused = false;
  $effect(() => {
    void focus.n;
    const id = untrack(() => takeFocus("note"));
    if (id) {
      focused = true;
      if (trash) void showTrash(false);
      void open(id);
    }
  });

  onMount(() => {
    void refresh().then(() => {
      if (!current && !focused && list[0]) void open(list[0].id);
    });
    // Başka bir pencerede (yapışkan not, hızlı not, ders kaydı) değişirse listeyi ve açık notu güncelle.
    const stop = on<string>("notes-changed", async (id) => {
      await refresh();
      if (current && id === current.id && !timer && document.activeElement?.id !== "note-title") {
        const fresh = await api.notes.get(id);
        if (fresh && editor?.reload(fresh.body)) current = fresh;
      }
    });
    const closeMenus = (e: MouseEvent) => {
      const t = e.target as HTMLElement;
      if (!t.closest(".newwrap")) newMenu = false;
      if (!t.closest(".stylewrap")) styleMenu = false;
    };
    document.addEventListener("click", closeMenus);
    return () => {
      void flush();
      void stop.then((f) => f());
      document.removeEventListener("click", closeMenus);
    };
  });

  // ---- otomatik kaydetme ----
  let timer: ReturnType<typeof setTimeout> | undefined;
  let dirty = false;

  function schedule() {
    dirty = true;
    saveState = "saving";
    clearTimeout(timer);
    timer = setTimeout(() => void flush(), 500);
  }

  async function flush() {
    clearTimeout(timer);
    timer = undefined;
    if (!dirty || !current) return;
    dirty = false;
    const n = current;
    const saved = await api.notes.save({ id: n.id, title: n.title, body: n.body, color: n.color, font: n.font, pinned: n.pinned });
    if (current?.id === saved.id) current.updatedAt = saved.updatedAt;
    saveState = "saved";
    await refresh();
  }

  function edit(patch: Partial<Note>) {
    if (!current) return;
    Object.assign(current, patch);
    schedule();
  }

  /** Yeni not şablonları: boş, yapılacaklar, ders notu. */
  const TEMPLATES = {
    blank: { title: "", body: "", color: "default" },
    todo: { title: "Yapılacaklar", body: "- [ ] ", color: "teal" },
    lesson: {
      title: "Ders notu",
      body: "## Konu\n\n\n\n## Önemli noktalar\n\n- \n\n## Sorular\n\n- \n\n## Ödev\n\n- [ ] ",
      color: "navy",
    },
  } as const;

  async function create(kind: keyof typeof TEMPLATES = "blank") {
    newMenu = false;
    await flush();
    if (trash) await showTrash(false);
    const t = TEMPLATES[kind];
    const n = await api.notes.save({ id: null, title: t.title, body: t.body, color: t.color, font: "inter", pinned: false });
    filter = "all";
    query = "";
    await refresh();
    current = n;
    setTimeout(() => {
      const el = document.getElementById("note-title") as HTMLInputElement | null;
      el?.focus();
      el?.select();
    }, 30);
  }

  async function moveToTrash() {
    if (!current) return;
    const id = current.id;
    await flush();
    await api.notes.trash(id);
    current = null;
    await refresh();
    notify("Not çöp kutusuna taşındı · 30 gün sonra silinir", "trash");
    if (filtered[0]) void open(filtered[0].id);
  }

  async function restore(id: string) {
    await api.notes.restore(id);
    await refresh();
    notify("Not geri alındı", "restore");
  }

  async function removeForever(id: string) {
    if (confirmDelete !== id) {
      confirmDelete = id;
      return;
    }
    await api.notes.remove(id);
    confirmDelete = null;
    await refresh();
    notify("Not kalıcı olarak silindi", "trash");
  }

  async function showTrash(v: boolean) {
    await flush();
    trash = v;
    current = null;
    await refresh();
    if (!v && list[0]) void open(list[0].id);
  }

  async function sticky() {
    if (!current) return;
    await flush();
    await api.notes.openSticky(current.id);
    notify(inTauri ? "Not masaüstüne yapıştırıldı" : "Yapışkan not masaüstü uygulamasında açılır", "sticky");
  }

  async function copyText() {
    if (!current) return;
    const text = `${current.title ? `${current.title}\n\n` : ""}${current.body}`;
    try {
      await navigator.clipboard.writeText(text);
      notify("Not panoya kopyalandı", "copy");
    } catch {
      notify("Panoya kopyalanamadı", "info");
    }
  }

  // ---- açık notun özeti ----
  const summary = $derived(current ? list.find((n) => n.id === current!.id) : undefined);
  const lecture = $derived(!!current?.body.trimStart().startsWith("*Ders kaydı ·"));
  const tasks = $derived.by(() => {
    const m = current?.body.match(/^\s*[-*+]\s*\[( |x|X)\]/gm) ?? [];
    return { done: m.filter((t) => /x\]/i.test(t)).length, total: m.length };
  });
  const words = $derived.by(() => {
    const body: string = current?.body ?? "";
    return body.split(/\s+/).filter((w: string) => /[\p{L}\d]/u.test(w) && !/^\[[ xX]\]$/.test(w)).length;
  });
  const fmtDate = (ms: number) => new Date(ms).toLocaleDateString("tr-TR", { day: "numeric", month: "long", year: "numeric" });
</script>

<div class="notes">
  <aside class="list panel">
    {#if trash}
      <div class="trashhead">
        <button class="back" onclick={() => showTrash(false)}><Icon name="back" size={16} />Notlar</button>
        <b><Icon name="trash" size={15} />Çöp kutusu</b>
      </div>
      <p class="sub small">Çöpe atılan notlar 30 gün burada durur, sonra kendiliğinden silinir.</p>
    {:else}
      <div class="newwrap">
        <div class="newbtn">
          <button class="btn main" onclick={() => create()}><Icon name="plus" />Yeni not</button>
          <button class="btn more" title="Şablonlar" aria-label="Şablonlar" aria-expanded={newMenu} onclick={() => (newMenu = !newMenu)}><span class="caret" class:up={newMenu}></span></button>
        </div>
        {#if newMenu}
          <div class="menu" role="menu">
            <button role="menuitem" onclick={() => create("blank")}><Icon name="note" size={16} /><span><b>Boş not</b><em>Düz yazı</em></span></button>
            <button role="menuitem" onclick={() => create("todo")}><Icon name="checklist" size={16} /><span><b>Yapılacaklar</b><em>İşaretlenebilir liste</em></span></button>
            <button role="menuitem" onclick={() => create("lesson")}><Icon name="book" size={16} /><span><b>Ders notu</b><em>Konu, önemli noktalar, sorular, ödev</em></span></button>
            <button role="menuitem" onclick={() => ((newMenu = false), onnavigate?.("lecture"))}><Icon name="mic" size={16} /><span><b>Ders kaydı başlat</b><em>Hoca konuşurken kendiliğinden yazılır</em></span></button>
          </div>
        {/if}
      </div>
      <label class="search">
        <Icon name="search" size={16} />
        <input type="search" placeholder="Notlarda ara" bind:value={query} />
      </label>
      <div class="filters" role="tablist" aria-label="Not türü">
        {#each FILTERS as f (f.id)}
          {@const c = count(f)}
          {#if f.id === "all" || c > 0}
            <button role="tab" aria-selected={filter === f.id} onclick={() => (filter = f.id)}>
              <Icon name={f.icon} size={13} />{f.label}<i class="tnum">{c}</i>
            </button>
          {/if}
        {/each}
      </div>
    {/if}

    <div class="items">
      {#each sections as sec (sec.label)}
        {#if !trash}<h4 class="sec">{sec.label}</h4>{/if}
        {#each sec.notes as n (n.id)}
          {#if trash}
            <div class="item trashed">
              <b>{n.title || "Başlıksız not"}</b>
              <span class="pv">{n.preview || "Boş not"}</span>
              <div class="row">
                <button class="link" onclick={() => restore(n.id)}>Geri al</button>
                <button class="link danger" onclick={() => removeForever(n.id)}>{confirmDelete === n.id ? "Emin misin? Tekrar bas" : "Kalıcı sil"}</button>
              </div>
            </div>
          {:else}
            <button class="item" aria-pressed={current?.id === n.id} onclick={() => open(n.id)} style:--c={colorOf(n.color).swatch}>
              <span class="txt">
                <b>
                  {#if n.kind === "lecture"}<Icon name="mic" size={13} />{:else if n.pinned}<Icon name="pin" size={13} />{/if}
                  <span class="t">{n.title || "Başlıksız not"}</span>
                </b>
                <span class="pv">{n.preview || "Boş not"}</span>
                <span class="meta">
                  <span>{ago(n.updatedAt)}</span>
                  {#if n.tasksTotal}
                    <span class="chip-t" class:done={n.tasksDone === n.tasksTotal}><Icon name="checklist" size={12} />{n.tasksDone}/{n.tasksTotal}</span>
                  {/if}
                  {#if n.images}<span class="chip-t"><Icon name="image" size={12} />{n.images}</span>{/if}
                </span>
              </span>
            </button>
          {/if}
        {/each}
      {:else}
        <div class="empty">
          <Icon name={trash ? "trash" : "note"} size={26} />
          <p>{trash ? "Çöp kutusu boş." : query ? "Aramaya uyan not yok." : filter !== "all" ? "Bu türde not yok." : "Henüz notun yok."}</p>
        </div>
      {/each}
    </div>

    {#if !trash}
      <div class="foot">
        <button class="link muted" onclick={() => showTrash(true)}><Icon name="trash" size={13} />Çöp kutusu{#if trashCount} · {trashCount}{/if}</button>
        <span class="hint"><kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>N</kbd> hızlı not</span>
      </div>
    {/if}
  </aside>

  {#if current && !trash}
    <article class="paper" style:background={colorOf(current.color).paper}>
      <div class="head">
        <input id="note-title" class="title" placeholder="Başlık" value={current.title} oninput={(e) => edit({ title: e.currentTarget.value })} />
        <div class="acts">
          <button class="act" title={current.pinned ? "Sabitlemeyi kaldır" : "Üste sabitle"} aria-pressed={current.pinned} onclick={() => edit({ pinned: !current!.pinned })}>
            <Icon name="pin" size={17} />
          </button>
          <div class="stylewrap">
            <button class="act" title="Renk ve yazı tipi" aria-expanded={styleMenu} onclick={() => (styleMenu = !styleMenu)}>
              <span class="cdot" style:background={colorOf(current.color).swatch}></span>
            </button>
            {#if styleMenu}
              <div class="menu right">
                <span class="mlabel">Renk</span>
                <div class="swatches" role="group" aria-label="Not rengi">
                  {#each NOTE_COLORS as c (c.id)}
                    <button class="sw" style:background={c.swatch} title={c.label} aria-label={c.label} aria-pressed={current.color === c.id} onclick={() => edit({ color: c.id })}></button>
                  {/each}
                </div>
                <span class="mlabel">Yazı tipi</span>
                <div class="fonts">
                  {#each NOTE_FONTS as f (f.id)}
                    <button aria-pressed={current.font === f.id} style:font-family={f.family} onclick={() => edit({ font: f.id })}>{f.label}</button>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
          <button class="act" title="Yazıyı kopyala" onclick={copyText}><Icon name="copy" size={17} /></button>
          <button class="act" title="Masaüstüne yapıştır (her zaman üstte)" onclick={sticky}><Icon name="sticky" size={17} /></button>
          <button class="act danger" title="Çöp kutusuna at" onclick={moveToTrash}><Icon name="trash" size={17} /></button>
        </div>
      </div>
      <div class="info">
        {#if lecture}<span class="tag lec"><Icon name="mic" size={12} />Ders kaydı</span>{/if}
        <span>{fmtDate(current.createdAt)}</span>
        <span>·</span>
        <span>{words} sözcük</span>
        {#if fontOf(current.font).id !== "inter"}<span>·</span><span>{fontOf(current.font).label}</span>{/if}
        <span class="state" class:ok={saveState === "saved"}>
          {#if saveState === "saving"}Kaydediliyor…{:else if saveState === "saved"}<Icon name="check" size={12} />Kaydedildi{:else}{summary ? `Düzenlendi ${ago(summary.updatedAt)}` : ""}{/if}
        </span>
      </div>
      {#if tasks.total}
        <div class="progress" title="{tasks.done}/{tasks.total} tamamlandı">
          <span class="bar"><i style:width="{(tasks.done / tasks.total) * 100}%"></i></span>
          <span class="small">{tasks.done === tasks.total ? "Hepsi tamam" : `${tasks.done}/${tasks.total} tamamlandı`}</span>
        </div>
      {/if}
      <NoteEditor bind:this={editor} docKey={current.id} value={current.body} font={current.font} onchange={(md) => edit({ body: md })} />
    </article>
  {:else}
    <article class="paper empty-paper">
      {#if trash}
        <Icon name="trash" size={34} />
        <h2>Çöp kutusu</h2>
        <p class="sub">Soldan bir notu geri alabilir ya da kalıcı olarak silebilirsin.</p>
      {:else}
        <Icon name="note" size={34} />
        <h2>Ne yazmak istersin?</h2>
        <p class="sub">Notların kendiliğinden kaydedilir; ayrıca Belgeler\GMID Tool\Notlar klasöründe .md dosyası olarak da durur.</p>
        <div class="starts">
          <button onclick={() => create("blank")}><Icon name="note" size={20} /><b>Boş not</b><em>Düz yazı, resim, liste</em></button>
          <button onclick={() => create("todo")}><Icon name="checklist" size={20} /><b>Yapılacaklar</b><em>İşaretle, ilerlemeyi gör</em></button>
          <button onclick={() => create("lesson")}><Icon name="book" size={20} /><b>Ders notu</b><em>Hazır başlıklarla</em></button>
          <button onclick={() => onnavigate?.("lecture")}><Icon name="mic" size={20} /><b>Ders kaydı</b><em>Hoca konuşurken yazılır</em></button>
        </div>
      {/if}
    </article>
  {/if}
</div>

<style>
  .notes { display: grid; grid-template-columns: minmax(260px, 320px) minmax(0, 1fr); gap: 16px; height: calc(100vh - 150px); min-height: 440px; max-width: 1400px; margin: 0 auto; }
  .list { display: flex; flex-direction: column; gap: 10px; min-height: 0; padding: 14px; }

  .newwrap { position: relative; }
  .newbtn { display: flex; }
  .newbtn .main { flex: 1; justify-content: center; border-radius: 10px 0 0 10px; }
  .newbtn .more { border-radius: 0 10px 10px 0; padding: 9px 10px; border-left: 1px solid rgba(0, 0, 0, 0.18); }
  .caret { width: 8px; height: 8px; border-right: 2px solid currentColor; border-bottom: 2px solid currentColor; transform: translateY(-2px) rotate(45deg); transition: transform 0.15s; }
  .caret.up { transform: translateY(2px) rotate(-135deg); }
  .menu { position: absolute; z-index: 20; top: calc(100% + 6px); left: 0; right: 0; display: flex; flex-direction: column; gap: 2px; padding: 6px; border-radius: 12px; background: #23262b; border: 1px solid var(--line-2); box-shadow: 0 14px 40px rgba(0, 0, 0, 0.45); }
  .menu > button { display: flex; align-items: center; gap: 10px; text-align: left; border: 0; background: none; padding: 8px 10px; border-radius: 8px; color: var(--muted); }
  .menu > button:hover { background: rgba(255, 255, 255, 0.06); color: var(--text); }
  .menu > button span { display: flex; flex-direction: column; }
  .menu > button b { color: var(--text); font-weight: 600; font-size: 13.5px; }
  .menu > button em { font-style: normal; font-size: 11.5px; color: var(--dim); }

  .search { display: flex; align-items: center; gap: 8px; background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; color: var(--dim); }
  .search:focus-within { border-color: var(--gold-dim); }
  .search input { flex: 1; min-width: 0; background: none; border: 0; outline: 0; }
  .filters { display: flex; gap: 4px; flex-wrap: wrap; }
  .filters button { display: inline-flex; align-items: center; gap: 5px; border: 1px solid var(--line); background: transparent; color: var(--muted); border-radius: 999px; padding: 4px 9px; font-size: 12px; }
  .filters button:hover { color: var(--text); }
  .filters button[aria-selected="true"] { border-color: var(--gold); color: var(--gold-glow); background: rgba(227, 194, 112, 0.1); }
  .filters i { font-style: normal; color: var(--dim); font-size: 11px; }

  .items { flex: 1; min-height: 0; overflow: auto; display: flex; flex-direction: column; gap: 5px; margin: 0 -6px; padding: 0 6px; }
  .sec { margin: 8px 2px 2px; font-size: 10.5px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; color: var(--gold-dim); }
  .sec:first-child { margin-top: 2px; }
  .item { position: relative; display: flex; text-align: left; width: 100%; border: 1px solid transparent; background: rgba(255, 255, 255, 0.025); border-radius: 11px; padding: 9px 10px 9px 14px; overflow: hidden; }
  .item::before { content: ""; position: absolute; left: 0; top: 8px; bottom: 8px; width: 3px; border-radius: 0 3px 3px 0; background: var(--c); opacity: 0.9; }
  .item:hover { background: rgba(255, 255, 255, 0.05); }
  .item[aria-pressed="true"] { border-color: var(--gold-dim); background: rgba(227, 194, 112, 0.08); }
  .item.trashed { flex-direction: column; gap: 2px; padding-left: 10px; }
  .item.trashed::before { display: none; }
  .txt { display: flex; flex-direction: column; gap: 2px; min-width: 0; width: 100%; }
  .txt b, .trashed b { display: flex; align-items: center; gap: 5px; font-weight: 600; min-width: 0; }
  .txt b .t { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .txt b :global(svg) { color: var(--gold); flex: none; }
  .pv { font-size: 12.5px; color: var(--muted); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .meta { display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--dim); margin-top: 1px; }
  .chip-t { display: inline-flex; align-items: center; gap: 3px; padding: 0 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.05); }
  .chip-t.done { color: var(--good); }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 6px; color: var(--dim); text-align: center; padding: 26px 0; }
  .empty p { margin: 0; }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 8px; padding-top: 8px; border-top: 1px solid var(--line); }
  .hint { font-size: 11.5px; color: var(--dim); white-space: nowrap; }
  kbd { font-family: var(--f-body); font-size: 10.5px; font-weight: 600; padding: 0 5px; border-radius: 5px; background: var(--coal); border: 1px solid var(--line-2); }
  .link { display: inline-flex; align-items: center; gap: 5px; border: 0; background: none; padding: 2px 0; color: var(--teal); font-size: 12.5px; font-weight: 600; }
  .link.muted { color: var(--muted); font-weight: 500; }
  .link.muted:hover { color: var(--text); }
  .link.danger { color: var(--bad); }
  .trashhead { display: flex; align-items: center; justify-content: space-between; }
  .trashhead b { display: inline-flex; align-items: center; gap: 6px; }
  .back { display: inline-flex; align-items: center; gap: 4px; border: 0; background: none; color: var(--muted); padding: 4px 0; font-weight: 600; }
  .back:hover { color: var(--text); }

  .paper { display: flex; flex-direction: column; min-height: 0; border: 1px solid var(--line); border-radius: var(--r-lg); padding: 18px 24px; }
  .head { display: flex; gap: 10px; align-items: center; }
  .title { flex: 1; min-width: 0; background: none; border: 0; outline: 0; font-family: var(--f-display); font-weight: 700; font-size: 24px; padding: 2px 0; }
  .title::placeholder { color: var(--dim); }
  .acts { display: flex; gap: 2px; align-items: center; }
  .act { border: 0; background: transparent; color: var(--muted); padding: 7px; border-radius: 8px; display: grid; place-items: center; }
  .act:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .act[aria-pressed="true"] { color: var(--gold); }
  .act.danger:hover { color: var(--bad); }
  .cdot { width: 17px; height: 17px; border-radius: 50%; border: 2px solid rgba(255, 255, 255, 0.35); }
  .stylewrap { position: relative; }
  .menu.right { left: auto; right: 0; width: 250px; gap: 8px; padding: 12px; }
  .mlabel { font-size: 10.5px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; color: var(--dim); }
  .swatches { display: flex; gap: 7px; flex-wrap: wrap; }
  .sw { width: 24px; height: 24px; border-radius: 50%; border: 2px solid rgba(255, 255, 255, 0.12); padding: 0; }
  .sw[aria-pressed="true"] { border-color: var(--text); box-shadow: 0 0 0 2px rgba(0, 0, 0, 0.4); }
  .fonts { display: flex; gap: 4px; }
  .fonts button { flex: 1; border: 1px solid var(--line-2); background: transparent; color: var(--muted); border-radius: 8px; padding: 5px 4px; font-size: 13px; }
  .fonts button[aria-pressed="true"] { border-color: var(--gold); color: var(--gold-glow); background: rgba(227, 194, 112, 0.1); }

  .info { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; font-size: 12px; color: var(--dim); margin: 2px 0 10px; }
  .tag { display: inline-flex; align-items: center; gap: 4px; padding: 1px 8px; border-radius: 999px; font-weight: 600; }
  .tag.lec { background: rgba(53, 160, 162, 0.16); color: var(--neon); }
  .state { margin-left: auto; display: inline-flex; align-items: center; gap: 4px; }
  .state.ok { color: var(--good); }
  .progress { display: flex; align-items: center; gap: 10px; margin: -2px 0 10px; color: var(--muted); }
  .bar { flex: 1; max-width: 260px; height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
  .bar i { display: block; height: 100%; background: linear-gradient(90deg, var(--teal), var(--good)); transition: width 0.3s; }

  .empty-paper { align-items: center; justify-content: center; text-align: center; gap: 12px; background: rgba(34, 37, 42, 0.6); color: var(--gold); }
  .empty-paper h2 { font-family: var(--f-display); color: var(--text); margin: 0; }
  .empty-paper p { max-width: 52ch; margin: 0; }
  .starts { display: grid; grid-template-columns: repeat(2, minmax(160px, 210px)); gap: 10px; margin-top: 8px; }
  .starts button { display: flex; flex-direction: column; align-items: flex-start; gap: 4px; text-align: left; padding: 14px; border-radius: 14px; border: 1px solid var(--line-2); background: rgba(255, 255, 255, 0.03); color: var(--gold); }
  .starts button:hover { border-color: var(--gold-dim); background: rgba(227, 194, 112, 0.06); }
  .starts b { color: var(--text); font-size: 14px; }
  .starts em { font-style: normal; font-size: 12px; color: var(--muted); }
  @media (max-width: 1000px) {
    .notes { grid-template-columns: minmax(0, 1fr); height: auto; }
    .list { max-height: 360px; }
    .paper { min-height: 480px; }
  }
</style>
