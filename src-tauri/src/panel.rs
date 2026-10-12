//! Oyun İçi Panel (Steam'deki Shift+Tab gibi): `Ctrl+Shift+G` ile oyunun bulunduğu ekranın üstünde yarı saydam
//! bir pencere açılır; aynı tuş, kapat düğmesi ya da (yazarken) `Esc` ile kapanır. Oyuna hiçbir şey enjekte
//! edilmez: panel, oyunun üstünde duran ayrı bir pencere (çerçevesiz/pencereli oyunlarda görünür).
//!
//! Panel **odak almadan** açılır: oyun ön planda kalır. Odak oyundan giderse birçok oyun (özellikle tam ekranda)
//! kendini simge durumuna küçültür, alt-tab yapılmış gibi masaüstüne düşülür. Fareyle tıklamak odağı almaz;
//! yalnızca nota yazmak için metin kutusuna tıklayınca klavye odağı alınır, kapanınca oyuna geri verilir.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const PANEL: &str = "panel";

/// Ekran dikdörtgeni (fiziksel piksel): x, y, genişlik, yükseklik.
type Rect = (i32, i32, u32, u32);

/// Panel açılmadan önce ön plandaki pencere (kapanınca odak ona döner).
#[derive(Default)]
pub struct Panel {
    #[allow(dead_code)]
    previous: Mutex<Option<isize>>,
    /// Panel klavye odağını aldıysa (not yazmak için) kapanınca odak oyuna geri verilir.
    focused: AtomicBool,
}

fn window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(w) = app.get_webview_window(PANEL) {
        return Ok(w);
    }
    WebviewWindowBuilder::new(app, PANEL, WebviewUrl::App("index.html".into()))
        .title("GMID Oyun Paneli")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()
}

pub fn toggle(app: &AppHandle) {
    let visible = app
        .get_webview_window(PANEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    if visible {
        hide(app);
    } else {
        show(app);
    }
}

pub fn show(app: &AppHandle) {
    // Ön plandaki pencere ve bulunduğu ekran (oyun hangi ekrandaysa panel orada açılır).
    let (prev, rect) = foreground();
    let state = app.state::<Panel>();
    *state.previous.lock().unwrap_or_else(|e| e.into_inner()) = prev;
    state.focused.store(false, Ordering::Relaxed);
    crate::window::on_ui_thread(app, "Oyun paneli", move |app| {
        let w = window(app)?;
        if let Some((x, y, width, height)) = rect {
            let _ = w.set_position(PhysicalPosition::new(x, y));
            let _ = w.set_size(PhysicalSize::new(width, height));
        } else if let Some(m) = w.primary_monitor()?.or(w.current_monitor()?) {
            let _ = w.set_position(*m.position());
            let _ = w.set_size(*m.size());
        }
        let _ = tauri::Emitter::emit_to(app, PANEL, "panel-open", ());
        // Odak oyunda kalsın (alt-tab olmasın).
        crate::overlay::show_no_activate(&w);
        Ok(())
    });
}

/// Panel açılmadan önceki ön plan penceresinin süreci (FPS'i o oyundan okumak için).
pub fn game_pid(app: &AppHandle) -> Option<u32> {
    let prev = *app
        .state::<Panel>()
        .previous
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    prev.and_then(pid_of)
}

#[cfg(windows)]
fn pid_of(h: isize) -> Option<u32> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(HWND(h as *mut _), Some(&mut pid)) };
    (pid != 0).then_some(pid)
}

#[cfg(not(windows))]
fn pid_of(_h: isize) -> Option<u32> {
    None
}

/// Not yazmak için klavye odağı (yalnız kullanıcı metin kutusuna tıklayınca).
pub fn take_focus(app: &AppHandle) {
    let Some(w) = app.get_webview_window(PANEL) else {
        return;
    };
    if app.state::<Panel>().focused.swap(true, Ordering::Relaxed) {
        return;
    }
    set_activatable(&w, true);
    let _ = w.set_focus();
}

pub fn hide(app: &AppHandle) {
    // Arayüz ölçümü (FPS, işlemci…) durdursun.
    let _ = tauri::Emitter::emit_to(app, PANEL, "panel-close", ());
    if let Some(w) = app.get_webview_window(PANEL) {
        crate::overlay::hide_window(&w);
        set_activatable(&w, false);
    }
    let state = app.state::<Panel>();
    let prev = state
        .previous
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    state.focused.store(false, Ordering::Relaxed);
    // Oyun hâlâ öndeyse dokunulmaz; değilse (panel odak aldıysa ya da oyun küçüldüyse) geri getirilir.
    restore_focus(prev);
}

#[cfg(windows)]
fn set_activatable(w: &WebviewWindow, on: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };
    let Ok(hwnd) = w.hwnd() else {
        return;
    };
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let bit = WS_EX_NOACTIVATE.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, if on { ex & !bit } else { ex | bit });
    }
}

#[cfg(not(windows))]
fn set_activatable(_w: &WebviewWindow, _on: bool) {}

#[cfg(windows)]
fn foreground() -> (Option<isize>, Option<Rect>) {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return (None, None);
        }
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let rect = GetMonitorInfoW(mon, &mut mi).as_bool().then(|| {
            let r = mi.rcMonitor;
            (
                r.left,
                r.top,
                (r.right - r.left) as u32,
                (r.bottom - r.top) as u32,
            )
        });
        (Some(hwnd.0 as isize), rect)
    }
}

#[cfg(not(windows))]
fn foreground() -> (Option<isize>, Option<Rect>) {
    (None, None)
}

#[cfg(windows)]
fn restore_focus(prev: Option<isize>) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, IsIconic, IsWindow, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    if let Some(h) = prev {
        let hwnd = HWND(h as *mut _);
        unsafe {
            if IsWindow(Some(hwnd)).as_bool() && GetForegroundWindow() != hwnd {
                // Oyun yine de küçüldüyse geri büyüt.
                if IsIconic(hwnd).as_bool() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                }
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }
}

#[cfg(not(windows))]
fn restore_focus(_prev: Option<isize>) {}
