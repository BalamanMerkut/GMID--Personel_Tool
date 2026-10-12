<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, inTauri } from "../api";
  import { focus, takeFocus } from "../focus.svelte";
  import { on } from "../bus";
  import NoteEditor from "../components/NoteEditor.svelte";
  import Icon from "../Icon.svelte";
  import { colorOf, NOTE_COLORS, NOTE_FONTS } from "../notestyle";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { Note, NoteSummary } from "../types";

  let trash = $state(false);
  let list: NoteSummary[] = $state([]);
  let query = $state("");
  let current: Note | null = $state(null);
  let saveState: "" | "saving" | "saved" = $state("");
  let confirmDelete: string | null = $state(null);
  let editor: ReturnType<typeof NoteEditor> | undefined = $state();

  const filtered = $derived(
    query.trim()
      ? list.filter((n) => `${n.title} ${n.preview}`.toLocaleLowerCase("tr").includes(query.trim().toLocaleLowerCase("tr")))
      : list,
  );

  async function refresh() {
    list = await api.notes.list(trash);
  }

  async function open(id: string) {
    await flush();
    current = await api.notes.get(id);
    confirmDelete = null;
  }

  // Aramadan gelen istek: o notu aç.
  let focused = false;
  $effect(() => {
    void focus.n;
    const id = untrack(() => takeFocus("note"));
    if (id) {
      focused = true;
      void open(id);
    }
  });

  onMount(() => {
    void refresh().then(() => {
      if (!current && !focused && list[0]) void open(list[0].id);
    });
    // Başka bir pencerede (yapışkan not, hızlı not) değişirse listeyi ve açık notu güncelle.
    const stop = on<string>("notes-changed", async (id) => {
      await refresh();
      if (current && id === current.id && !timer && document.activeElement?.id !== "note-title") {
        const fresh = await api.notes.get(id);
        if (fresh && editor?.reload(fresh.body)) current = fresh;
      }
    });
    return () => {
      void flush();
      void stop.then((f) => f());
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

  async function create() {
    await flush();
    trash = false;
    const n = await api.notes.save({ id: null, title: "", body: "", color: "default", font: "inter", pinned: false });
    await refresh();
    current = n;
    setTimeout(() => document.getElementById("note-title")?.focus(), 30);
  }

  async function moveToTrash() {
    if (!current) return;
    const id = current.id;
    await flush();
    await api.notes.trash(id);
    current = null;
    await refresh();
    notify("Not çöp kutusuna taşındı · 30 gün sonra silinir", "trash");
    if (list[0]) void open(list[0].id);
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
</script>

<div class="notes">
  <aside class="list panel">
    <div class="row">
      <button class="btn grow" onclick={create}><Icon name="plus" />Yeni not</button>
    </div>
    <label class="search">
      <Icon name="search" size={16} />
      <input type="search" placeholder="Notlarda ara" bind:value={query} />
    </label>
    <div class="seg">
      <button aria-pressed={!trash} onclick={() => showTrash(false)}>Notlar</button>
      <button aria-pressed={trash} onclick={() => showTrash(true)}><Icon name="trash" size={14} />Çöp kutusu</button>
    </div>
    <div class="items">
      {#each filtered as n (n.id)}
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
          <button class="item" aria-pressed={current?.id === n.id} onclick={() => open(n.id)}>
            <span class="dot" style:background={colorOf(n.color).swatch}></span>
            <span class="txt">
              <b>{#if n.pinned}<Icon name="pin" size={13} />{/if}{n.title || "Başlıksız not"}</b>
              <span class="pv">{n.preview || "Boş not"}</span>
              <span class="when">{ago(n.updatedAt)}</span>
            </span>
          </button>
        {/if}
      {:else}
        <p class="empty">{trash ? "Çöp kutusu boş." : query ? "Aramaya uyan not yok." : "Henüz notun yok."}</p>
      {/each}
    </div>
    {#if !trash}
      <p class="hint">İpucu: <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>N</kbd> ile her yerden hızlı not al.</p>
    {/if}
  </aside>

  {#if current && !trash}
    <article class="paper" style:background={colorOf(current.color).paper}>
      <div class="head">
        <input id="note-title" class="title" placeholder="Başlık" value={current.title}
          oninput={(e) => edit({ title: e.currentTarget.value })} />
        <div class="acts">
          <button class="act" title={current.pinned ? "Sabitlemeyi kaldır" : "Üste sabitle"} aria-pressed={current.pinned}
            onclick={() => edit({ pinned: !current!.pinned })}><Icon name="pin" size={17} /></button>
          <button class="act" title="Masaüstüne yapıştır (her zaman üstte)" onclick={sticky}><Icon name="sticky" size={17} /></button>
          <button class="act" title="Çöp kutusuna at" onclick={moveToTrash}><Icon name="trash" size={17} /></button>
        </div>
      </div>
      <div class="style row">
        <div class="swatches" role="group" aria-label="Not rengi">
          {#each NOTE_COLORS as c (c.id)}
            <button class="sw" style:background={c.swatch} title={c.label} aria-label={c.label} aria-pressed={current.color === c.id}
              onclick={() => edit({ color: c.id })}></button>
          {/each}
        </div>
        <div class="seg small-seg">
          {#each NOTE_FONTS as f (f.id)}
            <button aria-pressed={current.font === f.id} style:font-family={f.family} onclick={() => edit({ font: f.id })}>{f.label}</button>
          {/each}
        </div>
        <span class="state">{saveState === "saving" ? "Kaydediliyor…" : saveState === "saved" ? "Kaydedildi" : ""}</span>
      </div>
      <NoteEditor bind:this={editor} docKey={current.id} value={current.body} font={current.font} onchange={(md) => edit({ body: md })} />
    </article>
  {:else}
    <article class="paper empty-paper">
      <Icon name="note" size={34} />
      <h2>{trash ? "Çöp kutusu" : "Bir not seç ya da yeni bir tane yaz"}</h2>
      <p class="sub">{trash ? "Çöpe atılan notlar 30 gün burada durur, sonra kendiliğinden silinir." : "Notların kendiliğinden kaydedilir; ayrıca Belgeler\\GMID Tool\\Notlar klasöründe .md dosyası olarak da durur."}</p>
      {#if !trash}<button class="btn" onclick={create}><Icon name="plus" />Yeni not</button>{/if}
    </article>
  {/if}
</div>

<style>
  .notes { display: grid; grid-template-columns: minmax(240px, 300px) minmax(0, 1fr); gap: 16px; height: calc(100vh - 150px); min-height: 420px; max-width: 1400px; margin: 0 auto; }
  .list { display: flex; flex-direction: column; gap: 10px; min-height: 0; padding: 14px; }
  .grow { flex: 1; justify-content: center; }
  .search { display: flex; align-items: center; gap: 8px; background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 6px 10px; color: var(--dim); }
  .search input { flex: 1; min-width: 0; background: none; border: 0; outline: 0; }
  .seg button { display: inline-flex; align-items: center; gap: 6px; }
  .items { flex: 1; min-height: 0; overflow: auto; display: flex; flex-direction: column; gap: 6px; margin: 0 -4px; padding: 0 4px; }
  .item { display: flex; gap: 10px; text-align: left; width: 100%; border: 1px solid transparent; background: rgba(255, 255, 255, 0.025); border-radius: 10px; padding: 9px 10px; }
  .item:hover { background: rgba(255, 255, 255, 0.05); }
  .item[aria-pressed="true"] { border-color: var(--gold-dim); background: rgba(227, 194, 112, 0.07); }
  .item.trashed { flex-direction: column; gap: 2px; }
  .dot { width: 9px; height: 9px; border-radius: 50%; margin-top: 6px; flex: none; }
  .txt { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .txt b, .trashed b { display: flex; align-items: center; gap: 5px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .txt b :global(svg) { color: var(--gold); flex: none; }
  .pv { font-size: 12.5px; color: var(--muted); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .when { font-size: 11px; color: var(--dim); }
  .empty { color: var(--dim); text-align: center; padding: 20px 0; margin: 0; }
  .hint { font-size: 12px; color: var(--dim); margin: 0; }
  kbd { font-family: var(--f-body); font-size: 11px; font-weight: 600; padding: 1px 6px; border-radius: 5px; background: var(--coal); border: 1px solid var(--line-2); }
  .link { border: 0; background: none; padding: 2px 0; color: var(--teal); font-size: 12.5px; font-weight: 600; }
  .link.danger { color: var(--bad); }
  .paper { display: flex; flex-direction: column; min-height: 0; border: 1px solid var(--line); border-radius: var(--r-lg); padding: 18px 22px; }
  .head { display: flex; gap: 10px; align-items: center; }
  .title { flex: 1; min-width: 0; background: none; border: 0; outline: 0; font-family: var(--f-display); font-weight: 700; font-size: 22px; padding: 2px 0; }
  .title::placeholder { color: var(--dim); }
  .acts { display: flex; gap: 2px; }
  .act { border: 0; background: transparent; color: var(--muted); padding: 7px; border-radius: 8px; display: grid; place-items: center; }
  .act:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .act[aria-pressed="true"] { color: var(--gold); }
  .style { margin: 8px 0 10px; gap: 14px; }
  .swatches { display: flex; gap: 6px; }
  .sw { width: 20px; height: 20px; border-radius: 50%; border: 2px solid rgba(255, 255, 255, 0.12); padding: 0; }
  .sw[aria-pressed="true"] { border-color: var(--text); box-shadow: 0 0 0 2px rgba(0, 0, 0, 0.4); }
  .small-seg button { padding: 3px 10px; font-size: 12.5px; }
  .state { margin-left: auto; font-size: 12px; color: var(--dim); }
  .empty-paper { align-items: center; justify-content: center; text-align: center; gap: 12px; background: rgba(34, 37, 42, 0.6); color: var(--gold); }
  .empty-paper h2 { font-family: var(--f-display); color: var(--text); margin: 0; }
  .empty-paper p { max-width: 50ch; }
  @media (max-width: 1000px) {
    .notes { grid-template-columns: minmax(0, 1fr); height: auto; }
    .list { max-height: 320px; }
    .paper { min-height: 460px; }
  }
</style>
