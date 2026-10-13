// Çark & Çekiliş: adil rastgelelik ve çarkın geometrisi (ekrandan bağımsız, sınanabilir).

/** 0..n-1 arası eşit olasılıklı tam sayı (kripto rastgele, modülo yanlılığı yok). */
export function randInt(n: number): number {
  if (n <= 1) return 0;
  const limit = Math.floor(0x1_0000_0000 / n) * n;
  const buf = new Uint32Array(1);
  for (;;) {
    crypto.getRandomValues(buf);
    if (buf[0] < limit) return buf[0] % n;
  }
}

/** 0..1 arası rastgele (çarkın dilim içindeki duruş yeri gibi önemsiz yerler için). */
export const randFloat = () => randInt(1_000_000) / 1_000_000;

/** Fisher–Yates: yeni, karışık bir dizi. */
export function shuffle<T>(items: readonly T[]): T[] {
  const a = [...items];
  for (let i = a.length - 1; i > 0; i--) {
    const j = randInt(i + 1);
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

/** `count` tane farklı öğe seçer (sırası da rastgele). */
export const pickMany = <T>(items: readonly T[], count: number): T[] => shuffle(items).slice(0, Math.max(0, count));

/** Takımlara böler: sayılar en fazla bir farklı olur (7 kişi, 2 takım → 4 + 3). */
export function teams<T>(items: readonly T[], k: number): T[][] {
  const n = Math.max(1, Math.min(k, items.length || 1));
  const out: T[][] = Array.from({ length: n }, () => []);
  shuffle(items).forEach((x, i) => out[i % n].push(x));
  return out;
}

// ---- çark ----
// Açılar derece; 0 = tepe (ibre), saat yönünde artar. Çark `rotation` derece saat yönünde döner.

/** İbrenin gösterdiği dilim. */
export function sliceAt(rotation: number, count: number): number {
  if (count <= 0) return -1;
  const step = 360 / count;
  const a = (((-rotation % 360) + 360) % 360);
  return Math.min(count - 1, Math.floor(a / step));
}

/**
 * `target` dilimi ibrenin altında duracak şekilde son açıyı hesaplar: şu anki açıdan en az `turns` tam tur
 * ileride; dilimin kenarına değil, içinde `offset` (0..1) oranındaki bir yere denk gelir.
 */
export function landing(rotation: number, count: number, target: number, turns: number, offset: number): number {
  const step = 360 / count;
  const inSlice = 0.15 + 0.7 * Math.min(1, Math.max(0, offset));
  const a = (target + inSlice) * step; // ibrenin altında olacak çark açısı
  const base = rotation + 360 * turns;
  const want = (((-a - base) % 360) + 360) % 360;
  return base + want;
}

/** Yavaşlayarak durma (dörtlü). */
export const easeOut = (t: number) => 1 - Math.pow(1 - Math.min(1, Math.max(0, t)), 4);

/** Dilim renkleri (malikâne teması); komşu iki dilim (son ve ilk dahil) aynı renk olmaz. */
export const WHEEL_COLORS = ["#e3c270", "#35a0a2", "#7848a8", "#c0574f", "#3f7cc0", "#5cc08a", "#d98f3e", "#a0507a"];
/** Açık renkli dilimlerde yazı koyu olur. */
const LIGHT = new Set(["#e3c270", "#5cc08a", "#d98f3e"]);

export function sliceColor(i: number, count: number): string {
  const p = WHEEL_COLORS.length;
  let c = i % p;
  // Son dilim ilkiyle aynı renge düşüyorsa (ör. 9 dilim) ikinci rengi al (önceki dilim zaten sonuncu renkte).
  if (count > 1 && i === count - 1 && c === 0) c = 1;
  return WHEEL_COLORS[c];
}

export const inkFor = (color: string) => (LIGHT.has(color) ? "#1b1608" : "#fdf8ec");

/** SVG dilim yolu: merkez (c,c), yarıçap r; açılar tepe=0, saat yönü. */
export function slicePath(c: number, r: number, from: number, to: number): string {
  const pt = (deg: number) => {
    const rad = (deg * Math.PI) / 180;
    return `${(c + r * Math.sin(rad)).toFixed(2)} ${(c - r * Math.cos(rad)).toFixed(2)}`;
  };
  if (to - from >= 359.999) return `M ${c} ${c - r} A ${r} ${r} 0 1 1 ${c - 0.01} ${c - r} Z`;
  const large = to - from > 180 ? 1 : 0;
  return `M ${c} ${c} L ${pt(from)} A ${r} ${r} 0 ${large} 1 ${pt(to)} Z`;
}

/** Uzun adları dilime sığdırır. */
export function fit(name: string, max: number): string {
  const t = name.trim();
  return t.length > max ? `${t.slice(0, Math.max(1, max - 1)).trimEnd()}…` : t;
}

/** Yapıştırılan metni öğelere böler: satır satır; tek satırsa virgülle. */
export function splitItems(text: string): string[] {
  const lines = text.split(/\r?\n/).map((s) => s.trim()).filter(Boolean);
  const parts = lines.length === 1 && lines[0].includes(",") ? lines[0].split(",") : lines;
  return parts.map((s) => s.replace(/^\s*(?:\d+[.)-]|[-•*])\s+/, "").trim()).filter(Boolean);
}
