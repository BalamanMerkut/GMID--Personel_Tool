//! Tek tuşla ekran görüntüsü: hiçbir pencere açılmaz, odak oyundan ayrılmaz (alt-tab yok). Ön plandaki
//! pencerenin bulunduğu ekran yakalanır, panoya konur (Ctrl+V ile Discord'a, nota…) ve
//! `Videolar\GMID\<oyun>` klasörüne PNG olarak kaydedilir; Kliplerim'de de görünür.
//!
//! Yakalama önce DXGI masaüstü çoğaltmasıyla yapılır (tam ekran oyunlar dahil, sarı çerçeve yok); olmazsa
//! klasik ekran kopyası (GDI).

use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Yakalanan görüntü: RGBA, satır satır.
pub struct Shot {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Kısayola basılınca: yakala, panoya koy, kaydet, kısaca haber ver.
pub fn take(app: &AppHandle) {
    let started = std::time::Instant::now();
    let shot = match capture() {
        Ok(s) => s,
        Err(e) => {
            crate::log!("Ekran görüntüsü alınamadı: {e}");
            crate::overlay::show_osd(app, "image", "Ekran görüntüsü alınamadı");
            return;
        }
    };
    let Some(png) = crate::uninstall::icon::encode_png(shot.width, shot.height, &shot.rgba) else {
        crate::overlay::show_osd(app, "image", "Ekran görüntüsü kaydedilemedi");
        return;
    };
    // Pano sahibi olarak ana pencere (Windows, sahipsiz panoya yazmayı reddedebilir).
    #[cfg(windows)]
    let owner = app
        .get_webview_window("main")
        .and_then(|w| w.hwnd().ok())
        .map_or(0, |h| h.0 as isize);
    #[cfg(not(windows))]
    let owner = 0isize;
    let copied = to_clipboard(&shot, &png, owner);
    let game = current_game(app);
    let saved = app
        .path()
        .video_dir()
        .map_err(|e| e.to_string())
        .and_then(|v| save(&v, game.as_deref(), &png));
    if let Err(e) = &saved {
        crate::log!("Ekran görüntüsü dosyaya yazılamadı: {e}");
    }
    crate::log!(
        "Ekran görüntüsü: {}×{} ({} ms)",
        shot.width,
        shot.height,
        started.elapsed().as_millis()
    );
    let text = match (copied, saved.is_ok()) {
        (true, _) => "Ekran görüntüsü panoda · Ctrl+V ile yapıştır",
        (false, true) => "Ekran görüntüsü kaydedildi (panoya konamadı)",
        (false, false) => "Ekran görüntüsü alınamadı",
    };
    crate::overlay::show_osd(app, "image", text);
    // Kliplerim hemen görsün.
    let app = app.clone();
    std::thread::spawn(move || {
        let _ = app.state::<crate::clips::Clips>().scan(&app);
    });
}

/// Şu an açık oyunun adı (yoksa `None`: masaüstü).
fn current_game(app: &AppHandle) -> Option<String> {
    let ids = app.state::<crate::games::Games>().running_ids();
    let db = app.state::<crate::db::Db>();
    ids.iter().find_map(|id| {
        db.conn()
            .query_row("SELECT name FROM games WHERE id = ?1", [id], |r| {
                r.get::<_, String>(0)
            })
            .ok()
    })
}

/// `Videolar\GMID\<oyun>\<oyun> 2026.10.14 - 22.30.05.12.png` (NVIDIA'nın adlandırmasına benzer).
pub fn save(videos: &Path, game: Option<&str>, png: &[u8]) -> Result<PathBuf, String> {
    let name = game
        .map(safe_name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Masaüstü".into());
    let dir = videos.join("GMID").join(&name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y.%m.%d - %H.%M.%S.%3f");
    let path = dir.join(format!("{name} {stamp}.png"));
    std::fs::write(&path, png).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Windows'ta dosya adında olamayan karakterler atılır.
fn safe_name(s: &str) -> String {
    s.chars()
        .filter(|c| !"<>:\"/\\|?*".contains(*c) && !c.is_control())
        .collect::<String>()
        .trim()
        .trim_end_matches('.')
        .chars()
        .take(60)
        .collect()
}

/// RGBA → 24 bit, alttan yukarı DIB satırları (pano için; her satır 4 bayta tamamlanır).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn dib_rows(shot: &Shot) -> Vec<u8> {
    let (w, h) = (shot.width as usize, shot.height as usize);
    let stride = (w * 3).div_ceil(4) * 4;
    let mut out = vec![0u8; stride * h];
    for y in 0..h {
        let src = &shot.rgba[y * w * 4..(y + 1) * w * 4];
        let dst = &mut out[(h - 1 - y) * stride..(h - 1 - y) * stride + w * 3];
        for x in 0..w {
            dst[x * 3] = src[x * 4 + 2];
            dst[x * 3 + 1] = src[x * 4 + 1];
            dst[x * 3 + 2] = src[x * 4];
        }
    }
    out
}

#[cfg(windows)]
fn capture() -> Result<Shot, String> {
    use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let mon = unsafe { MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTONEAREST) };
    match win::dxgi(mon) {
        Ok(s) => Ok(s),
        Err(e) => {
            crate::log!("Ekran görüntüsü: DXGI olmadı ({e}), klasik yöntem deneniyor");
            win::gdi(mon)
        }
    }
}

#[cfg(not(windows))]
fn capture() -> Result<Shot, String> {
    Err("Yalnız Windows'ta".into())
}

#[cfg(windows)]
fn to_clipboard(shot: &Shot, png: &[u8], owner: isize) -> bool {
    match win::clipboard(shot, png, owner) {
        Ok(()) => true,
        Err(e) => {
            crate::log!("Ekran görüntüsü panoya konamadı: {e}");
            false
        }
    }
}

#[cfg(not(windows))]
fn to_clipboard(_shot: &Shot, _png: &[u8], _owner: isize) -> bool {
    false
}

#[cfg(windows)]
mod win {
    use super::Shot;
    use windows::core::{Interface, PCWSTR};
    use windows::Win32::Foundation::{HANDLE, HGLOBAL, HMODULE, HWND};
    use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
    use windows::Win32::Graphics::Direct3D11::{
        D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
        D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE,
        D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    };
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, IDXGIOutput1, IDXGIResource,
        DXGI_OUTDUPL_FRAME_INFO,
    };
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, GetMonitorInfoW, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        CAPTUREBLT, DIB_RGB_COLORS, HMONITOR, MONITORINFO, SRCCOPY,
    };
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::CF_DIB;

    /// Monitörün ekran kartındaki görüntüsü (oyun tam ekran olsa da).
    pub fn dxgi(mon: HMONITOR) -> Result<Shot, String> {
        unsafe {
            let factory: IDXGIFactory1 = CreateDXGIFactory1().map_err(|e| e.to_string())?;
            let mut i = 0;
            while let Ok(adapter) = factory.EnumAdapters1(i) {
                i += 1;
                let mut j = 0;
                while let Ok(output) = adapter.EnumOutputs(j) {
                    j += 1;
                    let desc = output.GetDesc().map_err(|e| e.to_string())?;
                    if desc.Monitor != mon {
                        continue;
                    }
                    let out1: IDXGIOutput1 = output.cast().map_err(|e| e.to_string())?;
                    return duplicate(&adapter, &out1);
                }
            }
            Err("ekran bulunamadı".into())
        }
    }

    unsafe fn duplicate(adapter: &IDXGIAdapter1, output: &IDXGIOutput1) -> Result<Shot, String> {
        let mut device: Option<ID3D11Device> = None;
        let mut ctx: Option<ID3D11DeviceContext> = None;
        unsafe {
            D3D11CreateDevice(
                adapter,
                D3D_DRIVER_TYPE_UNKNOWN,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut ctx),
            )
            .map_err(|e| format!("D3D11: {e}"))?;
        }
        let (device, ctx) = (device.ok_or("D3D11 yok")?, ctx.ok_or("D3D11 yok")?);
        let dup =
            unsafe { output.DuplicateOutput(&device) }.map_err(|e| format!("çoğaltma: {e}"))?;
        // İlk kare genelde hemen gelir; ekran hiç değişmiyorsa birkaç kez beklenir.
        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut res: Option<IDXGIResource> = None;
        let mut got = false;
        for _ in 0..6 {
            match unsafe { dup.AcquireNextFrame(150, &mut info, &mut res) } {
                Ok(()) if res.is_some() => {
                    got = true;
                    break;
                }
                Ok(()) => {
                    let _ = unsafe { dup.ReleaseFrame() };
                }
                Err(_) => {}
            }
        }
        if !got {
            return Err("kare gelmedi".into());
        }
        let result = (|| {
            let tex: ID3D11Texture2D = res.ok_or("kare yok")?.cast().map_err(|e| e.to_string())?;
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            unsafe { tex.GetDesc(&mut desc) };
            if desc.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
                return Err(format!("beklenmeyen biçim {:?}", desc.Format));
            }
            let staging_desc = D3D11_TEXTURE2D_DESC {
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
                MipLevels: 1,
                ArraySize: 1,
                ..desc
            };
            let mut staging: Option<ID3D11Texture2D> = None;
            unsafe { device.CreateTexture2D(&staging_desc, None, Some(&mut staging)) }
                .map_err(|e| e.to_string())?;
            let staging = staging.ok_or("doku yok")?;
            unsafe { ctx.CopyResource(&staging, &tex) };
            let mut map = D3D11_MAPPED_SUBRESOURCE::default();
            unsafe { ctx.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut map)) }
                .map_err(|e| e.to_string())?;
            let (w, h) = (desc.Width as usize, desc.Height as usize);
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..h {
                let row = unsafe {
                    std::slice::from_raw_parts(
                        (map.pData as *const u8).add(y * map.RowPitch as usize),
                        w * 4,
                    )
                };
                for x in 0..w {
                    let (s, d) = (&row[x * 4..x * 4 + 4], y * w * 4 + x * 4);
                    rgba[d] = s[2];
                    rgba[d + 1] = s[1];
                    rgba[d + 2] = s[0];
                    rgba[d + 3] = 255;
                }
            }
            unsafe { ctx.Unmap(&staging, 0) };
            Ok(Shot {
                width: w as u32,
                height: h as u32,
                rgba,
            })
        })();
        let _ = unsafe { dup.ReleaseFrame() };
        result
    }

    /// Klasik ekran kopyası (masaüstü ve pencereli oyunlar).
    pub fn gdi(mon: HMONITOR) -> Result<Shot, String> {
        unsafe {
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(mon, &mut mi).as_bool() {
                return Err("ekran bilgisi alınamadı".into());
            }
            let r = mi.rcMonitor;
            let (w, h) = (r.right - r.left, r.bottom - r.top);
            let screen = GetDC(None);
            let mem = CreateCompatibleDC(Some(screen));
            let bmp = CreateCompatibleBitmap(screen, w, h);
            let old = SelectObject(mem, bmp.into());
            let ok = BitBlt(
                mem,
                0,
                0,
                w,
                h,
                Some(screen),
                r.left,
                r.top,
                SRCCOPY | CAPTUREBLT,
            )
            .is_ok();
            let mut info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bgra = vec![0u8; (w * h * 4) as usize];
            let lines = GetDIBits(
                mem,
                bmp,
                0,
                h as u32,
                Some(bgra.as_mut_ptr() as *mut _),
                &mut info,
                DIB_RGB_COLORS,
            );
            SelectObject(mem, old);
            let _ = DeleteObject(bmp.into());
            let _ = DeleteDC(mem);
            ReleaseDC(None, screen);
            if !ok || lines == 0 {
                return Err("ekran kopyalanamadı".into());
            }
            for px in bgra.chunks_exact_mut(4) {
                px.swap(0, 2);
                px[3] = 255;
            }
            Ok(Shot {
                width: w as u32,
                height: h as u32,
                rgba: bgra,
            })
        }
    }

    /// Panoya iki biçimde: PNG (Discord, tarayıcılar, GMID notları) ve DIB (Paint, Word…).
    pub fn clipboard(shot: &Shot, png: &[u8], owner: isize) -> Result<(), String> {
        let rows = super::dib_rows(shot);
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: shot.width as i32,
            biHeight: shot.height as i32,
            biPlanes: 1,
            biBitCount: 24,
            biCompression: BI_RGB.0,
            biSizeImage: rows.len() as u32,
            ..Default::default()
        };
        let head = unsafe {
            std::slice::from_raw_parts(
                &header as *const _ as *const u8,
                std::mem::size_of::<BITMAPINFOHEADER>(),
            )
        };
        let dib = global(&[head, &rows])?;
        let png_mem = global(&[png])?;
        unsafe {
            // Başka bir program panoyu tutuyorsa biraz beklenir.
            let mut opened = false;
            for _ in 0..10 {
                if OpenClipboard(Some(HWND(owner as *mut _))).is_ok() {
                    opened = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
            if !opened {
                return Err("pano meşgul".into());
            }
            let _ = EmptyClipboard();
            let png_fmt = RegisterClipboardFormatW(PCWSTR(windows::core::w!("PNG").as_ptr()));
            let a = SetClipboardData(png_fmt, Some(HANDLE(png_mem.0)));
            let b = SetClipboardData(CF_DIB.0 as u32, Some(HANDLE(dib.0)));
            let _ = CloseClipboard();
            if a.is_err() && b.is_err() {
                return Err("panoya yazılamadı".into());
            }
        }
        Ok(())
    }

    /// Pano için taşınabilir bellek bloğu (sahibi pano olur).
    fn global(parts: &[&[u8]]) -> Result<HGLOBAL, String> {
        let len: usize = parts.iter().map(|p| p.len()).sum();
        unsafe {
            let mem = GlobalAlloc(GMEM_MOVEABLE, len).map_err(|e| e.to_string())?;
            let ptr = GlobalLock(mem) as *mut u8;
            if ptr.is_null() {
                return Err("bellek ayrılamadı".into());
            }
            let mut at = 0;
            for p in parts {
                std::ptr::copy_nonoverlapping(p.as_ptr(), ptr.add(at), p.len());
                at += p.len();
            }
            let _ = GlobalUnlock(mem);
            Ok(mem)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dib_is_bottom_up_bgr_with_padding() {
        // 2×2: üst satır kırmızı-yeşil, alt satır mavi-beyaz.
        let shot = Shot {
            width: 2,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, //
                0, 0, 255, 255, 255, 255, 255, 255,
            ],
        };
        let d = dib_rows(&shot);
        assert_eq!(d.len(), 16); // 2*3=6 → 8 bayt satır
        assert_eq!(&d[0..6], &[255, 0, 0, 255, 255, 255]); // alt satır önce: mavi, beyaz (BGR)
        assert_eq!(&d[8..14], &[0, 0, 255, 0, 255, 0]); // üst: kırmızı, yeşil
    }

    #[test]
    fn files_go_under_gmid_game_folder() {
        let dir = std::env::temp_dir().join(format!("gmid-shot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = save(&dir, Some("Counter-Strike 2: GO?"), b"png").unwrap();
        assert!(p.starts_with(dir.join("GMID").join("Counter-Strike 2 GO")));
        assert!(p.extension().is_some_and(|e| e == "png"));
        let q = save(&dir, None, b"png").unwrap();
        assert!(q.starts_with(dir.join("GMID").join("Masaüstü")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
