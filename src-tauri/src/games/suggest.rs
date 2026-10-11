//! Kütüphanede olmayan bir oyun açılınca "Kütüphaneye ekleyeyim mi?" diye sormak.
//!
//! İki ipucu:
//! 1. **Discord'un algılanabilir oyunlar listesi** (`/applications/detectable`, herkese açık, haftada bir indirilir):
//!    binlerce oyunun exe adı. Bilinen bir oyun 20 sn açık kalırsa sorulur.
//! 2. **Tam ekran:** listede olmayan bir program 1 dakikadan uzun süre ön planda ve tam ekransa (tarayıcı, video
//!    oynatıcı gibi bilinen programlar hariç) sorulur.
//!
//! "Bir daha sorma" denen program unutulmaz; "Şimdi değil" denen bir sonraki açılışında yeniden sorulur.

use super::procs::Proc;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

pub const ASK_KEY: &str = "games.ask";
const LIST_URL: &str = "https://discord.com/api/v9/applications/detectable";
const LIST_MAX_AGE: Duration = Duration::from_secs(7 * 86_400);
/// Bilinen bir oyun bu kadar açık kalınca sorulur (başlatıcıların kısa süreli açılışları sayılmasın).
pub const KNOWN_AFTER_MS: i64 = 20_000;
/// Listede olmayan program bu kadar tam ekran kalınca sorulur.
pub const FULLSCREEN_AFTER_MS: i64 = 60_000;

/// Çok genel adlar: yalnızca klasörüyle birlikte eşleşir.
const GENERIC: &[&str] = &[
    "launcher.exe",
    "game.exe",
    "javaw.exe",
    "java.exe",
    "python.exe",
    "pythonw.exe",
    "node.exe",
    "electron.exe",
    "start.exe",
    "play.exe",
    "client.exe",
    "app.exe",
    "main.exe",
    "setup.exe",
];

/// Tam ekran çalışsa da oyun olmayan programlar.
const NOT_GAMES: &[&str] = &[
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "opera.exe",
    "brave.exe",
    "vivaldi.exe",
    "iexplore.exe",
    "vlc.exe",
    "mpc-hc.exe",
    "mpc-hc64.exe",
    "mpc-be.exe",
    "mpc-be64.exe",
    "potplayer.exe",
    "potplayermini.exe",
    "potplayermini64.exe",
    "kmplayer.exe",
    "mpv.exe",
    "wmplayer.exe",
    "video.ui.exe",
    "microsoft.media.player.exe",
    "applicationframehost.exe",
    "microsoft.photos.exe",
    "photos.exe",
    "discord.exe",
    "spotify.exe",
    "obs64.exe",
    "explorer.exe",
    "powerpnt.exe",
    "winword.exe",
    "excel.exe",
    "acrobat.exe",
    "acrord32.exe",
    "sumatrapdf.exe",
    "mstsc.exe",
    "code.exe",
    "steam.exe",
    "steamwebhelper.exe",
    "epicgameslauncher.exe",
    "riotclientux.exe",
    "riotclientservices.exe",
    "battle.net.exe",
    "eadesktop.exe",
    "upc.exe",
    "gamebar.exe",
    "textinputhost.exe",
    "searchhost.exe",
    "startmenuexperiencehost.exe",
    "shellexperiencehost.exe",
    "lockapp.exe",
    "zoom.exe",
    "teams.exe",
    "ms-teams.exe",
    "telegram.exe",
    "whatsapp.exe",
    "gmid-tool.exe",
    "gmid tool.exe",
    "nvidia overlay.exe",
    "nvcontainer.exe",
    "taskmgr.exe",
    "netflix.exe",
    "primevideo.exe",
];

/// Discord'un listesi: exe adı → (klasörlü son ek, oyun adı).
#[derive(Default)]
pub struct DetectList {
    by_exe: HashMap<String, Vec<(String, String)>>,
}

impl DetectList {
    pub fn parse(json: &str) -> Option<Self> {
        let apps: Vec<serde_json::Value> = serde_json::from_str(json).ok()?;
        let mut by_exe: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for app in &apps {
            let Some(name) = app["name"].as_str() else {
                continue;
            };
            for x in app["executables"].as_array().into_iter().flatten() {
                if x["os"].as_str() != Some("win32") || x["is_launcher"].as_bool() == Some(true) {
                    continue;
                }
                let Some(raw) = x["name"].as_str() else {
                    continue;
                };
                if raw.starts_with('>') {
                    continue;
                }
                let suffix = raw.to_lowercase().replace('/', "\\");
                let base = suffix.rsplit('\\').next().unwrap_or(&suffix).to_string();
                if !base.ends_with(".exe")
                    || (!suffix.contains('\\') && GENERIC.contains(&base.as_str()))
                {
                    continue;
                }
                by_exe
                    .entry(base)
                    .or_default()
                    .push((suffix, name.to_string()));
            }
        }
        (!by_exe.is_empty()).then_some(Self { by_exe })
    }

    pub fn len(&self) -> usize {
        self.by_exe.len()
    }

    /// Program bilinen bir oyun mu; öyleyse adı.
    pub fn find(&self, p: &Proc) -> Option<&str> {
        self.by_exe.get(&p.name)?.iter().find_map(|(suffix, name)| {
            let ok = !suffix.contains('\\')
                || (!p.path.is_empty() && p.path.ends_with(&format!("\\{suffix}")));
            ok.then_some(name.as_str())
        })
    }
}

/// Windows'un kendi programları.
pub fn system_path(path: &str) -> bool {
    path.contains("\\windows\\") || path.contains("\\windowsapps\\")
}

pub fn not_a_game(p: &Proc) -> bool {
    NOT_GAMES.contains(&p.name.as_str()) || system_path(&p.path)
}

/// Pencere başlığından oyun adı: "ELDEN RING™" → "ELDEN RING"; boş ya da çok uzunsa exe adı.
pub fn clean_title(title: &str, exe: &str) -> String {
    let t: String = title
        .chars()
        .filter(|c| !matches!(c, '™' | '®' | '©'))
        .collect::<String>()
        .trim()
        .to_string();
    if t.is_empty() || t.chars().count() > 60 {
        // Windows yolu (ayraç \\): her sistemde aynı sonuç için elle ayrılır.
        let file = exe.rsplit(['\\', '/']).next().unwrap_or(exe);
        file.rsplit_once('.')
            .map_or(file, |(stem, _)| stem)
            .to_string()
    } else {
        t
    }
}

/// Listeyi önbellekten okur; yoksa ya da bir haftadan eskiyse Discord'dan indirir.
pub fn load_list(cache: &Path) -> Option<DetectList> {
    let fresh = std::fs::metadata(cache)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age < LIST_MAX_AGE);
    if !fresh {
        match download() {
            Ok(text) if DetectList::parse(&text).is_some() => {
                if let Some(d) = cache.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                let _ = std::fs::write(cache, &text);
            }
            Ok(_) => crate::log!("Oyun listesi beklenmedik biçimde geldi"),
            Err(e) => crate::log!("Oyun listesi indirilemedi: {e}"),
        }
    }
    // İndirilemediyse eski önbellek de iş görür.
    DetectList::parse(&std::fs::read_to_string(cache).ok()?)
}

fn download() -> Result<String, String> {
    let mut r = crate::net::agent(Duration::from_secs(15), Duration::from_secs(90))
        .get(LIST_URL)
        .header("User-Agent", "GMIDTool/0.1")
        .call()
        .map_err(|e| e.to_string())?;
    r.body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str, path: &str) -> Proc {
        Proc {
            pid: 1,
            name: name.into(),
            path: path.into(),
        }
    }

    #[test]
    fn discord_list_is_parsed_and_matched() {
        let json = r#"[
          {"id":"1","name":"Grand Theft Auto V","executables":[{"is_launcher":false,"name":"gta5.exe","os":"win32"},{"is_launcher":false,"name":"gta5","os":"darwin"}]},
          {"id":"2","name":"World of Warcraft","executables":[{"is_launcher":false,"name":"_retail_/wow.exe","os":"win32"}]},
          {"id":"3","name":"Some Launcher","executables":[{"is_launcher":true,"name":"somelauncher.exe","os":"win32"}]},
          {"id":"4","name":"Generic","executables":[{"is_launcher":false,"name":"game.exe","os":"win32"},{"is_launcher":false,"name":">weird.exe","os":"win32"}]},
          {"id":"5","name":"Hollow Knight","executables":[{"is_launcher":false,"name":"hollow knight/hollow_knight.exe","os":"win32"}]}
        ]"#;
        let l = DetectList::parse(json).unwrap();
        assert_eq!(
            l.find(&p("gta5.exe", r"d:\games\gtav\gta5.exe")),
            Some("Grand Theft Auto V")
        );
        assert_eq!(
            l.find(&p(
                "wow.exe",
                r"c:\program files\world of warcraft\_retail_\wow.exe"
            )),
            Some("World of Warcraft")
        );
        assert_eq!(
            l.find(&p("wow.exe", r"c:\başka\wow.exe")),
            None,
            "klasörü tutmuyor"
        );
        assert_eq!(
            l.find(&p("somelauncher.exe", r"c:\x\somelauncher.exe")),
            None,
            "başlatıcı"
        );
        assert_eq!(
            l.find(&p("game.exe", r"c:\x\game.exe")),
            None,
            "çok genel ad"
        );
        assert_eq!(
            l.find(&p(
                "hollow_knight.exe",
                r"d:\steam\common\hollow knight\hollow_knight.exe"
            )),
            Some("Hollow Knight")
        );
        assert!(DetectList::parse("{}").is_none());
    }

    #[test]
    fn titles_and_filters() {
        assert_eq!(clean_title("ELDEN RING™", "eldenring.exe"), "ELDEN RING");
        assert_eq!(clean_title("  ", r"c:\x\Hades2.exe"), "Hades2");
        assert!(not_a_game(&p(
            "chrome.exe",
            r"c:\program files\google\chrome.exe"
        )));
        assert!(not_a_game(&p("x.exe", r"c:\windows\system32\x.exe")));
        assert!(!not_a_game(&p("gta5.exe", r"d:\games\gta5.exe")));
    }
}
