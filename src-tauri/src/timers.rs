//! Kronometre ve geri sayımlar. Pencere kapalıyken de çalışsınlar diye çekirdekte tutulur
//! (uygulamadan tamamen çıkılınca sıfırlanır).

use crate::util::new_id;
use serde::Serialize;
use std::sync::Mutex;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Countdown {
    pub id: String,
    pub label: String,
    pub total_ms: i64,
    /// Çalışıyorsa bitiş zamanı.
    pub ends_at: Option<i64>,
    /// Duraklatılmışsa kalan süre.
    pub remaining_ms: i64,
    pub finished: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopwatchSnapshot {
    pub running: bool,
    pub elapsed_ms: i64,
    pub laps: Vec<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimersSnapshot {
    pub now: i64,
    pub stopwatch: StopwatchSnapshot,
    pub countdowns: Vec<Countdown>,
}

#[derive(Default)]
struct State {
    sw_started: Option<i64>,
    sw_accum: i64,
    laps: Vec<i64>,
    countdowns: Vec<Countdown>,
}

impl State {
    fn sw_elapsed(&self, now: i64) -> i64 {
        self.sw_accum + self.sw_started.map_or(0, |s| now - s)
    }
}

#[derive(Default)]
pub struct Timers(Mutex<State>);

impl Timers {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn snapshot(&self, now: i64) -> TimersSnapshot {
        let s = self.state();
        TimersSnapshot {
            now,
            stopwatch: StopwatchSnapshot {
                running: s.sw_started.is_some(),
                elapsed_ms: s.sw_elapsed(now),
                laps: s.laps.clone(),
            },
            countdowns: s.countdowns.clone(),
        }
    }

    pub fn stopwatch_toggle(&self, now: i64) {
        let mut s = self.state();
        match s.sw_started.take() {
            Some(start) => s.sw_accum += now - start,
            None => s.sw_started = Some(now),
        }
    }

    pub fn stopwatch_lap(&self, now: i64) {
        let mut s = self.state();
        if s.sw_started.is_some() {
            let e = s.sw_elapsed(now);
            s.laps.push(e);
        }
    }

    pub fn stopwatch_reset(&self) {
        let mut s = self.state();
        s.sw_started = None;
        s.sw_accum = 0;
        s.laps.clear();
    }

    pub fn add(&self, label: &str, ms: i64, now: i64) {
        let label = if label.trim().is_empty() {
            "Geri sayım"
        } else {
            label.trim()
        };
        self.state().countdowns.push(Countdown {
            id: new_id(),
            label: label.chars().take(40).collect(),
            total_ms: ms,
            ends_at: Some(now + ms),
            remaining_ms: ms,
            finished: false,
        });
    }

    /// Çalışıyorsa duraklatır, duruyorsa devam ettirir, bittiyse baştan başlatır.
    pub fn toggle(&self, id: &str, now: i64) {
        let mut s = self.state();
        if let Some(c) = s.countdowns.iter_mut().find(|c| c.id == id) {
            if c.finished {
                c.finished = false;
                c.ends_at = Some(now + c.total_ms);
                c.remaining_ms = c.total_ms;
            } else if let Some(end) = c.ends_at.take() {
                c.remaining_ms = (end - now).max(0);
            } else {
                c.ends_at = Some(now + c.remaining_ms);
            }
        }
    }

    pub fn remove(&self, id: &str) {
        self.state().countdowns.retain(|c| c.id != id);
    }

    /// Süresi dolanları "bitti" yapar ve döndürür.
    pub fn take_finished(&self, now: i64) -> Vec<Countdown> {
        let mut s = self.state();
        let mut done = Vec::new();
        for c in s.countdowns.iter_mut() {
            if c.ends_at.is_some_and(|e| e <= now) {
                c.ends_at = None;
                c.remaining_ms = 0;
                c.finished = true;
                done.push(c.clone());
            }
        }
        done
    }

    pub fn finished(&self) -> Vec<Countdown> {
        self.state()
            .countdowns
            .iter()
            .filter(|c| c.finished)
            .cloned()
            .collect()
    }

    pub fn next_end(&self) -> Option<i64> {
        self.state()
            .countdowns
            .iter()
            .filter_map(|c| c.ends_at)
            .min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_pause_resume_and_finish() {
        let t = Timers::default();
        t.add("Fırın", 10_000, 0);
        let id = t.snapshot(0).countdowns[0].id.clone();
        t.toggle(&id, 4_000); // duraklat: 6 sn kaldı
        assert_eq!(t.snapshot(9_000).countdowns[0].remaining_ms, 6_000);
        assert!(t.take_finished(20_000).is_empty());
        t.toggle(&id, 20_000); // devam: 26 sn'de biter
        assert_eq!(t.next_end(), Some(26_000));
        assert_eq!(t.take_finished(26_000).len(), 1);
        assert_eq!(t.finished().len(), 1);
        t.toggle(&id, 30_000); // bittiyse baştan
        assert_eq!(t.next_end(), Some(40_000));
    }

    #[test]
    fn stopwatch_accumulates_across_pauses() {
        let t = Timers::default();
        t.stopwatch_toggle(1_000);
        t.stopwatch_lap(3_000);
        t.stopwatch_toggle(5_000);
        t.stopwatch_toggle(10_000);
        let s = t.snapshot(12_000).stopwatch;
        assert_eq!(s.elapsed_ms, 6_000);
        assert_eq!(s.laps, vec![2_000]);
        assert!(s.running);
    }
}
