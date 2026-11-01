//! Paylaşılan klipler: GMID Tool klibi küçültüp (H.264 MP4, en çok ~50 MB) buraya yükler, akışa "klip"
//! olayı düşer. 7 gün sonra video da paylaşım da silinir; aldığı beğeni ve yorumlar profilde kalır.
//! Videolar veritabanının yanındaki `klip/` klasöründe, adları içeriğin SHA-256 özeti (tahmin edilemez).

use crate::api::{hata, kim, Sonuc};
use crate::sosyal::KLIP_OMRU_MS;
use crate::Sunucu;
use axum::body::to_bytes;
use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;

/// Tek klip en çok bu kadar olabilir (GMID Tool 50 MB'a sığdırır).
pub const KLIP_SINIRI: usize = 60 * 1024 * 1024;
/// Bir kişinin aynı anda yayında en çok bu kadar klibi olabilir.
const KISI_BASI: i64 = 10;
/// Bütün kliplerin toplamı bunu geçemez (sunucuda başka işler de var).
const TOPLAM_SINIR: u64 = 5 * 1024 * 1024 * 1024;

#[derive(Deserialize)]
pub struct KlipSorgu {
    /// Klibin adı (dosya adı ya da kullanıcının verdiği ad).
    ad: Option<String>,
    oyun: Option<String>,
    /// Süre (saniye).
    sure: Option<f64>,
    /// Önceden /api/resim ile yüklenmiş kapak ("/resim/…").
    kapak: Option<String>,
    /// Paylaşanın yazdığı kısa açıklama.
    not: Option<String>,
}

fn kisalt(s: Option<String>, n: usize) -> Option<String> {
    let s = s?.trim().to_string();
    (!s.is_empty()).then(|| s.chars().take(n).collect())
}

/// MP4/MOV mu (4. bayttan sonra "ftyp")?
pub fn video_mu(b: &[u8]) -> bool {
    b.len() > 12 && &b[4..8] == b"ftyp"
}

fn klasor_boyutu(k: &Path) -> u64 {
    std::fs::read_dir(k)
        .map(|d| {
            d.filter_map(Result::ok)
                .filter_map(|e| e.metadata().ok())
                .filter(|m| m.is_file())
                .map(|m| m.len())
                .sum()
        })
        .unwrap_or(0)
}

/// POST /api/klip?ad=…&oyun=…&sure=…&kapak=… (gövde: MP4)
pub async fn yukle(
    State(s): State<Arc<Sunucu>>,
    Query(q): Query<KlipSorgu>,
    istek: Request,
) -> Sonuc {
    // Önce giriş: girişsiz istek 60 MB'lık gövde belleğe okunmadan geri çevrilir.
    let ben = kim(&s, istek.headers())?;
    let govde = to_bytes(istek.into_body(), KLIP_SINIRI)
        .await
        .map_err(|_| {
            hata(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Klip çok büyük (en çok 60 MB)",
            )
        })?;
    if !video_mu(&govde) {
        return Err(hata(StatusCode::BAD_REQUEST, "Video değil (MP4 olmalı)"));
    }
    // Sınır denetiminden paylaşım kaydına kadar tek yükleme: aynı anda gelenler sınırı aşamasın.
    let _sira = s.klip_kilidi.lock().await;
    if s.db().klip_sayisi(&ben.id) >= KISI_BASI {
        return Err(hata(
            StatusCode::TOO_MANY_REQUESTS,
            format!("Aynı anda en çok {KISI_BASI} klip paylaşılabilir; eskileri 7 günde silinir"),
        ));
    }
    let ozet: String = Sha256::digest(&govde)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let ad = format!("{ozet}.mp4");
    let klasor = s.klip_klasoru();
    let yol = klasor.join(&ad);
    if !yol.exists() {
        if klasor_boyutu(&klasor) + govde.len() as u64 > TOPLAM_SINIR {
            return Err(hata(
                StatusCode::INSUFFICIENT_STORAGE,
                "Sunucuda klip yeri doldu; birkaç gün sonra yeniden dene",
            ));
        }
        let gecici = klasor.join(format!("{ad}.yeni"));
        let govde2 = govde.clone();
        let (k2, g2, y2) = (klasor.clone(), gecici.clone(), yol.clone());
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            std::fs::create_dir_all(&k2)?;
            std::fs::write(&g2, &govde2)?;
            std::fs::rename(&g2, &y2)
        })
        .await
        .map_err(|e| hata(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| hata(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }
    let kapak = kisalt(q.kapak, 200).filter(|k| k.starts_with("/resim/"));
    let simdi = crate::db::simdi();
    let veri = json!({
        "ad": kisalt(q.ad, 120).unwrap_or_else(|| "Klip".into()),
        "oyun": kisalt(q.oyun, 120),
        "not": kisalt(q.not, 300),
        "sure": q.sure.filter(|x| x.is_finite() && *x > 0.0).map(|x| x.min(36000.0)),
        "kapak": kapak,
        "url": format!("/klip/{ad}"),
        "boyut": govde.len(),
        "bitis": simdi + KLIP_OMRU_MS,
    });
    let id = s
        .db()
        .klip_paylas(&ben.id, &ozet, &veri)
        .map_err(|e| hata(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    s.durum.yay(json!({ "tip": "akis" }));
    Ok(Json(
        json!({ "id": id, "url": veri["url"], "bitis": veri["bitis"] }),
    ))
}

/// Video dosyası artık hiçbir paylaşımda yoksa siler.
pub fn dosyayi_sil(s: &Sunucu, ozet: &str) {
    if ozet.len() != 64 || !ozet.bytes().all(|c| c.is_ascii_hexdigit()) {
        return;
    }
    if !s.db().klip_kullaniliyor(ozet) {
        let _ = std::fs::remove_file(s.klip_klasoru().join(format!("{ozet}.mp4")));
    }
}

/// Süresi dolan klipleri ve hiçbir paylaşıma bağlı olmayan artık dosyaları siler. Dönen: silinen paylaşım sayısı.
pub fn temizle(s: &Sunucu) -> usize {
    let ozetler = s
        .db()
        .eski_klipleri_sil(crate::db::simdi())
        .unwrap_or_default();
    for o in &ozetler {
        dosyayi_sil(s, o);
    }
    // Yarım kalan yüklemeler ve paylaşımı silinmiş videolar (bir saatten eskiyse).
    if let Ok(d) = std::fs::read_dir(s.klip_klasoru()) {
        for e in d.filter_map(Result::ok) {
            let eski = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|x| x.as_secs() > 3600);
            if !eski {
                continue;
            }
            let ad = e.file_name().to_string_lossy().into_owned();
            match ad.strip_suffix(".mp4") {
                Some(o) => dosyayi_sil(s, o),
                None if ad.ends_with(".yeni") => {
                    let _ = std::fs::remove_file(e.path());
                }
                None => {}
            }
        }
    }
    ozetler.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_turu() {
        assert!(video_mu(b"\0\0\0\x20ftypisom\0\0\x02\0"));
        assert!(!video_mu(b"<html><body>"));
        assert!(!video_mu(b"\0\0\0"));
    }
}
