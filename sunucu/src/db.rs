//! Kalıcı veri (SQLite): kullanıcılar ve oturumlar. Oturum anahtarının kendisi saklanmaz, özeti (SHA-256)
//! saklanır: veritabanı ele geçse bile anahtarlarla giriş yapılamaz.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Bu kadar kullanılmayan oturum geçersiz olur.
const OTURUM_OMRU_MS: i64 = 90 * 24 * 3600 * 1000;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Kullanici {
    /// Discord kullanıcı kimliği.
    pub id: String,
    pub ad: String,
    pub avatar: Option<String>,
}

pub struct Db(pub(crate) Connection);

pub fn simdi() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn ozet(anahtar: &str) -> String {
    Sha256::digest(anahtar.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn yeni_anahtar() -> Result<String, String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| e.to_string())?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

impl Db {
    pub fn ac(yol: &Path) -> rusqlite::Result<Db> {
        Self::kur(Connection::open(yol)?)
    }

    #[cfg(test)]
    pub fn bellekte() -> Db {
        Self::kur(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn kur(c: Connection) -> rusqlite::Result<Db> {
        c.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS kullanicilar (
               id TEXT PRIMARY KEY, ad TEXT NOT NULL, avatar TEXT,
               ilk INTEGER NOT NULL, son INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS oturumlar (
               ozet TEXT PRIMARY KEY,
               kullanici TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
               olusturma INTEGER NOT NULL, son INTEGER NOT NULL);",
        )?;
        crate::sosyal::kur(&c)?;
        crate::gece::kur(&c)?;
        crate::topluluk::kur(&c)?;
        crate::hafta::kur(&c)?;
        Ok(Db(c))
    }

    /// Discord'dan gelen kullanıcıyı kaydeder (ad/avatar güncellenir) ve yeni bir oturum açar.
    pub fn giris(&self, k: &Kullanici) -> Result<String, String> {
        let t = simdi();
        self.0
            .execute(
                "INSERT INTO kullanicilar (id, ad, avatar, ilk, son) VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT(id) DO UPDATE SET ad = ?2, avatar = ?3, son = ?4",
                params![k.id, k.ad, k.avatar, t],
            )
            .map_err(|e| e.to_string())?;
        let anahtar = yeni_anahtar()?;
        self.0
            .execute(
                "INSERT INTO oturumlar (ozet, kullanici, olusturma, son) VALUES (?1, ?2, ?3, ?3)",
                params![ozet(&anahtar), k.id, t],
            )
            .map_err(|e| e.to_string())?;
        Ok(anahtar)
    }

    /// Anahtar geçerliyse sahibi; kullanıldıkça ömrü uzar.
    pub fn oturum(&self, anahtar: &str) -> Option<Kullanici> {
        let t = simdi();
        let o = ozet(anahtar);
        let k = self
            .0
            .query_row(
                "SELECT k.id, k.ad, k.avatar FROM oturumlar o JOIN kullanicilar k ON k.id = o.kullanici
                 WHERE o.ozet = ?1 AND o.son > ?2",
                params![o, t - OTURUM_OMRU_MS],
                |r| {
                    Ok(Kullanici {
                        id: r.get(0)?,
                        ad: r.get(1)?,
                        avatar: r.get(2)?,
                    })
                },
            )
            .optional()
            .ok()
            .flatten()?;
        let _ = self.0.execute(
            "UPDATE oturumlar SET son = ?1 WHERE ozet = ?2",
            params![t, o],
        );
        Some(k)
    }

    pub fn cikis(&self, anahtar: &str) {
        let _ = self.0.execute(
            "DELETE FROM oturumlar WHERE ozet = ?1",
            params![ozet(anahtar)],
        );
    }

    /// Süresi dolmuş oturumları siler (günde bir).
    /// Veritabanının tutarlı bir kopyasını yazar (sunucu çalışırken de güvenli).
    pub fn yedekle(&self, hedef: &std::path::Path) -> rusqlite::Result<()> {
        self.0
            .execute("VACUUM INTO ?1", [hedef.to_string_lossy().as_ref()])
            .map(|_| ())
    }

    pub fn temizle(&self) -> usize {
        self.0
            .execute(
                "DELETE FROM oturumlar WHERE son <= ?1",
                params![simdi() - OTURUM_OMRU_MS],
            )
            .unwrap_or(0)
    }

    #[cfg(test)]
    fn eskit(&self) {
        self.0.execute("UPDATE oturumlar SET son = 0", []).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ali() -> Kullanici {
        Kullanici {
            id: "1".into(),
            ad: "Ali".into(),
            avatar: None,
        }
    }

    #[test]
    fn oturum_acilir_bulunur_kapanir() {
        let db = Db::bellekte();
        let a = db.giris(&ali()).unwrap();
        assert_eq!(a.len(), 64);
        assert_eq!(db.oturum(&a), Some(ali()));
        assert_eq!(db.oturum("yanlis"), None);
        // Ad değişince güncellenir, eski oturum geçerli kalır.
        let mut yeni = ali();
        yeni.ad = "Ali Veli".into();
        let b = db.giris(&yeni).unwrap();
        assert_ne!(a, b);
        assert_eq!(db.oturum(&a).unwrap().ad, "Ali Veli");
        db.cikis(&a);
        assert_eq!(db.oturum(&a), None);
        assert!(db.oturum(&b).is_some());
    }

    #[test]
    fn eski_oturum_gecersiz() {
        let db = Db::bellekte();
        let a = db.giris(&ali()).unwrap();
        db.eskit();
        assert_eq!(db.oturum(&a), None);
        assert_eq!(db.temizle(), 1);
    }

    #[test]
    fn anahtar_saklanmaz() {
        let db = Db::bellekte();
        let a = db.giris(&ali()).unwrap();
        let saklanan: String =
            db.0.query_row("SELECT ozet FROM oturumlar", [], |r| r.get(0))
                .unwrap();
        assert_ne!(saklanan, a);
        assert_eq!(saklanan, ozet(&a));
    }
}
