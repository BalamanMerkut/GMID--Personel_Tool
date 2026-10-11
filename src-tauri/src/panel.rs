//! Oyun İçi Panel (Steam'deki Shift+Tab gibi): `Ctrl+Shift+G` ile oyunun bulunduğu ekranın üstünde yarı saydam
//! bir pencere açılır; aynı tuş ya da `Esc` ile kapanır ve odak oyuna geri döner. Oyuna hiçbir şey enjekte
//! edilmez: panel, oyunun üstünde duran ayrı bir pencere (çerçevesiz/pencereli oyunlarda görünür).

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
    *app.state::<Panel>()
        .previous
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = prev;
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
        w.show()?;
        w.set_focus()?;
        Ok(())
    });
}

pub fn hide(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(PANEL) {
        let _ = w.hide();
    }
    let prev = app
        .state::<Panel>()
        .previous
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    restore_focus(prev);
}

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
    use windows::Win32::UI::WindowsAndMessaging::{IsWindow, SetForegroundWindow};
    if let Some(h) = prev {
        let hwnd = HWND(h as *mut _);
        unsafe {
            if IsWindow(Some(hwnd)).as_bool() {
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }
}

#[cfg(not(windows))]
fn restore_focus(_prev: Option<isize>) {}
