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
    placeholder = "Yazmaya başla… (- [ ] yazınca yapılacak kutusu olur)",
    onchange,
  }: {
    /** Hangi not açık; değişince içerik yeniden yüklenir. */
    docKey: string;
    value: string;
    font?: string;
    toolbar?: boolean;
    autofocus?: boolean;
    placeholder?: string;
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
        Placeholder.configure({ placeholder }),
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

  type Tool = { icon: IconName; label: string; run: () => void; active?: () => boolean };
  const chain = () => editor!.chain().focus();
  /** Gruplar: başlık · yazı · liste · blok · resim. Sağda geri al / yinele. */
  const groups: Tool[][] = [
    [
      { icon: "h1", label: "Büyük başlık", run: () => chain().toggleHeading({ level: 1 }).run(), active: () => !!editor?.isActive("heading", { level: 1 }) },
      { icon: "h2", label: "Alt başlık", run: () => chain().toggleHeading({ level: 2 }).run(), active: () => !!editor?.isActive("heading", { level: 2 }) },
    ],
    [
      { icon: "bold", label: "Kalın (Ctrl+B)", run: () => chain().toggleBold().run(), active: () => !!editor?.isActive("bold") },
      { icon: "italic", label: "İtalik (Ctrl+I)", run: () => chain().toggleItalic().run(), active: () => !!editor?.isActive("italic") },
      { icon: "strike", label: "Üstü çizili", run: () => chain().toggleStrike().run(), active: () => !!editor?.isActive("strike") },
    ],
    [
      { icon: "list", label: "Madde listesi", run: () => chain().toggleBulletList().run(), active: () => !!editor?.isActive("bulletList") },
      { icon: "olist", label: "Numaralı liste", run: () => chain().toggleOrderedList().run(), active: () => !!editor?.isActive("orderedList") },
      { icon: "checklist", label: "Yapılacaklar", run: () => chain().toggleTaskList().run(), active: () => !!editor?.isActive("taskList") },
    ],
    [
      { icon: "quote", label: "Alıntı / önemli not", run: () => chain().toggleBlockquote().run(), active: () => !!editor?.isActive("blockquote") },
      { icon: "code", label: "Kod", run: () => chain().toggleCodeBlock().run(), active: () => !!editor?.isActive("codeBlock") },
      { icon: "divider", label: "Ayırıcı çizgi", run: () => chain().setHorizontalRule().run() },
    ],
  ];
  const history: Tool[] = [
    { icon: "undo", label: "Geri al (Ctrl+Z)", run: () => chain().undo().run() },
    { icon: "redo", label: "Yinele (Ctrl+Y)", run: () => chain().redo().run() },
  ];
</script>

{#if toolbar}
  <div class="toolbar" role="toolbar" aria-label="Biçimlendirme">
    {#each groups as g, gi (gi)}
      <div class="grp">
        {#each g as t (t.icon)}
          <button title={t.label} aria-label={t.label} aria-pressed={tick >= 0 && !!t.active?.()} onmousedown={(e) => e.preventDefault()} onclick={t.run}>
            <Icon name={t.icon} size={17} />
          </button>
        {/each}
      </div>
    {/each}
    <div class="grp">
      <button class="img-btn" title="Resim ekle (ekran görüntüsünü Ctrl+V ile de yapıştırabilirsin)" aria-label="Resim ekle" onmousedown={(e) => e.preventDefault()} onclick={pickImage}>
        <Icon name="image" size={17} /><span>Resim</span>
      </button>
    </div>
    <div class="grp end">
      {#each history as t (t.icon)}
        <button title={t.label} aria-label={t.label} disabled={tick >= 0 && !(t.icon === "undo" ? editor?.can().undo() : editor?.can().redo())} onmousedown={(e) => e.preventDefault()} onclick={t.run}>
          <Icon name={t.icon} size={17} />
        </button>
      {/each}
    </div>
  </div>
{/if}
<div class="editor" class:hand={font === "hand"} style:font-family={fontOf(font).family} bind:this={host}></div>

<style>
  .toolbar { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; padding: 6px; margin-bottom: 12px; border-radius: 12px; background: rgba(0, 0, 0, 0.22); border: 1px solid rgba(255, 255, 255, 0.06); }
  .grp { display: flex; gap: 1px; padding-right: 6px; border-right: 1px solid rgba(255, 255, 255, 0.08); }
  .grp:last-child, .grp.end { border-right: 0; padding-right: 0; }
  .grp.end { margin-left: auto; }
  .toolbar button { border: 0; background: transparent; color: var(--muted); padding: 6px 7px; border-radius: 8px; display: inline-flex; align-items: center; gap: 6px; font: inherit; font-size: 12.5px; font-weight: 600; }
  .toolbar button:hover:not(:disabled) { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .toolbar button[aria-pressed="true"] { color: var(--gold-glow); background: rgba(227, 194, 112, 0.16); }
  .toolbar button:disabled { opacity: 0.35; cursor: default; }
  .img-btn span { padding-right: 2px; }
  .editor { flex: 1; min-height: 0; overflow: auto; font-size: 15px; line-height: 1.6; color: #e6e1d4; }
  .editor.hand { font-size: 21px; line-height: 1.4; }
  .editor :global(.tiptap) { outline: none; min-height: 100%; padding-bottom: 24px; }
  .editor :global(.tiptap > * + *) { margin-top: 0.55em; }
  .editor :global(p) { margin: 0; }
  .editor :global(h1), .editor :global(h2), .editor :global(h3) { font-family: var(--f-display); margin: 0.6em 0 0.2em; line-height: 1.25; }
  .editor :global(h1) { font-size: 1.55em; }
  .editor :global(h2) { font-size: 1.25em; color: var(--gold-glow); }
  .editor :global(hr) { border: 0; border-top: 1px dashed rgba(255, 255, 255, 0.18); margin: 1em 0; }
  .editor :global(blockquote) { margin: 0; background: rgba(227, 194, 112, 0.07); border-left: 3px solid var(--gold); border-radius: 0 10px 10px 0; padding: 6px 12px; }
  .editor :global(ul), .editor :global(ol) { margin: 0; padding-left: 1.3em; }
  .editor :global(code) { font-family: "JetBrains Mono", ui-monospace, monospace; font-size: 0.88em; background: rgba(0, 0, 0, 0.3); padding: 1px 5px; border-radius: 5px; }
  .editor :global(pre) { background: rgba(0, 0, 0, 0.35); padding: 10px 12px; border-radius: 10px; overflow-x: auto; }
  .editor :global(pre code) { background: none; padding: 0; }
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
