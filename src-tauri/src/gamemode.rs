//! Oyun Modu: tek tıkla (ya da kütüphanedeki bir oyun açılınca kendiliğinden) oyuna hazırlanır,
//! kapanınca her şeyi eski haline getirir.
//!
//! - Güç planı → Yüksek performans (önceki plan hatırlanır)
//! - Ses çıkışı → seçilen cihaz (ör. kulaklık)
//! - Mikrofon seviyesi kilidi açılır
//! - GMID'in sesleri ve Windows bildirimleri susar (ekran göstergesi kalır)
//! - Hareketli arka plan durur
//! - Seçilen uygulamalar (ör. tarayıcı) kapatılır (yeniden açılmaz)

use crate::audio::{Audio, MicOp};
use crate::db::Db;
use crate::overlay::show_osd;
use crate::util::{hide_console, now_ms};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

pub const PROFILE_KEY: &str = "gamemode.profile";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    /// Kütüphanedeki bir oyun açılınca kendiliğinden aç, kapanınca kapat.
    pub auto: bool,
    pub power: bool,
    /// Geçilecek ses çıkışı (cihaz kimliği); `None` değiştirme.
    pub output: Option<String>,
    pub mic_lock: bool,
    pub quiet: bool,
    pub stop_background: bool,
    /// Kapatılacak programlar (exe adları).
    pub close_apps: Vec<String>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            auto: false,
            power: true,
            output: None,
            mic_lock: true,
            quiet: true,
            stop_background: true,
            close_apps: Vec::new(),
        }
    }
}

/// Açılırken değiştirilenlerin eski halleri.
#[derive(Clone, Debug, Default)]
struct Saved {
    power: Option<String>,
    output: Option<String>,
    /// Kilit değiştirildiyse önceki kilit (`Some(None)`: kilit kapalıydı).
    lock: Option<Option<f32>>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub active: bool,
    /// manual | auto
    pub reason: Option<String>,
    pub since: Option<i64>,
    pub quiet: bool,
    pub stop_background: bool,
}

#[derive(Default)]
pub struct GameMode {
    state: Mutex<Option<(Saved, Status)>>,
}

pub fn profile(db: &Db) -> Profile {
    db.setting_json(PROFILE_KEY).unwrap_or_default()
}

/// "Güç Düzeni GUID'si: 381b4222-f694-41f0-9685-ff5bb260df2e  (Dengeli)" → GUID (dil fark etmez).
pub fn parse_guid(text: &str) -> Option<String> {
    let b = text.as_bytes();
    let shape = [8, 4, 4, 4, 12];
    (0..b.len()).find_map(|start| {
        let mut i = start;
        for (n, len) in shape.iter().enumerate() {
            if i + len > b.len() || !b[i..i + len].iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            i += len;
            if n < 4 {
                if b.get(i) != Some(&b'-') {
                    return None;
                }
                i += 1;
            }
        }
        // Daha uzun bir onaltılık dizinin parçası olmasın.
        (start == 0 || !b[start - 1].is_ascii_hexdigit()).then(|| text[start..i].to_lowercase())
    })
}

fn powercfg(args: &[&str]) -> Result<String, String> {
    let mut cmd = std::process::Command::new("powercfg");
    cmd.args(args);
    hide_console(&mut cmd);
    let o = cmd.output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    if o.status.success() {
        Ok(text)
    } else {
        Err(format!("{text}{}", String::from_utf8_lossy(&o.stderr))
            .trim()
            .to_string())
    }
}

fn close_app(exe: &str) {
    // /F yok: programa "kapan" denir (kaydetmesi gereken bir şey varsa sorar).
    let mut cmd = std::process::Command::new("taskkill");
    cmd.args(["/IM", exe]);
    hide_console(&mut cmd);
    let _ = cmd.output();
}

impl GameMode {
    pub fn status(&self) -> Status {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|(_, s)| s.clone())
            .unwrap_or_default()
    }

    pub fn is_quiet(&self) -> bool {
        let s = self.status();
        s.active && s.quiet
    }

    pub fn enable(&self, app: &AppHandle, reason: &str) -> Status {
        if self.status().active {
            return self.status();
        }
        let db = app.state::<Db>();
        let p = profile(&db);
        let audio = app.state::<Audio>();
        let mut saved = Saved::default();
        if p.power && cfg!(windows) {
            match powercfg(&["/getactivescheme"]).map(|t| parse_guid(&t)) {
                Ok(Some(prev)) => {
                    saved.power = Some(prev);
                    // SCHEME_MIN: Yüksek performans (bazı dizüstülerde yok; o zaman dokunulmaz).
                    if let Err(e) = powercfg(&["/setactive", "SCHEME_MIN"]) {
                        crate::log!("Oyun Modu: güç planı değiştirilemedi: {e}");
                        saved.power = None;
                    }
                }
                Ok(None) => {}
                Err(e) => crate::log!("Oyun Modu: güç planı okunamadı: {e}"),
            }
        }
        if let Ok(st) = audio.state() {
            if let Some(target) = p.output.as_ref() {
                let current = st
                    .outputs
                    .iter()
                    .find(|d| d.is_default)
                    .map(|d| d.id.clone());
                if current.as_ref() != Some(target)
                    && st.outputs.iter().any(|d| &d.id == target)
                    && audio.set_default(target.clone()).is_ok()
                {
                    saved.output = current;
                }
            }
            if p.mic_lock && st.lock.is_none() {
                let level = crate::hotkeys::lock_target(app);
                if audio.mic(MicOp::SetLock(Some(level))).is_ok() {
                    saved.lock = Some(st.lock);
                }
            }
        }
        for exe in &p.close_apps {
            close_app(exe);
        }
        let status = Status {
            active: true,
            reason: Some(reason.to_string()),
            since: Some(now_ms()),
            quiet: p.quiet,
            stop_background: p.stop_background,
        };
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = Some((saved, status.clone()));
        show_osd(app, "bolt", "Oyun Modu açık");
        let _ = app.emit("gamemode", &status);
        status
    }

    /// Kapatır ve değiştirilenleri geri alır. `notify`: ekran göstergesi (uygulama kapanırken gösterilmez).
    pub fn disable(&self, app: &AppHandle) -> Status {
        self.restore(app, true)
    }

    pub fn restore(&self, app: &AppHandle, notify: bool) -> Status {
        let Some((saved, _)) = self.state.lock().unwrap_or_else(|e| e.into_inner()).take() else {
            return Status::default();
        };
        if let Some(guid) = saved.power {
            if let Err(e) = powercfg(&["/setactive", &guid]) {
                crate::log!("Oyun Modu: güç planı geri alınamadı: {e}");
            }
        }
        let audio = app.state::<Audio>();
        if let Some(prev) = saved.output {
            let _ = audio.set_default(prev);
        }
        if let Some(prev) = saved.lock {
            let _ = audio.mic(MicOp::SetLock(prev));
        }
        if notify {
            show_osd(app, "bolt", "Oyun Modu kapandı · ayarlar geri alındı");
        }
        let status = Status::default();
        let _ = app.emit("gamemode", &status);
        status
    }

    pub fn toggle(&self, app: &AppHandle) -> Status {
        if self.status().active {
            self.disable(app)
        } else {
            self.enable(app, "manual")
        }
    }

    /// Oyun algılama turundan: kendiliğinden açma/kapama (elle açılana dokunmaz).
    pub fn follow_games(&self, app: &AppHandle, any_running: bool) {
        let s = self.status();
        if any_running && !s.active {
            if profile(&app.state::<Db>()).auto {
                self.enable(app, "auto");
            }
        } else if !any_running && s.active && s.reason.as_deref() == Some("auto") {
            self.disable(app);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_scheme_guid_is_parsed_in_any_language() {
        assert_eq!(
            parse_guid("Power Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced)")
                .as_deref(),
            Some("381b4222-f694-41f0-9685-ff5bb260df2e")
        );
        assert_eq!(
            parse_guid(
                "Güç Düzeni GUID'si: 8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C  (Yüksek performans)"
            )
            .as_deref(),
            Some("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c")
        );
        assert_eq!(parse_guid("hiç yok"), None);
        assert_eq!(parse_guid("381b4222-f694-41f0-9685"), None);
    }

    #[test]
    fn profile_defaults() {
        let p: Profile = serde_json::from_str(r#"{"auto":true}"#).unwrap();
        assert!(p.auto && p.power && p.mic_lock && p.quiet);
        assert!(p.close_apps.is_empty());
    }
}
