//! Odak çalmayan küçük pencereler:
//! - **Ekran göstergesi (OSD):** sağ üstte 2 sn görünen bilgi ("Mikrofon kapalı"). Tıklamaları geçirir.
//! - **Uyarılar:** sağ altta hatırlatıcı/sayaç kartları ("Ertele", "Tamam").
//!
//! İkisi de oyunun ya da çalıştığın pencerenin odağını almaz; ihtiyaç olunca oluşturulur,
//! bir süre kullanılmayınca kapatılır (bellek tutmasın diye).

use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const OSD: &str = "osd";
pub const ALERTS: &str = "alerts";

const OSD_SIZE: (f64, f64) = (380.0, 76.0);
const ALERTS_WIDTH: f64 = 380.0;
const MARGIN: f64 = 20.0;

#[derive(Serialize, Clone)]
pub struct OsdMessage {
    pub id: u64,
    pub icon: String,
    pub text: String,
}

#[derive(Default)]
pub struct Overlay {
    osd_gen: AtomicU64,
    last_osd: Mutex<Option<OsdMessage>>,
    /// Uyarı penceresinin son yüksekliği (mantıksal piksel).
    alerts_height: Mutex<Option<f64>>,
}

impl Overlay {
    /// OSD penceresi yeni açıldıysa son mesajı buradan alır.
    pub fn last_osd(&self) -> Option<OsdMessage> {
        self.last_osd.lock().ok().and_then(|m| m.clone())
    }
}

fn tool_window(app: &AppHandle, label: &str, size: (f64, f64)) -> tauri::Result<WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("GMID Tool")
        .inner_size(size.0, size.1)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .focused(false)
        .visible(false)
        .build()?;
    Ok(w)
}

/// Sağ üstte kısa bir bilgi gösterir.
pub fn show_osd(app: &AppHandle, icon: &str, text: impl Into<String>) {
    let overlay = app.state::<Overlay>();
    let id = overlay.osd_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let msg = OsdMessage {
        id,
        icon: icon.into(),
        text: text.into(),
    };
    if let Ok(mut last) = overlay.last_osd.lock() {
        *last = Some(msg.clone());
    }

    crate::window::on_ui_thread(app, "Gösterge", move |app| {
        let w = match app.get_webview_window(OSD) {
            Some(w) => w,
            None => tool_window(app, OSD, OSD_SIZE)?,
        };
        let _ = app.emit_to(OSD, "osd", &msg);
        place(&w, OSD_SIZE, Corner::TopRight);
        show_no_activate(&w);
        // Tıklamalar alttaki oyuna/pencereye geçsin. Pencere ekrana çıktıktan sonra yapılmalı
        // (bazı sistemlerde gizli pencerede bu işlem başarısız olur).
        let _ = w.set_ignore_cursor_events(true);
        Ok(())
    });

    let app = app.clone();
    std::thread::spawn(move || {
        let current = || app.state::<Overlay>().osd_gen.load(Ordering::SeqCst) == id;
        std::thread::sleep(Duration::from_millis(2200));
        if current() {
            if let Some(w) = app.get_webview_window(OSD) {
                let _ = w.hide();
            }
        }
        // Bir dakika kullanılmazsa pencereyi tamamen kapat.
        std::thread::sleep(Duration::from_secs(60));
        if current() {
            if let Some(w) = app.get_webview_window(OSD) {
                let _ = w.destroy();
            }
        }
    });
}

/// Zamanı gelen hatırlatıcı/sayaç kartlarını sağ altta gösterir.
pub fn raise_alerts(app: &AppHandle) {
    crate::window::on_ui_thread(app, "Uyarı penceresi", |app| {
        let w = match app.get_webview_window(ALERTS) {
            Some(w) => w,
            None => tool_window(app, ALERTS, (ALERTS_WIDTH, 160.0))?,
        };
        let _ = app.emit_to(ALERTS, "alerts-changed", ());
        let height = app
            .state::<Overlay>()
            .alerts_height
            .lock()
            .ok()
            .and_then(|h| *h)
            .unwrap_or(160.0);
        place(&w, (ALERTS_WIDTH, height), Corner::BottomRight);
        show_no_activate(&w);
        Ok(())
    });
    play_notification_sound(app);
}

/// Windows'un kendi bildirimi (bildirim merkezinde de kalır). Uyarı kartına ek olarak gönderilir;
/// ayarlardan kapatılabilir (`reminders.toast`).
pub fn system_notification(app: &AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let enabled = app
        .state::<crate::db::Db>()
        .setting_json::<bool>("reminders.toast")
        .unwrap_or(true);
    if !enabled {
        return;
    }
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        crate::log!("Windows bildirimi gönderilemedi: {e}");
    }
}

/// Uyarı penceresini içeriğin yüksekliğine göre boyutlandırır.
pub fn fit_alerts(app: &AppHandle, height: f64) {
    let height = height.clamp(60.0, 640.0);
    if let Ok(mut h) = app.state::<Overlay>().alerts_height.lock() {
        *h = Some(height);
    }
    if let Some(w) = app.get_webview_window(ALERTS) {
        // Bazı sistemlerde boyutu sabit pencereler yeniden boyutlandırılamaz; kısa süreliğine aç.
        let _ = w.set_resizable(true);
        let _ = w.set_size(LogicalSize::new(ALERTS_WIDTH, height));
        let _ = w.set_resizable(false);
        // Bazı sistemler boyutu hemen ya da tam uygulamaz; yerleşimi gerçek boyuta göre yap.
        let actual = w
            .outer_size()
            .ok()
            .zip(w.scale_factor().ok())
            .map(|(s, f)| s.height as f64 / f)
            .filter(|h| *h > 0.0)
            .unwrap_or(height);
        place(&w, (ALERTS_WIDTH, actual), Corner::BottomRight);
    }
}

pub fn close_alerts(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(ALERTS) {
        // Pencere kendi düğmesinden kapatılıyor; işi bitsin diye önce gizle, sonra kapat.
        let _ = w.hide();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            let _ = w.destroy();
        });
    }
}

enum Corner {
    TopRight,
    BottomRight,
}

/// Pencereyi birincil ekranın köşesine yerleştirir. Boyut, pencereden okunmaz (yeni açılmış
/// gizli pencerede henüz doğru olmayabilir); verdiğimiz mantıksal boyuttan hesaplanır.
fn place(w: &WebviewWindow, size: (f64, f64), corner: Corner) {
    let Ok(Some(m)) = w.primary_monitor() else {
        return;
    };
    let scale = m.scale_factor();
    let (width, height) = ((size.0 * scale) as i32, (size.1 * scale) as i32);
    let margin = (MARGIN * scale) as i32;
    let pos = match corner {
        Corner::TopRight => PhysicalPosition::new(
            m.position().x + m.size().width as i32 - width - margin,
            m.position().y + margin,
        ),
        // Görev çubuğunun üstünde kalsın diye çalışma alanı kullanılır.
        Corner::BottomRight => {
            let area = m.work_area();
            PhysicalPosition::new(
                area.position.x + area.size.width as i32 - width - margin,
                area.position.y + area.size.height as i32 - height - margin,
            )
        }
    };
    let _ = w.set_position(pos);
}

/// Pencereyi öne getirir ama odağı almaz: oyun ya da yazdığın şey kesilmez.
#[cfg(windows)]
fn show_no_activate(w: &WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, IsWindowVisible, SetWindowLongPtrW, SetWindowPos, ShowWindow,
        GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE,
        WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };
    let Ok(hwnd) = w.hwnd() else {
        let _ = w.show();
        return;
    };
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            ex | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize,
        );
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        // Bir sebeple görünmediyse Tauri'nin kendi yolunu dene (odak alsa da kart görünsün).
        if !IsWindowVisible(hwnd).as_bool() {
            crate::log!("Pencere odaksız gösterilemedi, normal gösteriliyor");
            let _ = w.show();
        }
    }
}

#[cfg(not(windows))]
fn show_no_activate(w: &WebviewWindow) {
    let _ = w.show();
}

/// Windows'un kendi bildirim sesi (ayarlardan kapatılabilir).
fn play_notification_sound(app: &AppHandle) {
    let enabled = app
        .state::<crate::db::Db>()
        .setting_json::<bool>("reminders.sound")
        .unwrap_or(true);
    #[cfg(windows)]
    if enabled {
        unsafe {
            use windows::core::w;
            use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};
            let _ = PlaySoundW(
                w!("SystemNotification"),
                None,
                SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
            );
        }
    }
    #[cfg(not(windows))]
    let _ = enabled;
}
