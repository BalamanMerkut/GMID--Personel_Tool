//! Oyun İçi Panel (Steam'deki Shift+Tab gibi): `Ctrl+Shift+G` ile oyunun bulunduğu ekranın üstünde yarı saydam
//! bir pencere açılır; aynı tuş, kapat düğmesi ya da `Esc` ile kapanır ve oyun yeniden öne gelir. Oyuna hiçbir şey
//! enjekte edilmez (anti-cheat güvenli): panel, oyunun üstünde duran ayrı bir penceredir.
//!
//! İki kip:
//! - **Etkileşim** (Shift+Tab gibi): panel odağı alır, oyun fareyi bırakır; kenarlıksız/pencereli oyunlar arkada
//!   görünür kalır. Kendiliğinden kip, oyun özel tam ekranda değilse bunu seçer.
//! - **Bakış**: panel odak almaz, oyun önde kalır (özel tam ekranda odak gidince oyunlar küçülür). Fare, oyunun
//!   menüsü açıkken (imleç görünürken) tıklayabilir. Fare oyunda kilitli olduğundan panele geçmek için farenin
//!   **orta tuşu** (ayardan sol tık) kullanılır: panel açıkken tuş izlenir (kanca yok, yalnız tuş durumu okunur).
//!
//! Etkileşim kipinde oyun yine de küçülürse (bazı oyunlar odak gidince kendini küçültür) hemen geri getirilir,
//! panel bakış kipine geçer ve bu oyun hatırlanır: bir dahaki sefere doğrudan bakış kipinde açılır.
//! Panel penceresi açılışta gizli olarak hazırlanır; oyun sırasında ilk kez oluşturulurken odak kapıp oyunu
//! küçültmesin diye.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const PANEL: &str = "panel";
/// Odak alınca küçülen oyunlar (exe adları): bunlarda doğrudan bakış kipi.
const GLANCE_KEY: &str = "panel.glanceExes";
/// auto | interactive | glance
const MODE_KEY: &str = "panel.mode";
/// Bakış kipinde fareyi panele geçiren tuş: middle | left | off
const GRAB_KEY: &str = "panel.grab";

/// Ekran dikdörtgeni (fiziksel piksel): x, y, genişlik, yükseklik.
type Rect = (i32, i32, u32, u32);

/// Panel açılmadan önce ön plandaki pencere (kapanınca odak ona döner).
#[derive(Default)]
pub struct Panel {
    #[allow(dead_code)]
    previous: Mutex<Option<isize>>,
    /// Panel odak aldıysa (etkileşim kipi ya da not yazmak için).
    focused: AtomicBool,
    /// Her açılışta artar: eski açılışın tuş izleyicisi kendini bitirsin.
    opened: AtomicU32,
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
    /// Öndeki oyunun exe adı (kenarlıksız listesine eklemek için).
    pub exe: Option<String>,
    /// Bu oyun kenarlıksız tam ekran listesinde.
    pub borderless: bool,
    /// Bakış kipinde fareyi panele geçiren tuş: middle | left | off
    pub grab: String,
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
    let opened = state.opened.fetch_add(1, Ordering::Relaxed) + 1;

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
    let borderless = exe.as_ref().is_some_and(|e| {
        crate::borderless::list(&app.state::<crate::db::Db>()).contains(&e.to_lowercase())
    });
    let info = OpenInfo {
        mode: if interactive { "interactive" } else { "glance" },
        exclusive,
        minimizes: remembered,
        exe: exe.clone(),
        borderless,
        grab: grab_setting(app),
    };
    if exclusive {
        // Özel tam ekranda panel görünmez: kullanıcıya bir kez nedenini ve çaresini söyle.
        explain_fullscreen(app, exe.as_deref());
    }
    if !interactive {
        watch_grab(app, opened);
    }

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
                // Oyunlar odak gidince birkaç yüz milisaniye içinde küçülür; bir saniye boyunca bak.
                let minimized = (0..24).any(|_| {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    prev.is_some_and(is_minimized)
                });
                if minimized {
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
                    explain_fullscreen(&app, exe.as_deref());
                    watch_grab(&app, opened);
                    let _ = tauri::Emitter::emit_to(
                        &app,
                        PANEL,
                        "panel-open",
                        &OpenInfo {
                            mode: "glance",
                            exclusive,
                            minimizes: true,
                            exe: exe.clone(),
                            borderless: true,
                            grab: grab_setting(&app),
                        },
                    );
                }
            });
        }
        Ok(())
    });
}

/// Oyun özel tam ekranda (Windows onun üstünde pencere göstermez, odak gidince küçülür): oyunu kenarlıksız
/// listesine ekler (pencereli moda alınınca kendiliğinden kenarlıksız tam ekran olur) ve bir kez Windows
/// bildirimiyle ne yapılacağını söyler. Oyun Modu'nun sessiz kipi bu bildirimi engellemez.
fn explain_fullscreen(app: &AppHandle, exe: Option<&str>) {
    use tauri_plugin_notification::NotificationExt;
    let Some(exe) = exe else {
        return;
    };
    let db = app.state::<crate::db::Db>();
    if crate::borderless::list(&db).contains(&exe.to_lowercase()) {
        return;
    }
    if crate::borderless::set(&db, exe, true).is_err() {
        return;
    }
    crate::log!("Oyun İçi Panel: {exe} özel tam ekranda, kenarlıksız listesine eklendi");
    let _ = app
        .notification()
        .builder()
        .title("Oyun İçi Panel bu oyunda görünmüyor")
        .body("Oyun tam ekran (özel) modda; Windows onun üstünde pencere göstermiyor. Oyunun görüntü ayarından Pencereli (Windowed) moda al: GMID onu kenarlıksız tam ekrana çevirir ve panel Steam'deki gibi açılır.")
        .show();
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

fn exe_name(pid: u32) -> Option<String> {
    crate::games::procs::exe_of(pid)
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

/// Klavye ve fare odağı (etkileşim kipi, ya da bakış kipinde not yazmak için). Panel öne geçer ve oyunun fareyi
/// pencereye kilitlemesi (ClipCursor) kaldırılır: imleç panelde serbest dolaşır.
pub fn take_focus(app: &AppHandle) {
    let Some(w) = app.get_webview_window(PANEL) else {
        return;
    };
    if app.state::<Panel>().focused.swap(true, Ordering::Relaxed) {
        return;
    }
    set_activatable(&w, true);
    let _ = w.set_focus();
    force_foreground(&w);
}

/// Bakış kipinde fareyi panele geçiren tuş (Windows sanal tuş kodu); kapalıysa None.
fn grab_setting(app: &AppHandle) -> String {
    app.state::<crate::db::Db>()
        .setting_json(GRAB_KEY)
        .unwrap_or_else(|| "middle".into())
}

fn grab_button(app: &AppHandle) -> Option<i32> {
    match grab_setting(app).as_str() {
        "left" => Some(0x01), // VK_LBUTTON
        "off" => None,
        _ => Some(0x04), // VK_MBUTTON
    }
}

/// Bakış kipinde panel açıkken seçilen fare tuşunu izler: basılınca panel fareyi alır. Kanca kurulmaz; yalnız
/// tuşun durumu 25 ms'de bir okunur ve panel kapanınca (ya da odak alınca) izleme biter.
fn watch_grab(app: &AppHandle, opened: u32) {
    let Some(vk) = grab_button(app) else {
        return;
    };
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<Panel>();
        // Açılış anında basılıysa bırakılmasını bekle (yanlışlıkla hemen geçmesin).
        let mut was = button_down(vk);
        for _ in 0..(20 * 60 * 40) {
            std::thread::sleep(std::time::Duration::from_millis(25));
            if state.opened.load(Ordering::Relaxed) != opened
                || state.focused.load(Ordering::Relaxed)
                || !app
                    .get_webview_window(PANEL)
                    .is_some_and(|w| w.is_visible().unwrap_or(false))
            {
                return;
            }
            let down = button_down(vk);
            if down && !was {
                interact(&app);
                let _ = tauri::Emitter::emit_to(&app, PANEL, "panel-grabbed", ());
                return;
            }
            was = down;
        }
    });
}

#[cfg(windows)]
fn button_down(vk: i32) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 }
}

#[cfg(not(windows))]
fn button_down(_vk: i32) -> bool {
    false
}

/// Panel gerçekten öne geçsin (Windows arka plandan öne geçmeyi bazen engeller) ve imleç kilidi kalksın.
#[cfg(windows)]
fn force_foreground(w: &WebviewWindow) {
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, ClipCursor, GetForegroundWindow, GetWindowThreadProcessId,
        SetForegroundWindow,
    };
    let Ok(hwnd) = w.hwnd() else {
        return;
    };
    unsafe {
        let fg = GetForegroundWindow();
        if fg != hwnd {
            let theirs = GetWindowThreadProcessId(fg, None);
            let mine = GetCurrentThreadId();
            let attach = theirs != 0 && theirs != mine;
            if attach {
                let _ = AttachThreadInput(mine, theirs, true);
            }
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
            let _ = SetFocus(Some(hwnd));
            if attach {
                let _ = AttachThreadInput(mine, theirs, false);
            }
        }
        // Oyun imleci kendi penceresine kilitlediyse kaldır.
        let _ = ClipCursor(None);
    }
}

#[cfg(not(windows))]
fn force_foreground(_w: &WebviewWindow) {}

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

/// Öndeki pencere bulunduğu ekranı tamamen kaplıyor mu (oyun ya da tam ekran video). Masaüstü, görev çubuğu
/// ve GMID'in kendi pencereleri sayılmaz.
#[cfg(windows)]
pub fn foreground_fullscreen() -> bool {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetWindowRect};
    let (Some(h), Some((x, y, w, ht))) = foreground() else {
        return false;
    };
    if pid_of(h) == Some(std::process::id()) {
        return false;
    }
    let hwnd = HWND(h as *mut _);
    let mut name = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut name) } as usize;
    let class = String::from_utf16_lossy(&name[..n.min(name.len())]);
    if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
        return false;
    }
    let mut r = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut r) }.is_err() {
        return false;
    }
    r.left <= x && r.top <= y && r.right >= x + w as i32 && r.bottom >= y + ht as i32
}

#[cfg(not(windows))]
pub fn foreground_fullscreen() -> bool {
    false
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
