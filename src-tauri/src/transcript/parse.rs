//! Yazı biçimleri: sitenin altyazısı (WebVTT, YouTube'un kelime zamanlı otomatik altyazısı dahil),
//! Whisper'ın JSON çıktısı; dışa aktarma için SRT ve LRC (şarkı sözü).

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Word {
    /// Saniye
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    /// Kelime zamanları varsa (karaoke gibi kelime kelime işaretlemek için).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<Word>,
}

/// "00:01:02.345" ya da "01:02.345" → saniye
fn time(s: &str) -> Option<f64> {
    let s = s.trim().replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    let (h, m, sec) = match parts.as_slice() {
        [h, m, s] => (
            h.parse::<f64>().ok()?,
            m.parse::<f64>().ok()?,
            s.parse::<f64>().ok()?,
        ),
        [m, s] => (0.0, m.parse::<f64>().ok()?, s.parse::<f64>().ok()?),
        _ => return None,
    };
    Some(h * 3600.0 + m * 60.0 + sec)
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    decode_entities(&out)
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn clean(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// YouTube'un kelime zamanlı satırı: `kelime<00:00:01.120><c> diğer</c><00:00:01.500><c> kelime</c>`
fn timed_words(line: &str, start: f64, end: f64) -> Vec<Word> {
    let mut words: Vec<Word> = Vec::new();
    let mut at = start;
    let mut rest = line;
    loop {
        // Bir sonraki *zaman* etiketine kadarki metin bu kelimenin (<c> gibi biçim etiketleri atlanır).
        let mut from = 0;
        let next = loop {
            let Some(i) = rest[from..].find('<').map(|i| i + from) else {
                break None;
            };
            let Some(close) = rest[i..].find('>').map(|c| c + i) else {
                break None;
            };
            if let Some(t) = time(&rest[i + 1..close]) {
                break Some((i, close, t));
            }
            from = close + 1;
        };
        let (text, after, next_t) = match next {
            Some((i, close, t)) => (&rest[..i], &rest[close + 1..], Some(t)),
            None => (rest, "", None),
        };
        let t = clean(&strip_tags(text));
        if !t.is_empty() {
            words.push(Word {
                start: at,
                end: next_t.unwrap_or(end),
                text: t,
            });
        }
        match next_t {
            Some(nt) => {
                at = nt;
                rest = after;
            }
            None => break,
        }
    }
    words
}

/// WebVTT → satırlar. YouTube otomatik altyazısındaki "kayan" tekrarlar ayıklanır.
pub fn parse_vtt(src: &str) -> Vec<Segment> {
    let src = src.replace("\r\n", "\n");
    let mut out: Vec<Segment> = Vec::new();
    for block in src.split("\n\n") {
        let mut lines = block.lines();
        let Some(timing) = lines.by_ref().find(|l| l.contains("-->")) else {
            continue;
        };
        let mut parts = timing.split("-->");
        let (Some(a), Some(b)) = (parts.next(), parts.next()) else {
            continue;
        };
        let (Some(start), Some(end)) = (time(a), time(b.split_whitespace().next().unwrap_or("")))
        else {
            continue;
        };
        let text_lines: Vec<&str> = lines.collect();
        // Kelime zamanlı satır varsa (YouTube otomatik) sadece o yeni satırdır; üstteki eski satırın tekrarı.
        if let Some(timed) = text_lines.iter().find(|l| l.contains("<c>")) {
            let words = timed_words(timed, start, end);
            let text = words
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            if !text.is_empty() {
                out.push(Segment {
                    start,
                    end,
                    text,
                    words,
                });
            }
            continue;
        }
        let text = clean(&strip_tags(&text_lines.join(" ")));
        if text.is_empty()
            || end - start < 0.05 && out.last().is_some_and(|s| s.text.ends_with(&text))
        {
            continue;
        }
        match out.last_mut() {
            // Aynı yazı tekrar ediyorsa (kayan altyazı) süreyi uzat.
            Some(prev) if prev.text == text || prev.text.ends_with(&text) => {
                prev.end = prev.end.max(end)
            }
            _ => out.push(Segment {
                start,
                end,
                text,
                words: Vec::new(),
            }),
        }
    }
    // Kelime zamanlı satırların bitişi bir sonrakinin başlangıcına kadar sürsün.
    for i in 1..out.len() {
        if !out[i - 1].words.is_empty() && out[i - 1].end < out[i].start {
            out[i - 1].end = out[i].start;
        }
    }
    drop_noise(out)
}

#[derive(Deserialize)]
struct WOffsets {
    from: i64,
    to: i64,
}

#[derive(Deserialize)]
struct WToken {
    text: String,
    offsets: WOffsets,
}

#[derive(Deserialize)]
struct WSegment {
    offsets: WOffsets,
    text: String,
    #[serde(default)]
    tokens: Vec<WToken>,
}

#[derive(Deserialize)]
struct WOutput {
    transcription: Vec<WSegment>,
}

/// whisper.cpp `-ojf` (tam JSON) çıktısı → satırlar ve kelimeler.
pub fn parse_whisper(json: &str) -> Result<Vec<Segment>, String> {
    let out: WOutput =
        serde_json::from_str(json).map_err(|e| format!("Whisper çıktısı okunamadı: {e}"))?;
    let segs = out
        .transcription
        .into_iter()
        .map(|s| {
            let mut words: Vec<Word> = Vec::new();
            for t in s
                .tokens
                .iter()
                .filter(|t| !t.text.starts_with("[_") && !t.text.starts_with("<|"))
            {
                let starts_word = t.text.starts_with(' ') || words.is_empty();
                let (a, b) = (t.offsets.from as f64 / 1000.0, t.offsets.to as f64 / 1000.0);
                if starts_word {
                    let text = t.text.trim().to_string();
                    if !text.is_empty() {
                        words.push(Word {
                            start: a,
                            end: b,
                            text,
                        });
                    }
                } else if let Some(w) = words.last_mut() {
                    // Kelimenin devamı (alt parça).
                    w.text.push_str(&t.text);
                    w.end = b;
                }
            }
            Segment {
                start: s.offsets.from as f64 / 1000.0,
                end: s.offsets.to as f64 / 1000.0,
                text: clean(&s.text),
                words,
            }
        })
        .collect();
    Ok(drop_hallucinations(drop_noise(segs)))
}

/// "[Müzik]", "♪", "(alkış)" gibi sözsüz satırlar.
fn drop_noise(segs: Vec<Segment>) -> Vec<Segment> {
    segs.into_iter()
        .filter(|s| {
            let t = s
                .text
                .trim_matches(|c: char| c == '♪' || c == '♫' || c.is_whitespace());
            !(t.is_empty()
                || (t.starts_with('[') && t.ends_with(']'))
                || (t.starts_with('(') && t.ends_with(')')))
        })
        .collect()
}

/// Karşılaştırma için: küçük harf, noktalama yok, tek boşluk.
fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whisper'ın sessiz ya da yalnızca müzik olan yerlerde uydurduğu, eğitim verisindeki
/// altyazı imzalarından kalma satırlar. İçinde geçmesi yeter.
const HALLUCINATION_MARKS: &[&str] = &[
    "altyazı m k",
    "altyazi m k",
    "alt yazı m k",
    "subtitles by",
    "subtitled by",
    "transcription by",
    "transcript by",
    "captions by",
    "amara org",
    "untertitel",
    "sottotitoli",
    "sous titres",
    "sous titrage",
];

/// Bunlar ancak satırın tamamıysa atılır (gerçek konuşmanın içinde geçebilirler).
const HALLUCINATION_LINES: &[&str] = &[
    "altyazı",
    "izlediğiniz için teşekkürler",
    "izlediğiniz için teşekkür ederim",
    "izlediğiniz için teşekkür ederiz",
    "abone olmayı unutmayın",
    "abone olun",
    "kanalıma abone olmayı unutmayın",
    "thanks for watching",
    "thank you for watching",
    "thank you so much for watching",
    "please subscribe",
    "like and subscribe",
];

/// Şarkıda gerçekten de geçebilecek kısa satırlar: yalnızca uzun bir sessizliğe yayılmışlarsa atılır.
const HALLUCINATION_SHORT: &[&str] = &["you", "thank you", "teşekkürler", "bye"];

/// Whisper uydurmalarını ayıklar: bilinen imzalar ve uzun bir süreye yayılmış, birkaç kez aynen
/// tekrarlanan kısa satırlar (müzik arasında her 30 saniyede bir çıkan "Altyazı M.K." gibi).
fn drop_hallucinations(segs: Vec<Segment>) -> Vec<Segment> {
    let norm: Vec<String> = segs.iter().map(|s| normalize(&s.text)).collect();
    let suspicious = |i: usize| {
        let s = &segs[i];
        s.end - s.start >= 10.0 && norm[i].split(' ').count() <= 4
    };
    let mut repeats: std::collections::HashMap<&str, usize> = Default::default();
    for (i, n) in norm.iter().enumerate() {
        if suspicious(i) {
            *repeats.entry(n.as_str()).or_default() += 1;
        }
    }
    let drop: Vec<bool> = norm
        .iter()
        .enumerate()
        .map(|(i, n)| {
            n.is_empty()
                || HALLUCINATION_MARKS.iter().any(|m| n.contains(m))
                || HALLUCINATION_LINES.contains(&n.as_str())
                || (suspicious(i) && HALLUCINATION_SHORT.contains(&n.as_str()))
                || (suspicious(i) && repeats.get(n.as_str()).copied().unwrap_or(0) >= 3)
        })
        .collect();
    segs.into_iter()
        .zip(drop)
        .filter_map(|(s, d)| (!d).then_some(s))
        .collect()
}

fn srt_time(t: f64) -> String {
    let ms = (t.max(0.0) * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

/// Video oynatıcılar (VLC, MPC, Filmler) için altyazı dosyası.
pub fn to_srt(segs: &[Segment]) -> String {
    segs.iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{}\n{} --> {}\n{}\n",
                i + 1,
                srt_time(s.start),
                srt_time(s.end),
                s.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Müzik çalarlar (telefon, foobar2000, MusicBee…) için zamanlı şarkı sözü dosyası.
pub fn to_lrc(segs: &[Segment], title: &str) -> String {
    let mut out = format!("[ti:{title}]\n[re:GMID Tool]\n");
    for s in segs {
        let cs = (s.start.max(0.0) * 100.0).round() as u64;
        out.push_str(&format!(
            "[{:02}:{:02}.{:02}]{}\n",
            cs / 6000,
            cs / 100 % 60,
            cs % 100,
            s.text
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_auto_captions_with_word_timing() {
        let vtt = "WEBVTT\nKind: captions\nLanguage: en\n\n\
            00:00:00.320 --> 00:00:02.990 align:start position:0%\n \nwe're<00:00:00.719><c> no</c><00:00:00.880><c> strangers</c>\n\n\
            00:00:02.990 --> 00:00:03.000 align:start position:0%\nwe're no strangers\n \n\n\
            00:00:03.000 --> 00:00:05.500 align:start position:0%\nwe're no strangers\nto<00:00:03.400><c> love</c>\n";
        let segs = parse_vtt(vtt);
        assert_eq!(segs.len(), 2, "{segs:#?}");
        assert_eq!(segs[0].text, "we're no strangers");
        assert_eq!(segs[0].words.len(), 3);
        assert_eq!(
            segs[0].words[1],
            Word {
                start: 0.719,
                end: 0.88,
                text: "no".into()
            }
        );
        assert_eq!(segs[0].end, 3.0, "bir sonraki satıra kadar sürer");
        assert_eq!(segs[1].text, "to love");
        assert_eq!(segs[1].words[0].start, 3.0);
    }

    #[test]
    fn manual_subtitles() {
        let vtt = "WEBVTT\n\n1\n00:00:01.000 --> 00:00:03.000\n<i>Merhaba</i> &amp; hoş geldin\nmalikâneye\n\n\
                   00:00:04.000 --> 00:00:05.000\n[Müzik]\n\n00:00:06.000 --> 00:00:08.000\n♪ Gece sürüşü ♪\n";
        let segs = parse_vtt(vtt);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].text, "Merhaba & hoş geldin malikâneye");
        assert!(segs[0].words.is_empty());
        assert_eq!(segs[1].text, "♪ Gece sürüşü ♪");
    }

    #[test]
    fn whisper_full_json() {
        let json = r#"{"transcription":[
          {"offsets":{"from":0,"to":2500},"text":" Merhaba dünya","tokens":[
            {"text":"[_BEG_]","offsets":{"from":0,"to":0}},
            {"text":" Mer","offsets":{"from":0,"to":400}},{"text":"haba","offsets":{"from":400,"to":900}},
            {"text":" dünya","offsets":{"from":1000,"to":2400}},{"text":"[_TT_125]","offsets":{"from":2500,"to":2500}}]},
          {"offsets":{"from":2500,"to":4000},"text":" [Music]","tokens":[]}]}"#;
        let segs = parse_whisper(json).unwrap();
        assert_eq!(segs.len(), 1, "sözsüz satır atılır");
        assert_eq!(segs[0].text, "Merhaba dünya");
        assert_eq!(
            segs[0].words,
            vec![
                Word {
                    start: 0.0,
                    end: 0.9,
                    text: "Merhaba".into()
                },
                Word {
                    start: 1.0,
                    end: 2.4,
                    text: "dünya".into()
                },
            ]
        );
    }

    #[test]
    fn whisper_hallucinations() {
        let seg = |start: f64, end: f64, text: &str| Segment {
            start,
            end,
            text: text.into(),
            words: vec![],
        };
        let segs = vec![
            seg(0.0, 30.0, "Altyazı M.K."),
            seg(30.0, 33.0, "I'm the king of the world"),
            seg(33.0, 63.0, "Altyazı: M.K."),
            seg(63.0, 64.5, "Thank you"),
            seg(64.5, 90.0, "Thank you."),
            seg(90.0, 92.0, "Thanks for watching!"),
            seg(92.0, 94.0, "Na na na"),
            seg(94.0, 96.0, "Na na na"),
            seg(96.0, 98.0, "Na na na"),
            seg(100.0, 120.0, "Ooh"),
            seg(130.0, 150.0, "Ooh"),
            seg(160.0, 180.0, "Ooh"),
        ];
        let kept: Vec<String> = drop_hallucinations(segs)
            .into_iter()
            .map(|s| s.text)
            .collect();
        assert_eq!(
            kept,
            [
                "I'm the king of the world",
                "Thank you",
                "Na na na",
                "Na na na",
                "Na na na"
            ]
        );
    }

    #[test]
    fn exports() {
        let segs = vec![Segment {
            start: 61.5,
            end: 64.25,
            text: "Satır".into(),
            words: vec![],
        }];
        assert_eq!(to_srt(&segs), "1\n00:01:01,500 --> 00:01:04,250\nSatır\n");
        assert!(to_lrc(&segs, "Şarkı").contains("[01:01.50]Satır"));
    }
}
