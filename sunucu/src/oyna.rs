//! "Sunucuya girdi" bildirimi: biri GMID Tool'dan bir GMID sunucusuna Oyna dediğinde arkadaşlarına
//! "X, GMID · CS 1.6'ya girdi" bildirimi gider (aynı kişi için en çok 10 dakikada bir).

use crate::api::{hata, kim, Sonuc};
use crate::db::simdi;
use crate::Sunucu;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

const ARALIK_MS: i64 = 10 * 60 * 1000;

fn son_bildirim() -> &'static Mutex<HashMap<String, i64>> {
    static S: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
    S.get_or_init(Default::default)
}

/// Bu kişi için şimdi bildirim gönderilebilir mi (gönderilecekse zamanı yazar).
fn sira_geldi(kisi: &str, t: i64) -> bool {
    let mut m = son_bildirim().lock().unwrap_or_else(|e| e.into_inner());
    m.retain(|_, z| t - *z < ARALIK_MS);
    if m.contains_key(kisi) {
        return false;
    }
    m.insert(kisi.to_string(), t);
    true
}

#[derive(Deserialize)]
pub struct Istek {
    sunucu: String,
    #[serde(default)]
    adres: String,
}

/// POST /api/sunucuya-girdi {sunucu, adres}
pub async fn girdi(State(s): State<Arc<Sunucu>>, h: HeaderMap, Json(i): Json<Istek>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let ad: String = i.sunucu.trim().chars().take(80).collect();
    if ad.is_empty() {
        return Err(hata(StatusCode::BAD_REQUEST, "Sunucu adı yok"));
    }
    let adres: String = i.adres.trim().chars().take(120).collect();
    if !sira_geldi(&ben.id, simdi()) {
        return Ok(Json(json!({ "gonderildi": 0 })));
    }
    let (ark, _, _) = s
        .db()
        .arkadaslar(&ben.id)
        .map_err(|e| hata(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let v = json!({ "tip": "bildirim", "tur": "sunucu", "kisi": ben, "metin": ad, "adres": adres });
    for a in &ark {
        s.durum.kisiye(&a.id, &v);
    }
    Ok(Json(json!({ "gonderildi": ark.len() })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_dakikada_bir() {
        let t = 1_000_000_000;
        assert!(sira_geldi("x-test", t));
        assert!(!sira_geldi("x-test", t + 60_000));
        assert!(sira_geldi("y-test", t + 60_000));
        assert!(sira_geldi("x-test", t + ARALIK_MS + 1));
    }
}
