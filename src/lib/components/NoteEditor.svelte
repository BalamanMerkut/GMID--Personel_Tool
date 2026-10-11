<script lang="ts">
  // Notlar sadece bu bileşenle birlikte yüklenir (TipTap ve not yazı tipleri ayrı parçada).
  import "@fontsource/jetbrains-mono/latin-400.css";
  import "@fontsource/jetbrains-mono/latin-ext-400.css";
  import "@fontsource/caveat/latin-500.css";
  import "@fontsource/caveat/latin-ext-500.css";
  import { Editor } from "@tiptap/core";
  import { TaskItem, TaskList } from "@tiptap/extension-list";
  import { Placeholder } from "@tiptap/extensions";
  import { Markdown } from "@tiptap/markdown";
  import StarterKit from "@tiptap/starter-kit";
  import { onMount } from "svelte";
  import Icon from "../Icon.svelte";
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

  let host: HTMLDivElement;
  let editor: Editor | undefined = $state();
  let tick = $state(0); // araç çubuğu durumlarını güncellemek için
  let loadedKey = "";

  onMount(() => {
    editor = new Editor({
      element: host,
      extensions: [
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
    return () => editor?.destroy();
  });

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
  .editor :global(.is-editor-empty:first-child::before) { content: attr(data-placeholder); color: var(--dim); float: left; height: 0; pointer-events: none; }
</style>
