//! Zamanlı kapatma: "10 dakika sonra bilgisayarı kapat / yeniden başlat / uyut". Son bir dakikada ekranın
//! ortasında (oyundaysan panelde) geri sayım çıkar: İptal ya da Şimdi. Zamanı gelince açık programlar
//! kapatılır (kaydedilmemiş iş varsa kaybolur; uyarıda yazar). GMID kapanırsa plan da iptal olur.

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

/// Son kaç saniyede uyarılır.
pub const WARN_SECS: i64 = 60;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    /// shutdown | restart | sleep
    pub action: String,
    /// Ne zaman (ms).
    pub at: i64,
}

#[derive(Default)]
pub struct Power {
    plan: Mutex<Option<Plan>>,
    generation: AtomicU64,
}

impl Power {
    pub fn status(&self) -> Option<Plan> {
        self.plan.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

pub fn label(action: &str) -> &'static str {
    match action {
        "restart" => "yeniden başlatılacak",
        "sleep" => "uyku moduna geçecek",
        _ => "kapanacak",
    }
}

pub fn schedule(app: &AppHandle, action: &str, minutes: f64) -> Result<Plan, String> {
    if !matches!(action, "shutdown" | "restart" | "sleep") {
        return Err("Bilinmeyen işlem".into());
    }
    if !(0.5..=24.0 * 60.0).contains(&minutes) {
        return Err("Süre 1 dakika ile 24 saat arasında olmalı".into());
    }
    let at = crate::util::now_ms() + (minutes * 60_000.0) as i64;
    let plan = Plan {
        action: action.into(),
        at,
    };
    let state = app.state::<Power>();
    *state.plan.lock().unwrap_or_else(|e| e.into_inner()) = Some(plan.clone());
    let id = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    crate::log!("Zamanlı kapatma: {minutes} dk sonra {action}");
    let _ = app.emit("power-changed", &plan);
    crate::overlay::show_osd(
        app,
        "power",
        format!("Bilgisayar {} dk sonra {}", minutes.round(), label(action)),
    );
    let app = app.clone();
    std::thread::spawn(move || run(app, id, at));
    Ok(plan)
}

pub fn cancel(app: &AppHandle) {
    let state = app.state::<Power>();
    state.generation.fetch_add(1, Ordering::SeqCst);
    if state
        .plan
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .is_some()
    {
        crate::log!("Zamanlı kapatma iptal edildi");
    }
    let _ = app.emit("power-changed", ());
    crate::alarm::changed(app);
}

/// Planı beklemeden uygular.
pub fn now(app: &AppHandle) -> Result<(), String> {
    let plan = app.state::<Power>().status().ok_or("Plan yok")?;
    app.state::<Power>()
        .generation
        .fetch_add(1, Ordering::SeqCst);
    execute(&plan.action)
}

fn alive(app: &AppHandle, id: u64) -> bool {
    app.state::<Power>().generation.load(Ordering::SeqCst) == id
}

fn run(app: AppHandle, id: u64, at: i64) {
    let mut warned = false;
    loop {
        if !alive(&app, id) {
            return;
        }
        let left = at - crate::util::now_ms();
        if left <= 0 {
            break;
        }
        if !warned && left <= WARN_SECS * 1000 {
            warned = true;
            crate::alarm::show_power(&app);
        }
        std::thread::sleep(std::time::Duration::from_millis(
            left.clamp(200, 1000) as u64
        ));
    }
    let Some(plan) = app.state::<Power>().status() else {
        return;
    };
    *app.state::<Power>()
        .plan
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = None;
    if let Err(e) = execute(&plan.action) {
        crate::log!("Zamanlı kapatma uygulanamadı: {e}");
    }
}

fn execute(action: &str) -> Result<(), String> {
    crate::log!("Zamanlı kapatma uygulanıyor: {action}");
    #[cfg(windows)]
    {
        if action == "sleep" {
            use windows::Win32::System::Power::SetSuspendState;
            let ok = unsafe { SetSuspendState(false, false, false) };
            return if ok {
                Ok(())
            } else {
                Err("Uyku moduna geçilemedi".into())
            };
        }
        let flag = if action == "restart" { "/r" } else { "/s" };
        let mut cmd = std::process::Command::new("shutdown");
        cmd.args([flag, "/f", "/t", "0"]);
        crate::util::hide_console(&mut cmd);
        cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = action;
        Err("Yalnızca Windows'ta".into())
    }
}
