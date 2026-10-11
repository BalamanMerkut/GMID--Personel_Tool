//! Tarayıcı eklentisinin dosyaları uygulamanın içinde gelir; "Eklentiyi kur" denince
//! `GMID Tool\Eklenti` klasörüne yazılır (Chrome/Opera'da "Paketlenmemiş öğe yükle" ile bu klasör seçilir).
//! Uygulama güncellenince dosyalar da güncellenir; tarayıcıda eklentiyi bir kez yenilemek yeterli.

use std::path::Path;

macro_rules! files {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("../../../eklenti/", $name)) as &[u8])),*]
    };
}

const FILES: &[(&str, &[u8])] = files![
    "manifest.json",
    "background.js",
    "content.js",
    "sniff.js",
    "relay.js",
    "popup.html",
    "popup.css",
    "popup.js",
    "OKU-BENI.txt",
    "icons/16.png",
    "icons/32.png",
    "icons/48.png",
    "icons/128.png",
];

/// Dosyaları yazar (değişmeyenlere dokunmaz) ve klasörün yolunu döner.
pub fn install(dir: &Path) -> Result<String, String> {
    for (name, bytes) in FILES {
        let path = dir.join(name);
        if std::fs::read(&path).is_ok_and(|old| old == *bytes) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, bytes).map_err(|e| format!("Eklenti yazılamadı: {e}"))?;
    }
    Ok(dir.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    #[test]
    fn manifest_is_valid_json_with_matching_version() {
        let (_, bytes) = super::FILES[0];
        let m: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(m["manifest_version"], 3);
        // Eklenti köprüye (127.0.0.1) erişebilmeli ve portlar uygulamanınkilerle aynı olmalı.
        let hosts = m["host_permissions"].to_string();
        assert!(hosts.contains("<all_urls>") || hosts.contains("127.0.0.1"));
        let (_, bg) = super::FILES[1];
        let bg = String::from_utf8_lossy(bg);
        for p in super::super::bridge::PORTS {
            assert!(bg.contains(&p.to_string()), "{p}");
        }
    }
}
