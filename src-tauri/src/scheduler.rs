//! Hatırlatıcıların ve geri sayımların zamanı gelince uyarı gösteren arka plan iş parçacığı.
//! Sürekli kontrol etmez: bir sonraki olayın zamanına kadar uyur (en fazla 30 sn; uyku/saat
//! değişikliklerinden sonra da yakalasın diye).

use crate::{db::Db, overlay, reminders, timers::Timers, util::now_ms};
use std::sync::{Condvar, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const MAX_SLEEP_MS: i64 = 30_000;

#[derive(Default)]
pub struct Scheduler {
    dirty: Mutex<bool>,
    cv: Condvar,
}

impl Scheduler {
    /// Bir şey değişti (yeni hatırlatıcı, yeni sayaç…): hemen yeniden hesapla.
    pub fn poke(&self) {
        *self.dirty.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.cv.notify_one();
    }
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        // Uygulama kapanmadan önce "Tamam" denmemiş uyarılar varsa yeniden göster.
        let pending = reminders::pending_alerts(&app.state::<Db>()).unwrap_or_default();
        if !pending.is_empty() {
            overlay::raise_alerts(&app);
        }
        run(app)
    });
}

fn run(app: AppHandle) {
    loop {
        let now = now_ms();
        let db = app.state::<Db>();
        let timers = app.state::<Timers>();

        let due = reminders::take_due(&db, now).unwrap_or_default();
        let finished = timers.take_finished(now);
        if !due.is_empty() {
            let _ = app.emit("reminders-changed", ());
        }
        if !finished.is_empty() {
            let _ = app.emit("timers-changed", ());
        }
        if !due.is_empty() || !finished.is_empty() {
            overlay::raise_alerts(&app);
            for r in &due {
                overlay::system_notification(
                    &app,
                    "reminders.toast",
                    &format!("⏰ {}", r.title),
                    if r.note.is_empty() {
                        "Hatırlatıcının zamanı geldi"
                    } else {
                        &r.note
                    },
                );
            }
            for t in &finished {
                overlay::system_notification(
                    &app,
                    "reminders.toast",
                    &format!("⏱ Süre doldu: {}", t.label),
                    "Geri sayım bitti",
                );
            }
            crate::log!("Uyarı: {} hatırlatıcı, {} sayaç", due.len(), finished.len());
        }

        let next = [reminders::next_due(&db), timers.next_end()]
            .into_iter()
            .flatten()
            .min();
        let wait = next.map_or(MAX_SLEEP_MS, |t| (t - now_ms()).clamp(0, MAX_SLEEP_MS));

        let sched = app.state::<Scheduler>();
        let mut dirty = sched.dirty.lock().unwrap_or_else(|e| e.into_inner());
        if !*dirty && wait > 0 {
            dirty = sched
                .cv
                .wait_timeout(dirty, Duration::from_millis(wait as u64))
                .map(|(g, _)| g)
                .unwrap_or_else(|e| e.into_inner().0);
        }
        *dirty = false;
    }
}
