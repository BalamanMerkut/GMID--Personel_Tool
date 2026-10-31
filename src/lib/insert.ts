// Sesle yazma GMID'in kendi penceresindeyken (Notlar, Dene kutusu, yapışkan not): yazı Ctrl+V ile değil,
// odaktaki yazı alanına doğrudan konur. Yazı yalnız konuşulan pencereye gider; dikte göstergesi açılırken
// odak kaymış olsa da en son tıklanan yazı alanına geri dönülür.
import { on } from "./bus";

type Field = HTMLInputElement | HTMLTextAreaElement;

function writable(el: Element | null): el is HTMLElement {
  if (!(el instanceof HTMLElement)) return false;
  if (el instanceof HTMLTextAreaElement) return !el.readOnly && !el.disabled;
  if (el instanceof HTMLInputElement) return ["text", "search", "url", "email", ""].includes(el.type) && !el.readOnly && !el.disabled;
  return el.isContentEditable;
}

/** Yazıyı alana imlecin olduğu yere koyar; yazılabilir değilse `false`. */
export function insertText(text: string, target: Element | null = document.activeElement): boolean {
  if (!writable(target)) return false;
  if (target instanceof HTMLTextAreaElement || target instanceof HTMLInputElement) {
    const f: Field = target;
    const start = f.selectionStart ?? f.value.length;
    const end = f.selectionEnd ?? start;
    // Öncesinde boşluk yoksa araya bir boşluk (yeni yazı önceki sözcüğe yapışmasın).
    const before = f.value.slice(0, start);
    const add = before && !/\s$/.test(before) ? ` ${text}` : text;
    f.setRangeText(add, start, end, "end");
    f.dispatchEvent(new Event("input", { bubbles: true }));
    return true;
  }
  // Zengin yazı alanı (not düzenleyici): tarayıcının kendi "yazı ekle" komutu, geri al ile de geri alınır.
  if (document.activeElement !== target) target.focus();
  const sel = window.getSelection();
  const node = sel?.anchorNode;
  const prev = node?.nodeType === Node.TEXT_NODE ? (node.textContent ?? "").slice(0, sel!.anchorOffset) : "";
  return document.execCommand("insertText", false, prev && !/\s$/.test(prev) ? ` ${text}` : text);
}

/** En son odaklanılan yazı alanı (odak gösterge ya da pencere değişimiyle kaçarsa oraya dönülür). */
let last: HTMLElement | null = null;

export function listenDictationInsert(label: string): void {
  document.addEventListener("focusin", (e) => {
    const el = e.target as Element | null;
    // Zengin düzenleyicinin içindeki bir öğe odaklanırsa düzenleyicinin kendisi.
    const field = el instanceof HTMLElement && el.isContentEditable ? (el.closest("[contenteditable=true]") as HTMLElement | null) ?? el : el;
    if (writable(field)) last = field;
  });
  void on<{ text: string; window: string } | string>("dictation-insert", (p) => {
    const { text, window: to } = typeof p === "string" ? { text: p, window: label } : p;
    if (to !== label || !text) return;
    const active = document.activeElement;
    const target = writable(active) ? active : last?.isConnected ? last : null;
    if (!target) return;
    if (document.activeElement !== target) target.focus();
    insertText(text, target);
  });
}
