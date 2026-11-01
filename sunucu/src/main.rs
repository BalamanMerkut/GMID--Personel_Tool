//! GMID sunucusu. GMID Tool'un çevrim içi kısmı: Discord ile giriş, kim çevrim içi / ne oynuyor.
//! İleride: sohbet, ortak puan arşivi, oyun gecesi planlayıcı. GMID Tool bu sunucu olmadan da tam çalışır.
//!
//! Sunucuda yalnız 127.0.0.1'den dinler; dışarıya Caddy (https) açar. Kurulum: `kurulum/kur.sh`.

mod api;
mod ayar;
mod db;
mod discord;
mod durum;
mod sosyal;
mod sosyal_api;
mod ws;

use axum::routing::{delete, get, post, put};
use axum::Router;
use std::sync::{Arc, Mutex, MutexGuard};

pub struct Sunucu {
    pub ayar: ayar::Ayar,
    db: Mutex<db::Db>,
    pub durum: durum::Durum,
}

impl Sunucu {
    pub fn db(&self) -> MutexGuard<'_, db::Db> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn oturum(&self, anahtar: &str) -> Option<db::Kullanici> {
        self.db().oturum(anahtar)
    }

    /// Yüklenen kapak/afişler veritabanının yanında durur.
    pub fn resim_klasoru(&self) -> std::path::PathBuf {
        self.ayar
            .veritabani
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("resim")
    }
}

pub fn uygulama(s: Arc<Sunucu>) -> Router {
    Router::new()
        .route("/saglik", get(api::saglik))
        .route("/api/giris", post(api::giris))
        .route("/api/ben", get(api::ben))
        .route("/api/cikis", post(api::cikis))
        .route("/api/cevrimici", get(api::cevrimici))
        .route("/ws", get(ws::baglan))
        .route("/api/kisiler", get(sosyal_api::kisiler))
        .route("/api/profil", put(sosyal_api::profil_kaydet))
        .route("/api/profil/{id}", get(sosyal_api::profil))
        .route("/api/akis", get(sosyal_api::akis))
        .route("/api/akis/{id}", delete(sosyal_api::etkinlik_sil))
        .route("/api/akis/{id}/begen", post(sosyal_api::begen))
        .route("/api/akis/{id}/yorum", post(sosyal_api::yorum))
        .route("/api/yorum/{id}", delete(sosyal_api::yorum_sil))
        .route("/api/konusmalar", get(sosyal_api::konusmalar))
        .route(
            "/api/mesaj/{id}",
            get(sosyal_api::mesajlar).post(sosyal_api::mesaj_yaz),
        )
        .route("/api/resim", post(sosyal_api::resim_yukle))
        .route("/resim/{ad}", get(sosyal_api::resim))
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))
        // GMID Tool'un penceresi (tauri.localhost) sunucuya doğrudan istek atar. Giriş çerez değil
        // Authorization başlığıyla olduğu için her kaynağa izin vermek güvenlidir.
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(s)
}

async fn kapanis() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}

#[tokio::main]
async fn main() {
    let ayar = ayar::Ayar::ortamdan();
    let db = match db::Db::ac(&ayar.veritabani) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Veritabanı açılamadı ({}): {e}", ayar.veritabani.display());
            std::process::exit(1);
        }
    };
    let adres = ayar.adres;
    if ayar.discord.is_none() {
        eprintln!("Uyarı: Discord uygulaması ayarlanmadı; giriş kapalı (DISCORD_CLIENT_ID / DISCORD_CLIENT_SECRET)");
    }
    let s = Arc::new(Sunucu {
        ayar,
        db: Mutex::new(db),
        durum: durum::Durum::default(),
    });
    // Süresi dolan oturumlar günde bir temizlenir.
    let s2 = s.clone();
    tokio::spawn(async move {
        let mut t = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        loop {
            t.tick().await;
            let n = s2.db().temizle();
            if n > 0 {
                eprintln!("{n} eski oturum silindi");
            }
        }
    });
    let dinleyici = match tokio::net::TcpListener::bind(adres).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{adres} dinlenemedi: {e}");
            std::process::exit(1);
        }
    };
    eprintln!(
        "GMID sunucusu {} çalışıyor ({adres})",
        env!("CARGO_PKG_VERSION")
    );
    if let Err(e) = axum::serve(dinleyici, uygulama(s))
        .with_graceful_shutdown(kapanis())
        .await
    {
        eprintln!("Sunucu durdu: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn sunucu() -> Arc<Sunucu> {
        Arc::new(Sunucu {
            ayar: ayar::Ayar {
                adres: "127.0.0.1:0".parse().unwrap(),
                veritabani: ":memory:".into(),
                discord: None,
            },
            db: Mutex::new(db::Db::bellekte()),
            durum: durum::Durum::default(),
        })
    }

    async fn iste(s: &Arc<Sunucu>, r: Request<Body>) -> (StatusCode, serde_json::Value) {
        let cevap = uygulama(s.clone()).oneshot(r).await.unwrap();
        let kod = cevap.status();
        let govde = to_bytes(cevap.into_body(), 1 << 20).await.unwrap();
        (kod, serde_json::from_slice(&govde).unwrap_or_default())
    }

    #[tokio::test]
    async fn saglik_ve_yetki() {
        let s = sunucu();
        let (kod, v) = iste(&s, Request::get("/saglik").body(Body::empty()).unwrap()).await;
        assert_eq!(kod, StatusCode::OK);
        assert_eq!(v["durum"], "ok");
        assert_eq!(v["giris"], false);

        let (kod, _) = iste(&s, Request::get("/api/ben").body(Body::empty()).unwrap()).await;
        assert_eq!(kod, StatusCode::UNAUTHORIZED);

        let k = db::Kullanici {
            id: "9".into(),
            ad: "Ali".into(),
            avatar: None,
        };
        let a = s.db().giris(&k).unwrap();
        let ben = || {
            Request::get("/api/ben")
                .header("authorization", format!("Bearer {a}"))
                .body(Body::empty())
                .unwrap()
        };
        let (kod, v) = iste(&s, ben()).await;
        assert_eq!(kod, StatusCode::OK);
        assert_eq!(v["ad"], "Ali");

        s.durum.geldi(&k);
        let (_, v) = iste(
            &s,
            Request::get("/api/cevrimici")
                .header("authorization", format!("Bearer {a}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(v["kisiler"][0]["ad"], "Ali");

        let (kod, _) = iste(
            &s,
            Request::post("/api/cikis")
                .header("authorization", format!("Bearer {a}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(kod, StatusCode::NO_CONTENT);
        let (kod, _) = iste(&s, ben()).await;
        assert_eq!(kod, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn giris_ayarsizsa_kapali() {
        let s = sunucu();
        let r = Request::post("/api/giris")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"kod":"x","donus":"http://127.0.0.1:53682/"}"#,
            ))
            .unwrap();
        let (kod, v) = iste(&s, r).await;
        assert_eq!(kod, StatusCode::SERVICE_UNAVAILABLE);
        assert!(v["hata"].as_str().unwrap().contains("ayarlanmadı"));
    }
}
