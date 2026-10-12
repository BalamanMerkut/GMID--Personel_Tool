//! Kenarlıksız tam ekran (Borderless Gaming gibi): listedeki bir oyun **pencereli** açıldığında penceresinin
//! çerçevesi kaldırılır ve bulunduğu ekranı kaplar. Görüntü tam ekran gibidir ama Windows'un gözünde sıradan bir
//! pencere olduğu için Oyun İçi Panel (Shift+Tab gibi) oyunun üstünde açılır, oyun küçülmez, alt-tab anında olur.
//! Oyuna hiçbir şey enjekte edilmez; yalnız pencerenin stili ve konumu değiştirilir (anti-cheat güvenli).

use crate::db::Db;
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

const KEY: &str = "games.borderless";

/// Uygulanmış pencereler (her pencere için bir kez günlüğe yazılır).
#[derive(Default)]
pub struct Borderless {
    #[cfg_attr(not(windows), allow(dead_code))]
    done: Mutex<HashSet<isize>>,
}

/// Listedeki oyunlar (küçük harf exe adları).
pub fn list(db: &Db) -> Vec<String> {
    db.setting_json(KEY).unwrap_or_default()
}

pub fn set(db: &Db, exe: &str, on: bool) -> Result<Vec<String>, String> {
    let exe = exe.trim().to_lowercase();
    if exe.is_empty() || !exe.ends_with(".exe") {
        return Err("Geçersiz program adı".into());
    }
    let mut v = list(db);
    v.retain(|e| *e != exe);
    if on {
        v.push(exe);
        v.sort();
    }
    let j = serde_json::to_string(&v).map_err(|e| e.to_string())?;
    db.set_setting(KEY, &j).map_err(|e| e.to_string())?;
    Ok(v)
}

/// Arka planda: öndeki pencere listedeki bir oyunsa ve çerçeveliyse kenarlıksız tam ekrana çevirir.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let games = list(&app.state::<Db>());
        if games.is_empty() {
            continue;
        }
        tick(&app, &games);
    });
}

#[cfg(windows)]
fn tick(app: &AppHandle, games: &[String]) {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    let Some(exe) = crate::games::procs::exe_of(pid) else {
        return;
    };
    if !games.contains(&exe.to_lowercase()) {
        return;
    }
    if win::apply(hwnd) {
        let key = hwnd.0 as isize;
        if app
            .state::<Borderless>()
            .done
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key)
        {
            crate::log!("Kenarlıksız tam ekran: {exe}");
        }
    }
}

#[cfg(not(windows))]
fn tick(_app: &AppHandle, _games: &[String]) {}

#[cfg(windows)]
mod win {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GetWindowRect, IsIconic, IsWindowVisible, SetWindowLongPtrW,
        SetWindowPos, GWL_EXSTYLE, GWL_STYLE, SWP_FRAMECHANGED, SWP_NOOWNERZORDER, SWP_NOZORDER,
        SWP_SHOWWINDOW, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME,
        WS_EX_STATICEDGE, WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU,
        WS_THICKFRAME,
    };

    /// Çerçeveli ve yeterince büyükse kenarlıksız yapar; yaptıysa `true`.
    pub fn apply(hwnd: HWND) -> bool {
        unsafe {
            if IsIconic(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
                return false;
            }
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            if style & WS_CHILD.0 != 0 || style & (WS_CAPTION.0 | WS_THICKFRAME.0) == 0 {
                return false;
            }
            let mut r = RECT::default();
            if GetWindowRect(hwnd, &mut r).is_err() {
                return false;
            }
            // Başlatıcı ve ayar pencereleri gibi küçük pencerelere dokunma.
            if r.right - r.left < 640 || r.bottom - r.top < 480 {
                return false;
            }
            let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(mon, &mut mi).as_bool() {
                return false;
            }
            let m = mi.rcMonitor;
            let remove =
                WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0 | WS_MAXIMIZEBOX.0 | WS_MINIMIZEBOX.0;
            SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !remove) as isize);
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let remove_ex = WS_EX_DLGMODALFRAME.0
                | WS_EX_CLIENTEDGE.0
                | WS_EX_STATICEDGE.0
                | WS_EX_WINDOWEDGE.0;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (ex & !remove_ex) as isize);
            let _ = SetWindowPos(
                hwnd,
                None,
                m.left,
                m.top,
                m.right - m.left,
                m.bottom - m.top,
                SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
            );
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_lowercase_unique() {
        let dir = std::env::temp_dir().join(format!("gmid-borderless-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("t.db")).unwrap();
        set(&db, "CoD2MP_s.exe", true).unwrap();
        set(&db, "cod2mp_s.exe", true).unwrap();
        assert_eq!(
            set(&db, "hl.exe", true).unwrap(),
            ["cod2mp_s.exe", "hl.exe"]
        );
        assert_eq!(set(&db, "HL.EXE", false).unwrap(), ["cod2mp_s.exe"]);
        assert!(set(&db, "notepad", true).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
