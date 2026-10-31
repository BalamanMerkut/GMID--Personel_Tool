// Sesle yazma GMID'in kendi penceresindeyken (Notlar, Dene kutusu, yapışkan not): yazı Ctrl+V ile değil,
// odaktaki yazı alanına doğrudan konur. Yalnız odaktaki pencere yazar.
import { on } from "./bus";

/** Yazıyı odaktaki alana imlecin olduğu yere koyar; yazılabilir bir alan yoksa `false`. */
export function insertText(text: string): boolean {
  const el = document.activeElement as HTMLElement | null;
  if (!el) return false;
  const typed = el instanceof HTMLInputElement && ["text", "search", "url", "email", ""].includes(el.type);
  if (el instanceof HTMLTextAreaElement || typed) {
    const f = el as HTMLInputElement | HTMLTextAreaElement;
    if (f.readOnly || f.disabled) return false;
    const start = f.selectionStart ?? f.value.length;
    const end = f.selectionEnd ?? start;
    // Öncesinde boşluk yoksa araya bir boşluk (yeni yazı önceki sözcüğe yapışmasın).
    const before = f.value.slice(0, start);
    const add = before && !/\s$/.test(before) ? ` ${text}` : text;
    f.setRangeText(add, start, end, "end");
    f.dispatchEvent(new Event("input", { bubbles: true }));
    return true;
  }
  if (el.isContentEditable) {
    // Zengin yazı alanı (not düzenleyici): tarayıcının kendi "yazı ekle" komutu, geri al ile de geri alınır.
    return document.execCommand("insertText", false, text);
  }
  return false;
}

export function listenDictationInsert(): void {
  void on<string>("dictation-insert", (text) => {
    if (document.hasFocus()) insertText(text);
  });
}
