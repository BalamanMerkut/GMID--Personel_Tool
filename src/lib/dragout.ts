// Kartı basılı tutup sürükleyince dosyayı uygulamanın dışına (Masaüstü, klasör, Discord…) taşır; bırakılan yere
// kopyalanır. Kısa tıklama eskisi gibi çalışır; sürükleme olunca ardından gelen tıklama yutulur.
import { api, inTauri } from "./api";
import { notify } from "./toast.svelte";

/** Bu kadar piksel kayınca sürükleme başlar (titrek tıklama sürükleme sayılmasın). */
const THRESHOLD = 8;

export function dragOut(node: HTMLElement, getPath: () => string | null | undefined) {
  let get = getPath;
  let down = false;
  let dragged = false;
  let sx = 0;
  let sy = 0;

  function onDown(e: MouseEvent) {
    dragged = false;
    if (!inTauri || e.button !== 0) return;
    // Düğmeler, menüler, oynatıcı denetimleri kendi işini yapsın.
    if ((e.target as HTMLElement).closest("button, a, input, select, textarea, [contenteditable], video[controls]")) return;
    down = true;
    sx = e.clientX;
    sy = e.clientY;
    // Tarayıcının kendi resim sürüklemesi ve yazı seçimi başlamasın.
    e.preventDefault();
  }
  function onMove(e: MouseEvent) {
    if (!down) return;
    if (!(e.buttons & 1)) {
      down = false;
      return;
    }
    if (Math.hypot(e.clientX - sx, e.clientY - sy) < THRESHOLD) return;
    down = false;
    const path = get();
    if (!path) return;
    dragged = true;
    api.dragOut([path]).catch((err) => notify(String(err), "info"));
  }
  function onUp() {
    down = false;
  }
  function onClick(e: MouseEvent) {
    if (!dragged) return;
    dragged = false;
    e.stopPropagation();
    e.preventDefault();
  }

  node.addEventListener("mousedown", onDown);
  node.addEventListener("click", onClick, true);
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", onUp);
  return {
    update(next: () => string | null | undefined) {
      get = next;
    },
    destroy() {
      node.removeEventListener("mousedown", onDown);
      node.removeEventListener("click", onClick, true);
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    },
  };
}
