mod backup;
mod commands;
mod db;
mod paths;
mod tray;
mod window;

use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::{Manager, RunEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_window_state::StateFlags;

/// Windows açılışında otomatik başlatılınca uygulamaya verilen argüman: pencere açılmaz, tepsiye yerleşir.
const MINIMIZED_ARG: &str = "--minimized";

/// Bu açılışa ait durum.
pub struct Launch {
    /// Açılış animasyonu henüz oynamadıysa `true`.
    pub splash: AtomicBool,
    /// Tepsi menüsünden istenen, arayüz açılınca gidilecek ekran.
    pub pending_route: Mutex<Option<String>>,
}

pub fn run() {
    let minimized = std::env::args().any(|a| a == MINIMIZED_ARG);

    tauri::Builder::default()
        // İkinci kez çalıştırılırsa yeni kopya açılmaz, mevcut pencere öne gelir.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = window::show_main(app);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let handle = app.handle();
            let dir = paths::DataDir::resolve(handle)?;
            dir.ensure()?;
            let db = db::Db::open(&dir.db()).map_err(|e| format!("Veritabanı açılamadı: {e}"))?;
            // Arayüz sadece kendi arka plan klasöründeki dosyaları gösterebilir.
            app.asset_protocol_scope()
                .allow_directory(dir.backgrounds(), true)?;

            app.manage(db);
            app.manage(dir);
            app.manage(Launch {
                splash: AtomicBool::new(!minimized),
                pending_route: Mutex::new(None),
            });

            tray::create(handle)?;

            // Günlük yedek açılışı yavaşlatmasın diye arka planda.
            let h = handle.clone();
            std::thread::spawn(move || {
                let (db, dir) = (h.state::<db::Db>(), h.state::<paths::DataDir>());
                if let Err(e) = backup::daily_if_needed(&db, &dir.backups()) {
                    eprintln!("{e}");
                }
            });

            if !minimized {
                window::show_main(handle)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_setting,
            commands::take_splash,
            commands::take_pending_route,
            commands::app_info,
            commands::open_data_folder,
            commands::backup_now,
            commands::import_background,
        ])
        .build(tauri::generate_context!())
        .expect("GMID Tool başlatılamadı")
        .run(|app, event| {
            // Son pencere kapanınca uygulama kapanmasın, tepside çalışmaya devam etsin.
            // Tepsideki "Çıkış" ise (code = Some) gerçekten kapatır.
            if let RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                if close_to_tray(app) {
                    api.prevent_exit();
                }
            }
        });
}

fn close_to_tray(app: &tauri::AppHandle) -> bool {
    app.try_state::<db::Db>()
        .and_then(|db| db.setting("app.closeToTray").ok().flatten())
        .map_or(true, |v| v != "false")
}
