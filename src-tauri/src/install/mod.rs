//! Format Sonrası Kurulum: hazır katalog, kendi listen, winget ile sırayla sessiz kurulum ve
//! formattan sonra GMID kurulu değilken de çalışan "Format Kiti" (bkz. `kit`).
//!
//! winget Windows 10/11'de yerleşik, Microsoft'un resmî paket yöneticisi. Katalogdaki kimlikler
//! microsoft/winget-pkgs deposunda doğrulandı (2026-10). Mağaza uygulamaları `msstore`
//! kaynağından, winget'te olmayanlar (NVIDIA App gibi) resmî indirme sayfasıyla gelir.

pub mod kit;

use crate::db::Db;
use crate::util::{hide_console, now_ms};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub const LIST_KEY: &str = "install.list";
/// "Uygulama Yükleyicisi" (winget) Microsoft Store sayfası.
pub const APP_INSTALLER_STORE: &str = "ms-windows-store://pdp/?ProductId=9NBLGGH4NNS1";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    #[default]
    Winget,
    Msstore,
    /// winget'te yok: resmî indirme sayfası açılır.
    Web,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Winget => "winget",
            Source::Msstore => "msstore",
            Source::Web => "web",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// winget / Mağaza kimliği; web için kısa bir ad.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub source: Source,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl Item {
    /// Betiğe ve komut satırına güvenle konabilir mi (yalnız kimlik karakterleri, https adres).
    pub fn is_valid(&self) -> bool {
        let id_ok = !self.id.is_empty()
            && self.id.len() <= 128
            && self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._+-".contains(c));
        match self.source {
            Source::Web => {
                id_ok
                    && self.url.as_deref().is_some_and(|u| {
                        u.starts_with("https://")
                            && !u
                                .chars()
                                .any(|c| c.is_whitespace() || c == '\'' || c == '"')
                    })
            }
            _ => id_ok,
        }
    }
}

// (kimlik, ad, kategori, kaynak, adres, not)
type Entry = (
    &'static str,
    &'static str,
    &'static str,
    Source,
    &'static str,
    &'static str,
);

const W: Source = Source::Winget;

const CATALOG: &[Entry] = &[
    ("Google.Chrome", "Google Chrome", "Tarayıcılar", W, "", ""),
    ("Mozilla.Firefox", "Firefox", "Tarayıcılar", W, "", ""),
    ("Brave.Brave", "Brave", "Tarayıcılar", W, "", ""),
    ("Opera.OperaGX", "Opera GX", "Tarayıcılar", W, "", ""),
    ("Valve.Steam", "Steam", "Oyun platformları", W, "", ""),
    (
        "EpicGames.EpicGamesLauncher",
        "Epic Games",
        "Oyun platformları",
        W,
        "",
        "",
    ),
    (
        "ElectronicArts.EADesktop",
        "EA app",
        "Oyun platformları",
        W,
        "",
        "",
    ),
    (
        "Ubisoft.Connect",
        "Ubisoft Connect",
        "Oyun platformları",
        W,
        "",
        "",
    ),
    (
        "Blizzard.BattleNet",
        "Battle.net",
        "Oyun platformları",
        W,
        "",
        "",
    ),
    ("GOG.Galaxy", "GOG Galaxy", "Oyun platformları", W, "", ""),
    (
        "RiotGames.LeagueOfLegends.TR",
        "League of Legends (TR)",
        "Oyun platformları",
        W,
        "",
        "Riot Client ile gelir",
    ),
    (
        "RiotGames.Valorant.EU",
        "VALORANT",
        "Oyun platformları",
        W,
        "",
        "Riot Client ile gelir",
    ),
    (
        "Mojang.MinecraftLauncher",
        "Minecraft Launcher",
        "Oyun platformları",
        W,
        "",
        "",
    ),
    (
        "Overwolf.CurseForge",
        "CurseForge",
        "Oyun platformları",
        W,
        "",
        "Mod yöneticisi",
    ),
    (
        "Playnite.Playnite",
        "Playnite",
        "Oyun platformları",
        W,
        "",
        "",
    ),
    ("Discord.Discord", "Discord", "İletişim", W, "", ""),
    (
        "9NKSQGP7F2NH",
        "WhatsApp",
        "İletişim",
        Source::Msstore,
        "",
        "Microsoft Store'dan",
    ),
    (
        "Telegram.TelegramDesktop",
        "Telegram",
        "İletişim",
        W,
        "",
        "",
    ),
    (
        "TeamSpeakSystems.TeamSpeakClient",
        "TeamSpeak 3",
        "İletişim",
        W,
        "",
        "",
    ),
    ("Zoom.Zoom", "Zoom", "İletişim", W, "", ""),
    ("Anthropic.Claude", "Claude", "Araçlar", W, "", ""),
    ("RARLab.WinRAR", "WinRAR", "Araçlar", W, "", ""),
    ("7zip.7zip", "7-Zip", "Araçlar", W, "", ""),
    ("Notepad++.Notepad++", "Notepad++", "Araçlar", W, "", ""),
    ("Microsoft.PowerToys", "PowerToys", "Araçlar", W, "", ""),
    (
        "voidtools.Everything",
        "Everything",
        "Araçlar",
        W,
        "",
        "Anında dosya arama",
    ),
    (
        "ShareX.ShareX",
        "ShareX",
        "Araçlar",
        W,
        "",
        "Ekran görüntüsü",
    ),
    (
        "qBittorrent.qBittorrent",
        "qBittorrent",
        "Araçlar",
        W,
        "",
        "",
    ),
    (
        "AnyDesk.AnyDesk",
        "AnyDesk",
        "Araçlar",
        W,
        "",
        "Uzak masaüstü",
    ),
    (
        "Bitwarden.Bitwarden",
        "Bitwarden",
        "Araçlar",
        W,
        "",
        "Şifre yöneticisi",
    ),
    ("Google.GoogleDrive", "Google Drive", "Araçlar", W, "", ""),
    ("Obsidian.Obsidian", "Obsidian", "Araçlar", W, "", ""),
    (
        "Adobe.Acrobat.Reader.64-bit",
        "Adobe Acrobat Reader",
        "Araçlar",
        W,
        "",
        "",
    ),
    ("VideoLAN.VLC", "VLC", "Medya", W, "", ""),
    ("Spotify.Spotify", "Spotify", "Medya", W, "", ""),
    ("OBSProject.OBSStudio", "OBS Studio", "Medya", W, "", ""),
    ("clsid2.mpc-hc", "MPC-HC", "Medya", W, "", ""),
    ("Daum.PotPlayer", "PotPlayer", "Medya", W, "", ""),
    ("Audacity.Audacity", "Audacity", "Medya", W, "", ""),
    ("HandBrake.HandBrake", "HandBrake", "Medya", W, "", ""),
    ("GIMP.GIMP.3", "GIMP", "Medya", W, "", ""),
    (
        "nvidia-app",
        "NVIDIA App",
        "Donanım",
        Source::Web,
        "https://www.nvidia.com/tr-tr/software/nvidia-app/",
        "winget'te yok: indirme sayfası açılır",
    ),
    (
        "amd-adrenalin",
        "AMD Adrenalin",
        "Donanım",
        Source::Web,
        "https://www.amd.com/en/support/download/drivers.html",
        "winget'te yok: indirme sayfası açılır",
    ),
    (
        "Guru3D.Afterburner",
        "MSI Afterburner",
        "Donanım",
        W,
        "",
        "",
    ),
    (
        "Guru3D.RTSS",
        "RivaTuner (RTSS)",
        "Donanım",
        W,
        "",
        "FPS göstergesi",
    ),
    ("Logitech.GHUB", "Logitech G HUB", "Donanım", W, "", ""),
    (
        "RazerInc.RazerInstaller.Synapse4",
        "Razer Synapse 4",
        "Donanım",
        W,
        "",
        "",
    ),
    ("SteelSeries.GG", "SteelSeries GG", "Donanım", W, "", ""),
    ("Corsair.iCUE.5", "Corsair iCUE", "Donanım", W, "", ""),
    ("CPUID.CPU-Z", "CPU-Z", "Donanım", W, "", ""),
    ("TechPowerUp.GPU-Z", "GPU-Z", "Donanım", W, "", ""),
    ("REALiX.HWiNFO", "HWiNFO", "Donanım", W, "", ""),
    (
        "CrystalDewWorld.CrystalDiskInfo",
        "CrystalDiskInfo",
        "Donanım",
        W,
        "",
        "",
    ),
    (
        "Microsoft.VCRedist.2015+.x64",
        "Visual C++ (2015-2022)",
        "Oyun bileşenleri",
        W,
        "",
        "Çoğu oyun ister",
    ),
    (
        "Microsoft.DirectX",
        "DirectX (eski oyunlar)",
        "Oyun bileşenleri",
        W,
        "",
        "",
    ),
    (
        "Microsoft.DotNet.DesktopRuntime.8",
        ".NET 8 Masaüstü",
        "Oyun bileşenleri",
        W,
        "",
        "",
    ),
    (
        "Microsoft.VisualStudioCode",
        "VS Code",
        "Geliştirme",
        W,
        "",
        "",
    ),
    ("Git.Git", "Git", "Geliştirme", W, "", ""),
    ("Python.Python.3.13", "Python 3.13", "Geliştirme", W, "", ""),
    ("OpenJS.NodeJS.LTS", "Node.js LTS", "Geliştirme", W, "", ""),
];

/// Liste hiç düzenlenmediyse (Tasarım soru 6'nın varsayılan cevabı).
const DEFAULT_LIST: &[&str] = &[
    "Google.Chrome",
    "Valve.Steam",
    "Discord.Discord",
    "Anthropic.Claude",
    "RARLab.WinRAR",
    "EpicGames.EpicGamesLauncher",
    "nvidia-app",
    "VideoLAN.VLC",
    "7zip.7zip",
];

pub fn catalog() -> Vec<Item> {
    CATALOG
        .iter()
        .map(|&(id, name, cat, source, url, note)| Item {
            id: id.into(),
            name: name.into(),
            source,
            category: Some(cat.into()),
            url: (!url.is_empty()).then(|| url.into()),
            note: (!note.is_empty()).then(|| note.into()),
        })
        .collect()
}

pub fn list(db: &Db) -> Vec<Item> {
    if let Some(items) = db.setting_json::<Vec<Item>>(LIST_KEY) {
        return items;
    }
    let all = catalog();
    DEFAULT_LIST
        .iter()
        .filter_map(|id| all.iter().find(|i| i.id == *id).cloned())
        .collect()
}

pub fn save_list(db: &Db, items: &[Item]) -> Result<(), String> {
    let mut seen = HashSet::new();
    let clean: Vec<&Item> = items
        .iter()
        .filter(|i| i.is_valid() && seen.insert(i.id.to_lowercase()))
        .collect();
    let json = serde_json::to_string(&clean).map_err(|e| e.to_string())?;
    db.set_setting(LIST_KEY, &json).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------- winget

fn winget() -> Command {
    let mut cmd = Command::new("winget");
    hide_console(&mut cmd);
    cmd
}

pub fn winget_ready() -> bool {
    winget()
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn not_ready() -> String {
    "winget (Uygulama Yükleyicisi) bulunamadı. Microsoft Store'dan \"Uygulama Yükleyicisi\"ni kurup ya da güncelleyip tekrar dene."
        .into()
}

pub fn install_args(item: &Item) -> Vec<String> {
    [
        "install",
        "--id",
        &item.id,
        "-e",
        "--source",
        item.source.as_str(),
        "--silent",
        "--accept-package-agreements",
        "--accept-source-agreements",
        "--disable-interactivity",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// winget'in bilinen çıkış kodları (i32 olarak; asıl değerler 0x8A15xxxx).
const ALREADY: &[i32] = &[
    -1978335189, // 0x8A15002B: güncelleme yok (zaten kurulu ve güncel)
    -1978335135, // 0x8A150061: paket zaten kurulu
];
const REBOOT: &[i32] = &[
    -1978334967, // 0x8A150109: yeniden başlatma gerekiyor
    -1978334966, // 0x8A15010A
];
const NOT_FOUND: i32 = -1978335212; // 0x8A150014

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepState {
    Waiting,
    Running,
    Done,
    Already,
    Opened,
    Failed,
    Skipped,
}

pub fn outcome(code: i32) -> (StepState, Option<String>) {
    if code == 0 {
        (StepState::Done, None)
    } else if ALREADY.contains(&code) {
        (StepState::Already, Some("Zaten kurulu".into()))
    } else if REBOOT.contains(&code) {
        (
            StepState::Done,
            Some("Kuruldu · bilgisayarı yeniden başlatman gerekebilir".into()),
        )
    } else if code == NOT_FOUND {
        (StepState::Failed, Some("paket bulunamadı".into()))
    } else {
        (
            StepState::Failed,
            Some(format!("kod 0x{:08X}", code as u32)),
        )
    }
}

/// winget çıktısındaki "12.5 MB / 100 MB" ya da "45%" → 0..1.
pub fn progress_of(line: &str) -> Option<f32> {
    fn bytes(num: &str, unit: &str) -> Option<f64> {
        let n: f64 = num.replace(',', ".").parse().ok()?;
        let m = match unit {
            "B" => 1.0,
            "KB" => 1024.0,
            "MB" => 1024.0 * 1024.0,
            "GB" => 1024.0 * 1024.0 * 1024.0,
            _ => return None,
        };
        Some(n * m)
    }
    let t: Vec<&str> = line.split_whitespace().collect();
    for (i, w) in t.iter().enumerate() {
        if *w == "/" && i >= 2 && i + 2 < t.len() {
            if let (Some(a), Some(b)) = (bytes(t[i - 2], t[i - 1]), bytes(t[i + 1], t[i + 2])) {
                if b > 0.0 {
                    return Some((a / b).clamp(0.0, 1.0) as f32);
                }
            }
        }
    }
    t.iter().rev().find_map(|w| {
        let n: f32 = w.strip_suffix('%')?.parse().ok()?;
        Some((n / 100.0).clamp(0.0, 1.0))
    })
}

/// Çıktının anlamlı son satırı (ilerleme çubukları ve dönen çizgiler hariç).
fn meaningful(line: &str) -> bool {
    let l = line.trim();
    l.chars().filter(|c| c.is_alphabetic()).count() >= 4
        && progress_of(l).is_none()
        && !l.contains('█')
        && !l.contains('▒')
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: String,
    pub name: String,
    pub state: StepState,
    pub progress: Option<f32>,
    pub message: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RunState {
    pub running: bool,
    pub stopping: bool,
    pub steps: Vec<Step>,
    pub started: Option<i64>,
    pub finished: Option<i64>,
}

#[derive(Default)]
pub struct Installer {
    run: Mutex<RunState>,
    cancel: AtomicBool,
    /// Kurulu paket kimlikleri (küçük harf) ve ne zaman bakıldığı.
    installed: Mutex<Option<(i64, HashSet<String>)>>,
}

impl Installer {
    pub fn state(&self) -> RunState {
        self.run.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn update(&self, app: &AppHandle, f: impl FnOnce(&mut RunState)) {
        let snapshot = {
            let mut s = self.run.lock().unwrap_or_else(|e| e.into_inner());
            f(&mut s);
            s.clone()
        };
        let _ = app.emit("install", &snapshot);
    }

    fn set_step(&self, app: &AppHandle, i: usize, f: impl FnOnce(&mut Step)) {
        self.update(app, |s| {
            if let Some(step) = s.steps.get_mut(i) {
                f(step);
            }
        });
    }

    /// Şu an kurulan bitince durur (kurulumun ortasında kesmek programı yarım bırakabilir).
    pub fn stop(&self, app: &AppHandle) {
        self.cancel.store(true, Ordering::Relaxed);
        self.update(app, |s| s.stopping = s.running);
    }

    pub fn start(&self, app: &AppHandle, items: Vec<Item>) -> Result<RunState, String> {
        let items: Vec<Item> = items.into_iter().filter(Item::is_valid).collect();
        if items.is_empty() {
            return Err("Kurulacak program seçilmedi".into());
        }
        if self.state().running {
            return Err("Kurulum zaten sürüyor".into());
        }
        if items.iter().any(|i| i.source != Source::Web) && !winget_ready() {
            let _ = tauri_plugin_opener::open_url(APP_INSTALLER_STORE, None::<&str>);
            return Err(not_ready());
        }
        self.cancel.store(false, Ordering::Relaxed);
        self.update(app, |s| {
            *s = RunState {
                running: true,
                stopping: false,
                started: Some(now_ms()),
                finished: None,
                steps: items
                    .iter()
                    .map(|i| Step {
                        id: i.id.clone(),
                        name: i.name.clone(),
                        state: StepState::Waiting,
                        progress: None,
                        message: None,
                    })
                    .collect(),
            }
        });
        let app = app.clone();
        std::thread::spawn(move || {
            let me = app.state::<Installer>();
            for (i, item) in items.iter().enumerate() {
                if me.cancel.load(Ordering::Relaxed) {
                    me.set_step(&app, i, |s| {
                        s.state = StepState::Skipped;
                        s.message = Some("Durduruldu".into());
                    });
                    continue;
                }
                me.set_step(&app, i, |s| s.state = StepState::Running);
                let (state, message) = if item.source == Source::Web {
                    let url = item.url.clone().unwrap_or_default();
                    match tauri_plugin_opener::open_url(&url, None::<&str>) {
                        Ok(()) => (
                            StepState::Opened,
                            Some("İndirme sayfası tarayıcıda açıldı".into()),
                        ),
                        Err(e) => (StepState::Failed, Some(e.to_string())),
                    }
                } else {
                    me.run_one(&app, i, item)
                };
                me.set_step(&app, i, |s| {
                    s.state = state;
                    s.message = message;
                    s.progress = None;
                });
            }
            // Kurulu listesi değişti.
            *me.installed.lock().unwrap_or_else(|e| e.into_inner()) = None;
            me.update(&app, |s| {
                s.running = false;
                s.stopping = false;
                s.finished = Some(now_ms());
            });
            crate::overlay::show_osd(&app, "install", "Format Sonrası Kurulum bitti");
        });
        Ok(self.state())
    }

    fn run_one(&self, app: &AppHandle, i: usize, item: &Item) -> (StepState, Option<String>) {
        let mut child = match winget()
            .args(install_args(item))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                return (
                    StepState::Failed,
                    Some(format!("winget başlatılamadı: {e}")),
                )
            }
        };
        let mut last_line = String::new();
        let mut last_emit = Instant::now() - Duration::from_secs(1);
        if let Some(mut out) = child.stdout.take() {
            let mut buf = [0u8; 4096];
            let mut pending = String::new();
            while let Ok(n) = out.read(&mut buf) {
                if n == 0 {
                    break;
                }
                pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                // winget ilerlemeyi aynı satırın üstüne (\r) yazar.
                while let Some(pos) = pending.find(['\r', '\n']) {
                    let line: String = pending.drain(..=pos).collect();
                    let line = line.trim();
                    if let Some(p) = progress_of(line) {
                        if last_emit.elapsed() >= Duration::from_millis(250) {
                            last_emit = Instant::now();
                            self.set_step(app, i, |s| s.progress = Some(p));
                        }
                    } else if meaningful(line) {
                        last_line = line.to_string();
                        self.set_step(app, i, |s| {
                            s.progress = None;
                            s.message = Some(last_line.clone());
                        });
                    }
                }
            }
        }
        let code = match child.wait() {
            Ok(s) => s.code().unwrap_or(-1),
            Err(e) => return (StepState::Failed, Some(e.to_string())),
        };
        let (state, msg) = outcome(code);
        let msg = match (state, msg) {
            (StepState::Failed, Some(m)) if !last_line.is_empty() => {
                Some(format!("{m}: {last_line}"))
            }
            (_, m) => m,
        };
        if state == StepState::Failed {
            crate::log!(
                "Kurulum: {} ({}) başarısız, kod {code}: {last_line}",
                item.name,
                item.id
            );
        }
        (state, msg)
    }

    /// Kurulu programların winget kimlikleri (küçük harf). 10 dakika önbellekte tutulur.
    pub fn installed(&self, refresh: bool) -> Result<Vec<String>, String> {
        if !refresh {
            if let Some((at, ids)) = self
                .installed
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
            {
                if now_ms() - at < 10 * 60 * 1000 {
                    return Ok(ids.iter().cloned().collect());
                }
            }
        }
        if !winget_ready() {
            return Err(not_ready());
        }
        let tmp = std::env::temp_dir().join(format!("gmid-winget-{}.json", std::process::id()));
        let status = winget()
            .args(["export", "-o"])
            .arg(&tmp)
            .args(["--accept-source-agreements", "--disable-interactivity"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        let text = std::fs::read_to_string(&tmp).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp);
        if text.is_empty() {
            return Err(format!(
                "Kurulu programlar okunamadı (kod {:?})",
                status.code()
            ));
        }
        let ids = parse_export(&text);
        *self.installed.lock().unwrap_or_else(|e| e.into_inner()) = Some((now_ms(), ids.clone()));
        Ok(ids.into_iter().collect())
    }
}

/// `winget export` JSON'u → kimlikler (küçük harf).
pub fn parse_export(text: &str) -> HashSet<String> {
    let v: serde_json::Value =
        serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap_or_default();
    v["Sources"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|s| s["Packages"].as_array().into_iter().flatten())
        .filter_map(|p| p["PackageIdentifier"].as_str())
        .map(|s| s.to_lowercase())
        .collect()
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub id: String,
    pub name: String,
    pub version: String,
}

/// Microsoft'un paket deposunun tamamında arar.
pub fn search(query: &str) -> Result<Vec<Hit>, String> {
    let q = query.trim();
    if q.chars().count() < 2 {
        return Ok(Vec::new());
    }
    if !winget_ready() {
        return Err(not_ready());
    }
    let out = winget()
        .args(["search", q, "--source", "winget", "--count", "40"])
        .args(["--accept-source-agreements", "--disable-interactivity"])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    Ok(parse_search(&String::from_utf8_lossy(&out.stdout)))
}

/// `winget search` tablosu: başlık satırı, altında "-----" çizgisi, sonra satırlar. Sütunlar
/// başlıktaki kelimelerin yerinden bulunur (başlıklar Türkçe de olabilir).
pub fn parse_search(text: &str) -> Vec<Hit> {
    // İlerleme göstergesi aynı satırın üstüne \r ile yazılır; satırın son hali kalsın.
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.rsplit('\r').next().unwrap_or(l))
        .collect();
    let Some(dash) = lines
        .iter()
        .position(|l| l.trim().len() >= 10 && l.trim().chars().all(|c| c == '-'))
    else {
        return Vec::new();
    };
    let Some(header) = dash.checked_sub(1).map(|i| lines[i]) else {
        return Vec::new();
    };
    let hc: Vec<char> = header.chars().collect();
    let mut starts = Vec::new();
    for (i, c) in hc.iter().enumerate() {
        if !c.is_whitespace() && (i == 0 || hc[i - 1].is_whitespace()) {
            starts.push(i);
        }
    }
    if starts.len() < 3 {
        return Vec::new();
    }
    let col = |cs: &[char], n: usize| -> String {
        let a = starts[n].min(cs.len());
        let b = starts.get(n + 1).copied().unwrap_or(cs.len()).min(cs.len());
        cs[a..b].iter().collect::<String>().trim().to_string()
    };
    lines[dash + 1..]
        .iter()
        .filter_map(|l| {
            let cs: Vec<char> = l.chars().collect();
            let id = col(&cs, 1);
            let ok = id.contains('.')
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._+-".contains(c));
            ok.then(|| Hit {
                name: col(&cs, 0),
                id,
                version: col(&cs, 2),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_ids_are_safe_and_unique() {
        let all = catalog();
        let mut seen = HashSet::new();
        for i in &all {
            assert!(i.is_valid(), "{}", i.id);
            assert!(seen.insert(i.id.to_lowercase()), "{}", i.id);
        }
        for id in DEFAULT_LIST {
            assert!(all.iter().any(|i| i.id == *id), "{id}");
        }
    }

    #[test]
    fn unsafe_items_are_rejected() {
        let mut i = Item {
            id: "Foo.Bar".into(),
            name: "x".into(),
            source: Source::Winget,
            category: None,
            url: None,
            note: None,
        };
        assert!(i.is_valid());
        i.id = "Foo'; rm".into();
        assert!(!i.is_valid());
        i.id = "site".into();
        i.source = Source::Web;
        i.url = Some("http://x".into());
        assert!(!i.is_valid());
        i.url = Some("https://ornek.com/a'b".into());
        assert!(!i.is_valid());
        i.url = Some("https://ornek.com/indir".into());
        assert!(i.is_valid());
    }

    #[test]
    fn exit_codes() {
        assert_eq!(outcome(0).0, StepState::Done);
        assert_eq!(outcome(0x8A15002Bu32 as i32).0, StepState::Already);
        assert_eq!(outcome(0x8A150061u32 as i32).0, StepState::Already);
        assert_eq!(outcome(0x8A150014u32 as i32).0, StepState::Failed);
        assert_eq!(outcome(0x8A150109u32 as i32).0, StepState::Done);
        assert_eq!(outcome(1603).1.as_deref(), Some("kod 0x00000643"));
    }

    #[test]
    fn progress_lines() {
        assert_eq!(progress_of("  ██████▒▒▒▒  50.0 MB /  100 MB"), Some(0.5));
        assert_eq!(progress_of("1024 KB / 2,0 MB"), Some(0.5));
        assert_eq!(progress_of("  ███  45%"), Some(0.45));
        assert_eq!(progress_of("Successfully installed"), None);
        assert!(meaningful("Paket yüklemesi başlatılıyor..."));
        assert!(!meaningful("   - "));
        assert!(!meaningful("  ██████▒▒▒▒  50.0 MB /  100 MB"));
    }

    #[test]
    fn search_table_is_parsed() {
        let text = "   - \r   \\ \r\
Name                 Id                          Version      Match         Source\n\
---------------------------------------------------------------------------------\n\
Google Chrome        Google.Chrome               141.0.7390.66              winget\n\
Chrome Remote Deskt… Google.ChromeRemoteDesktop… 141.0.7390.4  Tag: chrome  winget\n\
Ungoogled Chromium   eloston.ungoogled-chromium  140.0.7339   Tag: chrome   winget\n";
        let hits = parse_search(text);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "Google.Chrome");
        assert_eq!(hits[0].name, "Google Chrome");
        assert_eq!(hits[0].version, "141.0.7390.66");
        assert_eq!(hits[1].id, "eloston.ungoogled-chromium");
        // Türkçe başlıklar
        let tr = "Ad        Kimlik         Sürüm\n------------------------------\nDiscord   Discord.Discord 1.0\n";
        let hits = parse_search(tr);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Discord");
        assert!(parse_search("Girişle eşleşen paket bulunamadı.").is_empty());
    }

    #[test]
    fn export_json_is_parsed() {
        let text = "\u{feff}{\"Sources\":[{\"Packages\":[{\"PackageIdentifier\":\"Google.Chrome\"},{\"PackageIdentifier\":\"Valve.Steam\"}],\"SourceDetails\":{\"Name\":\"winget\"}},{\"Packages\":[{\"PackageIdentifier\":\"9NKSQGP7F2NH\"}]}]}";
        let ids = parse_export(text);
        assert!(
            ids.contains("google.chrome")
                && ids.contains("valve.steam")
                && ids.contains("9nksqgp7f2nh")
        );
    }
}
