// Topluluk ekranlarının ortak yazı yardımcıları.
import type { Olay } from "../../social.svelte";

export const TUR_AD: Record<string, string> = { game: "oyun", movie: "film", series: "dizi" };

const GUN = new Intl.DateTimeFormat("tr-TR", { day: "numeric", month: "short" });

/** "az önce", "5 dk", "3 sa", "2 gün", "12 Eki" */
export function once(ms: number): string {
  const s = Math.max(0, (Date.now() - ms) / 1000);
  if (s < 60) return "az önce";
  if (s < 3600) return `${Math.floor(s / 60)} dk`;
  if (s < 86400) return `${Math.floor(s / 3600)} sa`;
  if (s < 7 * 86400) return `${Math.floor(s / 86400)} gün`;
  return GUN.format(ms);
}

export const puanYaz = (p: number | null | undefined) =>
  p == null ? null : p.toLocaleString("tr-TR", { maximumFractionDigits: 1 });

/** "bir diziyi puanladı", "bir oyunu bitirdi", "oynamaya başladı" … */
export function eylem(o: Olay): string {
  if (o.tur === "gonderi") return "";
  if (o.tur === "oyun") return "oynamaya başladı";
  if (o.tur === "klip") return o.veri.oyun ? `${o.veri.oyun} klibi paylaştı` : "bir klip paylaştı";
  const v = o.veri;
  const nesne: Record<string, [string, string]> = {
    game: ["bir oyunu", "bir oyun"],
    movie: ["bir filmi", "bir film"],
    series: ["bir diziyi", "bir dizi"],
  };
  const [belirli, yalin] = nesne[v.tur ?? "game"] ?? nesne.game;
  switch (v.durum) {
    case "playing": return `${yalin} oynuyor`;
    case "watching": return `${yalin} izliyor`;
    case "finished": case "completed": return `${belirli} bitirdi`;
    case "watched": return `${belirli} izledi`;
    case "dropped": return `${belirli} bıraktı`;
    case "wishlist": case "watchlist": return `${belirli} listesine ekledi`;
    default: return `${belirli} puanladı`;
  }
}

/** Dakikayı "124 sa" ya da "35 dk" yazar. */
export const saat = (dk: number) => (dk >= 60 ? `${Math.round(dk / 60).toLocaleString("tr-TR")} sa` : `${dk} dk`);

/** Klibin silinmesine kalan süre: "6 gün sonra silinir", "bugün silinir". */
export function kalan(bitis: number): string {
  const ms = bitis - Date.now();
  if (ms <= 86400000) return "bugün silinir";
  if (ms <= 2 * 86400000) return "yarın silinir";
  return `${Math.ceil(ms / 86400000)} gün sonra silinir`;
}

/** Saniyeyi "1:05" yazar. */
export const sure = (sn: number) => `${Math.floor(sn / 60)}:${String(Math.floor(sn % 60)).padStart(2, "0")}`;

/** Yazıyı düz parçalara ve #etiketlere böler (sunucudaki kuralla aynı: önünde harf yok, 2–30 harf/rakam/_). */
export function parcala(metin: string): { t: string; etiket: boolean }[] {
  const l: { t: string; etiket: boolean }[] = [];
  const re = /#([\p{L}\p{N}_]{2,30})(?![\p{L}\p{N}_])/gu;
  let son = 0;
  for (const m of metin.matchAll(re)) {
    const i = m.index ?? 0;
    const onceki = i > 0 ? metin[i - 1] : "";
    if (onceki && /[\p{L}\p{N}_]/u.test(onceki)) continue;
    if (/^\d+$/.test(m[1])) continue;
    if (i > son) l.push({ t: metin.slice(son, i), etiket: false });
    l.push({ t: m[0], etiket: true });
    son = i + m[0].length;
  }
  if (son < metin.length) l.push({ t: metin.slice(son), etiket: false });
  return l;
}
