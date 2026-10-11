// Uygulama genelindeki oynatıcı: ekran değişse de müzik/video çalmaya devam eder.
// Açıkken bütün pencereyi kaplar; küçültülünce uygulamanın altında çalma çubuğu olur.
import type { Download } from "./types";

export const player = $state<{ item: Download | null; queue: Download[]; open: boolean }>({
  item: null,
  queue: [],
  open: false,
});

/** `queue`: sıradakiler (önceki/sonraki ve liste görünümü için). */
export function play(item: Download, queue: Download[]) {
  player.queue = queue.some((d) => d.id === item.id) ? queue : [item, ...queue];
  player.item = item;
  player.open = true;
}

export function stopPlayer() {
  player.item = null;
  player.open = false;
}
