//! Sosyal HTTP uçları: kişiler, profiller, akış (beğeni/yorum), özel mesajlar, resimler.
//! Hepsi giriş ister (`Authorization: Bearer …`), resimlerin kendisi hariç (adları tahmin edilemez özet).

use crate::api::{hata, kim, Sonuc};
use crate::sosyal::{metin, Istek, Profil};
use crate::Sunucu;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

fn ic(e: String) -> (StatusCode, Json<serde_json::Value>) {
    hata(StatusCode::INTERNAL_SERVER_ERROR, e)
}

pub async fn kisiler(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> Sonuc {
    let ben = kim(&s, &h)?;
    let db = s.db();
    let l = db.kisiler().map_err(ic)?;
    let l: Vec<_> = l
        .into_iter()
        .map(|(k, son)| {
            let c = s.durum.kisi(&k.id);
            let iliski = db.iliski(&ben.id, &k.id);
            json!({ "id": k.id, "ad": k.ad, "avatar": k.avatar, "son": son, "iliski": iliski,
                    "cevrimici": c.is_some(), "oyun": c.and_then(|c| c.oyun) })
        })
        .collect();
    Ok(Json(json!({ "kisiler": l })))
}

pub async fn profil(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(id): Path<String>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let db = s.db();
    let k = db
        .kullanici(&id)
        .ok_or_else(|| hata(StatusCode::NOT_FOUND, "Böyle biri yok"))?;
    let (p, t) = db.profil(&id).unzip();
    let c = s.durum.kisi(&id);
    // Başkalarından alınan beğeni ve yorumlar (süresi dolup silinen kliplerinki dahil).
    let puan = db.toplam_puan(&id);
    Ok(Json(json!({
        "kisi": k, "cevrimici": c.is_some(), "oyun": c.and_then(|c| c.oyun),
        "profil": p, "guncelleme": t,
        "iliski": db.iliski(&ben.id, &id), "arkadas": db.arkadas_sayisi(&id),
        "begeni": puan.0, "yorum": puan.1,
        "rozetler": db.rozetler(&id),
        "gorunum": db.gorunum(&id),
    })))
}

pub async fn profil_kaydet(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Json(p): Json<Profil>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let n = s.db().profil_kaydet(&ben.id, p.sinirla()).map_err(ic)?;
    if n > 0 {
        s.durum.yay(json!({ "tip": "akis" }));
    }
    Ok(Json(json!({ "akis": n })))
}

#[derive(Deserialize)]
pub struct AkisSorgu {
    once: Option<i64>,
    kisi: Option<String>,
    sayi: Option<i64>,
    /// 1: yalnız arkadaşlar
    arkadaslar: Option<u8>,
    /// "#etiket" ya da oyun/film/dizi adı.
    konu: Option<String>,
    /// Yalnız bu türden (profil sekmeleri).
    tur: Option<String>,
    /// Yalnız bu kişinin beğendikleri.
    begenen: Option<String>,
}

#[derive(Deserialize)]
pub struct GundemSorgu {
    arkadaslar: Option<u8>,
}

/// Gündem: son 7 günde en çok geçen oyun/film/dizi ve #etiketler (beğeniler de sayılır).
pub async fn gundem(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Query(q): Query<GundemSorgu>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let l = s
        .db()
        .gundem(&ben.id, q.arkadaslar == Some(1), 6)
        .map_err(ic)?;
    Ok(Json(json!({ "gundem": l })))
}

pub async fn akis(State(s): State<Arc<Sunucu>>, h: HeaderMap, Query(q): Query<AkisSorgu>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let konu: Option<String> = q.konu.map(|k| k.chars().take(120).collect());
    let f = crate::sosyal::Suzgec {
        once: q.once,
        kisi: q.kisi.as_deref(),
        arkadaslar: q.arkadaslar == Some(1),
        konu: konu.as_deref(),
        tur: q.tur.as_deref(),
        begenen: q.begenen.as_deref(),
        sayi: q.sayi.unwrap_or(30),
    };
    let mut l = s.db().akis_suzgec(&ben.id, &f).map_err(ic)?;
    s.db().anketleri_doldur(&ben.id, &mut l).map_err(ic)?;
    Ok(Json(json!({ "olaylar": l })))
}

fn sahip(s: &Sunucu, id: i64) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    s.db()
        .etkinlik_sahibi(id)
        .ok_or_else(|| hata(StatusCode::NOT_FOUND, "Bu paylaşım silinmiş"))
}

pub async fn begen(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(id): Path<i64>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let sahibi = sahip(&s, id)?;
    let (begendim, sayi) = s.db().begen(&ben.id, id).map_err(ic)?;
    if begendim && sahibi != ben.id {
        s.durum.kisiye(
            &sahibi,
            &json!({ "tip": "bildirim", "tur": "begeni", "kisi": ben, "etkinlik": id }),
        );
    }
    s.durum.yay(json!({ "tip": "akis", "id": id }));
    Ok(Json(json!({ "begendim": begendim, "begeni": sayi })))
}

#[derive(Deserialize)]
pub struct MetinIstek {
    metin: String,
}

pub async fn yorum(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<MetinIstek>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let m =
        metin(&i.metin).ok_or_else(|| hata(StatusCode::BAD_REQUEST, "Yorum boş ya da çok uzun"))?;
    let sahibi = sahip(&s, id)?;
    let yid = s.db().yorum_yaz(&ben.id, id, &m).map_err(ic)?;
    if sahibi != ben.id {
        s.durum.kisiye(
            &sahibi,
            &json!({ "tip": "bildirim", "tur": "yorum", "kisi": ben, "etkinlik": id, "metin": m }),
        );
    }
    anilanlara_haber(&s, &ben, &m, id, Some(&sahibi));
    s.durum.yay(json!({ "tip": "akis", "id": id }));
    Ok(Json(json!({ "id": yid })))
}

/// Yazıda @Ad ile anılanlara bildirim (yanıtta paylaşım sahibi zaten "yorum" bildirimi alır).
fn anilanlara_haber(
    s: &Sunucu,
    ben: &crate::db::Kullanici,
    metin: &str,
    etkinlik: i64,
    haric: Option<&str>,
) {
    let anilan = s.db().anilanlar(metin, &ben.id);
    let v = json!({ "tip": "bildirim", "tur": "anma", "kisi": ben, "etkinlik": etkinlik, "metin": metin });
    for k in anilan.iter().filter(|k| Some(k.id.as_str()) != haric) {
        s.durum.kisiye(&k.id, &v);
    }
}

#[derive(Deserialize)]
pub struct GonderiIstek {
    metin: String,
    resim: Option<String>,
    /// Anket seçenekleri (2–4); metin soru olur.
    #[serde(default)]
    anket: Option<Vec<String>>,
    /// Anket süresi (saat, 1–168; yoksa 24).
    #[serde(default)]
    anket_saat: Option<i64>,
}

/// Serbest gönderi: yazı (en çok 500 harf) ve isteğe bağlı resim (önce /api/resim ile yüklenir).
pub async fn gonderi(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Json(g): Json<GonderiIstek>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let m = g.metin.trim();
    let resim = g
        .resim
        .filter(|r| r.starts_with("/resim/") && r.len() <= 200);
    if (m.is_empty() && resim.is_none()) || m.chars().count() > 500 {
        return Err(hata(
            StatusCode::BAD_REQUEST,
            "Gönderi boş ya da çok uzun (en çok 500 harf)",
        ));
    }
    let anket = match &g.anket {
        Some(sec) => {
            if m.is_empty() {
                return Err(hata(StatusCode::BAD_REQUEST, "Anketin sorusunu yaz"));
            }
            Some(
                crate::topluluk::anket_verisi(sec, g.anket_saat)
                    .map_err(|x| hata(StatusCode::BAD_REQUEST, x))?,
            )
        }
        None => None,
    };
    let id = s
        .db()
        .gonderi_yaz(&ben.id, m, resim.as_deref(), anket.as_ref())
        .map_err(ic)?
        .ok_or_else(|| {
            hata(
                StatusCode::TOO_MANY_REQUESTS,
                "Biraz yavaş: 10 dakikada en çok 10 gönderi",
            )
        })?;
    anilanlara_haber(&s, &ben, m, id, None);
    s.durum.yay(json!({ "tip": "akis" }));
    Ok(Json(json!({ "id": id })))
}

#[derive(Deserialize)]
pub struct SikayetIstek {
    neden: Option<String>,
}

/// Şikâyet et: çevrim içi yöneticilere bildirim gider; yönetici akışta şikâyet sayısını görür ve silebilir.
pub async fn sikayet(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<i64>,
    govde: Option<Json<SikayetIstek>>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    sahip(&s, id)?;
    let neden: Option<String> = govde
        .and_then(|Json(g)| g.neden)
        .map(|n| n.trim().chars().take(300).collect::<String>())
        .filter(|n| !n.is_empty());
    let yeni = s
        .db()
        .sikayet_et(&ben.id, id, neden.as_deref())
        .map_err(ic)?;
    if yeni {
        let yoneticiler: Vec<String> = {
            let db = s.db();
            db.kisiler()
                .map_err(ic)?
                .into_iter()
                .map(|(k, _)| k.id)
                .filter(|k| db.yonetici_mi(k, &s.ayar.yoneticiler))
                .collect()
        };
        let v = json!({ "tip": "bildirim", "tur": "sikayet", "kisi": ben, "etkinlik": id, "metin": neden });
        for y in yoneticiler {
            s.durum.kisiye(&y, &v);
        }
    }
    Ok(Json(json!({ "yeni": yeni })))
}

pub async fn yorum_sil(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(id): Path<i64>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let yonetici = s.yonetici_mi(&ben.id);
    if !s.db().yorum_sil(&ben.id, id, yonetici).map_err(ic)? {
        return Err(hata(StatusCode::FORBIDDEN, "Bu yorumu silemezsin"));
    }
    s.durum.yay(json!({ "tip": "akis" }));
    Ok(Json(json!({})))
}

pub async fn etkinlik_sil(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<i64>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    // Yönetici herkesin paylaşımını silebilir (uygunsuz içerik).
    let silinen = if s.yonetici_mi(&ben.id) {
        s.db().etkinlik_sil_yonetici(id).map_err(ic)?
    } else {
        s.db().etkinlik_sil(&ben.id, id).map_err(ic)?
    };
    let Some((tur, veri)) = silinen else {
        return Err(hata(
            StatusCode::FORBIDDEN,
            "Yalnız kendi paylaşımını silebilirsin",
        ));
    };
    // Oyun gecesi kartı silinince gecenin kendisi de iptal olur.
    if tur == "gece" {
        if let Some(gid) = veri["gece"].as_i64() {
            s.db().gece_sil(gid).map_err(ic)?;
            s.durum.yay(json!({ "tip": "gece", "id": gid }));
        }
    }
    if tur == "klip" {
        if let Some(ozet) = veri["url"]
            .as_str()
            .and_then(|u| u.strip_prefix("/klip/"))
            .and_then(|u| u.strip_suffix(".mp4"))
        {
            crate::klip::dosyayi_sil(&s, ozet);
        }
    }
    s.durum.yay(json!({ "tip": "akis" }));
    Ok(Json(json!({})))
}

pub async fn konusmalar(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> Sonuc {
    let ben = kim(&s, &h)?;
    Ok(Json(
        json!({ "konusmalar": s.db().konusmalar(&ben.id).map_err(ic)? }),
    ))
}

#[derive(Deserialize)]
pub struct MesajSorgu {
    once: Option<i64>,
}

pub async fn mesajlar(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<MesajSorgu>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let db = s.db();
    let l = db.mesajlar(&ben.id, &id, q.once, 60).map_err(ic)?;
    let okunan = db.okundu(&ben.id, &id).map_err(ic)?;
    drop(db);
    if okunan > 0 {
        // Karşı taraf "görüldü"yü, benim diğer bilgisayarım okunmamış sayısını güncellesin.
        let v = json!({ "tip": "okundu", "kisi": ben.id, "diger": id });
        s.durum.kisiye(&id, &v);
        s.durum.kisiye(&ben.id, &v);
    }
    Ok(Json(json!({ "mesajlar": l })))
}

pub async fn mesaj_yaz(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(i): Json<MetinIstek>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    if id == ben.id {
        return Err(hata(StatusCode::BAD_REQUEST, "Kendine mesaj atamazsın"));
    }
    let m =
        metin(&i.metin).ok_or_else(|| hata(StatusCode::BAD_REQUEST, "Mesaj boş ya da çok uzun"))?;
    let db = s.db();
    if db.kullanici(&id).is_none() {
        return Err(hata(StatusCode::NOT_FOUND, "Böyle biri yok"));
    }
    if !db.arkadas_mi(&ben.id, &id) {
        return Err(hata(
            StatusCode::FORBIDDEN,
            "Mesaj atmak için önce arkadaş olun",
        ));
    }
    let mesaj = db.mesaj_yaz(&ben.id, &id, &m).map_err(ic)?;
    drop(db);
    let v = json!({ "tip": "mesaj", "mesaj": mesaj, "kisi": ben });
    s.durum.kisiye(&id, &v);
    s.durum.kisiye(&ben.id, &v);
    Ok(Json(json!(mesaj)))
}

pub async fn arkadaslar(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> Sonuc {
    let ben = kim(&s, &h)?;
    let (ark, gelen, giden) = s.db().arkadaslar(&ben.id).map_err(ic)?;
    let ark: Vec<_> = ark
        .into_iter()
        .map(|k| {
            let c = s.durum.kisi(&k.id);
            json!({ "id": k.id, "ad": k.ad, "avatar": k.avatar,
                    "cevrimici": c.is_some(), "oyun": c.and_then(|c| c.oyun) })
        })
        .collect();
    Ok(Json(
        json!({ "arkadaslar": ark, "gelen": gelen, "giden": giden }),
    ))
}

/// Arkadaş ekle: istek gönderir ya da gelen isteği kabul eder.
pub async fn arkadas_ekle(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    if id == ben.id {
        return Err(hata(StatusCode::BAD_REQUEST, "Kendini ekleyemezsin"));
    }
    let db = s.db();
    if db.kullanici(&id).is_none() {
        return Err(hata(StatusCode::NOT_FOUND, "Böyle biri yok"));
    }
    let r = db.arkadas_ekle(&ben.id, &id).map_err(ic)?;
    drop(db);
    let olay = match r {
        Istek::Gonderildi => "istek",
        Istek::Kabul => "kabul",
        Istek::ZatenArkadas => "arkadas",
    };
    if r != Istek::ZatenArkadas {
        s.durum
            .kisiye(&id, &json!({ "tip": "arkadas", "olay": olay, "kisi": ben }));
        s.durum
            .kisiye(&ben.id, &json!({ "tip": "arkadas", "olay": "guncel" }));
    }
    Ok(Json(json!({ "durum": olay })))
}

/// Arkadaşlıktan çıkar, isteği reddet ya da geri al.
pub async fn arkadas_sil(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<String>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    if s.db().arkadas_sil(&ben.id, &id).map_err(ic)? {
        let v = json!({ "tip": "arkadas", "olay": "guncel" });
        s.durum.kisiye(&id, &v);
        s.durum.kisiye(&ben.id, &v);
    }
    Ok(Json(json!({})))
}

/// Kapak/afiş yükleme (en çok 400 KB; JPEG, PNG, WebP, GIF). Aynı resim bir kez saklanır.
pub async fn resim_yukle(State(s): State<Arc<Sunucu>>, h: HeaderMap, govde: Bytes) -> Sonuc {
    kim(&s, &h)?;
    if govde.len() > 400 * 1024 {
        return Err(hata(StatusCode::PAYLOAD_TOO_LARGE, "Resim çok büyük"));
    }
    let uzanti = resim_turu(&govde).ok_or_else(|| hata(StatusCode::BAD_REQUEST, "Resim değil"))?;
    let ozet: String = Sha256::digest(&govde)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let ad = format!("{ozet}.{uzanti}");
    let klasor = s.resim_klasoru();
    let yol = klasor.join(&ad);
    if !yol.exists() {
        std::fs::create_dir_all(&klasor).map_err(|e| ic(e.to_string()))?;
        let gecici = klasor.join(format!("{ad}.yeni"));
        std::fs::write(&gecici, &govde).map_err(|e| ic(e.to_string()))?;
        std::fs::rename(&gecici, &yol).map_err(|e| ic(e.to_string()))?;
    }
    Ok(Json(json!({ "url": format!("/resim/{ad}") })))
}

pub fn resim_turu(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if b.len() > 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("webp")
    } else if b.starts_with(b"GIF8") {
        Some("gif")
    } else {
        None
    }
}

pub async fn resim(State(s): State<Arc<Sunucu>>, Path(ad): Path<String>) -> Response {
    let (ozet, uzanti) = ad.split_once('.').unwrap_or(("", ""));
    let tur = match uzanti {
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    if ozet.len() != 64
        || !ozet
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    match tokio::fs::read(s.resim_klasoru().join(&ad)).await {
        Ok(b) => (
            [
                (header::CONTENT_TYPE, tur),
                (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            b,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resim_turleri() {
        assert_eq!(resim_turu(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(resim_turu(b"\x89PNG\r\n\x1a\n...."), Some("png"));
        assert_eq!(resim_turu(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(resim_turu(b"<svg onload=alert(1)>"), None);
    }
}
