//! Ses & Mikrofon ve kısayol komutları.

use crate::audio::{Audio, AudioState, MicOp, MicStatus};
use crate::hotkeys::{self, HotkeyStatus};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn audio_state(audio: State<'_, Audio>) -> Result<AudioState, String> {
    audio.state()
}

#[tauri::command]
pub fn audio_set_default(audio: State<'_, Audio>, id: String) -> Result<(), String> {
    audio.set_default(id)
}

#[tauri::command]
pub fn audio_set_volume(audio: State<'_, Audio>, id: String, level: f32) -> Result<(), String> {
    audio.set_volume(id, level)
}

#[tauri::command]
pub fn audio_set_mute(audio: State<'_, Audio>, id: String, muted: bool) -> Result<(), String> {
    audio.set_mute(id, muted)
}

/// `level`: kilitlenecek seviye (0–1); `None` kilidi kapatır.
#[tauri::command]
pub fn mic_set_lock(audio: State<'_, Audio>, level: Option<f32>) -> Result<MicStatus, String> {
    audio.mic(MicOp::SetLock(level))
}

#[tauri::command]
pub fn hotkeys_list(app: AppHandle) -> Vec<HotkeyStatus> {
    hotkeys::list(&app)
}

/// `accel`: yeni kısayol; `None` varsayılana döndürür, boş metin kapatır.
#[tauri::command]
pub fn hotkey_set(
    app: AppHandle,
    id: String,
    accel: Option<String>,
) -> Result<Vec<HotkeyStatus>, String> {
    hotkeys::set(&app, &id, accel)
}
