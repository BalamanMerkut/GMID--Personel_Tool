// Fırsatlar'da görülmemiş bir şey var mı: yeni ücretsiz oyun ya da istek listesinde yeni / daha büyük indirim.
// Kenar menüdeki kırmızı balonu besler; Fırsatlar ekranı açılınca görülmüş sayılır.
import { api } from "./api";
import { on } from "./bus";
import { getJson, setJson } from "./settings.svelte";
import type { DealsSnapshot } from "./types";

const KEY = "deals.viewed";

export const dealsBadge = $state({ count: 0, wish: 0 });
let keys: string[] = [];
let started = false;

const keysOf = (s: DealsSnapshot) => [
  ...s.free.filter((g) => g.store !== "mobile").map((g) => `f:${g.id}`),
  ...s.wishlist.map((d) => `w:${d.gameId}:${d.discount}`),
];

function update(s: DealsSnapshot) {
  keys = keysOf(s);
  const seen = getJson<string[] | null>(KEY, null);
  // İlk açılışta ücretsizlerin hepsi "yeni" sayılmasın; istek listesindeki indirimler ise gösterilsin (kaçmasın).
  const base = seen ?? keys.filter((k) => k.startsWith("f:"));
  if (seen === null) void setJson(KEY, base);
  const fresh = keys.filter((k) => !base.includes(k));
  dealsBadge.count = fresh.length;
  dealsBadge.wish = fresh.filter((k) => k.startsWith("w:")).length;
}

export function markDealsSeen() {
  dealsBadge.count = 0;
  dealsBadge.wish = 0;
  void setJson(KEY, keys);
}

export function startDealsBadge() {
  if (started) return;
  started = true;
  void api.deals.get().then(update).catch(() => {});
  void on<DealsSnapshot>("deals", update);
}
