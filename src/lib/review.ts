// Oyun Arşivi puanlama kartı: sorular, ağırlıklar ve etiketler (Rust tarafındaki `reviews.rs` ile aynı hesap).
import type { Recommend, ReviewStatus, Scores } from "./types";

export const QUESTIONS: { key: keyof Scores; icon: string; label: string; hint: string; weight: number; none: boolean }[] = [
  { key: "gameplay", icon: "🎮", label: "Oynaması ne kadar zevkli?", hint: "Eğlence, kontroller, zorluk dengesi, içerik", weight: 35, none: false },
  { key: "story", icon: "📖", label: "Hikâye ve atmosfer nasıl?", hint: "Senaryo, karakterler, dünya", weight: 20, none: true },
  { key: "enjoyment", icon: "❤️", label: "Seni ne kadar sardı, aklında kaldı mı?", hint: "Tekrar oynar mısın", weight: 20, none: false },
  { key: "visuals", icon: "🎨", label: "Görsellik ve ses nasıl?", hint: "Sanat tarzı, grafik, müzik", weight: 15, none: false },
  { key: "performance", icon: "⚙️", label: "Performans / optimizasyon nasıl?", hint: "FPS, hata, çökme", weight: 10, none: true },
];

export const GUIDE = "5 vasat · 7 iyi · 8 çok iyi · 9 mükemmel · 10 başyapıt";

/** "Yok" denen soruların etkisi diğerlerine oransal dağılır. */
export function autoScore(s: Scores): number | null {
  let sum = 0;
  let weight = 0;
  for (const q of QUESTIONS) {
    const v = s[q.key];
    if (v == null) continue;
    sum += Math.min(10, Math.max(1, v)) * q.weight;
    weight += q.weight;
  }
  return weight ? Math.round((sum / weight) * 10) / 10 : null;
}

export function overallOf(s: Scores, adjust: number): number | null {
  const a = autoScore(s);
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

export const RECOMMEND: { id: Recommend; icon: string; label: string }[] = [
  { id: "yes", icon: "👍", label: "Kesin oyna" },
  { id: "maybe", icon: "🤷", label: "Duruma göre" },
  { id: "no", icon: "👎", label: "Uzak dur" },
];

export const statusLabel = (s: ReviewStatus | null) => STATUS.find((x) => x.id === s)?.label ?? null;
export const recommendOf = (r: Recommend | null) => RECOMMEND.find((x) => x.id === r) ?? null;

/** Puanın tonu: 9+ altın, 7+ yeşil, 5+ amber, altı kırmızımsı. */
export function scoreTone(v: number): "gold" | "good" | "mid" | "low" {
  return v >= 9 ? "gold" : v >= 7 ? "good" : v >= 5 ? "mid" : "low";
}

export const fmtScore = (v: number) => (Number.isInteger(v) ? `${v}` : v.toFixed(1)).replace(".", ",");

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
