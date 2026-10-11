<script lang="ts">
  // Hızlı not ve yapışkan not pencereleri: kenarlıksız, her zaman üstte, kendiliğinden kaydeder.
  import { onMount } from "svelte";
  import { api, inTauri } from "../api";
  import { on } from "../bus";
  import NoteEditor from "../components/NoteEditor.svelte";
  import Icon from "../Icon.svelte";
  import { colorOf } from "../notestyle";
  import type { Note } from "../types";

  /** `id` yoksa hızlı not: ilk yazıldığında yeni not oluşur. */
  let { id = null }: { id?: string | null } = $props();
  const quick = $derived(id === null);

  let note: Note | null = $state(null);
  let title = $state("");
  let body = $state("");
  let editor: ReturnType<typeof NoteEditor> | undefined = $state();
  let timer: ReturnType<typeof setTimeout> | undefined;

  onMount(() => {
    if (id) {
      void api.notes.get(id).then((n) => {
        note = n;
        title = n?.title ?? "";
        body = n?.body ?? "";
      });
    }
    const stop = on<string>("notes-changed", async (changed) => {
      if (!note || changed !== note.id || timer) return;
      const fresh = await api.notes.get(changed);
      if (!fresh || fresh.deletedAt) return close();
      if (editor?.reload(fresh.body)) {
        note = fresh;
        body = fresh.body;
        if (document.activeElement?.id !== "nw-title") title = fresh.title;
      }
    });
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && void close();
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      void stop.then((f) => f());
    };
  });

  function schedule() {
    clearTimeout(timer);
    timer = setTimeout(() => void flush(), 400);
  }

  async function flush() {
    clearTimeout(timer);
    timer = undefined;
    const empty = !title.trim() && !body.trim();
    if (!note && empty) return;
    note = await api.notes.save({
      id: note?.id ?? null,
      title,
      body,
      color: note?.color ?? "default",
      font: note?.font ?? "inter",
      pinned: note?.pinned ?? false,
    });
  }

  async function close() {
    await flush();
    // Boş bırakılan hızlı not kaydedilmez.
    if (quick && note && !title.trim() && !body.trim()) await api.notes.remove(note.id);
    if (!inTauri) return;
    if (quick) {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().close();
    } else if (note) {
      await api.notes.closeSticky(note.id);
    }
  }
</script>

<div class="win" style:background={colorOf(note?.color ?? "default").paper}>
  <header data-tauri-drag-region>
    <span class="label" data-tauri-drag-region><Icon name={quick ? "note" : "sticky"} size={15} />{quick ? "Hızlı not" : "Yapışkan not"}</span>
    <button class="x" title={quick ? "Kaydet ve kapat (Esc)" : "Masaüstünden kaldır"} onclick={close}><Icon name="x" size={16} /></button>
  </header>
  <input id="nw-title" class="title" placeholder="Başlık" bind:value={title} oninput={schedule} />
  {#if quick || note}
    <NoteEditor bind:this={editor} docKey={quick ? "quick" : (id ?? "")} value={body} font={note?.font ?? "inter"} toolbar={false} autofocus={quick}
      onchange={(md) => { body = md; schedule(); }} />
  {/if}
  {#if quick}<p class="foot">Kendiliğinden kaydedilir · Notlar ekranında da görünür</p>{/if}
</div>

<style>
  :global(body) { background: var(--night); }
  .win { position: fixed; inset: 0; display: flex; flex-direction: column; padding: 0 14px 10px; border: 1px solid var(--line-2); }
  header { display: flex; align-items: center; justify-content: space-between; height: 36px; flex: none; cursor: grab; }
  .label { display: flex; align-items: center; gap: 6px; font-size: 12px; font-weight: 600; letter-spacing: 0.06em; text-transform: uppercase; color: var(--gold-dim); }
  .x { border: 0; background: transparent; color: var(--muted); padding: 5px; border-radius: 7px; display: grid; place-items: center; }
  .x:hover { color: var(--text); background: rgba(255, 255, 255, 0.08); }
  .title { background: none; border: 0; outline: 0; font-family: var(--f-display); font-weight: 700; font-size: 17px; padding: 2px 0 6px; }
  .title::placeholder { color: var(--dim); }
  .foot { margin: 6px 0 0; font-size: 11px; color: var(--dim); }
</style>
