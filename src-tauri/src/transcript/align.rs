//! Zamansız şarkı sözünü (veritabanındaki düz metin) duyulan sözlerin zamanlarıyla eşleştirir.
//!
//! Eski şarkıların çoğunda LRCLIB'de yalnızca düz söz var. Whisper ya da sitenin otomatik altyazısı
//! kelimeleri yanlış duysa da *ne zaman* söylendiklerini bilir. İki kelime dizisi, benzerliğe göre
//! hizalanır (Needleman–Wunsch); her satırın başlangıcı, eşleşen ilk kelimenin zamanı olur.
//! Böylece yazı veritabanından birebir doğru, zamanlama da bu bilgisayardan gelir.

use super::parse::Segment;

/// Karşılaştırma için kelime: küçük harf, yalnızca harf/rakam, Türkçe harfler sadeleşir
/// (Whisper "ş" yerine "s" yazsa da eşleşsin).
fn fold(word: &str) -> String {
    word.chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| {
            Some(match c {
                'ı' | 'i' | 'î' | 'ì' | 'í' => 'i',
                'ş' => 's',
                'ç' => 'c',
                'ğ' => 'g',
                'ö' | 'ò' | 'ó' => 'o',
                'ü' | 'û' | 'ù' | 'ú' => 'u',
                'â' | 'à' | 'á' | 'ä' => 'a',
                'é' | 'è' | 'ê' => 'e',
                c if c.is_alphanumeric() => c,
                _ => return None,
            })
        })
        .collect()
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// 0..1 benzerlik.
fn similarity(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 0.0;
    }
    1.0 - levenshtein(&a, &b) as f64 / longest as f64
}

/// Duyulan kelimeler: (kelime, başlangıç, bitiş). Kelime zamanı yoksa satır süresi kelimelere bölünür.
fn heard_words(segs: &[Segment]) -> Vec<(String, f64, f64)> {
    let mut out = Vec::new();
    for s in segs {
        if !s.words.is_empty() {
            for w in &s.words {
                let f = fold(&w.text);
                if !f.is_empty() {
                    out.push((f, w.start, w.end));
                }
            }
            continue;
        }
        let words: Vec<String> = s
            .text
            .split_whitespace()
            .map(fold)
            .filter(|w| !w.is_empty())
            .collect();
        let step = (s.end - s.start).max(0.0) / words.len().max(1) as f64;
        for (i, w) in words.into_iter().enumerate() {
            let t = s.start + step * i as f64;
            out.push((w, t, t + step));
        }
    }
    out
}

/// Düz söz satırlarını duyulan sözlerin zamanlarıyla eşleştirir. Eşleşme çok zayıfsa (yanlış şarkı
/// ya da müzik baskın) `None`.
pub fn align(lines: &[String], heard: &[Segment], duration: Option<f64>) -> Option<Vec<Segment>> {
    let lines: Vec<&str> = lines
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    // (kelime, satır numarası)
    let lyric: Vec<(String, usize)> = lines
        .iter()
        .enumerate()
        .flat_map(|(i, l)| {
            l.split_whitespace()
                .map(fold)
                .filter(|w| !w.is_empty())
                .map(move |w| (w, i))
        })
        .collect();
    let heard = heard_words(heard);
    let (n, m) = (lyric.len(), heard.len());
    if n == 0 || m == 0 || n * m > 4_000_000 {
        return None;
    }

    // Puan tablosu: eşleşme benzerliğe göre ödüllendirilir, atlama küçük bir ceza alır.
    const GAP: f64 = -0.4;
    let score = |a: &str, b: &str| {
        let s = similarity(a, b);
        if s >= 0.6 {
            1.0 + s
        } else {
            -1.0
        }
    };
    let w = m + 1;
    let mut dp = vec![0f64; (n + 1) * w];
    // 0: çapraz (eşleşme), 1: söz kelimesi atlandı, 2: duyulan kelime atlandı
    let mut from = vec![0u8; (n + 1) * w];
    for i in 1..=n {
        dp[i * w] = dp[(i - 1) * w] + GAP;
        from[i * w] = 1;
    }
    for j in 1..=m {
        // Başta ve sonda duyulan fazladan kelimeler (giriş konuşması, tekrarlar) cezasız.
        dp[j] = 0.0;
        from[j] = 2;
    }
    for i in 1..=n {
        for j in 1..=m {
            let diag = dp[(i - 1) * w + j - 1] + score(&lyric[i - 1].0, &heard[j - 1].0);
            let up = dp[(i - 1) * w + j] + GAP;
            let left = dp[i * w + j - 1] + if i == n { 0.0 } else { GAP };
            let (best, dir) = if diag >= up && diag >= left {
                (diag, 0)
            } else if up >= left {
                (up, 1)
            } else {
                (left, 2)
            };
            dp[i * w + j] = best;
            from[i * w + j] = dir;
        }
    }

    // Geri izleme: her söz kelimesine eşleştiği duyulan kelimenin zamanı.
    let mut times: Vec<Option<(f64, f64)>> = vec![None; n];
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        match from[i * w + j] {
            0 => {
                if similarity(&lyric[i - 1].0, &heard[j - 1].0) >= 0.6 {
                    times[i - 1] = Some((heard[j - 1].1, heard[j - 1].2));
                }
                i -= 1;
                j -= 1;
            }
            1 => i -= 1,
            _ => j -= 1,
        }
    }
    let matched = times.iter().filter(|t| t.is_some()).count();
    if (matched as f64) < (n as f64 * 0.25).max(3.0) {
        return None;
    }

    // Satır başlangıcı: ilk eşleşen kelime; satırın sonu: son eşleşen kelime.
    let mut starts: Vec<Option<f64>> = vec![None; lines.len()];
    let mut ends: Vec<Option<f64>> = vec![None; lines.len()];
    for (k, t) in times.iter().enumerate() {
        if let Some((a, b)) = t {
            let line = lyric[k].1;
            // Satırın ilk kelimesi eşleşmediyse, eşleşen kelimeden geriye doğru tahmin.
            let before = lyric[..k].iter().rev().take_while(|x| x.1 == line).count();
            let guess = (a - before as f64 * 0.35).max(0.0);
            starts[line] = Some(starts[line].map_or(guess, |s: f64| s.min(guess)));
            ends[line] = Some(ends[line].map_or(*b, |e: f64| e.max(*b)));
        }
    }
    // Eşleşmeyen satırlar komşuların arasına yerleşir.
    let total = duration.unwrap_or_else(|| heard.last().map_or(0.0, |h| h.2) + 5.0);
    let mut out_starts: Vec<f64> = vec![0.0; lines.len()];
    let mut k = 0;
    while k < lines.len() {
        if let Some(s) = starts[k] {
            out_starts[k] = s;
            k += 1;
            continue;
        }
        let prev = if k == 0 {
            None
        } else {
            Some(ends[k - 1].unwrap_or(out_starts[k - 1] + 2.0))
        };
        let gap_end = (k..lines.len()).find(|x| starts[*x].is_some());
        let lo = prev.unwrap_or_else(|| {
            gap_end
                .and_then(|g| starts[g])
                .map_or(0.0, |s| (s - 4.0).max(0.0))
        });
        let hi = gap_end.and_then(|g| starts[g]).unwrap_or(total);
        let count = gap_end.unwrap_or(lines.len()) - k;
        let step = ((hi - lo).max(0.0)) / (count + 1) as f64;
        for x in 0..count {
            out_starts[k + x] = lo + step * (x + 1) as f64;
        }
        k += count;
    }
    // Sıra bozulmasın.
    for k in 1..out_starts.len() {
        if out_starts[k] < out_starts[k - 1] {
            out_starts[k] = out_starts[k - 1] + 0.1;
        }
    }
    Some(
        lines
            .iter()
            .enumerate()
            .map(|(k, text)| {
                let start = out_starts[k];
                let next = out_starts.get(k + 1).copied().unwrap_or(total);
                let end = ends[k]
                    .map(|e| (e + 0.6).min(next))
                    .unwrap_or(next)
                    .max(start + 0.5)
                    .min(start + 12.0);
                Segment {
                    start,
                    end,
                    text: text.to_string(),
                    words: Vec::new(),
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::parse::Word;

    fn heard(items: &[(&str, f64)]) -> Vec<Segment> {
        vec![Segment {
            start: items[0].1,
            end: items.last().unwrap().1 + 0.5,
            text: String::new(),
            words: items
                .iter()
                .map(|(t, s)| Word {
                    start: *s,
                    end: s + 0.4,
                    text: t.to_string(),
                })
                .collect(),
        }]
    }

    #[test]
    fn plain_lyrics_get_heard_timing() {
        let lines: Vec<String> = ["Sabır taşım çatladı", "Gel artık gel", "", "Yandı yüreğim"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        // Whisper bazı kelimeleri yanlış duymuş ("sır", "catladi"), araya fazladan kelime eklemiş.
        let h = heard(&[
            ("eee", 1.0),
            ("sır", 10.0),
            ("taşım", 10.6),
            ("catladi", 11.2),
            ("gel", 15.0),
            ("artik", 15.4),
            ("gel", 15.9),
            ("yandi", 21.0),
            ("yureğim", 21.6),
        ]);
        let segs = align(&lines, &h, Some(200.0)).unwrap();
        let got: Vec<(&str, f64)> = segs.iter().map(|s| (s.text.as_str(), s.start)).collect();
        assert_eq!(got.len(), 3, "boş satır atlanır");
        assert_eq!(got[0].0, "Sabır taşım çatladı", "yazı veritabanından gelir");
        assert!((got[0].1 - 10.0).abs() < 0.5, "{got:?}");
        assert!((got[1].1 - 15.0).abs() < 0.01, "{got:?}");
        assert!((got[2].1 - 21.0).abs() < 0.01, "{got:?}");
        assert!(segs[0].end <= segs[1].start);
    }

    #[test]
    fn unmatched_lines_are_placed_between_neighbours() {
        let lines: Vec<String> = ["bir iki üç", "dört beş altı", "yedi sekiz dokuz"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let h = heard(&[
            ("bir", 5.0),
            ("iki", 5.5),
            ("uc", 6.0),
            ("yedi", 20.0),
            ("sekiz", 20.5),
            ("dokuz", 21.0),
        ]);
        let segs = align(&lines, &h, Some(60.0)).unwrap();
        assert!(segs[1].start > 6.0 && segs[1].start < 20.0, "{segs:?}");
    }

    #[test]
    fn wrong_song_is_rejected() {
        let lines = vec!["tamamen başka bir şarkının sözleri burada".to_string()];
        let h = heard(&[
            ("hello", 1.0),
            ("world", 2.0),
            ("music", 3.0),
            ("yeah", 4.0),
        ]);
        assert!(align(&lines, &h, Some(60.0)).is_none());
        assert_eq!(fold("Şarkı-Çiçeği!"), "sarkicicegi");
    }
}
