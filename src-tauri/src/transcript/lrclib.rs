//! LRCLIB (lrclib.net): ücretsiz, açık, topluluğun satır satır zamanladığı şarkı sözü veritabanı.
//! Şarkılarda Whisper'dan çok daha doğru sonuç verir; yalnızca şarkının adı ve sanatçısı gönderilir.

use super::parse::Segment;
use serde::Deserialize;
use std::time::Duration;

const API: &str = "https://lrclib.net/api/search";
const USER_AGENT: &str = "GMID Tool (github.com/BalamanMerkut/GMID--Personel_Tool)";
pub const ENABLED_KEY: &str = "transcript.lyricsDb";
/// Bulunan kaydın süresi indirilen şarkıdan en fazla bu kadar saniye farklı olabilir.
const DURATION_TOLERANCE: f64 = 6.0;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Track {
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    synced_lyrics: Option<String>,
    #[serde(default)]
    instrumental: bool,
}

/// Başlıktaki "(Official Video)", "[4K]", "| Lyrics" gibi ekleri atar.
pub fn clean_title(title: &str) -> String {
    let mut out = String::new();
    let mut depth = 0i32;
    for c in title.chars() {
        match c {
            '(' | '[' | '【' | '{' => depth += 1,
            ')' | ']' | '】' | '}' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let out = out.split(['|', '/']).next().unwrap_or("").to_string();
    let lower = out.to_lowercase();
    let mut cut = out.len();
    for noise in [
        " official music video",
        " official video",
        " official audio",
        " official lyric video",
        " lyric video",
        " lyrics",
        " music video",
        " ft.",
        " feat.",
        " ft ",
        " feat ",
    ] {
        if let Some(i) = lower.find(noise) {
            // Küçük harfe çevirmek bazı harflerin bayt uzunluğunu değiştirebilir; sınır kontrolü şart.
            if out.is_char_boundary(i) {
                cut = cut.min(i);
            }
        }
    }
    out[..cut]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c: char| c == '-' || c == '–' || c == '"' || c.is_whitespace())
        .to_string()
}

/// "Sanatçı - Şarkı" başlığını ayırır; ayrılamazsa sanatçı yükleyenin adından tahmin edilir.
pub fn artist_and_track(title: &str, uploader: &str) -> (String, String) {
    let t = clean_title(title);
    for sep in [" - ", " – ", " — ", " ~ "] {
        if let Some((a, b)) = t.split_once(sep) {
            let (a, b) = (a.trim(), b.trim());
            if !a.is_empty() && !b.is_empty() {
                return (a.to_string(), b.to_string());
            }
        }
    }
    let artist = uploader
        .trim_end_matches(" - Topic")
        .trim_end_matches("VEVO")
        .trim_end_matches(" Official")
        .trim();
    (artist.to_string(), t)
}

/// Şarkının zamanlı sözlerini arar. Bulunamazsa `Ok(None)`.
pub fn find(
    title: &str,
    uploader: &str,
    duration: Option<f64>,
) -> Result<Option<Vec<Segment>>, String> {
    let (artist, track) = artist_and_track(title, uploader);
    if track.is_empty() {
        return Ok(None);
    }
    let mut queries: Vec<Vec<(&str, String)>> = Vec::new();
    if !artist.is_empty() {
        queries.push(vec![
            ("track_name", track.clone()),
            ("artist_name", artist.clone()),
        ]);
        queries.push(vec![("q", format!("{artist} {track}"))]);
    }
    // Sanatçısız arama yanlış şarkıyı bulabilir; yalnızca süre tutarsa kabul edilir.
    if duration.is_some() {
        queries.push(vec![("q", track.clone())]);
    }
    let agent = crate::net::agent(Duration::from_secs(10), Duration::from_secs(15));
    let mut last_err = None;
    for q in queries {
        let mut req = agent.get(API).header("User-Agent", USER_AGENT);
        for (k, v) in &q {
            req = req.query(*k, v);
        }
        let body = match req.call() {
            Ok(mut r) if r.status() == 200 => r
                .body_mut()
                .with_config()
                .limit(4 * 1024 * 1024)
                .read_to_string()
                .map_err(|e| e.to_string()),
            Ok(r) => Err(format!("LRCLIB {} döndü", r.status())),
            Err(e) => Err(e.to_string()),
        };
        let body = match body {
            Ok(b) => b,
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        };
        let tracks: Vec<Track> = serde_json::from_str(&body).unwrap_or_default();
        if let Some(segs) = best(&tracks, duration) {
            return Ok(Some(segs));
        }
    }
    match last_err {
        Some(e) => Err(format!("Şarkı sözü veritabanına ulaşılamadı ({e})")),
        None => Ok(None),
    }
}

/// Zamanlı sözü olan ve süresi tutan ilk sonuç.
fn best(tracks: &[Track], duration: Option<f64>) -> Option<Vec<Segment>> {
    tracks
        .iter()
        .filter(|t| !t.instrumental)
        .filter(|t| match (duration, t.duration) {
            (Some(d), Some(td)) => (d - td).abs() <= DURATION_TOLERANCE,
            (Some(_), None) => false,
            (None, _) => true,
        })
        .find_map(|t| {
            let segs = parse_lrc(t.synced_lyrics.as_deref()?, duration.or(t.duration));
            (segs.len() >= 3).then_some(segs)
        })
}

/// "[01:02.34]Söz" satırlarını (bir satırda birden çok zaman olabilir) sıralı satırlara çevirir.
/// Boş satırlar müzik arası demektir: önceki satırı orada bitirir.
pub fn parse_lrc(lrc: &str, duration: Option<f64>) -> Vec<Segment> {
    let mut marks: Vec<(f64, String)> = Vec::new();
    for line in lrc.lines() {
        let mut rest = line.trim();
        let mut times = Vec::new();
        while let Some(inner) = rest.strip_prefix('[') {
            let Some(close) = inner.find(']') else { break };
            if let Some(t) = lrc_time(&inner[..close]) {
                times.push(t);
            } else {
                // [ar:…], [ti:…] gibi bilgi satırları
                times.clear();
                rest = "";
                break;
            }
            rest = &inner[close + 1..];
        }
        let text = rest.trim().to_string();
        for t in times {
            marks.push((t, text.clone()));
        }
    }
    marks.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<Segment> = Vec::new();
    for (i, (start, text)) in marks.iter().enumerate() {
        if text.is_empty()
            || text
                .chars()
                .all(|c| c == '♪' || c == '♫' || c.is_whitespace())
        {
            continue;
        }
        let next = marks.get(i + 1).map(|m| m.0);
        let end = next
            .or(duration)
            .unwrap_or(start + 5.0)
            .min(start + 12.0)
            .max(*start);
        out.push(Segment {
            start: *start,
            end,
            text: text.clone(),
            words: vec![],
        });
    }
    out
}

/// "01:02.34" | "01:02" | "1:02.345" → saniye
fn lrc_time(s: &str) -> Option<f64> {
    let (m, sec) = s.split_once(':')?;
    let m: f64 = m.trim().parse().ok()?;
    let sec: f64 = sec.trim().replace(':', ".").parse().ok()?;
    Some(m * 60.0 + sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_titles() {
        assert_eq!(
            artist_and_track(
                "Michael Jackson - Earth Song (Official Video)",
                "michaeljacksonVEVO"
            ),
            ("Michael Jackson".into(), "Earth Song".into())
        );
        assert_eq!(
            artist_and_track("Tarkan - Şımarık [4K] | Lyrics", ""),
            ("Tarkan".into(), "Şımarık".into())
        );
        assert_eq!(
            artist_and_track("Bohemian Rhapsody (Remastered 2011)", "Queen - Topic"),
            ("Queen".into(), "Bohemian Rhapsody".into())
        );
        assert_eq!(
            artist_and_track("Daft Punk - Get Lucky ft. Pharrell Williams", ""),
            ("Daft Punk".into(), "Get Lucky".into())
        );
    }

    #[test]
    fn parses_lrc() {
        let lrc = "[ar:Queen]\n[00:01.50]Is this the real life?\n[00:05.00][00:20.00]Is this just fantasy?\n[00:09.10]\n[00:12.00]Caught in a landslide";
        let segs = parse_lrc(lrc, Some(30.0));
        let got: Vec<(f64, f64, &str)> = segs
            .iter()
            .map(|s| (s.start, s.end, s.text.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                (1.5, 5.0, "Is this the real life?"),
                (5.0, 9.1, "Is this just fantasy?"),
                (12.0, 20.0, "Caught in a landslide"),
                (20.0, 30.0, "Is this just fantasy?"),
            ]
        );
    }

    #[test]
    fn picks_by_duration() {
        let t = |d: f64, lyrics: &str| Track {
            duration: Some(d),
            synced_lyrics: Some(lyrics.into()),
            instrumental: false,
        };
        let a = "[00:01.00]A1\n[00:02.00]A2\n[00:03.00]A3";
        let b = "[00:01.00]B1\n[00:02.00]B2\n[00:03.00]B3";
        let segs = best(&[t(120.0, a), t(242.0, b)], Some(245.0)).unwrap();
        assert_eq!(segs[0].text, "B1");
        assert!(best(&[t(120.0, a)], Some(245.0)).is_none());
    }
}
