//! Oyun İçi Panel (Steam'deki Shift+Tab gibi): `Ctrl+Shift+G` ile oyunun bulunduğu ekranın üstünde yarı saydam
//! bir pencere açılır; aynı tuş, kapat düğmesi ya da `Esc` ile kapanır ve oyun yeniden öne gelir. Oyuna hiçbir şey
//! enjekte edilmez (anti-cheat güvenli): panel, oyunun üstünde duran ayrı bir penceredir.
//!
//! İki kip:
//! - **Etkileşim** (Shift+Tab gibi): panel odağı alır, oyun fareyi bırakır; kenarlıksız/pencereli oyunlar arkada
//!   görünür kalır. Kendiliğinden kip, oyun özel tam ekranda değilse bunu seçer.
//! - **Bakış**: panel odak almaz, oyun önde kalır (özel tam ekranda odak gidince oyunlar küçülür). Fare, oyunun
//!   menüsü açıkken (imleç görünürken) tıklayabilir; "Fareyi serbest bırak" ile etkileşime geçilir.
//!
//! Etkileşim kipinde oyun yine de küçülürse (bazı oyunlar odak gidince kendini küçültür) hemen geri getirilir,
//! panel bakış kipine geçer ve bu oyun hatırlanır: bir dahaki sefere doğrudan bakış kipinde açılır.
//! Panel penceresi açılışta gizli olarak hazırlanır; oyun sırasında ilk kez oluşturulurken odak kapıp oyunu
//! küçültmesin diye.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const PANEL: &str = "panel";
/// Odak alınca küçülen oyunlar (exe adları): bunlarda doğrudan bakış kipi.
const GLANCE_KEY: &str = "panel.glanceExes";
/// auto | interactive | glance
const MODE_KEY: &str = "panel.mode";

/// Ekran dikdörtgeni (fiziksel piksel): x, y, genişlik, yükseklik.
type Rect = (i32, i32, u32, u32);

/// Panel açılmadan önce ön plandaki pencere (kapanınca odak ona döner).
#[derive(Default)]
pub struct Panel {
    #[allow(dead_code)]
    previous: Mutex<Option<isize>>,
    /// Panel odak aldıysa (etkileşim kipi ya da not yazmak için).
    focused: AtomicBool,
}

/// Arayüze: hangi kipte açıldı ve neden.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenInfo {
    /// interactive | glance
    pub mode: &'static str,
    /// Oyun özel (exclusive) tam ekranda: panel oyunun üstünde görünmeyebilir.
    pub exclusive: bool,
    /// Bu oyun odak alınca küçüldüğü için bakış kipine geçildi.
    pub minimizes: bool,
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

/// Uygulama açılırken paneli gizli hazırlar (oyun sırasında oluşturulup odak kapmasın).
pub fn prepare(app: &AppHandle) {
    crate::window::on_ui_thread(app, "Oyun paneli hazırlığı", |app| {
        window(app)?;
        Ok(())
    });
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

fn glance_list(app: &AppHandle) -> Vec<String> {
    app.state::<crate::db::Db>()
        .setting_json(GLANCE_KEY)
        .unwrap_or_default()
}

pub fn show(app: &AppHandle) {
    // Ön plandaki pencere ve bulunduğu ekran (oyun hangi ekrandaysa panel orada açılır).
    let (prev, rect) = foreground();
    let state = app.state::<Panel>();
    *state.previous.lock().unwrap_or_else(|e| e.into_inner()) = prev;
    state.focused.store(false, Ordering::Relaxed);

    let exclusive = exclusive_fullscreen();
    let exe = prev.and_then(pid_of).and_then(exe_name);
    let remembered = exe.as_ref().is_some_and(|e| glance_list(app).contains(e));
    let mode: String = app
        .state::<crate::db::Db>()
        .setting_json(MODE_KEY)
        .unwrap_or_else(|| "auto".into());
    let interactive = match mode.as_str() {
        "interactive" => true,
        "glance" => false,
        _ => !exclusive && !remembered,
    };
    let info = OpenInfo {
        mode: if interactive { "interactive" } else { "glance" },
        exclusive,
        minimizes: remembered,
    };

    let app2 = app.clone();
    crate::window::on_ui_thread(app, "Oyun paneli", move |app| {
        let w = window(app)?;
        place(&w, rect);
        let _ = tauri::Emitter::emit_to(app, PANEL, "panel-open", &info);
        // Önce odak almadan göster; etkileşim kipinde ardından odak alınır.
        crate::overlay::show_no_activate(&w);
        if interactive {
            take_focus(app);
            // Oyun odak gidince kendini küçülttüyse: geri getir, bakış kipine geç, hatırla.
            let app = app2.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(450));
                if prev.is_some_and(is_minimized) {
                    crate::log!("Oyun paneli: oyun odak gidince küçüldü, bakış kipine geçiliyor");
                    if let Some(e) = exe.clone() {
                        let mut list = glance_list(&app);
                        if !list.contains(&e) {
                            list.push(e);
                            if let Ok(j) = serde_json::to_string(&list) {
                                let _ = app.state::<crate::db::Db>().set_setting(GLANCE_KEY, &j);
                            }
                        }
                    }
                    if let Some(w) = app.get_webview_window(PANEL) {
                        set_activatable(&w, false);
                    }
                    app.state::<Panel>().focused.store(false, Ordering::Relaxed);
                    restore_focus(prev);
                    let _ = tauri::Emitter::emit_to(
                        &app,
                        PANEL,
                        "panel-open",
                        &OpenInfo {
                            mode: "glance",
                            exclusive,
                            minimizes: true,
                        },
                    );
                }
            });
        }
        Ok(())
    });
}

/// Pencereyi oyunun ekranına, odak almadan yerleştirir.
#[cfg(windows)]
fn place(w: &WebviewWindow, rect: Option<Rect>) {
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE};
    let Some((x, y, width, height)) = rect.or_else(|| {
        let m = w.primary_monitor().ok().flatten()?;
        Some((
            m.position().x,
            m.position().y,
            m.size().width,
            m.size().height,
        ))
    }) else {
        return;
    };
    if let Ok(hwnd) = w.hwnd() {
        unsafe {
            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                width as i32,
                height as i32,
                SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
fn place(w: &WebviewWindow, rect: Option<Rect>) {
    if let Some((x, y, width, height)) = rect {
        let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = w.set_size(tauri::PhysicalSize::new(width, height));
    }
}

/// Windows'a göre ön plandaki uygulama özel (exclusive) Direct3D tam ekranda mı.
#[cfg(windows)]
fn exclusive_fullscreen() -> bool {
    use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
    unsafe { SHQueryUserNotificationState() }.is_ok_and(|s| s == QUNS_RUNNING_D3D_FULL_SCREEN)
}

#[cfg(not(windows))]
fn exclusive_fullscreen() -> bool {
    false
}

#[cfg(windows)]
fn is_minimized(h: isize) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::IsIconic;
    unsafe { IsIconic(HWND(h as *mut _)).as_bool() }
}

#[cfg(not(windows))]
fn is_minimized(_h: isize) -> bool {
    false
}

#[cfg(windows)]
fn exe_name(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            h,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
        .is_ok();
        let _ = CloseHandle(h);
        ok.then(|| {
            let path = String::from_utf16_lossy(&buf[..size as usize]).to_lowercase();
            path.rsplit('\\').next().unwrap_or(&path).to_string()
        })
    }
}

#[cfg(not(windows))]
fn exe_name(_pid: u32) -> Option<String> {
    None
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

/// Klavye ve fare odağı (etkileşim kipi, ya da bakış kipinde not yazmak için).
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

/// Bakış kipinden etkileşime: "Fareyi serbest bırak".
pub fn interact(app: &AppHandle) {
    let app2 = app.clone();
    crate::window::on_ui_thread(app, "Oyun paneli", move |_| {
        take_focus(&app2);
        Ok(())
    });
}

pub fn hide(app: &AppHandle) {
    // Arayüz ölçümü (FPS, işlemci…) durdursun.
    let _ = tauri::Emitter::emit_to(app, PANEL, "panel-close", ());
    app.state::<crate::perf::Perf>()
        .presentmon
        .stop(app.state::<crate::paths::DataDir>().root());
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
