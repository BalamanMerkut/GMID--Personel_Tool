//! "Sorun bildir": tek bir zip — kullanıcının açıklaması, sürüm, Windows bilgisi ve günlük dosyaları. Kişisel klasör
//! yolları (kullanıcı adı) gizlenir. Zip `Raporlar\` klasörüne yazılır; arayüz klasörü açar, özeti panoya kopyalar ve
//! GMID Discord'unu açar.

use std::io::Write;
use std::path::Path;

/// Sistem bilgisi satırları.
pub fn system_info() -> Vec<(String, String)> {
    let mut v = vec![
        (
            "GMID Tool".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        ),
        (
            "Derleme".to_string(),
            crate::updater::current_build()
                .map(|b| b.to_string())
                .unwrap_or_else(|| "geliştirme".into()),
        ),
    ];
    #[cfg(windows)]
    {
        let key = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
        let name = reg_string(key, "ProductName").unwrap_or_default();
        let ver = reg_string(key, "DisplayVersion").unwrap_or_default();
        let build = reg_string(key, "CurrentBuildNumber").unwrap_or_default();
        // Windows 11 de kayıtta "Windows 10" yazar; derleme 22000+ ise 11'dir.
        let name = if build.parse::<u32>().is_ok_and(|b| b >= 22000) {
            name.replace("Windows 10", "Windows 11")
        } else {
            name
        };
        v.push((
            "Windows".into(),
            format!("{name} {ver} (derleme {build})").trim().to_string(),
        ));
    }
    #[cfg(not(windows))]
    v.push(("Sistem".into(), std::env::consts::OS.to_string()));
    v.push((
        "Saat".into(),
        chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S %z")
            .to_string(),
    ));
    v
}

#[cfg(windows)]
fn reg_string(key: &str, value: &str) -> Option<String> {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    let mut buf = [0u16; 256];
    let mut size = (buf.len() * 2) as u32;
    let r = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            &HSTRING::from(key),
            &HSTRING::from(value),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };
    if r.is_err() {
        return None;
    }
    let len = (size as usize / 2).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// Kullanıcı klasörü yolunu gizler: "C:\Users\Ali\…" → "%USERPROFILE%\…" (büyük/küçük harf fark etmez).
pub fn redact(text: &str, home: &str) -> String {
    if home.len() < 4 {
        return text.to_string();
    }
    let lower = text.to_lowercase();
    let needle = home.to_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while let Some(pos) = lower[i..].find(&needle) {
        out.push_str(&text[i..i + pos]);
        out.push_str("%USERPROFILE%");
        i += pos + needle.len();
    }
    out.push_str(&text[i..]);
    out
}

/// Zip'i oluşturur, yolunu ve panoya konacak kısa özeti döner.
pub fn create(
    root: &Path,
    home: &str,
    description: &str,
    with_logs: bool,
) -> Result<(String, String), String> {
    let dir = root.join("Raporlar");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let path = dir.join(format!("GMID-Sorun-{stamp}.zip"));
    let info = system_info();
    let desc = description.trim();

    let mut text = String::from("GMID Tool · Sorun raporu\r\n========================\r\n\r\n");
    for (k, v) in &info {
        text += &format!("{k}: {v}\r\n");
    }
    text += "\r\nNe oldu?\r\n--------\r\n";
    text += if desc.is_empty() {
        "(açıklama yazılmadı)"
    } else {
        desc
    };
    text += "\r\n";

    let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default();
    z.start_file("rapor.txt", opts).map_err(|e| e.to_string())?;
    z.write_all(redact(&text, home).as_bytes())
        .map_err(|e| e.to_string())?;
    if with_logs {
        for name in ["gmid.log", "gmid.old.log"] {
            if let Ok(log) = std::fs::read_to_string(root.join(name)) {
                // Son ~2000 satır yeter.
                let lines: Vec<&str> = log.lines().collect();
                let tail = lines[lines.len().saturating_sub(2000)..].join("\r\n");
                z.start_file(name, opts).map_err(|e| e.to_string())?;
                z.write_all(redact(&tail, home).as_bytes())
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    z.finish().map_err(|e| e.to_string())?;

    let first = desc
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(200)
        .collect::<String>();
    let version = info
        .iter()
        .filter(|(k, _)| k == "GMID Tool" || k == "Derleme" || k == "Windows")
        .map(|(k, v)| format!("{k} {v}"))
        .collect::<Vec<_>>()
        .join(" · ");
    let summary = format!(
        "🐞 GMID Tool sorunu: {}\n{version}\n(rapor dosyası ekte: {})",
        if first.is_empty() {
            "açıklama yok"
        } else {
            &first
        },
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    );
    Ok((path.to_string_lossy().into_owned(), redact(&summary, home)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn home_is_redacted() {
        assert_eq!(
            redact(
                r"Açılamadı: C:\Users\Ali\Documents\GMID Tool\a.mp4 ve c:\users\ali\x",
                r"C:\Users\Ali"
            ),
            r"Açılamadı: %USERPROFILE%\Documents\GMID Tool\a.mp4 ve %USERPROFILE%\x"
        );
        assert_eq!(redact("yol yok", ""), "yol yok");
    }

    #[test]
    fn report_zip_has_text_and_logs() {
        let root = std::env::temp_dir().join(format!("gmid-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("gmid.log"),
            "2026-10-12 12:00:00 Hata: /home/ali/x açılamadı\n",
        )
        .unwrap();
        let (path, summary) =
            create(&root, "/home/ali", "Panel açılmıyor\nikinci satır", true).unwrap();
        assert!(summary.contains("Panel açılmıyor"));
        assert!(!summary.contains("ikinci satır"));
        let mut z = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let mut log = String::new();
        z.by_name("gmid.log")
            .unwrap()
            .read_to_string(&mut log)
            .unwrap();
        assert!(log.contains("%USERPROFILE%/x") && !log.contains("/home/ali"));
        let mut txt = String::new();
        z.by_name("rapor.txt")
            .unwrap()
            .read_to_string(&mut txt)
            .unwrap();
        assert!(txt.contains("GMID Tool:") && txt.contains("ikinci satır"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
