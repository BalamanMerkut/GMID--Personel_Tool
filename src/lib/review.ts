// Oyun Arşivi ve Film & Dizi Arşivi puanlama kartı: sorular, ağırlıklar ve etiketler
// (Rust tarafındaki `reviews.rs` ile aynı hesap).
import type { Recommend, ReviewStatus, Scores } from "./types";

export type Kind = "game" | "title";
export interface Question {
  key: string;
  icon: string;
  label: string;
  hint: string;
  weight: number;
  /** "Yok" seçeneği var mı. */
  none: boolean;
}

export const QUESTIONS: Question[] = [
  { key: "gameplay", icon: "🎮", label: "Oynaması ne kadar zevkli?", hint: "Eğlence, kontroller, zorluk dengesi, içerik", weight: 35, none: false },
  { key: "story", icon: "📖", label: "Hikâye ve atmosfer nasıl?", hint: "Senaryo, karakterler, dünya", weight: 20, none: true },
  { key: "enjoyment", icon: "❤️", label: "Seni ne kadar sardı, aklında kaldı mı?", hint: "Tekrar oynar mısın", weight: 20, none: false },
  { key: "visuals", icon: "🎨", label: "Görsellik ve ses nasıl?", hint: "Sanat tarzı, grafik, müzik", weight: 15, none: false },
  { key: "performance", icon: "⚙️", label: "Performans / optimizasyon nasıl?", hint: "FPS, hata, çökme", weight: 10, none: true },
];

/** Film ve dizi soruları. */
export const TITLE_QUESTIONS: Question[] = [
  { key: "story", icon: "📖", label: "Hikâye ve senaryo nasıl?", hint: "Konu, diyaloglar, sonun tatmin ediciliği", weight: 25, none: false },
  { key: "acting", icon: "🎭", label: "Oyunculuk nasıl?", hint: "İnandırıcılık, karakterler", weight: 20, none: false },
  { key: "direction", icon: "🎬", label: "Yönetmenlik ve görsellik nasıl?", hint: "Görüntü, müzik, atmosfer", weight: 20, none: false },
  { key: "pace", icon: "⏱️", label: "Tempo nasıl?", hint: "Sıktı mı, süresini hak etti mi", weight: 15, none: false },
  { key: "enjoyment", icon: "❤️", label: "Sende ne bıraktı?", hint: "Keyif, etki, tekrar izler misin", weight: 20, none: false },
];

export const questionsOf = (kind: Kind) => (kind === "title" ? TITLE_QUESTIONS : QUESTIONS);

export const GUIDE = "5 vasat · 7 iyi · 8 çok iyi · 9 mükemmel · 10 başyapıt";

/** "Yok" denen soruların etkisi diğerlerine oransal dağılır. */
export function autoScore(s: Scores, kind: Kind = "game"): number | null {
  let sum = 0;
  let weight = 0;
  for (const q of questionsOf(kind)) {
    const v = s[q.key];
    if (v == null) continue;
    sum += Math.min(10, Math.max(1, v)) * q.weight;
    weight += q.weight;
  }
  return weight ? Math.round((sum / weight) * 10) / 10 : null;
}

export function overallOf(s: Scores, adjust: number, kind: Kind = "game"): number | null {
  const a = autoScore(s, kind);
  if (a == null) return null;
  return Math.round(Math.min(10, Math.max(1, a + Math.max(-1, Math.min(1, adjust)))) * 10) / 10;
}

export const STATUS: { id: ReviewStatus; label: string }[] = [
  { id: "playing", label: "Oynuyorum" },
  { id: "finished", label: "Bitirdim" },
  { id: "completed", label: "%100" },
  { id: "dropped", label: "Bıraktım" },
  { id: "wishlist", label: "İstiyorum" },
];

/** Film ve dizi durumları. */
export const TITLE_STATUS: { id: ReviewStatus; label: string }[] = [
  { id: "watching", label: "İzliyorum" },
  { id: "watched", label: "İzledim" },
  { id: "dropped", label: "Yarıda bıraktım" },
  { id: "watchlist", label: "İzlenecek" },
];

export const statusesOf = (kind: Kind) => (kind === "title" ? TITLE_STATUS : STATUS);
/** Henüz oynanmadı/izlenmedi: puan sorulmaz. */
export const unratedOf = (kind: Kind): ReviewStatus => (kind === "title" ? "watchlist" : "wishlist");

export const RECOMMEND: { id: Recommend; icon: string; label: string }[] = [
  { id: "yes", icon: "👍", label: "Kesin oyna" },
  { id: "maybe", icon: "🤷", label: "Duruma göre" },
  { id: "no", icon: "👎", label: "Uzak dur" },
];

export const statusLabel = (s: ReviewStatus | null) => [...STATUS, ...TITLE_STATUS].find((x) => x.id === s)?.label ?? null;
/** Filmde tavsiye sözcükleri. */
export const recommendLabel = (r: Recommend, kind: Kind) =>
  kind === "title" ? ({ yes: "Kesin izle", maybe: "Duruma göre", no: "Vakit kaybı" } as const)[r] : RECOMMEND.find((x) => x.id === r)!.label;
/** Nerede izlendi önerileri. */
export const WHERE = ["Sinema", "Netflix", "Prime Video", "Disney+", "HBO Max", "YouTube", "Televizyon", "Bilgisayar"];
export const recommendOf = (r: Recommend | null) => RECOMMEND.find((x) => x.id === r) ?? null;

/** Puanın tonu: 9+ altın, 7+ yeşil, 5+ amber, altı kırmızımsı. */
export function scoreTone(v: number | null): "gold" | "good" | "mid" | "low" | "none" {
  if (v == null) return "none";
  return v >= 9 ? "gold" : v >= 7 ? "good" : v >= 5 ? "mid" : "low";
}

export const fmtScore = (v: number | null) => v == null ? "—" : (Number.isInteger(v) ? `${v}` : v.toFixed(1)).replace(".", ",");

/** Notlarda `||spoiler||` → gizli parça. */
export function spoilerParts(text: string): { text: string; spoiler: boolean }[] {
  const out: { text: string; spoiler: boolean }[] = [];
  const re = /\|\|([\s\S]+?)\|\|/g;
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push({ text: text.slice(last, m.index), spoiler: false });
    out.push({ text: m[1], spoiler: true });
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push({ text: text.slice(last), spoiler: false });
  return out;
}
