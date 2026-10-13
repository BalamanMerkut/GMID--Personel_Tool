//! Dosyaları uygulamadan dışarı sürükleme (Masaüstü, bir klasör, Discord, tarayıcı…). Windows Gezgini'nin
//! kullandığı sürükleme kullanılır: hedef dosyayı kopyalar, kaynaktaki dosya yerinde kalır (taşıma yok).

use std::path::PathBuf;
use tauri::WebviewWindow;

/// Sürüklemeyi başlatır; fare basılıyken çağrılmalı. Bırakılınca kendiliğinden biter.
pub fn start(w: &WebviewWindow, paths: Vec<String>) -> Result<(), String> {
    let files: Vec<PathBuf> = paths
        .into_iter()
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect();
    if files.is_empty() {
        return Err("Dosya bulunamadı (taşınmış ya da silinmiş olabilir).".into());
    }
    #[cfg(windows)]
    {
        let hwnd = w.hwnd().map_err(|e| e.to_string())?.0 as isize;
        // Sürükleme döngüsü pencerenin iş parçacığında dönmeli.
        w.run_on_main_thread(move || {
            if let Err(e) = unsafe { imp::drag(hwnd, &files) } {
                crate::log!("Sürükleme başlatılamadı: {e}");
            }
        })
        .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = (w, files);
        Err("Sürükleme yalnız Windows'ta".into())
    }
}

#[cfg(windows)]
mod imp {
    use std::path::PathBuf;
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{E_FAIL, HWND};
    use windows::Win32::System::Com::{IBindCtx, IDataObject};
    use windows::Win32::System::Ole::{IDropSource, OleInitialize, DROPEFFECT_COPY};
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{
        BHID_DataObject, ILCreateFromPathW, ILFree, SHCreateShellItemArrayFromIDLists, SHDoDragDrop,
    };

    pub unsafe fn drag(hwnd: isize, files: &[PathBuf]) -> windows::core::Result<()> {
        // Zaten açıksa yalnız sayacı artırır.
        let _ = unsafe { OleInitialize(None) };
        let pidls: Vec<*mut ITEMIDLIST> = files
            .iter()
            .map(|f| unsafe { ILCreateFromPathW(&HSTRING::from(f.as_os_str())) })
            .filter(|p| !p.is_null())
            .collect();
        if pidls.is_empty() {
            return Err(E_FAIL.into());
        }
        let list: Vec<*const ITEMIDLIST> = pidls.iter().map(|p| *p as *const _).collect();
        let r = (|| unsafe {
            let items = SHCreateShellItemArrayFromIDLists(&list)?;
            let data: IDataObject = items.BindToHandler(None::<&IBindCtx>, &BHID_DataObject)?;
            // Bırakan taraf yalnız kopyalayabilir; dosya kütüphaneden kaybolmaz.
            SHDoDragDrop(
                Some(HWND(hwnd as *mut _)),
                &data,
                None::<&IDropSource>,
                DROPEFFECT_COPY,
            )?;
            Ok(())
        })();
        for p in pidls {
            unsafe { ILFree(Some(p as *const _)) };
        }
        r
    }
}
