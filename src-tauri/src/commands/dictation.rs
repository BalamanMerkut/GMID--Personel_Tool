//! Sesle yazma: durum, ayarların uygulanması ve kurulum.

use crate::dictation::{self, engine, Dictation, Status};
use crate::paths::DataDir;
use crate::transcript::whisper;
use tauri::{AppHandle, Manager};

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
pub fn dictation_status(app: AppHandle) -> Status {
    app.state::<Dictation>().status(&app)
}

/// Ayarlar kaydedildikten sonra çağrılır: açıp kapatır, sunucular yeni ayarla yeniden başlar.
#[tauri::command]
pub fn dictation_apply(app: AppHandle) -> Status {
    dictation::apply(&app);
    app.state::<Dictation>().status(&app)
}

/// `part`: "whisper" (seçili model + program) | "gpu" (NVIDIA hızlandırma) | "llm" (toparlayıcı)
#[tauri::command]
pub async fn dictation_install(app: AppHandle, part: String) -> CmdResult<Status> {
    blocking(app, move |app| {
        let root = app.state::<DataDir>().root().to_path_buf();
        match part.as_str() {
            "whisper" => {
                let model: String = app
                    .state::<crate::db::Db>()
                    .setting_json(whisper::MODEL_KEY)
                    .unwrap_or_else(|| "turbo".into());
                whisper::install(app, &root, &model)?
            }
            "gpu" => {
                whisper::install_cuda(app, &root)?;
                // Yeni kurulan sürümü denesin.
                let d = app.state::<Dictation>();
                *d.engine
                    .gpu_failed
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
                d.engine.shutdown();
            }
            "llm" => engine::install_llm(app, &root)?,
            _ => return Err("Bilinmeyen parça".to_string()),
        }
        Ok(app.state::<Dictation>().status(app))
    })
    .await?
}

/// Modelleri şimdi yükler; bittiğinde (NVIDIA ile mi çalıştığı dahil) durumu döner.
#[tauri::command]
pub async fn dictation_warm_up(app: AppHandle) -> CmdResult<Status> {
    blocking(app, |app| {
        dictation::warm_up(app);
        app.state::<Dictation>().status(app)
    })
    .await
}
