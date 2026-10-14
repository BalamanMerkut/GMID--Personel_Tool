<script lang="ts">
  // Notlar sadece bu bileşenle birlikte yüklenir (TipTap ve not yazı tipleri ayrı parçada).
  import "@fontsource/jetbrains-mono/latin-400.css";
  import "@fontsource/jetbrains-mono/latin-ext-400.css";
  import "@fontsource/caveat/latin-500.css";
  import "@fontsource/caveat/latin-ext-500.css";
  import { Editor, mergeAttributes } from "@tiptap/core";
  import Image from "@tiptap/extension-image";
  import { TaskItem, TaskList } from "@tiptap/extension-list";
  import { Placeholder } from "@tiptap/extensions";
  import { Markdown } from "@tiptap/markdown";
  import StarterKit from "@tiptap/starter-kit";
  import { onMount } from "svelte";
  import { api, inTauri } from "../api";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";
  import type { IconName } from "../icons";
  import { fontOf } from "../notestyle";

  let {
    docKey,
    value,
    font = "inter",
    toolbar = true,
    autofocus = false,
    onchange,
  }: {
    /** Hangi not açık; değişince içerik yeniden yüklenir. */
    docKey: string;
    value: string;
    font?: string;
    toolbar?: boolean;
    autofocus?: boolean;
    onchange: (markdown: string) => void;
  } = $props();

  // Resimler `Notlar\Resimler`de durur; notta göreli yol ("Resimler/x.png") yazılır, gösterirken tam yola çevrilir.
  let base = "";
  const ready = api.notes.folder().then((f) => (base = f)).catch(() => {});
  const resolveSrc = (src: string | null) => {
    if (!src || /^(https?:|data:|blob:|asset:)/i.test(src) || !base) return src;
    return api.fileSrc(`${base}\\${src.replace(/\//g, "\\")}`);
  };
  /** Düzenleyiciden kopyalanan resmin tam yolu yeniden göreli yola döner. */
  const unresolve = (src: string | null) => {
    if (!src) return src;
    let s = src;
    try {
      s = decodeURIComponent(src);
    } catch {
      /* olduğu gibi */
    }
    const m = s.match(/Resimler[\\/]([^\\/?#"]+)$/);
    return m ? `Resimler/${m[1]}` : src;
  };
  const NoteImage = Image.extend({
    addAttributes() {
      return {
        ...this.parent?.(),
        src: { default: null, parseHTML: (el) => unresolve(el.getAttribute("src")) },
      };
    },
    renderHTML({ HTMLAttributes }) {
      return ["img", mergeAttributes(this.options.HTMLAttributes, HTMLAttributes, { src: resolveSrc(HTMLAttributes.src as string | null) })];
    },
  }).configure({ inline: true, allowBase64: true });

  /** Panodan ya da sürüklenerek gelen resim dosyası kaydedilip imlecin yerine (ya da `pos`a) eklenir. */
  async function insertFile(file: File, pos?: number) {
    if (file.size > 25 * 1024 * 1024) return notify("Resim çok büyük (en fazla 25 MB)", "info");
    const data = await new Promise<string>((ok, no) => {
      const r = new FileReader();
      r.onload = () => ok(String(r.result));
      r.onerror = () => no(r.error);
      r.readAsDataURL(file);
    });
    try {
      const src = await api.notes.pasteImage(data);
      insertImage(src, pos);
    } catch (e) {
      notify(String(e), "info");
    }
  }
  function insertImage(src: string, pos?: number) {
    if (!editor) return;
    const node = { type: "image", attrs: { src } };
    if (pos == null) editor.chain().focus().insertContent(node).run();
    else editor.chain().focus().insertContentAt(pos, node).run();
  }
  async function pickImage() {
    if (!inTauri) return notify("Resim seçmek masaüstü uygulamasında çalışır; ekran görüntüsünü Ctrl+V ile yapıştırabilirsin", "info");
    try {
      const src = await api.notes.pickImage();
      if (src) insertImage(src);
    } catch (e) {
      notify(String(e), "info");
    }
  }
  const imageFiles = (list: FileList | null | undefined) => [...(list ?? [])].filter((f) => f.type.startsWith("image/"));

  let host: HTMLDivElement;
  let editor: Editor | undefined = $state();
  let tick = $state(0); // araç çubuğu durumlarını güncellemek için
  let loadedKey = "";

  onMount(() => {
    let gone = false;
    void ready.then(() => {
      if (!gone) create();
    });
    return () => {
      gone = true;
      editor?.destroy();
    };
  });

  function create() {
    editor = new Editor({
      element: host,
      editorProps: {
        // Ekran görüntüsü (Win+Shift+S, PrtSc) ya da kopyalanan resim Ctrl+V ile nota gelir.
        handlePaste: (_view, event) => {
          const files = imageFiles(event.clipboardData?.files);
          if (!files.length) return false;
          event.preventDefault();
          for (const f of files) void insertFile(f);
          return true;
        },
        handleDrop: (view, event, _slice, moved) => {
          const files = moved ? [] : imageFiles(event.dataTransfer?.files);
          if (!files.length) return false;
          event.preventDefault();
          const pos = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos;
          for (const f of files) void insertFile(f, pos);
          return true;
        },
      },
      extensions: [
        NoteImage,
        StarterKit.configure({ link: false }),
        TaskList,
        TaskItem.configure({ nested: true }),
        Placeholder.configure({ placeholder: "Yazmaya başla… (- [ ] yazınca yapılacak kutusu olur)" }),
        Markdown,
      ],
      content: value,
      contentType: "markdown",
      autofocus: autofocus ? "end" : false,
      onUpdate: ({ editor }) => onchange(editor.getMarkdown()),
      onTransaction: () => tick++,
    });
    loadedKey = docKey;
  }

  // Başka bir nota geçildiyse içeriği değiştir (kaydetme olayı tetiklemeden).
  $effect(() => {
    const key = docKey;
    const md = value;
    if (!editor || key === loadedKey) return;
    loadedKey = key;
    editor.commands.setContent(md, { contentType: "markdown", emitUpdate: false });
  });

  /**
   * Dışarıdan (ör. başka pencerede) değişen içeriği yükler. Şu an bu editörde yazıyorsan
   * dokunmaz ve `false` döner (senin yazdığın kaydedilir).
   */
  export function reload(md: string): boolean {
    if (!editor || editor.isFocused) return false;
    if (editor.getMarkdown() !== md) {
      editor.commands.setContent(md, { contentType: "markdown", emitUpdate: false });
    }
    return true;
  }

  type Tool = { icon: IconName; label: string; run: () => void; active: () => boolean };
  const tools: Tool[] = [
    { icon: "bold", label: "Kalın (Ctrl+B)", run: () => editor?.chain().focus().toggleBold().run(), active: () => !!editor?.isActive("bold") },
    { icon: "italic", label: "İtalik (Ctrl+I)", run: () => editor?.chain().focus().toggleItalic().run(), active: () => !!editor?.isActive("italic") },
    { icon: "heading", label: "Başlık", run: () => editor?.chain().focus().toggleHeading({ level: 2 }).run(), active: () => !!editor?.isActive("heading") },
    { icon: "list", label: "Madde listesi", run: () => editor?.chain().focus().toggleBulletList().run(), active: () => !!editor?.isActive("bulletList") },
    { icon: "checklist", label: "Yapılacaklar", run: () => editor?.chain().focus().toggleTaskList().run(), active: () => !!editor?.isActive("taskList") },
    { icon: "code", label: "Kod", run: () => editor?.chain().focus().toggleCodeBlock().run(), active: () => !!editor?.isActive("codeBlock") },
  ];
</script>

{#if toolbar}
  <div class="toolbar" role="toolbar" aria-label="Biçimlendirme">
    {#each tools as t (t.icon)}
      <button title={t.label} aria-label={t.label} aria-pressed={tick >= 0 && t.active()} onmousedown={(e) => e.preventDefault()} onclick={t.run}>
        <Icon name={t.icon} size={17} />
      </button>
    {/each}
    <button title="Resim ekle (ya da ekran görüntüsünü Ctrl+V ile yapıştır)" aria-label="Resim ekle" onmousedown={(e) => e.preventDefault()} onclick={pickImage}>
      <Icon name="image" size={17} />
    </button>
  </div>
{/if}
<div class="editor" class:hand={font === "hand"} style:font-family={fontOf(font).family} bind:this={host}></div>

<style>
  .toolbar { display: flex; gap: 2px; flex-wrap: wrap; padding-bottom: 8px; border-bottom: 1px solid rgba(255, 255, 255, 0.08); margin-bottom: 8px; }
  .toolbar button { border: 0; background: transparent; color: var(--muted); padding: 6px 7px; border-radius: 7px; display: grid; place-items: center; }
  .toolbar button:hover { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .toolbar button[aria-pressed="true"] { color: var(--gold-glow); background: rgba(227, 194, 112, 0.14); }
  .editor { flex: 1; min-height: 0; overflow: auto; font-size: 15px; line-height: 1.6; color: #e6e1d4; }
  .editor.hand { font-size: 21px; line-height: 1.4; }
  .editor :global(.tiptap) { outline: none; min-height: 100%; padding-bottom: 24px; }
  .editor :global(.tiptap > * + *) { margin-top: 0.55em; }
  .editor :global(p) { margin: 0; }
  .editor :global(h1), .editor :global(h2), .editor :global(h3) { font-family: var(--f-display); margin: 0.6em 0 0.2em; line-height: 1.25; }
  .editor :global(h2) { font-size: 1.3em; }
  .editor :global(ul), .editor :global(ol) { margin: 0; padding-left: 1.3em; }
  .editor :global(code) { font-family: "JetBrains Mono", ui-monospace, monospace; font-size: 0.88em; background: rgba(0, 0, 0, 0.3); padding: 1px 5px; border-radius: 5px; }
  .editor :global(pre) { background: rgba(0, 0, 0, 0.35); padding: 10px 12px; border-radius: 10px; overflow-x: auto; }
  .editor :global(pre code) { background: none; padding: 0; }
  .editor :global(blockquote) { margin: 0; padding-left: 12px; border-left: 3px solid var(--gold-dim); color: var(--muted); }
  .editor :global(ul[data-type="taskList"]) { list-style: none; padding-left: 2px; }
  .editor :global(ul[data-type="taskList"] li) { display: flex; gap: 10px; align-items: flex-start; }
  .editor :global(ul[data-type="taskList"] li > label) { margin-top: 0.22em; flex: none; }
  .editor :global(ul[data-type="taskList"] li > div) { flex: 1; min-width: 0; }
  .editor :global(ul[data-type="taskList"] input[type="checkbox"]) { accent-color: var(--gold); width: 16px; height: 16px; cursor: pointer; }
  .editor :global(li[data-checked="true"] > div) { color: var(--dim); text-decoration: line-through; }
  .editor :global(img) { display: block; max-width: 100%; max-height: 520px; height: auto; margin: 6px 0; border-radius: 10px; border: 1px solid rgba(255, 255, 255, 0.08); cursor: default; }
  .editor :global(img.ProseMirror-selectednode) { outline: 2px solid var(--gold); outline-offset: 2px; }
  .editor :global(.is-editor-empty:first-child::before) { content: attr(data-placeholder); color: var(--dim); float: left; height: 0; pointer-events: none; }
</style>
