//! Program simgeleri: kaldırma kaydındaki `DisplayIcon` (yoksa MSI'ın `ProductIcon`'u, kurulum klasöründeki
//! exe ya da kaldırıcının kendisi) dosyasından Windows'un simgesi çıkarılır ve PNG'ye çevrilir.

use super::{reg, Program};
use std::path::Path;

/// Simge boyutu (piksel). Listede 32 px gösterilir; 2x ekranlarda net dursun.
pub const SIZE: u32 = 64;

/// "%SystemRoot%\x.exe" → "C:\Windows\x.exe" (bilinmeyen değişken olduğu gibi kalır).
pub fn expand_env(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('%') {
            Some(j) if j > 0 => {
                let name = &after[..j];
                match std::env::var(name) {
                    Ok(v) => out.push_str(&v),
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[j + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `"C:\x\app.exe",-101` → ("C:\x\app.exe", -101)
pub fn split_icon(s: &str) -> (String, i32) {
    let s = expand_env(s.trim());
    if let Some(i) = s.rfind(',') {
        if let Ok(n) = s[i + 1..].trim().parse::<i32>() {
            return (s[..i].trim().trim_matches('"').to_string(), n);
        }
    }
    (s.trim_matches('"').to_string(), 0)
}

/// MSI ürün kodu → kayıt defterindeki "sıkıştırılmış" biçim
/// ({8BDFE669-9705-4184-9368-DB9CE581E0E7} → 966EFDB8507948143986BDC95E180E7E).
pub fn packed_guid(code: &str) -> Option<String> {
    let hex: String = code.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    let wrapped = code.trim().starts_with('{') && code.trim().ends_with('}');
    if hex.len() != 32 || !wrapped {
        return None;
    }
    let rev = |s: &str| s.chars().rev().collect::<String>();
    let mut out = rev(&hex[0..8]) + &rev(&hex[8..12]) + &rev(&hex[12..16]);
    for k in (16..32).step_by(2) {
        out += &rev(&hex[k..k + 2]);
    }
    Some(out.to_uppercase())
}

fn is_file(p: &str) -> bool {
    !p.is_empty() && Path::new(p).is_file()
}

/// Simgenin alınacağı dosya ve dizin, sırayla denenecek adaylar.
pub fn sources(p: &Program) -> Vec<(String, i32)> {
    let mut out: Vec<(String, i32)> = Vec::new();
    let mut push = |s: (String, i32)| {
        if is_file(&s.0) && !out.contains(&s) {
            out.push(s);
        }
    };
    if !p.icon.trim().is_empty() {
        push(split_icon(&p.icon));
    }
    // MSI: ürünün kendi simgesi C:\Windows\Installer\{kod}\ altında durur.
    let code = p.id.rsplit('\\').next().unwrap_or("");
    if let Some(packed) = packed_guid(code) {
        for root in [
            r"HKLM\SOFTWARE\Classes\Installer\Products",
            r"HKCU\Software\Microsoft\Installer\Products",
        ] {
            let key = format!("{root}\\{packed}");
            if let Some((_, v)) = reg::values(&key)
                .into_iter()
                .find(|(n, _)| n.eq_ignore_ascii_case("ProductIcon"))
            {
                push(split_icon(&v));
            }
        }
    }
    // Kurulum klasöründe adı programa en çok benzeyen exe.
    let loc = p.location.trim().trim_matches('"');
    if !loc.is_empty() {
        if let Some(exe) = best_exe(Path::new(loc), &p.name) {
            push((exe, 0));
        }
    }
    // Son çare: kaldırıcının kendi exe'si.
    if let Some((exe, _)) = super::split_command(&p.uninstall) {
        if !exe.to_lowercase().starts_with("msiexec") {
            push((expand_env(&exe), 0));
        }
    }
    out
}

/// Klasörün ilk katındaki exe'lerden program adına en uygunu (kaldırıcı/güncelleyici değil).
fn best_exe(dir: &Path, name: &str) -> Option<String> {
    const SKIP: [&str; 6] = ["unins", "uninst", "update", "crash", "helper", "setup"];
    let want = super::rules::norm(&super::rules::strip_version(name));
    let mut best: Option<(i32, String)> = None;
    for e in std::fs::read_dir(dir).ok()?.flatten().take(400) {
        let path = e.path();
        if !path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("exe"))
        {
            continue;
        }
        let stem = super::rules::norm(&path.file_stem()?.to_string_lossy());
        if stem.is_empty() || SKIP.iter().any(|s| stem.contains(s)) {
            continue;
        }
        let score = if stem == want {
            3
        } else if !want.is_empty() && (want.contains(&stem) || stem.contains(&want)) {
            2
        } else {
            1
        };
        if best.as_ref().map_or(true, |(s, _)| score > *s) {
            best = Some((score, path.to_string_lossy().into_owned()));
        }
    }
    best.map(|b| b.1)
}

/// Simgeyi PNG olarak döndürür (data URL'ye çevrilmeye hazır).
pub fn png_for(p: &Program) -> Option<Vec<u8>> {
    sources(p)
        .into_iter()
        .find_map(|(f, i)| extract_png(&f, i, SIZE))
}

/// RGBA pikselleri PNG'ye çevirir.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().ok()?;
        wr.write_image_data(rgba).ok()?;
    }
    Some(out)
}

#[cfg(windows)]
pub fn extract_png(file: &str, index: i32, size: u32) -> Option<Vec<u8>> {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::SHDefExtractIconW;
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};
    let mut h = HICON::default();
    let hr = unsafe {
        SHDefExtractIconW(
            &HSTRING::from(file),
            index,
            0,
            Some(&mut h),
            None,
            size & 0xFFFF,
        )
    };
    if hr.0 != 0 || h.is_invalid() {
        return None;
    }
    let px = unsafe { icon_rgba(h) };
    unsafe {
        let _ = DestroyIcon(h);
    }
    let (w, hgt, rgba) = px?;
    encode_png(w, hgt, &rgba)
}

#[cfg(not(windows))]
pub fn extract_png(_file: &str, _index: i32, _size: u32) -> Option<Vec<u8>> {
    None
}

/// HICON → (genişlik, yükseklik, RGBA). Alfa kanalı olmayan eski simgelerde maske kullanılır.
#[cfg(windows)]
unsafe fn icon_rgba(
    h: windows::Win32::UI::WindowsAndMessaging::HICON,
) -> Option<(u32, u32, Vec<u8>)> {
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    let mut ii = ICONINFO::default();
    unsafe { GetIconInfo(h, &mut ii) }.ok()?;
    let free = |b: HBITMAP| {
        if !b.is_invalid() {
            let _ = unsafe { DeleteObject(HGDIOBJ(b.0)) };
        }
    };
    let (color, mask) = (ii.hbmColor, ii.hbmMask);
    let result = (|| {
        if color.is_invalid() {
            return None; // tek renkli (çok eski) simge
        }
        let mut bm = BITMAP::default();
        let n = unsafe {
            GetObjectW(
                HGDIOBJ(color.0),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bm as *mut _ as *mut _),
            )
        };
        let (w, hgt) = (bm.bmWidth, bm.bmHeight);
        if n == 0 || w <= 0 || hgt <= 0 || w > 512 || hgt > 512 {
            return None;
        }
        let dc = unsafe { CreateCompatibleDC(None) };
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -hgt, // yukarıdan aşağı
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let len = (w * hgt * 4) as usize;
        let mut px = vec![0u8; len];
        let got = unsafe {
            GetDIBits(
                dc,
                color,
                0,
                hgt as u32,
                Some(px.as_mut_ptr() as *mut _),
                &mut bi,
                DIB_RGB_COLORS,
            )
        };
        if got == 0 {
            unsafe {
                let _ = DeleteDC(dc);
            }
            return None;
        }
        if px.chunks_exact(4).all(|p| p[3] == 0) {
            // Alfa yok: maskede siyah = görünür.
            let mut m = vec![0u8; len];
            let ok = unsafe {
                GetDIBits(
                    dc,
                    mask,
                    0,
                    hgt as u32,
                    Some(m.as_mut_ptr() as *mut _),
                    &mut bi,
                    DIB_RGB_COLORS,
                )
            };
            for (p, q) in px.chunks_exact_mut(4).zip(m.chunks_exact(4)) {
                p[3] = if ok == 0 || q[0] == 0 { 255 } else { 0 };
            }
        }
        unsafe {
            let _ = DeleteDC(dc);
        }
        for p in px.chunks_exact_mut(4) {
            p.swap(0, 2); // BGRA → RGBA
        }
        Some((w as u32, hgt as u32, px))
    })();
    free(color);
    free(mask);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_values() {
        assert_eq!(
            split_icon(r#""C:\Program Files\App\app.exe",-101"#),
            (r"C:\Program Files\App\app.exe".to_string(), -101)
        );
        assert_eq!(
            split_icon(r"C:\App\app.exe,0"),
            (r"C:\App\app.exe".to_string(), 0)
        );
        assert_eq!(
            split_icon(r"C:\App\app.ico"),
            (r"C:\App\app.ico".to_string(), 0)
        );
        // Virgüllü klasör adı dizin sanılmaz.
        assert_eq!(
            split_icon(r"C:\A, B\x.exe"),
            (r"C:\A, B\x.exe".to_string(), 0)
        );
        std::env::set_var("GMID_ICON_TEST", r"C:\W");
        assert_eq!(
            expand_env(r"%GMID_ICON_TEST%\x.exe,%YOK_BOYLE%"),
            r"C:\W\x.exe,%YOK_BOYLE%"
        );
        assert_eq!(expand_env("100%"), "100%");
    }

    #[test]
    fn msi_packed_code() {
        assert_eq!(
            packed_guid("{8BDFE669-9705-4184-9368-DB9CE581E0E7}").as_deref(),
            Some("966EFDB8507948143986BDC95E180E7E")
        );
        assert_eq!(packed_guid("Steam App 730"), None);
        assert_eq!(packed_guid("8BDFE669-9705-4184-9368-DB9CE581E0E7"), None);
    }

    #[test]
    fn png_roundtrip() {
        let rgba = [255u8, 0, 0, 255].repeat(4);
        let png = encode_png(2, 2, &rgba).unwrap();
        assert_eq!(&png[1..4], b"PNG");
    }

    #[test]
    fn picks_program_exe() {
        let d = std::env::temp_dir().join(format!("gmid-icon-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        for f in [
            "unins000.exe",
            "updater.exe",
            "MyGame.exe",
            "tool.exe",
            "readme.txt",
        ] {
            std::fs::write(d.join(f), b"x").unwrap();
        }
        let got = best_exe(&d, "My Game").unwrap();
        assert!(got.ends_with("MyGame.exe"), "{got}");
        std::fs::remove_dir_all(&d).unwrap();
    }
}
