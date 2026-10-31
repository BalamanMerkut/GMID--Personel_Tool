//! Ders kaydı komutları.

use crate::lecture::{Lecture, Status};
use tauri::{AppHandle, State};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn lecture_status(l: State<'_, Lecture>) -> Status {
    l.status()
}

/// `mic`: aygıt kimliği (boşsa varsayılan). `keepAudio`: sesi de `Dersler` klasörüne kaydet.
#[tauri::command]
pub async fn lecture_start(
    app: AppHandle,
    title: String,
    mic: String,
    keep_audio: bool,
) -> CmdResult<Status> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        app.state::<Lecture>().start(&app, &title, &mic, keep_audio)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn lecture_pause(app: AppHandle, l: State<'_, Lecture>, paused: bool) -> Status {
    l.pause(&app, paused);
    l.status()
}

#[tauri::command]
pub fn lecture_stop(app: AppHandle, l: State<'_, Lecture>) -> Status {
    l.stop(&app);
    l.status()
}

#[tauri::command]
pub fn lecture_clear(l: State<'_, Lecture>) -> Status {
    l.clear();
    l.status()
}
