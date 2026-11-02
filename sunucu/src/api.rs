//! HTTP uçları: giriş, ben kimim, çıkış, çevrim içi listesi.

use crate::db::Kullanici;
use crate::{discord, Sunucu};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

pub type Sonuc = Result<Json<Value>, (StatusCode, Json<Value>)>;

pub fn hata(kod: StatusCode, mesaj: impl Into<String>) -> (StatusCode, Json<Value>) {
    (kod, Json(json!({ "hata": mesaj.into() })))
}

fn anahtar(h: &HeaderMap) -> Option<&str> {
    h.get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

pub fn kim(s: &Sunucu, h: &HeaderMap) -> Result<Kullanici, (StatusCode, Json<Value>)> {
    anahtar(h)
        .and_then(|a| s.oturum(a))
        .ok_or_else(|| hata(StatusCode::UNAUTHORIZED, "Giriş gerekli"))
}

pub async fn saglik(State(s): State<Arc<Sunucu>>) -> Json<Value> {
    Json(json!({
        "durum": "ok",
        "surum": env!("CARGO_PKG_VERSION"),
        "giris": s.ayar.discord.is_some(),
        // Masaüstü uygulama Discord giriş sayfasını bununla açar (gizli değildir; gizli anahtar burada kalır).
        "discord": s.ayar.discord.as_ref().map(|d| d.client_id.clone()),
        "cevrimici": s.durum.liste().len(),
    }))
}

#[derive(Deserialize)]
pub struct GirisIstek {
    kod: String,
    donus: String,
}

pub async fn giris(State(s): State<Arc<Sunucu>>, Json(i): Json<GirisIstek>) -> Sonuc {
    let Some(d) = s.ayar.discord.clone() else {
        return Err(hata(
            StatusCode::SERVICE_UNAVAILABLE,
            "Discord ile giriş henüz ayarlanmadı",
        ));
    };
    let k = tokio::task::spawn_blocking(move || discord::giris(&d, &i.kod, &i.donus))
        .await
        .map_err(|e| hata(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| hata(StatusCode::FORBIDDEN, e))?;
    let a = s
        .db()
        .giris(&k)
        .map_err(|e| hata(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    eprintln!("Giriş: {} ({})", k.ad, k.id);
    Ok(Json(json!({ "anahtar": a, "kullanici": k })))
}

pub async fn ben(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> Sonuc {
    let k = kim(&s, &h)?;
    let yonetici = s.yonetici_mi(&k.id);
    Ok(Json(
        json!({ "id": k.id, "ad": k.ad, "avatar": k.avatar, "yonetici": yonetici }),
    ))
}

pub async fn cikis(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> StatusCode {
    if let Some(a) = anahtar(&h) {
        s.db().cikis(a);
    }
    StatusCode::NO_CONTENT
}

pub async fn cevrimici(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> Sonuc {
    kim(&s, &h)?;
    Ok(Json(json!({ "kisiler": s.durum.liste() })))
}
