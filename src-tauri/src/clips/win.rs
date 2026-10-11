//! Windows'a özgü: Geri Dönüşüm Kutusu'na gönderme ve dosyayı panoya koyma (Discord'da Ctrl+V ile ek olur).

use std::path::Path;

#[cfg(windows)]
fn wide_list(paths: &[&Path]) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let mut w: Vec<u16> = Vec::new();
    for p in paths {
        w.extend(p.as_os_str().encode_wide());
        w.push(0);
    }
    w.push(0);
    w
}

/// Dosyayı Geri Dönüşüm Kutusu'na gönderir (geri alınabilir).
#[cfg(windows)]
pub fn recycle(path: &Path) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };
    let from = wide_list(&[path]);
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI).0 as u16,
        ..Default::default()
    };
    let r = unsafe { SHFileOperationW(&mut op) };
    if r != 0 {
        return Err(format!("Silinemedi (kod {r}); dosya açık olabilir"));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn recycle(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| e.to_string())
}

/// Dosyayı panoya "kopyalanmış dosya" olarak koyar (Gezgin'de Ctrl+C gibi).
#[cfg(windows)]
pub fn copy_file(path: &Path) -> Result<(), String> {
    use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::UI::Shell::DROPFILES;
    const CF_HDROP: u32 = 15;
    let list = wide_list(&[path]);
    let header = std::mem::size_of::<DROPFILES>();
    let bytes = header + list.len() * 2;
    unsafe {
        let mem: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE, bytes).map_err(|e| e.to_string())?;
        let ptr = GlobalLock(mem) as *mut u8;
        if ptr.is_null() {
            let _ = GlobalFree(Some(mem));
            return Err("Bellek ayrılamadı".into());
        }
        let df = DROPFILES {
            pFiles: header as u32,
            fWide: true.into(),
            ..Default::default()
        };
        std::ptr::copy_nonoverlapping(&df as *const DROPFILES as *const u8, ptr, header);
        std::ptr::copy_nonoverlapping(list.as_ptr() as *const u8, ptr.add(header), list.len() * 2);
        let _ = GlobalUnlock(mem);
        if OpenClipboard(None).is_err() {
            let _ = GlobalFree(Some(mem));
            return Err("Pano başka bir program tarafından kullanılıyor".into());
        }
        let _ = EmptyClipboard();
        let set = SetClipboardData(CF_HDROP, Some(HANDLE(mem.0)));
        let _ = CloseClipboard();
        if set.is_err() {
            let _ = GlobalFree(Some(mem));
            return Err("Panoya konamadı".into());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn copy_file(_path: &Path) -> Result<(), String> {
    Err("Panoya dosya koymak yalnızca Windows'ta".into())
}
