//! Notlar, hatırlatıcılar, sayaçlar ve küçük pencerelerin komutları.

use crate::notes::{self, Note, NoteInput, NoteSummary};
use crate::overlay::{self, OsdMessage, Overlay};
use crate::reminders::{self, Reminder, ReminderInput};
use crate::scheduler::Scheduler;
use crate::timers::{Countdown, Timers, TimersSnapshot};
use crate::util::now_ms;
use crate::{db::Db, paths::DataDir, window};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn notes_dir(dir: &DataDir) -> std::path::PathBuf {
    dir.folder(Some("Notlar"))
}

// ---- notlar ----

#[tauri::command]
pub fn notes_list(db: State<'_, Db>, trash: bool) -> CmdResult<Vec<NoteSummary>> {
    notes::list(&db, trash).map_err(err)
}

#[tauri::command]
pub fn note_get(db: State<'_, Db>, id: String) -> CmdResult<Option<Note>> {
    notes::get(&db, &id).map_err(err)
}

/// Kaydeder ve diğer pencerelere (yapışkan notlar, ana pencere) haber verir.
#[tauri::command]
pub fn note_save(
    app: AppHandle,
    db: State<'_, Db>,
    dir: State<'_, DataDir>,
    input: NoteInput,
) -> CmdResult<Note> {
    let note = notes::save(&db, &notes_dir(&dir), input).map_err(err)?;
    let _ = app.emit("notes-changed", &note.id);
    Ok(note)
}

#[tauri::command]
pub fn note_trash(
    app: AppHandle,
    db: State<'_, Db>,
    dir: State<'_, DataDir>,
    id: String,
) -> CmdResult<()> {
    window::close_sticky(&app, &id);
    notes::trash(&db, &notes_dir(&dir), &id).map_err(err)?;
    let _ = app.emit("notes-changed", &id);
    Ok(())
}

#[tauri::command]
pub fn note_restore(
    app: AppHandle,
    db: State<'_, Db>,
    dir: State<'_, DataDir>,
    id: String,
) -> CmdResult<()> {
    notes::restore(&db, &notes_dir(&dir), &id).map_err(err)?;
    let _ = app.emit("notes-changed", &id);
    Ok(())
}

#[tauri::command]
pub fn note_delete(
    app: AppHandle,
    db: State<'_, Db>,
    dir: State<'_, DataDir>,
    id: String,
) -> CmdResult<()> {
    window::close_sticky(&app, &id);
    notes::delete(&db, &notes_dir(&dir), &id).map_err(err)?;
    let _ = app.emit("notes-changed", &id);
    Ok(())
}

#[tauri::command]
pub fn open_quick_note(app: AppHandle) {
    window::open_quick_note(&app);
}

#[tauri::command]
pub fn open_sticky(app: AppHandle, id: String) {
    window::open_sticky(&app, &id);
}

#[tauri::command]
pub fn close_sticky(app: AppHandle, id: String) {
    window::close_sticky(&app, &id);
}

// ---- hatırlatıcılar ----

fn reminders_changed(app: &AppHandle) {
    app.state::<Scheduler>().poke();
    let _ = app.emit("reminders-changed", ());
    let _ = app.emit_to(overlay::ALERTS, "alerts-changed", ());
}

#[tauri::command]
pub fn reminders_list(db: State<'_, Db>) -> CmdResult<Vec<Reminder>> {
    reminders::list(&db).map_err(err)
}

#[tauri::command]
pub fn reminder_save(
    app: AppHandle,
    db: State<'_, Db>,
    input: ReminderInput,
) -> CmdResult<Reminder> {
    if input.title.trim().is_empty() {
        return Err("Hatırlatıcıya bir başlık yaz.".into());
    }
    let r = reminders::save(&db, input).map_err(err)?;
    reminders_changed(&app);
    Ok(r)
}

#[tauri::command]
pub fn reminder_delete(app: AppHandle, db: State<'_, Db>, id: String) -> CmdResult<()> {
    reminders::delete(&db, &id).map_err(err)?;
    reminders_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reminder_complete(app: AppHandle, db: State<'_, Db>, id: String) -> CmdResult<()> {
    reminders::complete(&db, &id, now_ms()).map_err(err)?;
    reminders_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reminder_snooze(
    app: AppHandle,
    db: State<'_, Db>,
    id: String,
    minutes: i64,
) -> CmdResult<()> {
    reminders::snooze(&db, &id, minutes.clamp(1, 24 * 60), now_ms()).map_err(err)?;
    reminders_changed(&app);
    Ok(())
}

// ---- sayaçlar ----

fn timers_changed(app: &AppHandle) {
    app.state::<Scheduler>().poke();
    let _ = app.emit("timers-changed", ());
    let _ = app.emit_to(overlay::ALERTS, "alerts-changed", ());
}

#[tauri::command]
pub fn timers_state(timers: State<'_, Timers>) -> TimersSnapshot {
    timers.snapshot(now_ms())
}

#[tauri::command]
pub fn stopwatch_toggle(app: AppHandle, timers: State<'_, Timers>) -> TimersSnapshot {
    timers.stopwatch_toggle(now_ms());
    timers_changed(&app);
    timers.snapshot(now_ms())
}

#[tauri::command]
pub fn stopwatch_lap(app: AppHandle, timers: State<'_, Timers>) -> TimersSnapshot {
    timers.stopwatch_lap(now_ms());
    timers_changed(&app);
    timers.snapshot(now_ms())
}

#[tauri::command]
pub fn stopwatch_reset(app: AppHandle, timers: State<'_, Timers>) -> TimersSnapshot {
    timers.stopwatch_reset();
    timers_changed(&app);
    timers.snapshot(now_ms())
}

#[tauri::command]
pub fn countdown_add(
    app: AppHandle,
    timers: State<'_, Timers>,
    label: String,
    ms: i64,
) -> CmdResult<TimersSnapshot> {
    if !(1_000..=24 * 60 * 60 * 1000).contains(&ms) {
        return Err("Süre 1 saniye ile 24 saat arasında olmalı.".into());
    }
    timers.add(&label, ms, now_ms());
    timers_changed(&app);
    Ok(timers.snapshot(now_ms()))
}

#[tauri::command]
pub fn countdown_toggle(app: AppHandle, timers: State<'_, Timers>, id: String) -> TimersSnapshot {
    timers.toggle(&id, now_ms());
    timers_changed(&app);
    timers.snapshot(now_ms())
}

#[tauri::command]
pub fn countdown_remove(app: AppHandle, timers: State<'_, Timers>, id: String) -> TimersSnapshot {
    timers.remove(&id);
    timers_changed(&app);
    timers.snapshot(now_ms())
}

// ---- gösterge ve uyarı pencereleri ----

#[tauri::command]
pub fn take_osd(overlay: State<'_, Overlay>) -> Option<OsdMessage> {
    overlay.last_osd()
}

/// Sesle yazma göstergesi yeni açıldıysa son durumu alır.
#[tauri::command]
pub fn dictation_view_state(overlay: State<'_, Overlay>) -> crate::overlay::DictationView {
    overlay.last_dictation()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alerts {
    reminders: Vec<Reminder>,
    timers: Vec<Countdown>,
    /// "Kütüphaneye ekleyeyim mi?" kartları.
    games: Vec<crate::games::Suggestion>,
}

#[tauri::command]
pub fn alerts_list(
    db: State<'_, Db>,
    timers: State<'_, Timers>,
    games: State<'_, crate::games::Games>,
) -> CmdResult<Alerts> {
    Ok(Alerts {
        reminders: reminders::pending_alerts(&db).map_err(err)?,
        timers: timers.finished(),
        games: games.showing_suggestions(&db),
    })
}

#[tauri::command]
pub fn alerts_fit(app: AppHandle, height: f64) {
    overlay::fit_alerts(&app, height);
}

#[tauri::command]
pub fn alerts_close(app: AppHandle) {
    overlay::close_alerts(&app);
}
