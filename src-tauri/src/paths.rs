//! Uygulamanın kendi ürettiği her şey tek bir klasörde durur: `Belgeler\GMID Tool`.

use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Ana klasörün altında açılışta hazır bulunan alt klasörler.
const SUBFOLDERS: &[&str] = &[
    "Videolar",
    "Notlar",
    "Arkaplanlar",
    "Kapaklar",
    "Kurulum",
    "Yedekler",
    "Araclar",
    "Sunucular",
    "Eklenti",
    "Discord'a Hazır",
    "Ozetler",
    "Raporlar",
    "Karaoke",
];

#[derive(Clone)]
pub struct DataDir(PathBuf);

impl DataDir {
    /// Önce Belgeler; Windows'un Belgeler kaydı bozuk ya da yönlendirilmiş olsa bile
    /// açılabilsin diye sırayla kullanıcı klasörü ve uygulamanın kendi veri klasörü denenir.
    pub fn resolve(app: &AppHandle) -> tauri::Result<Self> {
        let path = app.path();
        let base = path
            .document_dir()
            .or_else(|_| path.home_dir().map(|h| h.join("Documents")))
            .or_else(|_| path.app_local_data_dir())?;
        Ok(Self(base.join("GMID Tool")))
    }

    /// Ana klasörü ve alt klasörleri oluşturur (varsa dokunmaz).
    pub fn ensure(&self) -> std::io::Result<()> {
        for sub in SUBFOLDERS {
            std::fs::create_dir_all(self.0.join(sub))?;
        }
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.0
    }

    pub fn db(&self) -> PathBuf {
        self.0.join("gmid.db")
    }

    pub fn backups(&self) -> PathBuf {
        self.0.join("Yedekler")
    }

    pub fn backgrounds(&self) -> PathBuf {
        self.0.join("Arkaplanlar")
    }

    /// Sunucu logoları (topluluk listesinden indirilen ve elle seçilen).
    pub fn servers(&self) -> PathBuf {
        self.0.join("Sunucular")
    }

    /// Ana klasör ya da bilinen alt klasörlerinden biri; bilinmeyen isimler ana klasöre düşer.
    pub fn folder(&self, sub: Option<&str>) -> PathBuf {
        match sub {
            Some(s) if SUBFOLDERS.contains(&s) => self.0.join(s),
            _ => self.0.clone(),
        }
    }
}
