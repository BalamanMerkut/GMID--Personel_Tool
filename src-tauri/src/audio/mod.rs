//! Ses cihazları ve mikrofon. Windows'un ses arayüzleri tek bir iş parçacığında yaşar
//! (COM nesneleri iş parçacıkları arasında taşınamaz); geri kalan her şey ona mesaj yollar.

use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Sender, SyncSender};
use std::time::Duration;
use tauri::AppHandle;

#[cfg(windows)]
mod win;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    /// 0.0 – 1.0
    pub level: f32,
    pub muted: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AudioState {
    pub supported: bool,
    pub outputs: Vec<Device>,
    pub inputs: Vec<Device>,
    /// Mikrofon kilidi açıksa tutulacak seviye (0.0 – 1.0).
    pub lock: Option<f32>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MicStatus {
    pub level: f32,
    pub muted: bool,
    pub lock: Option<f32>,
}

/// Kullanıcının cihazlara verdiği takma ad, simge ve gizleme tercihi (`audio.devices` ayarı).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DevicePrefs {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub hidden: bool,
}

#[cfg_attr(not(windows), allow(dead_code))]
pub enum MicOp {
    ToggleMute,
    Step(f32),
    Max,
    SetLock(Option<f32>),
    ToggleLock(f32),
}

type Reply<T> = SyncSender<Result<T, String>>;

#[allow(dead_code)] // Windows dışında iç olaylar hiç üretilmez.
pub(crate) enum Cmd {
    State(Reply<AudioState>),
    SetDefault(String, Reply<()>),
    SetVolume(String, f32, Reply<()>),
    SetMute(String, bool, Reply<()>),
    Mic(MicOp, Reply<MicStatus>),
    // Windows'tan gelen olaylar
    MicNotify(f32),
    DevicesChanged,
    DefaultChanged,
}

pub struct Audio {
    tx: Option<Sender<Cmd>>,
}

const UNSUPPORTED: &str = "Ses ayarları sadece Windows'ta çalışır.";

impl Audio {
    /// `lock`: açılışta etkin olacak mikrofon kilidi seviyesi.
    pub fn start(app: AppHandle, lock: Option<f32>) -> Self {
        #[cfg(windows)]
        {
            Self {
                tx: Some(win::spawn(app, lock)),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (app, lock);
            Self { tx: None }
        }
    }

    fn call<T>(&self, make: impl FnOnce(Reply<T>) -> Cmd) -> Result<T, String> {
        let tx = self.tx.as_ref().ok_or_else(|| UNSUPPORTED.to_string())?;
        let (rtx, rrx) = mpsc::sync_channel(1);
        tx.send(make(rtx))
            .map_err(|_| "Ses servisi çalışmıyor.".to_string())?;
        rrx.recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Ses servisi yanıt vermedi.".to_string())?
    }

    pub fn state(&self) -> Result<AudioState, String> {
        if self.tx.is_none() {
            return Ok(AudioState::default());
        }
        self.call(Cmd::State)
    }

    pub fn set_default(&self, id: String) -> Result<(), String> {
        self.call(|r| Cmd::SetDefault(id, r))
    }

    pub fn set_volume(&self, id: String, level: f32) -> Result<(), String> {
        self.call(|r| Cmd::SetVolume(id, level.clamp(0.0, 1.0), r))
    }

    pub fn set_mute(&self, id: String, muted: bool) -> Result<(), String> {
        self.call(|r| Cmd::SetMute(id, muted, r))
    }

    pub fn mic(&self, op: MicOp) -> Result<MicStatus, String> {
        self.call(|r| Cmd::Mic(op, r))
    }
}

/// Takma ad verilmemiş cihazlar için adından simge tahmini.
pub fn guess_icon(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if [
        "headphone",
        "headset",
        "kulaklık",
        "earphone",
        "airpods",
        "buds",
    ]
    .iter()
    .any(|k| n.contains(k))
    {
        "headphones"
    } else if ["hdmi", "display", "monitor", "nvidia high definition", "tv"]
        .iter()
        .any(|k| n.contains(k))
    {
        "monitor"
    } else {
        "speaker"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_guess() {
        assert_eq!(guess_icon("Kulaklık (HyperX Cloud II)"), "headphones");
        assert_eq!(
            guess_icon("LG ULTRAGEAR (NVIDIA High Definition Audio)"),
            "monitor"
        );
        assert_eq!(guess_icon("Hoparlör (Realtek(R) Audio)"), "speaker");
    }
}
