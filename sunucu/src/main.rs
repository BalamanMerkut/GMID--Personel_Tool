//! GMID sunucusu. GMID Tool'un çevrim içi kısmı: Discord ile giriş, kim çevrim içi / ne oynuyor.
//! İleride: sohbet, ortak puan arşivi, oyun gecesi planlayıcı. GMID Tool bu sunucu olmadan da tam çalışır.
//!
//! Sunucuda yalnız 127.0.0.1'den dinler; dışarıya Caddy (https) açar. Kurulum: `kurulum/kur.sh`.

mod api;
mod ayar;
mod db;
mod discord;
mod durum;
mod klip;
mod sayfa;
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
    /// Klip yüklemeleri sırayla: kişi başı ve toplam sınır aynı anda gelen yüklemelerle aşılmasın.
    pub klip_kilidi: tokio::sync::Mutex<()>,
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

    /// Paylaşılan klipler (7 gün durur).
    pub fn klip_klasoru(&self) -> std::path::PathBuf {
        self.ayar
            .veritabani
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("klip")
    }
}

pub fn uygulama(s: Arc<Sunucu>) -> Router {
    // Videolar parça parça (Range) gönderilir: oynatıcıda ileri sarılabilir.
    let klipler = tower_http::services::ServeDir::new(s.klip_klasoru());
    Router::new()
        .route("/", get(sayfa::ana))
        .route("/indir", get(sayfa::indir))
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
        .route("/api/gonderi", post(sosyal_api::gonderi))
        .route("/api/gundem", get(sosyal_api::gundem))
        .route("/api/akis/{id}/begen", post(sosyal_api::begen))
        .route("/api/akis/{id}/yorum", post(sosyal_api::yorum))
        .route("/api/yorum/{id}", delete(sosyal_api::yorum_sil))
        .route("/api/arkadaslar", get(sosyal_api::arkadaslar))
        .route(
            "/api/arkadas/{id}",
            post(sosyal_api::arkadas_ekle).delete(sosyal_api::arkadas_sil),
        )
        .route("/api/konusmalar", get(sosyal_api::konusmalar))
        .route(
            "/api/mesaj/{id}",
            get(sosyal_api::mesajlar).post(sosyal_api::mesaj_yaz),
        )
        .route("/api/resim", post(sosyal_api::resim_yukle))
        .route("/resim/{ad}", get(sosyal_api::resim))
        .route(
            "/api/klip",
            post(klip::yukle).layer(axum::extract::DefaultBodyLimit::max(klip::KLIP_SINIRI)),
        )
        .nest_service("/klip", klipler)
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
        klip_kilidi: tokio::sync::Mutex::new(()),
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
    // Süresi dolan klipler (7 gün) saatte bir silinir; aldıkları beğeniler profilde kalır.
    let s3 = s.clone();
    tokio::spawn(async move {
        let mut t = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            t.tick().await;
            let s4 = s3.clone();
            if let Ok(n) = tokio::task::spawn_blocking(move || klip::temizle(&s4)).await {
                if n > 0 {
                    eprintln!("{n} eski klip silindi");
                }
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
                // Resim ve klip klasörleri bunun yanında: her deneme kendi geçici klasöründe.
                veritabani: std::env::temp_dir()
                    .join(format!(
                        "gmid-deneme-{}-{:?}",
                        std::process::id(),
                        std::thread::current().id()
                    ))
                    .join("gmid.db"),
                discord: None,
            },
            db: Mutex::new(db::Db::bellekte()),
            durum: durum::Durum::default(),
            klip_kilidi: tokio::sync::Mutex::new(()),
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
    async fn tanitim_ve_indirme() {
        let s = sunucu();
        let r = uygulama(s.clone())
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let govde = to_bytes(r.into_body(), 1 << 20).await.unwrap();
        assert!(String::from_utf8_lossy(&govde).contains("/indir"));
        let r = uygulama(s)
            .oneshot(Request::get("/indir").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::TEMPORARY_REDIRECT);
        assert!(r.headers()["location"]
            .to_str()
            .unwrap()
            .ends_with("GMID-Tool-Kurulum.exe"));
    }

    #[tokio::test]
    async fn klip_paylas_sil_puan_kalir() {
        let s = sunucu();
        let ali = db::Kullanici {
            id: "1".into(),
            ad: "Ali".into(),
            avatar: None,
        };
        let veli = db::Kullanici {
            id: "2".into(),
            ad: "Veli".into(),
            avatar: None,
        };
        let a = s.db().giris(&ali).unwrap();
        let v = s.db().giris(&veli).unwrap();
        let yetki = |k: &str| format!("Bearer {k}");
        // Girişsiz yükleme gövde okunmadan reddedilir.
        let (kod, _) = iste(
            &s,
            Request::post("/api/klip")
                .body(Body::from(vec![0u8; 3 * 1024 * 1024]))
                .unwrap(),
        )
        .await;
        assert_eq!(kod, StatusCode::UNAUTHORIZED);
        // Video olmayan reddedilir.
        let (kod, _) = iste(
            &s,
            Request::post("/api/klip")
                .header("authorization", yetki(&a))
                .body(Body::from("<html>"))
                .unwrap(),
        )
        .await;
        assert_eq!(kod, StatusCode::BAD_REQUEST);
        // 3 MB'lık "video" (genel 2 MB sınırına takılmaz).
        let mut govde = b"\0\0\0\x20ftypisom".to_vec();
        govde.resize(3 * 1024 * 1024, 7);
        let (kod, cevap) = iste(
            &s,
            Request::post("/api/klip?ad=Ace&oyun=Valorant&sure=12.5&not=Son%20saniye%21")
                .header("authorization", yetki(&a))
                .body(Body::from(govde))
                .unwrap(),
        )
        .await;
        assert_eq!(kod, StatusCode::OK, "{cevap}");
        let id = cevap["id"].as_i64().unwrap();
        let akis = s.db().akis("2", None, None, 10).unwrap();
        assert_eq!(akis[0].tur, "klip");
        assert_eq!(akis[0].veri["oyun"], "Valorant");
        assert_eq!(akis[0].veri["not"], "Son saniye!");
        // Veli beğenir, Ali kendi klibini beğenir (kendi beğenisi puana sayılmaz).
        s.db().begen("2", id).unwrap();
        s.db().begen("1", id).unwrap();
        assert_eq!(s.db().toplam_puan("1"), (1, 0));
        // 7 gün geçti: paylaşım akıştan düşer, temizlikte silinir; beğeni kalır.
        s.db()
            .0
            .execute(
                "UPDATE etkinlikler SET zaman = zaman - ?1",
                [sosyal::KLIP_OMRU_MS + 1],
            )
            .unwrap();
        assert!(s.db().akis("2", None, None, 10).unwrap().is_empty());
        assert_eq!(klip::temizle(&s), 1);
        assert_eq!(s.db().toplam_puan("1"), (1, 0));
        let (_, p) = iste(
            &s,
            Request::get("/api/profil/1")
                .header("authorization", yetki(&v))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(p["begeni"], 1);
    }

    #[tokio::test]
    async fn gonderi_paylas() {
        let s = sunucu();
        let ali = db::Kullanici {
            id: "1".into(),
            ad: "Ali".into(),
            avatar: None,
        };
        let a = s.db().giris(&ali).unwrap();
        let gonder = |govde: &'static str| {
            Request::post("/api/gonderi")
                .header("authorization", format!("Bearer {a}"))
                .header("content-type", "application/json")
                .body(Body::from(govde))
                .unwrap()
        };
        let (kod, _) = iste(&s, gonder(r#"{"metin":"   "}"#)).await;
        assert_eq!(kod, StatusCode::BAD_REQUEST);
        let (kod, _) = iste(
            &s,
            gonder(r#"{"metin":"x","resim":"https://kotu.site/a.jpg"}"#),
        )
        .await;
        assert_eq!(kod, StatusCode::OK, "yabancı resim yok sayılır, yazı yeter");
        for _ in 0..9 {
            let (kod, _) = iste(&s, gonder(r#"{"metin":"Bu akşam CS2?"}"#)).await;
            assert_eq!(kod, StatusCode::OK);
        }
        let (kod, _) = iste(&s, gonder(r#"{"metin":"bir tane daha"}"#)).await;
        assert_eq!(kod, StatusCode::TOO_MANY_REQUESTS);
        let akis = s.db().akis("1", None, None, 20).unwrap();
        assert_eq!(akis.len(), 10);
        assert_eq!(akis[0].tur, "gonderi");
        assert_eq!(akis[0].veri["metin"], "Bu akşam CS2?");
        assert!(akis[9].veri["resim"].is_null());
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
