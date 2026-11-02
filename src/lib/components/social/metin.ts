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
  if (o.tur === "ozet") return "haftalık özet";
  if (o.tur === "turnuva") return o.veri.durum === "bitti" ? "turnuvası bitti" : "turnuva açtı";
  if (o.tur === "gece") return o.veri.hemen ? "oyuncu arıyor" : "oyun gecesi planladı";
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

export interface Parca {
  t: string;
  etiket: boolean;
  /** @anma: anılan üyenin kimliği. */
  anma?: string;
}

const HARF = /[\p{L}\p{N}_]/u;

/**
 * Yazıyı düz parçalara, #etiketlere ve @anmalara böler (sunucudaki kurallarla aynı). @anma için bilinen üye adları
 * verilir; ad birden çok kelime olabilir ("@Ali Veli"), en uzun eşleşen seçilir.
 */
export function parcala(metin: string, uyeler: { id: string; ad: string }[] = []): Parca[] {
  const l: Parca[] = [];
  const adlar = uyeler.filter((u) => u.ad.trim()).slice().sort((a, b) => b.ad.length - a.ad.length);
  const kucuk = metin.toLocaleLowerCase("tr");
  let duz = "";
  let i = 0;
  const bas = () => {
    if (duz) l.push({ t: duz, etiket: false });
    duz = "";
  };
  while (i < metin.length) {
    const c = metin[i];
    const onceHarf = i > 0 && HARF.test(metin[i - 1]);
    if (c === "#" && !onceHarf) {
      const m = /^#([\p{L}\p{N}_]{2,30})(?![\p{L}\p{N}_])/u.exec(metin.slice(i));
      if (m && !/^\d+$/.test(m[1])) {
        bas();
        l.push({ t: m[0], etiket: true });
        i += m[0].length;
        continue;
      }
    }
    if (c === "@" && !onceHarf) {
      const u = adlar.find((u) => {
        const ad = u.ad.toLocaleLowerCase("tr");
        if (!kucuk.startsWith(ad, i + 1)) return false;
        const sonraki = metin[i + 1 + u.ad.length];
        return !sonraki || !HARF.test(sonraki);
      });
      if (u) {
        bas();
        l.push({ t: metin.slice(i, i + 1 + u.ad.length), etiket: false, anma: u.id });
        i += 1 + u.ad.length;
        continue;
      }
    }
    duz += c;
    i++;
  }
  bas();
  return l;
}

const GUN_ADI = new Intl.DateTimeFormat("tr-TR", { weekday: "long" });
const GUN_AY = new Intl.DateTimeFormat("tr-TR", { day: "numeric", month: "long" });
const SAAT = new Intl.DateTimeFormat("tr-TR", { hour: "2-digit", minute: "2-digit" });

/** Oyun gecesinin zamanı: { gun: "Bugün" | "Yarın" | "Cuma", tarih: "7 Kasım", saat: "21:00" }. */
export function geceZamani(ms: number): { gun: string; tarih: string; saat: string } {
  const d = new Date(ms);
  const bugun = new Date();
  bugun.setHours(0, 0, 0, 0);
  const fark = Math.round((new Date(d).setHours(0, 0, 0, 0) - bugun.getTime()) / 86400000);
  const gun = fark === 0 ? "Bugün" : fark === 1 ? "Yarın" : GUN_ADI.format(d).replace(/^./, (c) => c.toLocaleUpperCase("tr"));
  return { gun, tarih: GUN_AY.format(d), saat: SAAT.format(d) };
}

/** "45 dk sonra", "3 sa sonra", "şimdi oynanıyor", "bitti". */
export function geceKalan(ms: number): string {
  const dk = Math.round((ms - Date.now()) / 60000);
  if (dk > 48 * 60) return `${Math.round(dk / 1440)} gün sonra`;
  if (dk > 90) return `${Math.round(dk / 60)} sa sonra`;
  if (dk > 0) return `${dk} dk sonra`;
  if (dk > -180) return "şimdi oynanıyor";
  return "bitti";
}
