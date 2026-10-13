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

// ---- dengeli takımlar ----

/** Seviyeli kişileri takımlara böler: kişi sayıları en çok bir, güç toplamları olabildiğince yakın.
 *  Aynı seviyedekilerin sırası rastgele (her seferinde başka bir eşit dağılım). */
export function balancedTeams(people: readonly { name: string; level: number }[], k: number): { members: string[]; power: number }[] {
  const n = Math.max(1, Math.min(k, people.length || 1));
  const cap = Math.ceil(people.length / n);
  const out = Array.from({ length: n }, () => ({ members: [] as { name: string; level: number }[], power: 0 }));
  const sorted = shuffle(people).sort((a, b) => b.level - a.level);
  for (const p of sorted) {
    const open = out.filter((t) => t.members.length < cap);
    const low = Math.min(...open.map((t) => t.power));
    const fewest = Math.min(...open.filter((t) => t.power === low).map((t) => t.members.length));
    const pick = open.filter((t) => t.power === low && t.members.length === fewest);
    const t = pick[randInt(pick.length)];
    t.members.push(p);
    t.power += p.level;
  }
  // Takas iyileştirmesi: en güçlü ile en zayıf arasında farkı azaltan bir takas varsa yap.
  for (let round = 0; round < 50; round++) {
    out.sort((a, b) => b.power - a.power);
    const [hi, lo] = [out[0], out[out.length - 1]];
    const gap = hi.power - lo.power;
    let best = null as [number, number, number] | null;
    for (let i = 0; i < hi.members.length; i++) {
      for (let j = 0; j < lo.members.length; j++) {
        const d = hi.members[i].level - lo.members[j].level;
        const next = Math.abs(gap - 2 * d);
        if (d > 0 && next < gap && (!best || next < best[2])) best = [i, j, next];
      }
    }
    if (!best) break;
    const [i, j] = best;
    const x = hi.members[i];
    const y = lo.members[j];
    hi.members[i] = y;
    lo.members[j] = x;
    hi.power += y.level - x.level;
    lo.power += x.level - y.level;
  }
  return shuffle(out).map((t) => ({ members: shuffle(t.members).map((m) => m.name), power: t.power }));
}

// ---- turnuva ağacı ----

/** Bir maç: `null` ad = bay (rakipsiz geçer) ya da henüz belli değil. */
export type Match = { a: string | null; b: string | null; winner: string | null };

/** Eleme turnuvası: kişi sayısı 2'nin kuvveti değilse ilk turda bazılarına bay geçer. */
export function bracket(names: readonly string[]): Match[][] {
  const players = shuffle(names);
  let size = 2;
  while (size < players.length) size *= 2;
  const m = size / 2;
  const byes = size - players.length;
  const first: Match[] = [];
  for (let i = 0; i < m; i++) {
    const a = players[i] ?? null;
    const b = i < byes ? null : (players[m + i - byes] ?? null);
    first.push({ a, b, winner: null });
  }
  const rounds: Match[][] = [shuffle(first)];
  for (let c = m / 2; c >= 1; c /= 2) rounds.push(Array.from({ length: c }, () => ({ a: null, b: null, winner: null })));
  return settle(rounds);
}

/** Kazananları sonraki tura taşır; bay geçenleri kendiliğinden ilerletir; geçersiz kalan seçimleri siler. */
export function settle(rounds: Match[][]): Match[][] {
  const out = rounds.map((r) => r.map((m) => ({ ...m })));
  for (let r = 0; r < out.length; r++) {
    out[r].forEach((m, k) => {
      if (r > 0) {
        m.a = out[r - 1][2 * k].winner;
        m.b = out[r - 1][2 * k + 1].winner;
        // Önceki maçlardan biri bay sonucu boşsa (iki bay) diğeri geçer.
      }
      if (m.winner && m.winner !== m.a && m.winner !== m.b) m.winner = null;
      if (r === 0 && m.a && !m.b) m.winner = m.a;
      if (r === 0 && !m.a && m.b) m.winner = m.b;
    });
  }
  return out;
}

/** Maçın kazananını seçer (aynı kişiye yeniden tıklamak seçimi kaldırır). */
export function choose(rounds: Match[][], r: number, k: number, name: string): Match[][] {
  const next = rounds.map((x) => x.map((m) => ({ ...m })));
  const m = next[r][k];
  if (!m.a || !m.b) return rounds;
  m.winner = m.winner === name ? null : name;
  return settle(next);
}

export function roundName(matches: number, index: number): string {
  if (matches === 1) return "Final";
  if (matches === 2) return "Yarı final";
  if (matches === 4) return "Çeyrek final";
  if (matches === 8) return "Son 16";
  return `${index + 1}. tur`;
}

/** Kürsü: şampiyon, finalde kaybeden, yarı finalde kaybedenler. */
export function podium(rounds: Match[][]): { first: string | null; second: string | null; thirds: string[] } {
  const fin = rounds[rounds.length - 1]?.[0];
  if (!fin?.winner) return { first: null, second: null, thirds: [] };
  const loser = (m: Match) => (m.a && m.b && m.winner ? (m.winner === m.a ? m.b : m.a) : null);
  const semis = rounds.length >= 2 ? rounds[rounds.length - 2] : [];
  return { first: fin.winner, second: loser(fin), thirds: semis.map(loser).filter((x): x is string => !!x) };
}
