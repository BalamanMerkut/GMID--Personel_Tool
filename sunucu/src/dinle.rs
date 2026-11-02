//! Birlikte dinleme: biri GMID'in oynatıcısında bir şarkı çalarken "Arkadaşlarla dinle" der; bir dinleme odası
//! açılır. Katılanlar aynı şarkıyı (kaynak bağlantısından, kendi bilgisayarlarında) aynı yerden dinler; sahip
//! sardıkça / duraklattıkça herkes eşitlenir. Odalar bellekte durur; sahip çıkınca oda kapanır.
//!
//! İstemciye giden: `{"tip":"dinle","oda":{…},"simdi":ms}` ya da `{"tip":"dinle","kapandi":"<sahip>"}`.

use crate::api::{hata, kim, Sonuc};
use crate::db::{simdi, Kullanici};
use crate::Sunucu;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Sarki {
    pub baslik: String,
    /// Kaynak sayfa (YouTube vb.); dinleyenin GMID'i sesi buradan alır.
    pub url: String,
    pub kapak: Option<String>,
    pub sure: Option<f64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Oda {
    pub sahip: Kullanici,
    pub sarki: Sarki,
    /// `guncelleme` anındaki konum (saniye).
    pub konum: f64,
    pub caliyor: bool,
    /// Son durum değişikliği (ms, sunucu saati).
    pub guncelleme: i64,
    pub dinleyiciler: Vec<Kullanici>,
}

#[derive(Default)]
pub struct Odalar(Mutex<HashMap<String, Oda>>);

impl Odalar {
    fn kilit(&self) -> std::sync::MutexGuard<'_, HashMap<String, Oda>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn liste(&self) -> Vec<Oda> {
        self.kilit().values().cloned().collect()
    }

    /// Sahip durumu bildirir (oda yoksa açılır; dinleyiciler korunur).
    pub fn guncelle(&self, sahip: &Kullanici, sarki: Sarki, konum: f64, caliyor: bool) -> Oda {
        let mut k = self.kilit();
        let dinleyiciler = k
            .get(&sahip.id)
            .map(|o| o.dinleyiciler.clone())
            .unwrap_or_default();
        let oda = Oda {
            sahip: sahip.clone(),
            sarki,
            konum: konum.max(0.0),
            caliyor,
            guncelleme: simdi(),
            dinleyiciler,
        };
        k.insert(sahip.id.clone(), oda.clone());
        oda
    }

    pub fn kapat(&self, sahip: &str) -> bool {
        self.kilit().remove(sahip).is_some()
    }

    /// Katıl / ayrıl. Kişi aynı anda tek odada dinler. Dönen: değişen odalar.
    pub fn katil(&self, sahip: &str, kisi: &Kullanici, katil: bool) -> Option<Vec<Oda>> {
        let mut k = self.kilit();
        if katil && !k.contains_key(sahip) {
            return None;
        }
        let mut degisen = Vec::new();
        for (id, oda) in k.iter_mut() {
            let vardi = oda.dinleyiciler.iter().any(|d| d.id == kisi.id);
            let olmali = katil && id == sahip && kisi.id != *id;
            if vardi && !olmali {
                oda.dinleyiciler.retain(|d| d.id != kisi.id);
                degisen.push(oda.clone());
            } else if !vardi && olmali {
                oda.dinleyiciler.push(kisi.clone());
                degisen.push(oda.clone());
            }
        }
        Some(degisen)
    }

    /// Kişi tamamen çıktı: odası kapanır, dinlediği odadan ayrılır. Dönen: (kapandı mı, değişen odalar).
    pub fn gitti(&self, id: &str) -> (bool, Vec<Oda>) {
        let kapandi = self.kapat(id);
        let kisi = Kullanici {
            id: id.into(),
            ad: String::new(),
            avatar: None,
        };
        let degisen = self.katil("", &kisi, false).unwrap_or_default();
        (kapandi, degisen)
    }
}

fn yay(s: &Sunucu, oda: &Oda) {
    s.durum
        .yay(json!({ "tip": "dinle", "oda": oda, "simdi": simdi() }));
}

fn yay_kapandi(s: &Sunucu, sahip: &str) {
    s.durum.yay(json!({ "tip": "dinle", "kapandi": sahip }));
}

/// Kişi tamamen çevrim dışı oldu (ws.rs çağırır).
pub fn kisi_gitti(s: &Sunucu, id: &str) {
    let (kapandi, degisen) = s.dinleme.gitti(id);
    if kapandi {
        yay_kapandi(s, id);
    }
    for o in &degisen {
        yay(s, o);
    }
}

pub async fn liste(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> Sonuc {
    kim(&s, &h)?;
    Ok(Json(
        json!({ "odalar": s.dinleme.liste(), "simdi": simdi() }),
    ))
}

#[derive(Deserialize)]
pub struct DurumIstek {
    baslik: String,
    url: String,
    kapak: Option<String>,
    sure: Option<f64>,
    konum: f64,
    caliyor: bool,
}

/// POST /api/dinle: sahibin çaldığı şarkı ve konumu (oda yoksa açılır).
pub async fn durum(State(s): State<Arc<Sunucu>>, h: HeaderMap, Json(i): Json<DurumIstek>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let url = i.url.trim();
    if !(url.starts_with("https://") || url.starts_with("http://")) || url.len() > 500 {
        return Err(hata(
            StatusCode::BAD_REQUEST,
            "Bu parçanın internet kaynağı yok (yalnız indirilen videolar/şarkılar paylaşılabilir)",
        ));
    }
    if !i.konum.is_finite() {
        return Err(hata(StatusCode::BAD_REQUEST, "Geçersiz konum"));
    }
    let sarki = Sarki {
        baslik: i.baslik.trim().chars().take(200).collect(),
        url: url.to_string(),
        kapak: i
            .kapak
            .filter(|k| k.starts_with("https://") && k.len() <= 500),
        sure: i.sure.filter(|x| x.is_finite() && *x > 0.0),
    };
    let oda = s.dinleme.guncelle(&ben, sarki, i.konum, i.caliyor);
    yay(&s, &oda);
    Ok(Json(json!({ "oda": oda })))
}

/// DELETE /api/dinle: odayı kapat.
pub async fn kapat(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> Sonuc {
    let ben = kim(&s, &h)?;
    if s.dinleme.kapat(&ben.id) {
        yay_kapandi(&s, &ben.id);
    }
    Ok(Json(json!({})))
}

/// POST /api/dinle/{sahip}/katil (DELETE: ayrıl)
pub async fn katil(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(sahip): Path<String>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let degisen = s
        .dinleme
        .katil(&sahip, &ben, true)
        .ok_or_else(|| hata(StatusCode::NOT_FOUND, "Bu dinleme odası kapanmış"))?;
    for o in &degisen {
        yay(&s, o);
    }
    let oda = s.dinleme.liste().into_iter().find(|o| o.sahip.id == sahip);
    Ok(Json(json!({ "oda": oda, "simdi": simdi() })))
}

pub async fn ayril(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(_sahip): Path<String>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    for o in s.dinleme.katil("", &ben, false).unwrap_or_default() {
        yay(&s, &o);
    }
    Ok(Json(json!({})))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(id: &str) -> Kullanici {
        Kullanici {
            id: id.into(),
            ad: id.into(),
            avatar: None,
        }
    }

    fn sarki(b: &str) -> Sarki {
        Sarki {
            baslik: b.into(),
            url: "https://youtu.be/x".into(),
            kapak: None,
            sure: Some(200.0),
        }
    }

    #[test]
    fn oda_katil_ayril_kapan() {
        let o = Odalar::default();
        assert!(
            o.katil("1", &k("2"), true).is_none(),
            "olmayan odaya katılınmaz"
        );
        o.guncelle(&k("1"), sarki("A"), 10.0, true);
        o.guncelle(&k("3"), sarki("B"), 0.0, true);
        assert_eq!(o.katil("1", &k("2"), true).unwrap().len(), 1);
        // Başka odaya geçince ilkinden çıkar.
        let d = o.katil("3", &k("2"), true).unwrap();
        assert_eq!(d.len(), 2);
        // Durum güncellenince dinleyiciler korunur.
        let oda = o.guncelle(&k("3"), sarki("C"), 42.0, false);
        assert_eq!(oda.dinleyiciler.len(), 1);
        assert!(!oda.caliyor);
        // Sahip kendi odasına dinleyici olarak eklenmez.
        assert!(o.katil("1", &k("1"), true).unwrap().is_empty());
        // Dinleyici gidince odadan çıkar; sahip gidince oda kapanır.
        let (kapandi, degisen) = o.gitti("2");
        assert!(!kapandi);
        assert_eq!(degisen.len(), 1);
        let (kapandi, _) = o.gitti("3");
        assert!(kapandi);
        assert_eq!(o.liste().len(), 1);
    }
}
