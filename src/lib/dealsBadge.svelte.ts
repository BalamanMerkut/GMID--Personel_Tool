// Fırsatlar'da görülmemiş bir şey var mı: yeni ücretsiz oyun ya da istek listesinde yeni / daha büyük indirim.
// Kenar menüdeki kırmızı balonu ve Fırsatlar'daki YENİ etiketlerini besler. Fırsatlar'a girince görülmüş sayılır:
// o ziyarette yeniler hâlâ işaretli görünür, bir sonraki girişte normal kart olurlar.
import { api } from "./api";
import { on } from "./bus";
import { getJson, setJson } from "./settings.svelte";
import type { DealsSnapshot } from "./types";

const KEY = "deals.viewed";
/** Her fırsatın ilk görüldüğü an (ms; 0: GMID'den önce vardı, bilinmiyor). */
const FIRST = "deals.firstSeen";

export const dealsBadge = $state({
  count: 0,
  wish: 0,
  /** Bu ziyarette YENİ gösterilecekler. */
  visitNew: [] as string[],
  firstSeen: {} as Record<string, number>,
});
let keys: string[] = [];
let fresh: string[] = [];
let started = false;
let onPage = false;

export const freeKey = (id: string) => `f:${id}`;
export const wishKey = (gameId: string, discount: number) => `w:${gameId}:${discount}`;

const keysOf = (s: DealsSnapshot) => [
  ...s.free.filter((g) => g.store !== "mobile").map((g) => freeKey(g.id)),
  ...s.wishlist.map((d) => wishKey(d.gameId, d.discount)),
];

function update(s: DealsSnapshot) {
  keys = keysOf(s);
  // İlk görülme anları: ilk çalıştırmada eldekiler "eski" (0) sayılır, sonra gelenlere o an yazılır.
  const stored = getJson<Record<string, number> | null>(FIRST, null);
  const now = Date.now();
  const first: Record<string, number> = {};
  for (const k of keys) first[k] = stored?.[k] ?? (stored === null ? 0 : now);
  dealsBadge.firstSeen = first;
  if (JSON.stringify(first) !== JSON.stringify(stored)) void setJson(FIRST, first);

  const seen = getJson<string[] | null>(KEY, null);
  // İlk açılışta ücretsizlerin hepsi "yeni" sayılmasın; istek listesindeki indirimler ise gösterilsin (kaçmasın).
  const base = seen ?? keys.filter((k) => k.startsWith("f:"));
  if (seen === null) void setJson(KEY, base);
  fresh = keys.filter((k) => !base.includes(k));
  dealsBadge.count = fresh.length;
  dealsBadge.wish = fresh.filter((k) => k.startsWith("w:")).length;
  // Fırsatlar açıkken gelenler de bu ziyarette YENİ görünür ve görülmüş sayılır.
  if (onPage && fresh.length) markDealsSeen();
}

function markDealsSeen() {
  dealsBadge.visitNew = [...new Set([...dealsBadge.visitNew, ...fresh])];
  fresh = [];
  dealsBadge.count = 0;
  dealsBadge.wish = 0;
  void setJson(KEY, keys);
}

/** Fırsatlar açılınca: görülmemişler bu ziyaret boyunca YENİ, kenar menüdeki balon söner. */
export function enterDeals() {
  onPage = true;
  dealsBadge.visitNew = [];
  markDealsSeen();
}

export function leaveDeals() {
  onPage = false;
  dealsBadge.visitNew = [];
}

export function startDealsBadge() {
  if (started) return;
  started = true;
  void api.deals.get().then(update).catch(() => {});
  void on<DealsSnapshot>("deals", update);
}
