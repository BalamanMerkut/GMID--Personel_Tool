//! Yazılar (altyazı / Whisper) ve Whisper kurulumu.

use crate::transcript::{self, whisper, Transcript};
use crate::{db::Db, paths::DataDir};
use tauri::{AppHandle, Manager, State};

type CmdResult<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppHandle) -> T + Send + 'static,
) -> CmdResult<T> {
    tauri::async_runtime::spawn_blocking(move || f(&app))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn transcript_get(db: State<'_, Db>, id: String) -> Option<Transcript> {
    transcript::get(&db, &id)
}

/// `mode`: "auto" | "subtitles" | "whisper"; `lang`: "orig" | "tr" | "en"
#[tauri::command]
pub async fn transcript_create(
    app: AppHandle,
    id: String,
    mode: String,
    lang: String,
) -> CmdResult<Transcript> {
    blocking(app, move |app| transcript::create(app, &id, &mode, &lang)).await?
}

#[tauri::command]
pub fn transcript_delete(db: State<'_, Db>, id: String) {
    transcript::delete(&db, &id);
}

#[tauri::command]
pub fn whisper_status(db: State<'_, Db>, dir: State<'_, DataDir>) -> whisper::WhisperStatus {
    let model: String = db
        .setting_json(whisper::MODEL_KEY)
        .unwrap_or_else(|| "turbo".into());
    whisper::status(dir.root(), &model)
}

/// Whisper programını ve modeli indirir; modeli varsayılan yapar.
#[tauri::command]
pub async fn whisper_install(app: AppHandle, model: String) -> CmdResult<whisper::WhisperStatus> {
    blocking(app, move |app| {
        let (db, dir) = (app.state::<Db>(), app.state::<DataDir>());
        whisper::install(app, dir.root(), &model)?;
        let _ = db.set_setting(
            whisper::MODEL_KEY,
            &format!("\"{}\"", whisper::find_model(&model).id),
        );
        Ok(whisper::status(dir.root(), &model))
    })
    .await?
}
