//! Gece yedeği: veritabanının (gönderiler, beğeniler, yorumlar, arkadaşlıklar, profiller, mesajlar) tam bir
//! kopyası her gün `yedek/gmid-YYYY-AA-GG.db` olarak alınır; en yeni 7 gün saklanır. SQLite'ın `VACUUM INTO`
//! komutu sunucu çalışırken tutarlı bir kopya yazar. Resimler ve klipler yedeklenmez (klipler zaten 7 günlük).
//!
//! Geri yüklemek için: `systemctl stop gmid-sunucu`, yedeği `gmid.db` yerine kopyala, `systemctl start gmid-sunucu`.

use crate::Sunucu;
use std::path::{Path, PathBuf};

/// Kaç günlük yedek saklanır.
pub const SAKLA: usize = 7;

/// Gün sayısından (1970-01-01'den beri) takvim tarihi (Howard Hinnant'ın algoritması).
pub fn tarih(gun: i64) -> (i64, u32, u32) {
    let z = gun + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

pub fn klasor(s: &Sunucu) -> PathBuf {
    s.ayar
        .veritabani
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("yedek")
}

/// Bugünün yedeği yoksa alır, eskileri siler. Dönen: alınan yedeğin yolu (bugün zaten varsa `None`).
pub fn al(s: &Sunucu, simdi_ms: i64) -> Result<Option<PathBuf>, String> {
    if s.ayar.veritabani.as_os_str() == ":memory:" {
        return Ok(None);
    }
    let k = klasor(s);
    std::fs::create_dir_all(&k).map_err(|e| e.to_string())?;
    let (y, m, d) = tarih(simdi_ms.div_euclid(86_400_000));
    let yol = k.join(format!("gmid-{y:04}-{m:02}-{d:02}.db"));
    let yeni = if yol.exists() {
        None
    } else {
        let gecici = k.join(format!("gmid-{y:04}-{m:02}-{d:02}.db.yeni"));
        let _ = std::fs::remove_file(&gecici);
        s.db()
            .yedekle(&gecici)
            .map_err(|e| format!("yedek alınamadı: {e}"))?;
        std::fs::rename(&gecici, &yol).map_err(|e| e.to_string())?;
        Some(yol)
    };
    temizle(&k);
    Ok(yeni)
}

/// En yeni `SAKLA` yedek kalır (adlar tarih sırasında).
fn temizle(k: &Path) {
    let Ok(d) = std::fs::read_dir(k) else {
        return;
    };
    let mut l: Vec<PathBuf> = d
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("gmid-") && n.ends_with(".db"))
        })
        .collect();
    l.sort();
    let fazla = l.len().saturating_sub(SAKLA);
    for p in &l[..fazla] {
        let _ = std::fs::remove_file(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takvim() {
        assert_eq!(tarih(0), (1970, 1, 1));
        assert_eq!(tarih(19_723), (2024, 1, 1));
        assert_eq!(tarih(20_759), (2026, 11, 2));
        assert_eq!(tarih(19_782), (2024, 2, 29));
    }

    #[test]
    fn yedi_gun_kalir() {
        let k = std::env::temp_dir().join(format!("gmid-yedek-{}", std::process::id()));
        std::fs::create_dir_all(&k).unwrap();
        for g in 1..=10 {
            std::fs::write(k.join(format!("gmid-2026-11-{g:02}.db")), b"x").unwrap();
        }
        std::fs::write(k.join("baska.txt"), b"x").unwrap();
        temizle(&k);
        let mut kalan: Vec<String> = std::fs::read_dir(&k)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        kalan.sort();
        assert_eq!(kalan.len(), SAKLA + 1);
        assert_eq!(kalan[0], "baska.txt");
        assert_eq!(kalan[1], "gmid-2026-11-04.db");
        let _ = std::fs::remove_dir_all(&k);
    }
}
