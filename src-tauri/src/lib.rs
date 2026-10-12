mod audio;
mod backup;
mod clips;
mod commands;
mod db;
mod deals;
mod dictation;
mod downloader;
mod fps;
mod gamemode;
mod games;
mod hotkeys;
mod install;
mod log;
mod net;
mod notes;
mod overlay;
mod panel;
mod paths;
mod perf;
mod playlists;
mod reminders;
mod report;
mod restore;
mod reviews;
mod scheduler;
mod search;
mod servers;
mod timers;
mod titles;
mod transcript;
mod tray;
mod updater;
mod util;
mod window;
mod wrapped;

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
            window::show_main(app);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                // Gösterge ve uyarılar her seferinde ekranın köşesine yerleştirilir.
                .with_denylist(&[overlay::OSD, overlay::ALERTS])
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED_ARG]),
        ))
        .plugin(hotkeys::plugin())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(move |app| {
            let handle = app.handle();
            let dir = paths::DataDir::resolve(handle)?;
            dir.ensure()?;
            log::init(dir.root().join("gmid.log"));
            // Kontrol Odası'ndan seçilen yedek, veritabanı açılmadan yerine konur.
            match restore::apply_pending(dir.root()) {
                Ok(true) => log!("Bekleyen yedek yerine kondu"),
                Ok(false) => {}
                Err(e) => log!("Yedek geri yüklenemedi: {e}"),
            }
            let db = db::Db::open(&dir.db()).map_err(|e| format!("Veritabanı açılamadı: {e}"))?;
            // Yedekten açıldıysa (Format Kiti ya da Kontrol Odası) yollar bu bilgisayara çevrilir.
            if let Ok(home) = app.path().home_dir() {
                restore::finish(&db, dir.root(), &home);
            }
            // Arayüz sadece kendi arka plan klasöründeki dosyaları gösterebilir.
            app.asset_protocol_scope()
                .allow_directory(dir.backgrounds(), true)?;
            app.asset_protocol_scope()
                .allow_directory(dir.servers(), false)?;
            // Videolarım: küçük resimler ve üzerine gelince oynayan önizleme.
            app.asset_protocol_scope()
                .allow_directory(dir.folder(Some("Videolar")), true)?;
            app.asset_protocol_scope()
                .allow_directory(dir.folder(Some("Kapaklar")), true)?;

            // Mikrofon kilidi açılışta da etkin olsun.
            let mic_lock: Option<f32> = db.setting_json("audio.micLock");
            if let Err(e) = notes::purge_trash(&db) {
                log!("Çöp kutusu temizlenemedi: {e}");
            }

            downloader::recover(&db);
            app.manage(db);
            app.manage(dir);
            app.manage(overlay::Overlay::default());
            app.manage(timers::Timers::default());
            app.manage(scheduler::Scheduler::default());
            app.manage(hotkeys::Hotkeys::default());
            app.manage(servers::Servers::default());
            app.manage(downloader::Downloader::default());
            app.manage(downloader::tools::Tools::default());
            app.manage(downloader::bridge::Bridge::default());
            app.manage(transcript::Transcriber::default());
            app.manage(dictation::Dictation::default());
            app.manage(games::Games::default());
            app.manage(gamemode::GameMode::default());
            app.manage(panel::Panel::default());
            app.manage(clips::Clips::default());
            app.manage(install::Installer::default());
            app.manage(updater::Updater::default());
            app.manage(deals::Deals::default());
            app.manage(perf::Perf::default());
            app.manage(audio::Audio::start(handle.clone(), mic_lock));
            app.manage(Launch {
                splash: AtomicBool::new(!minimized),
                pending_route: Mutex::new(None),
            });

            tray::create(handle)?;
            hotkeys::register_all(handle);
            scheduler::start(handle.clone());
            downloader::bridge::start(handle.clone());
            dictation::apply(handle);
            games::start(handle);
            clips::start(handle);
            titles::fetch_meta(handle);
            updater::start(handle);
            deals::start(handle);
            // Önceki oturumdan sırada kalanlar varsa devam et. Eklenti kuruluysa dosyalarını
            // güncelle (tarayıcıda bir kez "yenile" demek yeter).
            let h = handle.clone();
            std::thread::spawn(move || {
                let ext = h.state::<paths::DataDir>().folder(Some("Eklenti"));
                if ext.join("manifest.json").is_file() {
                    if let Err(e) = downloader::extension::install(&ext) {
                        log!("Eklenti güncellenemedi: {e}");
                    }
                }
                h.state::<downloader::Downloader>().pump(&h);
                // Kaybolmuş ya da hiç gelmemiş video küçük resimlerini onar.
                let n = downloader::thumbs::repair(&h, None);
                if n > 0 {
                    log!("{n} video küçük resmi onarıldı");
                }
            });
            window::restore_stickies(handle);

            // Günlük yedek açılışı yavaşlatmasın diye arka planda.
            let h = handle.clone();
            std::thread::spawn(move || {
                let (db, dir) = (h.state::<db::Db>(), h.state::<paths::DataDir>());
                if let Err(e) = backup::daily_if_needed(&db, &dir.backups()) {
                    log!("{e}");
                }
            });

            if !minimized {
                window::show_main(handle);
            }
            // Oyun İçi Panel'i şimdiden gizli hazırla (oyunda ilk açılışta odak kapmasın).
            panel::prepare(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_setting,
            commands::take_splash,
            commands::take_pending_route,
            commands::app_info,
            commands::open_data_folder,
            commands::open_log,
            commands::backup_now,
            commands::import_background,
            commands::open_url,
            commands::report_create,
            commands::search_all,
            commands::wrapped_get,
            commands::wrapped_save,
            commands::deals_get,
            commands::deals_refresh,
            commands::update_status,
            commands::update_check,
            commands::update_install,
            commands::install::install_catalog,
            commands::install::install_list,
            commands::install::install_save_list,
            commands::install::install_state,
            commands::install::install_run,
            commands::install::install_stop,
            commands::install::install_installed,
            commands::install::install_search,
            commands::install::install_find_setup,
            commands::install::install_backup_size,
            commands::install::install_export,
            commands::install::install_open_kit,
            commands::install::backup_restore_stage,
            commands::install::backup_restore_cancel,
            commands::install::backup_restore_apply,
            commands::sound::audio_state,
            commands::sound::audio_set_default,
            commands::sound::audio_set_volume,
            commands::sound::audio_set_mute,
            commands::sound::audio_apps,
            commands::sound::audio_set_app_volume,
            commands::sound::audio_set_app_mute,
            commands::sound::mic_set_lock,
            commands::sound::hotkeys_list,
            commands::sound::hotkey_set,
            commands::tools::notes_list,
            commands::tools::note_get,
            commands::tools::note_save,
            commands::tools::note_trash,
            commands::tools::note_restore,
            commands::tools::note_delete,
            commands::tools::open_quick_note,
            commands::tools::open_sticky,
            commands::tools::close_sticky,
            commands::tools::reminders_list,
            commands::tools::reminder_save,
            commands::tools::reminder_delete,
            commands::tools::reminder_complete,
            commands::tools::reminder_snooze,
            commands::tools::timers_state,
            commands::tools::stopwatch_toggle,
            commands::tools::stopwatch_lap,
            commands::tools::stopwatch_reset,
            commands::tools::countdown_add,
            commands::tools::countdown_toggle,
            commands::tools::countdown_remove,
            commands::tools::take_osd,
            commands::tools::dictation_view_state,
            commands::tools::alerts_list,
            commands::tools::alerts_fit,
            commands::tools::alerts_close,
            commands::servers::servers_games,
            commands::servers::servers_list,
            commands::servers::server_save,
            commands::servers::server_delete,
            commands::servers::server_import_logo,
            commands::servers::servers_sync,
            commands::servers::community_discord,
            commands::servers::servers_cached,
            commands::servers::servers_query,
            commands::servers::server_join,
            commands::servers::server_set_exe,
            commands::videos::videos_tools,
            commands::videos::videos_install_tools,
            commands::videos::videos_update,
            commands::videos::videos_probe,
            commands::videos::videos_download,
            commands::videos::media_tracks,
            commands::videos::playlists_list,
            commands::videos::videos_repair_thumb,
            commands::videos::playlist_save,
            commands::videos::playlist_delete,
            commands::videos::media_track,
            commands::videos::videos_download_playlist,
            commands::videos::videos_list,
            commands::videos::videos_live,
            commands::videos::videos_pause,
            commands::videos::videos_resume,
            commands::videos::videos_cancel,
            commands::videos::videos_remove,
            commands::videos::videos_open,
            commands::videos::videos_reveal,
            commands::videos::bridge_status,
            commands::videos::bridge_pair,
            commands::videos::bridge_unpair,
            commands::videos::extension_install,
            commands::transcripts::transcript_get,
            commands::transcripts::transcript_create,
            commands::transcripts::transcript_delete,
            commands::transcripts::whisper_status,
            commands::transcripts::whisper_install,
            commands::dictation::dictation_status,
            commands::games::games_list,
            commands::games::games_scan,
            commands::games::games_import,
            commands::games::games_add_manual,
            commands::games::games_running_apps,
            commands::games::games_set_hidden,
            commands::games::games_rename,
            commands::games::games_remove,
            commands::games::games_refresh_meta,
            commands::games::games_sessions,
            commands::games::games_launch,
            commands::games::games_reveal,
            commands::games::games_set_cover,
            commands::games::games_suggestions,
            commands::games::games_suggestion_answer,
            commands::games::gamemode_status,
            commands::games::gamemode_set,
            commands::games::panel_hide,
            commands::games::panel_focus,
            commands::games::panel_interact,
            commands::games::panel_perf,
            commands::games::fps_status,
            commands::games::fps_setup,
            commands::games::games_store_search,
            commands::games::games_add_archive,
            commands::games::reviews_list,
            commands::games::review_save,
            commands::games::review_delete,
            commands::games::titles_list,
            commands::games::titles_search,
            commands::games::titles_add,
            commands::games::titles_discard,
            commands::games::titles_refresh,
            commands::games::titles_set_poster,
            commands::clips::clips_list,
            commands::clips::clips_roots,
            commands::clips::clips_rescan,
            commands::clips::clips_set_favorite,
            commands::clips::clips_rename,
            commands::clips::clips_set_game,
            commands::clips::clips_delete,
            commands::clips::clips_open,
            commands::clips::clips_reveal,
            commands::clips::clips_fit_discord,
            commands::clips::clips_plan,
            commands::dictation::dictation_apply,
            commands::dictation::dictation_install,
            commands::dictation::dictation_warm_up,
        ])
        .build(tauri::generate_context!())
        .expect("GMID Tool başlatılamadı")
        .run(|app, event| {
            // Son pencere kapanınca uygulama kapanmasın, tepside çalışmaya devam etsin.
            // Tepsideki "Çıkış" ise (code = Some) gerçekten kapatır.
            match event {
                RunEvent::ExitRequested {
                    code: None, api, ..
                } if close_to_tray(app) => api.prevent_exit(),
                // Gerçekten kapanırken inen videoları durdur (açılışta kaldığı yerden devam eder).
                RunEvent::Exit => {
                    if let Some(d) = app.try_state::<downloader::Downloader>() {
                        d.shutdown();
                    }
                    if let Some(d) = app.try_state::<dictation::Dictation>() {
                        d.engine.shutdown();
                    }
                    if let (Some(g), Some(db)) =
                        (app.try_state::<games::Games>(), app.try_state::<db::Db>())
                    {
                        g.shutdown(&db);
                    }
                    // Panel açıkken kapanırsa FPS ölçümü (PresentMon) de kapansın.
                    if let (Some(p), Some(dir)) = (
                        app.try_state::<perf::Perf>(),
                        app.try_state::<paths::DataDir>(),
                    ) {
                        p.presentmon.stop(dir.root());
                    }
                    // Oyun Modu açıksa güç planı, ses çıkışı ve kilit geri alınsın.
                    if let Some(gm) = app.try_state::<gamemode::GameMode>() {
                        gm.restore(app, false);
                    }
                }
                _ => {}
            }
        });
}

fn close_to_tray(app: &tauri::AppHandle) -> bool {
    app.try_state::<db::Db>()
        .and_then(|db| db.setting("app.closeToTray").ok().flatten())
        .map_or(true, |v| v != "false")
}
