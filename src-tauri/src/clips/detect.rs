//! Kayıt programlarının klasörlerinde klip ve ekran görüntüsü bulma; dosyanın hangi programdan geldiğini ve
//! hangi oyuna ait olabileceğini (klasör ya da dosya adından) çıkarma.
//!
//! | Program | Konum | Ad |
//! |---|---|---|
//! | NVIDIA (Instant Replay) | `Videolar\<Oyun>\` ya da `Videolar\NVIDIA\<Oyun>\` | `Oyun 2024.10.11 - 21.33.12.02.DVR.mp4` |
//! | Xbox Game Bar | `Videolar\Captures\` | `Oyun 2024-10-11 21-33-12.mp4` |
//! | AMD ReLive | `Videolar\Radeon ReLive\<Oyun>\` | |
//! | OBS | `Videolar\` | `2024-10-11 21-33-12.mkv`, `Replay 2024-10-11 21-33-12.mkv` |
//! | Steam | `<Steam>\userdata\<hesap>\760\remote\<appid>\screenshots\` | |
//! | Medal | `C:\Medal\…\<Oyun>\` | |

use std::path::{Path, PathBuf};

pub const VIDEO_EXT: &[&str] = &["mp4", "mkv", "mov", "webm", "avi"];
pub const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp"];

#[derive(Clone, Debug, PartialEq)]
pub struct FoundFile {
    pub path: PathBuf,
    /// clip | shot
    pub kind: &'static str,
    /// nvidia | xbox | amd | obs | steam | medal | folder
    pub source: &'static str,
    /// Klasör ya da dosya adındaki oyun adı (ör. "Valorant").
    pub hint: Option<String>,
    pub appid: Option<i64>,
    pub size: u64,
    /// Değiştirilme zamanı (ms): kaydın bittiği an.
    pub modified: i64,
}

/// İzlenecek bir kök klasör ve nasıl okunacağı.
#[derive(Clone, Debug, PartialEq)]
pub enum Root {
    /// Windows'un Videolar klasörü (NVIDIA, Xbox, AMD, OBS bir arada).
    Videos(PathBuf),
    /// `<Steam>\userdata` (ekran görüntüleri).
    SteamUserdata(PathBuf),
    Medal(PathBuf),
    /// Kullanıcının eklediği klasör: içindeki her klip/resim.
    Folder(PathBuf),
}

impl Root {
    pub fn path(&self) -> &Path {
        match self {
            Root::Videos(p) | Root::SteamUserdata(p) | Root::Medal(p) | Root::Folder(p) => p,
        }
    }
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

pub fn kind_of(p: &Path) -> Option<&'static str> {
    let e = ext_of(p);
    if VIDEO_EXT.contains(&e.as_str()) {
        Some("clip")
    } else if IMAGE_EXT.contains(&e.as_str()) {
        Some("shot")
    } else {
        None
    }
}

/// `mask`: `#` rakam, diğerleri birebir. `s` içinde ilk eşleşmenin başlangıcı.
fn find_mask(s: &str, mask: &str) -> Option<usize> {
    let b = s.as_bytes();
    let m = mask.as_bytes();
    if b.len() < m.len() {
        return None;
    }
    (0..=b.len() - m.len()).find(|&i| {
        m.iter().enumerate().all(|(j, &c)| {
            let x = b[i + j];
            if c == b'#' {
                x.is_ascii_digit()
            } else {
                x == c
            }
        })
    })
}

const NVIDIA_STAMP: &str = "####.##.## - ##.##.##";
const DASH_STAMP: &str = "####-##-## ##-##-##";

/// Dosya adında tarihten önceki kısım: "Valorant 2024.10.11 - …" → "Valorant".
fn name_before(stem: &str, at: usize) -> Option<String> {
    let n = stem[..at].trim().trim_end_matches(['-', '_']).trim();
    (!n.is_empty()).then(|| n.to_string())
}

/// "Desktop", "Masaüstü" gibi oyun olmayan klasör adları.
pub fn useful_hint(h: &str) -> Option<String> {
    let low = h.trim().to_lowercase();
    let junk = [
        "desktop",
        "masaüstü",
        "clips",
        "screenshots",
        "captures",
        "nvidia",
        "radeon relive",
        "medal",
        "replay",
        "videos",
        "videolar",
        "recordings",
        "kayıtlar",
    ];
    (!low.is_empty() && !junk.contains(&low.as_str())).then(|| h.trim().to_string())
}

fn meta(path: &Path) -> Option<(u64, i64)> {
    let m = std::fs::metadata(path).ok()?;
    if !m.is_file() || m.len() < 10 * 1024 {
        return None;
    }
    let modified = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis() as i64;
    Some((m.len(), modified))
}

fn files(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect()
        })
        .unwrap_or_default()
}

fn dirs(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .filter(|p| {
                    !p.file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with('.'))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn folder_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn push(
    out: &mut Vec<FoundFile>,
    path: PathBuf,
    source: &'static str,
    hint: Option<String>,
    appid: Option<i64>,
) {
    let Some(kind) = kind_of(&path) else { return };
    let Some((size, modified)) = meta(&path) else {
        return;
    };
    out.push(FoundFile {
        path,
        kind,
        source,
        hint: hint.and_then(|h| useful_hint(&h)),
        appid,
        size,
        modified,
    });
}

/// Dizindeki (alt klasörler dahil, `depth` kadar) tüm medya dosyaları; ipucu: dosyanın bulunduğu klasörün adı.
fn walk(out: &mut Vec<FoundFile>, dir: &Path, root: &Path, source: &'static str, depth: u8) {
    for f in files(dir) {
        let hint = (dir != root).then(|| folder_name(dir));
        push(out, f, source, hint, None);
    }
    if depth > 0 {
        for d in dirs(dir) {
            walk(out, &d, root, source, depth - 1);
        }
    }
}

fn scan_videos(out: &mut Vec<FoundFile>, videos: &Path) {
    // Kök: yalnızca OBS adlı kayıtlar (filmler, başka videolar karışmasın).
    for f in files(videos) {
        let stem = f
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if find_mask(&stem, DASH_STAMP).is_some() {
            push(out, f, "obs", None, None);
        } else if let Some(at) = find_mask(&stem, NVIDIA_STAMP) {
            push(out, f, "nvidia", name_before(&stem, at), None);
        }
    }
    for d in dirs(videos) {
        let name = folder_name(&d).to_lowercase();
        match name.as_str() {
            "captures" => {
                for f in files(&d) {
                    let stem = f
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let hint = find_mask(&stem, DASH_STAMP).and_then(|at| name_before(&stem, at));
                    push(out, f, "xbox", hint, None);
                }
            }
            "radeon relive" => walk(out, &d, &d, "amd", 2),
            "nvidia" => walk(out, &d, &d, "nvidia", 2),
            _ => {
                // Oyun adlı klasör: NVIDIA'nın adlandırmasına uyanlar (ya da içindeki "Desktop" gibi alt klasörler).
                let folder = folder_name(&d);
                for f in files(&d) {
                    let stem = f
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if find_mask(&stem, NVIDIA_STAMP).is_some() {
                        push(out, f, "nvidia", Some(folder.clone()), None);
                    }
                }
            }
        }
    }
}

fn scan_steam(out: &mut Vec<FoundFile>, userdata: &Path) {
    for account in dirs(userdata) {
        for app in dirs(&account.join("760").join("remote")) {
            let Ok(appid) = folder_name(&app).parse::<i64>() else {
                continue;
            };
            for f in files(&app.join("screenshots")) {
                push(out, f, "steam", None, Some(appid));
            }
        }
    }
}

pub fn scan(roots: &[Root]) -> Vec<FoundFile> {
    let mut out = Vec::new();
    for r in roots {
        match r {
            Root::Videos(p) => scan_videos(&mut out, p),
            Root::SteamUserdata(p) => scan_steam(&mut out, p),
            Root::Medal(p) => walk(&mut out, p, p, "medal", 3),
            Root::Folder(p) => walk(&mut out, p, p, "folder", 3),
        }
    }
    // Aynı dosya iki kökte (ör. eklenen klasör Videolar'ın altındaysa) bir kez.
    let mut seen = std::collections::HashSet::new();
    out.retain(|f| seen.insert(f.path.to_string_lossy().to_lowercase()));
    out
}

/// Karşılaştırma için: küçük harf, yalnızca harf ve rakam.
pub fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Klasör/dosya adındaki ipucuna en iyi uyan oyun: birebir, yoksa biri ötekini içeriyorsa (en az 4 harf).
pub fn match_hint<'a>(hint: &str, games: &'a [(String, String)]) -> Option<&'a str> {
    let h = norm(hint);
    if h.len() < 2 {
        return None;
    }
    games
        .iter()
        .find(|(_, name)| norm(name) == h)
        .or_else(|| {
            games.iter().find(|(_, name)| {
                let n = norm(name);
                n.len() >= 4 && h.len() >= 4 && (n.contains(&h) || h.contains(&n))
            })
        })
        .map(|(id, _)| id.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, vec![0u8; 12 * 1024]).unwrap();
    }

    #[test]
    fn finds_recordings_by_program() {
        let root = std::env::temp_dir().join(format!("gmid-clips-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let v = root.join("Videos");
        touch(
            &v.join("Valorant")
                .join("Valorant 2024.10.11 - 21.33.12.02.DVR.mp4"),
        );
        touch(
            &v.join("Desktop")
                .join("Desktop 2024.10.11 - 21.40.00.01.mp4"),
        );
        touch(&v.join("Valorant").join("tatil.mp4")); // NVIDIA adına uymuyor: alınmaz
        touch(&v.join("Captures").join("Hades II 2024-10-11 22-01-05.mp4"));
        touch(&v.join("Captures").join("Hades II 2024-10-11 22-02-00.png"));
        touch(&v.join("2024-10-11 23-00-00.mkv"));
        touch(&v.join("Film.mkv")); // kökte rastgele film: alınmaz
        touch(&v.join("Radeon ReLive").join("Elden Ring").join("x.mp4"));
        let ud = root.join("userdata");
        touch(
            &ud.join("123")
                .join("760/remote/1145350/screenshots/20241011_1.jpg"),
        );
        touch(
            &ud.join("123")
                .join("760/remote/1145350/screenshots/thumbnails/20241011_1.jpg"),
        );
        // Yazılmaya yeni başlamış (çok küçük) dosya alınmaz.
        std::fs::write(
            v.join("Captures").join("tiny 2024-10-11 22-03-00.mp4"),
            b"x",
        )
        .unwrap();

        let found = scan(&[Root::Videos(v.clone()), Root::SteamUserdata(ud)]);
        let by = |s: &str| found.iter().find(|f| f.path.to_string_lossy().contains(s));
        assert_eq!(found.len(), 7, "{found:#?}");
        let nv = by("21.33.12").unwrap();
        assert_eq!(
            (nv.source, nv.kind, nv.hint.as_deref()),
            ("nvidia", "clip", Some("Valorant"))
        );
        assert_eq!(by("Desktop 2024").unwrap().hint, None);
        let xb = by("22-01-05").unwrap();
        assert_eq!((xb.source, xb.hint.as_deref()), ("xbox", Some("Hades II")));
        assert_eq!(by("22-02-00").unwrap().kind, "shot");
        assert_eq!(by("23-00-00").unwrap().source, "obs");
        assert_eq!(by("x.mp4").unwrap().hint.as_deref(), Some("Elden Ring"));
        let st = by("20241011_1").unwrap();
        assert_eq!((st.source, st.appid), ("steam", Some(1145350)));
        assert!(by("tatil").is_none() && by("Film").is_none());
    }

    #[test]
    fn hints_match_games() {
        let games = vec![
            ("a".to_string(), "Counter-Strike 2".to_string()),
            ("b".to_string(), "Hades II".to_string()),
            ("c".to_string(), "VALORANT".to_string()),
        ];
        assert_eq!(match_hint("Counter-strike 2", &games), Some("a"));
        assert_eq!(match_hint("Valorant", &games), Some("c"));
        assert_eq!(match_hint("Hades", &games), Some("b"));
        assert_eq!(match_hint("Minecraft", &games), None);
        assert_eq!(useful_hint("Desktop"), None);
    }
}
