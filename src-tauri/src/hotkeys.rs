//! Her yerden (oyun içinden de) çalışan kısayol tuşları.

use crate::audio::{guess_icon, Audio, DevicePrefs, MicOp, MicStatus};
use crate::{db::Db, overlay::show_osd, window};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub struct Action {
    pub id: &'static str,
    /// Ayarlar ekranındaki başlık: "Ses", "Mikrofon", "Uygulama", "Video İndirici"
    pub group: &'static str,
    pub label: &'static str,
    pub default: &'static str,
}

pub const ACTIONS: &[Action] = &[
    Action {
        id: "audio.cycleOutput",
        group: "Ses",
        label: "Ses çıkışını değiştir (kulaklık ↔ hoparlör)",
        default: "Ctrl+Alt+A",
    },
    Action {
        id: "mic.toggleMute",
        group: "Mikrofon",
        label: "Mikrofonu sessize al / aç",
        default: "Ctrl+Alt+M",
    },
    Action {
        id: "mic.toggleLock",
        group: "Mikrofon",
        label: "Mikrofon seviyesi kilidini aç / kapat",
        default: "Ctrl+Alt+L",
    },
    Action {
        id: "mic.up",
        group: "Mikrofon",
        label: "Mikrofon +5",
        default: "Ctrl+Alt+PageUp",
    },
    Action {
        id: "mic.down",
        group: "Mikrofon",
        label: "Mikrofon −5",
        default: "Ctrl+Alt+PageDown",
    },
    Action {
        id: "mic.max",
        group: "Mikrofon",
        label: "Mikrofonu %100'e çek",
        default: "Ctrl+Alt+Home",
    },
    Action {
        id: "note.quick",
        group: "Uygulama",
        label: "Hızlı not",
        default: "Ctrl+Alt+N",
    },
    Action {
        id: "app.show",
        group: "Uygulama",
        label: "GMID Tool penceresini aç",
        default: "Ctrl+Alt+G",
    },
    Action {
        id: "gamemode.toggle",
        group: "Oyun",
        label: "Oyun Modu'nu aç / kapat",
        default: "Ctrl+Alt+O",
    },
    Action {
        id: "panel.toggle",
        group: "Oyun",
        label: "Oyun İçi Panel'i aç / kapat",
        default: "Ctrl+Shift+G",
    },
    Action {
        id: "screen.shot",
        group: "Oyun",
        label: "Ekran görüntüsü al (panoya kopyalanır, oyundan çıkmaz)",
        default: "Ctrl+Alt+P",
    },
    Action {
        id: "videos.clipboard",
        group: "Video İndirici",
        label: "Panodaki bağlantıyı indir (önerilen kalite)",
        default: "Ctrl+Alt+D",
    },
];

const SETTING: &str = "hotkeys";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyStatus {
    pub id: String,
    pub group: String,
    pub label: String,
    /// Boşsa kısayol kapalı.
    pub accel: String,
    pub default_accel: String,
    pub error: Option<String>,
}

struct Bound {
    accel: String,
    shortcut: Option<Shortcut>,
    error: Option<String>,
}

#[derive(Default)]
pub struct Hotkeys(Mutex<HashMap<&'static str, Bound>>);

pub fn plugin() -> TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let state = app.state::<Hotkeys>();
            let action = {
                let map = state.0.lock().unwrap_or_else(|e| e.into_inner());
                map.iter()
                    .find(|(_, b)| b.shortcut.as_ref().is_some_and(|s| s.id() == shortcut.id()))
                    .map(|(id, _)| *id)
            };
            if let Some(action) = action {
                let app = app.app_handle().clone();
                // Kısayol işleyicisi beklemesin diye iş ayrı iş parçacığında.
                std::thread::spawn(move || run(&app, action));
            }
        })
        .build()
}

fn overrides(app: &AppHandle) -> HashMap<String, String> {
    app.state::<Db>().setting_json(SETTING).unwrap_or_default()
}

pub fn register_all(app: &AppHandle) {
    let custom = overrides(app);
    for a in ACTIONS {
        let accel = custom
            .get(a.id)
            .cloned()
            .unwrap_or_else(|| a.default.into());
        bind(app, a.id, accel);
    }
}

fn bind(app: &AppHandle, id: &'static str, accel: String) {
    let state = app.state::<Hotkeys>();
    let mut map = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(old) = map.remove(id).and_then(|b| b.shortcut) {
        let _ = app.global_shortcut().unregister(old);
    }
    let mut bound = Bound {
        accel: accel.clone(),
        shortcut: None,
        error: None,
    };
    if !accel.is_empty() {
        match accel.parse::<Shortcut>() {
            Err(_) => bound.error = Some("Bu tuş birleşimi tanınmadı.".into()),
            Ok(sc) => match app.global_shortcut().register(sc) {
                Ok(()) => bound.shortcut = Some(sc),
                Err(_) => bound.error = Some("Başka bir program bu kısayolu kullanıyor.".into()),
            },
        }
    }
    map.insert(id, bound);
}

pub fn list(app: &AppHandle) -> Vec<HotkeyStatus> {
    let state = app.state::<Hotkeys>();
    let map = state.0.lock().unwrap_or_else(|e| e.into_inner());
    ACTIONS
        .iter()
        .map(|a| {
            let b = map.get(a.id);
            HotkeyStatus {
                id: a.id.into(),
                group: a.group.into(),
                label: a.label.into(),
                accel: b.map_or_else(|| a.default.into(), |b| b.accel.clone()),
                default_accel: a.default.into(),
                error: b.and_then(|b| b.error.clone()),
            }
        })
        .collect()
}

/// Kısayolu değiştirir. `None` varsayılana döndürür, boş metin kapatır.
pub fn set(app: &AppHandle, id: &str, accel: Option<String>) -> Result<Vec<HotkeyStatus>, String> {
    let action = ACTIONS
        .iter()
        .find(|a| a.id == id)
        .ok_or("Bilinmeyen kısayol.")?;
    let accel = accel.map(|a| a.trim().to_string());
    let effective = accel.clone().unwrap_or_else(|| action.default.into());
    // Aynı tuşu başka bir işlem kullanıyorsa izin verme.
    if !effective.is_empty() {
        if let Some(other) = list(app)
            .into_iter()
            .find(|h| h.id != id && h.accel.eq_ignore_ascii_case(&effective))
        {
            return Err(format!(
                "Bu kısayol zaten \"{}\" için kullanılıyor.",
                other.label
            ));
        }
    }
    bind(app, action.id, effective);
    let mut custom = overrides(app);
    match accel {
        Some(a) => custom.insert(id.into(), a),
        None => custom.remove(id),
    };
    let json = serde_json::to_string(&custom).map_err(|e| e.to_string())?;
    app.state::<Db>()
        .set_setting(SETTING, &json)
        .map_err(|e| e.to_string())?;
    Ok(list(app))
}

// ---- kısayolların yaptığı işler ----

fn run(app: &AppHandle, action: &str) {
    match action {
        "audio.cycleOutput" => cycle_output(app),
        "mic.toggleMute" => mic(app, MicOp::ToggleMute),
        "mic.toggleLock" => mic(app, MicOp::ToggleLock(lock_target(app))),
        "mic.up" => mic(app, MicOp::Step(0.05)),
        "mic.down" => mic(app, MicOp::Step(-0.05)),
        "mic.max" => mic(app, MicOp::Max),
        "note.quick" => window::open_quick_note(app),
        "app.show" => window::show_main(app),
        "videos.clipboard" => download_clipboard(app),
        "screen.shot" => crate::screenshot::take(app),
        "gamemode.toggle" => {
            app.state::<crate::gamemode::GameMode>().toggle(app);
        }
        "panel.toggle" => crate::panel::toggle(app),
        _ => {}
    }
}

/// Kopyalanan video bağlantısını önerilen kaliteyle sıraya ekler (IDM'deki gibi).
fn download_clipboard(app: &AppHandle) {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let text = app.clipboard().read_text().unwrap_or_default();
    let url = text.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return show_osd(app, "info", "Panoda video bağlantısı yok");
    }
    show_osd(app, "download", "Video aranıyor…");
    match app.state::<crate::downloader::Downloader>().enqueue(
        app,
        url,
        "auto",
        None,
        "app",
        &crate::downloader::ytdlp::Context::default(),
        None,
    ) {
        Ok(d) => show_osd(app, "download", format!("Sıraya eklendi: {}", d.title)),
        Err(e) if e == "PLAYLIST" => {
            show_osd(
                app,
                "download",
                "Bu bir çalma listesi; Video İndirici'de seç",
            );
            crate::window::show_main_at(app, "videos".into());
        }
        Err(e) if e == "TOOLS_MISSING" => {
            show_osd(app, "info", "Önce Video İndirici'de araçları kur")
        }
        Err(e) => show_osd(app, "info", e),
    }
}

/// Kilit açılınca tutulacak seviye (ayar: `audio.micLockLevel`, varsayılan %100).
pub fn lock_target(app: &AppHandle) -> f32 {
    app.state::<Db>()
        .setting_json::<f32>("audio.micLockLevel")
        .unwrap_or(1.0)
        .clamp(0.0, 1.0)
}

pub fn mic(app: &AppHandle, op: MicOp) {
    let was_lock_op = matches!(op, MicOp::ToggleLock(_));
    let op_kind = if matches!(op, MicOp::ToggleMute) {
        OpKind::Mute
    } else {
        OpKind::Other
    };
    match app.state::<Audio>().mic(op) {
        Ok(s) => {
            if !was_lock_op && matches!(op_kind, OpKind::Mute) {
                crate::chime::mic(app, s.muted);
            }
            let (icon, text) = mic_message(&s, was_lock_op);
            show_osd(app, icon, text);
        }
        Err(e) => show_osd(app, "info", e),
    }
}

enum OpKind {
    Mute,
    Other,
}

fn mic_message(s: &MicStatus, lock_op: bool) -> (&'static str, String) {
    let pct = (s.level * 100.0).round() as u32;
    if lock_op {
        return match s.lock {
            Some(l) => (
                "lock",
                format!("Mikrofon kilidi açık · %{}", (l * 100.0).round() as u32),
            ),
            None => ("mic", "Mikrofon kilidi kapalı".into()),
        };
    }
    if s.muted {
        ("micoff", "Mikrofon kapalı".into())
    } else {
        let lock = if s.lock.is_some() { " · kilitli" } else { "" };
        ("mic", format!("Mikrofon açık · %{pct}{lock}"))
    }
}

/// Gizlenmemiş çıkış cihazları arasında sıradakine geçer.
pub fn cycle_output(app: &AppHandle) {
    let audio = app.state::<Audio>();
    let prefs: HashMap<String, DevicePrefs> = app
        .state::<Db>()
        .setting_json("audio.devices")
        .unwrap_or_default();
    let state = match audio.state() {
        Ok(s) if s.supported => s,
        Ok(_) => return show_osd(app, "info", "Ses ayarları sadece Windows'ta çalışır."),
        Err(e) => return show_osd(app, "info", e),
    };
    let visible: Vec<_> = state
        .outputs
        .iter()
        .filter(|d| !prefs.get(&d.id).is_some_and(|p| p.hidden))
        .collect();
    if visible.len() < 2 {
        return show_osd(app, "speaker", "Geçilecek başka ses cihazı yok");
    }
    let current = visible
        .iter()
        .position(|d| d.is_default)
        .unwrap_or(visible.len() - 1);
    let next = visible[(current + 1) % visible.len()];
    let pref = prefs.get(&next.id);
    let name = pref
        .and_then(|p| p.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| next.name.clone());
    let icon = match pref.and_then(|p| p.icon.as_deref()) {
        Some("headphones") => "headphones",
        Some("monitor") => "monitor",
        Some("speaker") => "speaker",
        _ => guess_icon(&next.name),
    };
    match audio.set_default(next.id.clone()) {
        Ok(()) => show_osd(app, icon, format!("Ses çıkışı: {name}")),
        Err(e) => show_osd(app, "info", e),
    }
}
