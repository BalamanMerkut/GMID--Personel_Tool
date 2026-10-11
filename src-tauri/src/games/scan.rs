//! Kurulu oyunları bulur:
//! - **Steam:** `libraryfolders.vdf` → her kütüphanedeki `appmanifest_<appid>.acf`; oynama süresi ve son
//!   oynama `userdata/<hesap>/config/localconfig.vdf` içinden (API anahtarı gerekmez).
//! - **Epic:** `C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests\*.item` (JSON). Epic süre tutmaz;
//!   süre GMID'in kendi takibinden gelir.

use super::vdf::{self, Vdf};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Taramada bulunan (henüz kütüphaneye eklenmemiş olabilir) oyun.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub source: String,
    pub source_id: String,
    pub name: String,
    pub install_dir: Option<String>,
    pub exes: Vec<String>,
    pub launch: Option<String>,
    pub steam_appid: Option<i64>,
    /// Platformun tuttuğu süre (dakika).
    pub platform_minutes: i64,
    /// Platformun bildiği son oynama (ms).
    pub last_played: Option<i64>,
    /// Kütüphanede zaten var mı?
    #[serde(default)]
    pub already: bool,
}

/// Oyun olmayan Steam kayıtları (çalışma ortamları, sürücüler, araçlar).
fn steam_skip(appid: &str, name: &str) -> bool {
    const SKIP_IDS: &[&str] = &[
        "228980", "1070560", "1391110", "1628350", "1493710", "250820",
    ];
    let n = name.to_lowercase();
    SKIP_IDS.contains(&appid)
        || n.contains("redistributable")
        || n.starts_with("proton ")
        || n.starts_with("steam linux runtime")
        || n.starts_with("steamvr")
        || n.contains("dedicated server")
        || n.ends_with(" sdk")
}

/// Oyun klasöründeki .exe adları (kurulum, çökme bildirici, yükleyici gibi yardımcılar hariç).
pub fn game_exes(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let path = e.path();
            let name = e.file_name().to_string_lossy().to_lowercase();
            if path.is_dir() {
                // Ön koşul kurulumları ve araç klasörleri atlanır.
                if depth < 3
                    && ![
                        "_commonredist",
                        "redist",
                        "redistributables",
                        "directx",
                        "vcredist",
                        "support",
                        "__installer",
                        "installers",
                    ]
                    .contains(&name.as_str())
                {
                    walk(&path, depth + 1, out);
                }
            } else if name.ends_with(".exe")
                && ![
                    "unins",
                    "uninstall",
                    "setup",
                    "crash",
                    "report",
                    "redist",
                    "vc_",
                    "dxsetup",
                    "dotnet",
                    "ue4prereq",
                    "installer",
                    "updater",
                    "cefsharp",
                    "unitycrashhandler",
                ]
                .iter()
                .any(|bad| name.contains(bad))
                && !out.contains(&name)
            {
                out.push(name);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, 0, &mut out);
    out.truncate(40);
    out
}

fn read(path: &Path) -> Option<Vdf> {
    std::fs::read_to_string(path).ok().map(|s| vdf::parse(&s))
}

/// Steam kütüphane klasörleri (ana klasör dahil).
fn steam_libraries(steam: &Path) -> Vec<PathBuf> {
    let mut libs = vec![steam.to_path_buf()];
    if let Some(v) = read(&steam.join("steamapps").join("libraryfolders.vdf")) {
        if let Some(m) = v.get("libraryfolders").and_then(Vdf::map) {
            for (k, entry) in m {
                if !k.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                // Yeni biçim: { "path" "…" }, eski biçim: "1" "D:\\SteamLibrary"
                let p = entry.get_str("path").or_else(|| entry.str());
                if let Some(p) = p {
                    let p = PathBuf::from(p);
                    if !libs.iter().any(|l| {
                        l.to_string_lossy()
                            .eq_ignore_ascii_case(&p.to_string_lossy())
                    }) {
                        libs.push(p);
                    }
                }
            }
        }
    }
    libs
}

/// Bütün Steam hesaplarındaki oynama süreleri: appid → (dakika, son oynama ms).
fn steam_playtimes(steam: &Path) -> HashMap<String, (i64, Option<i64>)> {
    let mut out: HashMap<String, (i64, Option<i64>)> = HashMap::new();
    let Ok(users) = std::fs::read_dir(steam.join("userdata")) else {
        return out;
    };
    for u in users.flatten() {
        let Some(v) = read(&u.path().join("config").join("localconfig.vdf")) else {
            continue;
        };
        let Some(apps) = v
            .path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"])
            .and_then(Vdf::map)
        else {
            continue;
        };
        for (appid, a) in apps {
            let minutes = a
                .get_str("Playtime")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let last = a
                .get_str("LastPlayed")
                .and_then(|s| s.parse::<i64>().ok())
                .filter(|t| *t > 0)
                .map(|t| t * 1000);
            let e = out.entry(appid.clone()).or_insert((0, None));
            e.0 = e.0.max(minutes);
            e.1 = e.1.max(last);
        }
    }
    out
}

pub fn steam(steam: &Path) -> Vec<Found> {
    let times = steam_playtimes(steam);
    let mut out: Vec<Found> = Vec::new();
    for lib in steam_libraries(steam) {
        let apps = lib.join("steamapps");
        let Ok(rd) = std::fs::read_dir(&apps) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
                continue;
            }
            let Some(v) = read(&e.path()) else {
                continue;
            };
            let Some(st) = v.get("AppState") else {
                continue;
            };
            let (Some(appid), Some(title)) = (st.get_str("appid"), st.get_str("name")) else {
                continue;
            };
            if steam_skip(appid, title) || out.iter().any(|f| f.source_id == appid) {
                continue;
            }
            let dir = st
                .get_str("installdir")
                .map(|d| apps.join("common").join(d))
                .filter(|d| d.is_dir());
            let (minutes, last) = times.get(appid).copied().unwrap_or((0, None));
            out.push(Found {
                source: "steam".into(),
                source_id: appid.to_string(),
                name: title.to_string(),
                exes: dir.as_deref().map(game_exes).unwrap_or_default(),
                install_dir: dir.map(|d| d.to_string_lossy().into_owned()),
                launch: Some(format!("steam://rungameid/{appid}")),
                steam_appid: appid.parse().ok(),
                platform_minutes: minutes,
                last_played: last,
                already: false,
            });
        }
    }
    out
}

pub fn epic(manifests: &Path) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    let Ok(rd) = std::fs::read_dir(manifests) else {
        return out;
    };
    for e in rd.flatten() {
        if e.path().extension().and_then(|x| x.to_str()) != Some("item") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(e.path()) else {
            continue;
        };
        let Ok(j) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let s = |k: &str| j[k].as_str().unwrap_or_default().to_string();
        let (app, name, dir) = (s("AppName"), s("DisplayName"), s("InstallLocation"));
        let incomplete = j["bIsIncompleteInstall"].as_bool().unwrap_or(false);
        let is_game = j["AppCategories"]
            .as_array()
            .map_or(true, |c| c.iter().any(|x| x.as_str() == Some("games")));
        if app.is_empty()
            || name.is_empty()
            || incomplete
            || !is_game
            || app.starts_with("UE_")
            || out.iter().any(|f| f.source_id == app)
        {
            continue;
        }
        let dir_path = PathBuf::from(&dir);
        let mut exes = if dir_path.is_dir() {
            game_exes(&dir_path)
        } else {
            Vec::new()
        };
        if let Some(main) = Path::new(&s("LaunchExecutable"))
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
        {
            if !exes.contains(&main) {
                exes.insert(0, main);
            }
        }
        out.push(Found {
            source: "epic".into(),
            launch: Some(format!(
                "com.epicgames.launcher://apps/{app}?action=launch&silent=true"
            )),
            source_id: app,
            name,
            install_dir: (!dir.is_empty()).then_some(dir),
            exes,
            steam_appid: None,
            platform_minutes: 0,
            last_played: None,
            already: false,
        });
    }
    out
}

/// Bu bilgisayardaki Steam klasörü.
pub fn steam_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(p) = registry_string(r"Software\Valve\Steam", "SteamPath") {
            let p = PathBuf::from(p.replace('/', "\\"));
            if p.is_dir() {
                return Some(p);
            }
        }
        let p = PathBuf::from(r"C:\Program Files (x86)\Steam");
        p.is_dir().then_some(p)
    }
    #[cfg(not(windows))]
    {
        let p = PathBuf::from(std::env::var_os("HOME")?).join(".steam/steam");
        p.is_dir().then_some(p)
    }
}

pub fn epic_dir() -> Option<PathBuf> {
    let base = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    let p = base.join(r"Epic\EpicGamesLauncher\Data\Manifests");
    p.is_dir().then_some(p)
}

#[cfg(windows)]
fn registry_string(key: &str, value: &str) -> Option<String> {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
    let mut buf = vec![0u16; 1024];
    let mut len = (buf.len() * 2) as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key),
            &HSTRING::from(value),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut len),
        )
    };
    if r.is_err() {
        return None;
    }
    let n = (len as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..n.min(buf.len())]))
}

/// Bütün kaynakları tarar.
/// Riot oyunları (League of Legends, VALORANT…): Riot istemcisi her ürün için bir ayar dosyası tutar
/// (`ProgramData\Riot Games\Metadata\<ürün>.<kanal>\<ürün>.<kanal>.product_settings.yaml`), içinde kurulum yolu.
/// Başlatma Riot istemcisiyle: `RiotClientServices.exe --launch-product=valorant --launch-patchline=live`.
pub fn riot(base: &Path) -> Vec<Found> {
    const PRODUCTS: &[(&str, &str, &[&str])] = &[
        (
            "valorant",
            "VALORANT",
            &["valorant-win64-shipping.exe", "valorant.exe"],
        ),
        (
            "league_of_legends",
            "League of Legends",
            &[
                "league of legends.exe",
                "leagueclient.exe",
                "leagueclientux.exe",
            ],
        ),
        ("bacon", "Legends of Runeterra", &["lor.exe"]),
        ("lion", "2XKO", &[]),
    ];
    let client: Option<String> = std::fs::read_to_string(base.join("RiotClientInstalls.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|j| {
            ["rc_default", "rc_live"]
                .iter()
                .find_map(|k| j[*k].as_str().map(str::to_string))
        });
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(base.join("Metadata")) else {
        return out;
    };
    for e in rd.flatten() {
        let folder = e.file_name().to_string_lossy().into_owned();
        let Some((product, patchline)) = folder.split_once('.') else {
            continue;
        };
        let Some((_, name, known)) = PRODUCTS.iter().find(|(p, _, _)| *p == product) else {
            continue;
        };
        let Ok(yaml) =
            std::fs::read_to_string(e.path().join(format!("{folder}.product_settings.yaml")))
        else {
            continue;
        };
        let Some(dir) = yaml.lines().find_map(|l| {
            l.trim()
                .strip_prefix("product_install_full_path:")
                .map(|v| v.trim().trim_matches(['"', '\'']).to_string())
        }) else {
            continue;
        };
        if dir.is_empty()
            || !Path::new(&dir).is_dir()
            || out.iter().any(|f: &Found| f.source_id == product)
        {
            continue;
        }
        let dir = dir.replace('/', "\\");
        let mut exes: Vec<String> = known.iter().map(|x| x.to_string()).collect();
        if exes.is_empty() {
            exes = game_exes(Path::new(&dir));
        }
        out.push(Found {
            source: "riot".into(),
            source_id: product.to_string(),
            name: name.to_string(),
            launch: client.as_ref().map(|c| {
                format!("\"{c}\" --launch-product={product} --launch-patchline={patchline}")
            }),
            install_dir: Some(dir),
            exes,
            steam_appid: None,
            platform_minutes: 0,
            last_played: None,
            already: false,
        });
    }
    out
}

pub fn riot_dir() -> Option<PathBuf> {
    let base = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    let p = base.join("Riot Games");
    p.is_dir().then_some(p)
}

/// `"C:/a b/x.exe" --arg1 --arg2` → ("C:/a b/x.exe", ["--arg1", "--arg2"]); tırnaksızsa tamamı yol.
pub fn split_command(s: &str) -> (String, Vec<String>) {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            let args = rest[end + 1..]
                .split_whitespace()
                .map(str::to_string)
                .collect();
            return (rest[..end].to_string(), args);
        }
    }
    (s.to_string(), Vec::new())
}

/// Minecraft: Java sürümü `javaw.exe` ile çalışır (hangi başlatıcıyla açılırsa açılsın), bu yüzden
/// "pencere başlığı Minecraft ile başlayan javaw.exe" olarak tanınır (`javaw.exe@minecraft`).
/// Bedrock (Microsoft Store) `Minecraft.Windows.exe`.
pub fn minecraft(appdata: &Path, local: &Path, launchers: &[PathBuf]) -> Vec<Found> {
    let mut out = Vec::new();
    let java_dir = appdata.join(".minecraft");
    let launcher = launchers.iter().find(|p| p.is_file());
    if java_dir.is_dir() || launcher.is_some() {
        out.push(Found {
            source: "other".into(),
            source_id: "minecraft-java".into(),
            name: "Minecraft".into(),
            install_dir: java_dir
                .is_dir()
                .then(|| java_dir.to_string_lossy().into_owned()),
            exes: vec!["javaw.exe@minecraft".into()],
            launch: Some(match launcher {
                Some(l) => l.to_string_lossy().into_owned(),
                // Microsoft Store'dan kurulan başlatıcı.
                None => r#""explorer.exe" shell:AppsFolder\Microsoft.4297127D64EC6_8wekyb3d8bbwe!Minecraft"#
                    .into(),
            }),
            steam_appid: None,
            platform_minutes: 0,
            last_played: None,
            already: false,
        });
    }
    if local
        .join("Packages")
        .join("Microsoft.MinecraftUWP_8wekyb3d8bbwe")
        .is_dir()
    {
        out.push(Found {
            source: "xbox".into(),
            source_id: "minecraft-bedrock".into(),
            name: "Minecraft for Windows".into(),
            install_dir: None,
            exes: vec!["minecraft.windows.exe".into()],
            launch: Some("minecraft://".into()),
            steam_appid: None,
            platform_minutes: 0,
            last_played: None,
            already: false,
        });
    }
    out
}

/// `<sürücü>:\XboxGames\<Oyun>\Content\MicrosoftGame.config` (Xbox uygulaması / Game Pass).
pub fn xbox(roots: &[PathBuf]) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else {
            continue;
        };
        for e in rd.flatten() {
            let content = e.path().join("Content");
            let Ok(xml) = std::fs::read_to_string(content.join("MicrosoftGame.config")) else {
                continue;
            };
            let folder = e.file_name().to_string_lossy().into_owned();
            // Minecraft başlatıcısı ayrıca ele alınıyor.
            if folder.eq_ignore_ascii_case("Minecraft Launcher") {
                continue;
            }
            let name = xml_attr(&xml, "ShellVisuals", "DefaultDisplayName")
                .filter(|n| !n.starts_with("ms-resource"))
                .unwrap_or_else(|| folder.clone());
            let id = xml_attr(&xml, "Identity", "Name").unwrap_or_else(|| folder.clone());
            let exe = xml_attr(&xml, "Executable", "Name");
            if out.iter().any(|f| f.source_id == id) {
                continue;
            }
            let mut exes = game_exes(&content);
            if let Some(main) = exe
                .as_deref()
                .and_then(|x| Path::new(x).file_name())
                .map(|n| n.to_string_lossy().to_lowercase())
            {
                if !exes.contains(&main) {
                    exes.insert(0, main);
                }
            }
            out.push(Found {
                source: "xbox".into(),
                source_id: id,
                name,
                install_dir: Some(content.to_string_lossy().into_owned()),
                exes,
                launch: exe.map(|x| content.join(x).to_string_lossy().into_owned()),
                steam_appid: None,
                platform_minutes: 0,
                last_played: None,
                already: false,
            });
        }
    }
    out
}

/// `<Tag ... Attr="değer"` → değer (basit; bu dosyalar için yeterli).
fn xml_attr(xml: &str, tag: &str, attr: &str) -> Option<String> {
    // "<Executable" ararken "<ExecutableList>" eşleşmesin: etiket adından sonra boşluk gelmeli.
    let open = format!("<{tag}");
    let start = xml.match_indices(&open).map(|(i, _)| i).find(|&i| {
        xml[i + open.len()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    })?;
    let rest = &xml[start..];
    let end = rest.find('>').unwrap_or(rest.len());
    let head = &rest[..end];
    let key = format!("{attr}=\"");
    let i = head.find(&key)? + key.len();
    let v = &head[i..];
    Some(v[..v.find('"')?].to_string()).filter(|s| !s.is_empty())
}

fn env_dir(key: &str) -> Option<PathBuf> {
    std::env::var_os(key).map(PathBuf::from)
}

pub fn all() -> Vec<Found> {
    let mut out = steam_dir().map(|d| steam(&d)).unwrap_or_default();
    out.extend(epic_dir().map(|d| epic(&d)).unwrap_or_default());
    out.extend(riot_dir().map(|d| riot(&d)).unwrap_or_default());
    if let (Some(appdata), Some(local)) = (env_dir("APPDATA"), env_dir("LOCALAPPDATA")) {
        let mut launchers: Vec<PathBuf> = Vec::new();
        for key in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(p) = env_dir(key) {
                launchers.push(p.join("Minecraft Launcher").join("MinecraftLauncher.exe"));
            }
        }
        for letter in b'C'..=b'Z' {
            launchers.push(PathBuf::from(format!(
                "{}:\\XboxGames\\Minecraft Launcher\\Content\\Minecraft.exe",
                letter as char
            )));
        }
        out.extend(minecraft(&appdata, &local, &launchers));
    }
    if cfg!(windows) {
        let roots: Vec<PathBuf> = (b'C'..=b'Z')
            .map(|l| PathBuf::from(format!("{}:\\XboxGames", l as char)))
            .filter(|p| p.is_dir())
            .collect();
        out.extend(xbox(&roots));
    }
    out.sort_by_key(|f| f.name.to_lowercase());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minecraft_and_xbox_are_found() {
        let root = std::env::temp_dir().join(format!("gmid-mc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let appdata = root.join("Roaming");
        let local = root.join("Local");
        std::fs::create_dir_all(appdata.join(".minecraft")).unwrap();
        std::fs::create_dir_all(local.join("Packages/Microsoft.MinecraftUWP_8wekyb3d8bbwe"))
            .unwrap();
        let found = minecraft(&appdata, &local, &[root.join("yok.exe")]);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].exes, ["javaw.exe@minecraft"]);
        assert!(found[0]
            .launch
            .as_deref()
            .unwrap()
            .contains("shell:AppsFolder"));
        assert_eq!(found[1].launch.as_deref(), Some("minecraft://"));

        let xg = root.join("XboxGames");
        let fh = xg.join("Forza Horizon 5/Content");
        std::fs::create_dir_all(&fh).unwrap();
        std::fs::write(fh.join("ForzaHorizon5.exe"), "").unwrap();
        std::fs::write(
            fh.join("MicrosoftGame.config"),
            r#"<Game configVersion="1"><Identity Name="Microsoft.624F8B84B80" Publisher="CN=x" Version="1"/>
            <ExecutableList><Executable Name="ForzaHorizon5.exe" Id="Game"/></ExecutableList>
            <ShellVisuals DefaultDisplayName="Forza Horizon 5" PublisherDisplayName="Xbox"/></Game>"#,
        )
        .unwrap();
        std::fs::create_dir_all(xg.join("Minecraft Launcher/Content")).unwrap();
        std::fs::write(
            xg.join("Minecraft Launcher/Content/MicrosoftGame.config"),
            "<Game/>",
        )
        .unwrap();
        let x = xbox(&[xg]);
        assert_eq!(x.len(), 1, "{x:?}");
        assert_eq!(
            (x[0].name.as_str(), x[0].source_id.as_str()),
            ("Forza Horizon 5", "Microsoft.624F8B84B80")
        );
        assert!(x[0].exes.contains(&"forzahorizon5.exe".to_string()));
        assert!(x[0]
            .launch
            .as_deref()
            .unwrap()
            .ends_with("ForzaHorizon5.exe"));
    }

    #[test]
    fn riot_games_are_found() {
        let root = std::env::temp_dir().join(format!("gmid-riot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let base = root.join("Riot Games");
        let val = root.join("Games/VALORANT/live");
        std::fs::create_dir_all(&val).unwrap();
        std::fs::create_dir_all(base.join("Metadata/valorant.live")).unwrap();
        std::fs::create_dir_all(base.join("Metadata/league_of_legends.live")).unwrap();
        std::fs::create_dir_all(base.join("Metadata/Riot Client")).unwrap();
        std::fs::write(
            base.join("RiotClientInstalls.json"),
            r#"{"rc_default":"C:/Riot Games/Riot Client/RiotClientServices.exe"}"#,
        )
        .unwrap();
        std::fs::write(
            base.join("Metadata/valorant.live/valorant.live.product_settings.yaml"),
            format!(
                "product_install_full_path: \"{}\"\nproduct_install_root: \"x\"\n",
                val.to_string_lossy().replace('\\', "/")
            ),
        )
        .unwrap();
        // LoL kaldırılmış: klasörü yok, alınmaz.
        std::fs::write(
            base.join(
                "Metadata/league_of_legends.live/league_of_legends.live.product_settings.yaml",
            ),
            "product_install_full_path: \"Z:/yok/League of Legends\"\n",
        )
        .unwrap();
        let found = riot(&base);
        assert_eq!(found.len(), 1, "{found:?}");
        let v = &found[0];
        assert_eq!(
            (v.source.as_str(), v.source_id.as_str(), v.name.as_str()),
            ("riot", "valorant", "VALORANT")
        );
        assert!(v.exes.contains(&"valorant-win64-shipping.exe".to_string()));
        let (exe, args) = split_command(v.launch.as_deref().unwrap());
        assert_eq!(exe, "C:/Riot Games/Riot Client/RiotClientServices.exe");
        assert_eq!(
            args,
            ["--launch-product=valorant", "--launch-patchline=live"]
        );
        assert_eq!(split_command(r"C:\Oyun\a.exe").1.len(), 0);
    }

    #[test]
    fn steam_and_epic_are_found() {
        let root = std::env::temp_dir().join(format!("gmid-scan-{}", std::process::id()));
        let steam_root = root.join("Steam");
        let lib2 = root.join("Lib2");
        let apps = steam_root.join("steamapps");
        std::fs::create_dir_all(apps.join("common/Hades II/bin")).unwrap();
        std::fs::create_dir_all(lib2.join("steamapps/common/dota 2 beta")).unwrap();
        std::fs::write(apps.join("common/Hades II/bin/Hades2.exe"), "").unwrap();
        std::fs::write(apps.join("common/Hades II/unins000.exe"), "").unwrap();
        std::fs::write(
            apps.join("libraryfolders.vdf"),
            format!(
                "\"libraryfolders\" {{ \"0\" {{ \"path\" \"{}\" }} \"1\" {{ \"path\" \"{}\" }} }}",
                steam_root.display(),
                lib2.display()
            ),
        )
        .unwrap();
        std::fs::write(
            apps.join("appmanifest_1145350.acf"),
            "\"AppState\" { \"appid\" \"1145350\" \"name\" \"Hades II\" \"installdir\" \"Hades II\" }",
        )
        .unwrap();
        std::fs::write(
            apps.join("appmanifest_228980.acf"),
            "\"AppState\" { \"appid\" \"228980\" \"name\" \"Steamworks Common Redistributables\" \"installdir\" \"x\" }",
        )
        .unwrap();
        std::fs::write(
            lib2.join("steamapps/appmanifest_570.acf"),
            "\"AppState\" { \"appid\" \"570\" \"name\" \"Dota 2\" \"installdir\" \"dota 2 beta\" }",
        )
        .unwrap();
        let cfg = steam_root.join("userdata/123/config");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::write(
            cfg.join("localconfig.vdf"),
            "\"UserLocalConfigStore\" { \"Software\" { \"valve\" { \"Steam\" { \"apps\" { \"570\" { \"Playtime\" \"600\" \"LastPlayed\" \"1700000000\" } } } } } }",
        )
        .unwrap();

        let mut found = steam(&steam_root);
        found.sort_by_key(|f| f.name.clone());
        assert_eq!(found.len(), 2, "çalışma ortamı atlanır");
        assert_eq!(found[0].name, "Dota 2");
        assert_eq!(found[0].platform_minutes, 600);
        assert_eq!(found[0].last_played, Some(1_700_000_000_000));
        assert_eq!(found[1].exes, ["hades2.exe"]);
        assert_eq!(
            found[1].launch.as_deref(),
            Some("steam://rungameid/1145350")
        );

        let man = root.join("Manifests");
        std::fs::create_dir_all(&man).unwrap();
        std::fs::write(
            man.join("a.item"),
            r#"{"AppName":"Fortnite","DisplayName":"Fortnite","InstallLocation":"","LaunchExecutable":"FortniteGame/Binaries/Win64/FortniteLauncher.exe","AppCategories":["public","games"]}"#,
        )
        .unwrap();
        std::fs::write(
            man.join("b.item"),
            r#"{"AppName":"UE_5.3","DisplayName":"Unreal Engine","InstallLocation":"","AppCategories":["engines"]}"#,
        )
        .unwrap();
        let e = epic(&man);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].exes, ["fortnitelauncher.exe"]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
