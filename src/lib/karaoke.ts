// Şarkı sözlerinde kelime kelime dolum. Kelime zamanı olmayan satırlarda (ör. LRCLIB: yalnız satır zamanı) kelime
// zamanları tahmin edilir: satırın süresi kelimelerin uzunluğuna göre paylaştırılır.
import type { Segment, Word } from "./types";

/** Saniyede ~9 harf: satır bundan uzun sürüyorsa (araya müzik giriyorsa) dolum erken biter, boşlukta beklemez. */
const PER_CHAR = 0.11;

export function wordsOf(segs: readonly Segment[], i: number): Word[] {
  const s = segs[i];
  if (!s) return [];
  if (s.words?.length) return s.words;
  const parts = s.text.split(/\s+/).filter(Boolean);
  if (!parts.length) return [];
  const next = segs[i + 1]?.start ?? Infinity;
  const end = Math.min(s.end > s.start ? s.end : next, next);
  const chars = parts.reduce((n, w) => n + w.length + 1, 0);
  const span = Math.max(0.3, Math.min(Number.isFinite(end) ? end - s.start : Infinity, Math.max(1, chars * PER_CHAR)));
  let t = s.start;
  return parts.map((text) => {
    const d = (span * (text.length + 1)) / chars;
    const w = { start: t, end: t + d, text };
    t += d;
    return w;
  });
}

/** 0 (söylenmedi) … 1 (söylendi): kelimenin o anki doluluğu. */
export function fill(w: Word, time: number): number {
  if (time <= w.start) return 0;
  if (time >= w.end) return 1;
  return (time - w.start) / Math.max(0.01, w.end - w.start);
}
