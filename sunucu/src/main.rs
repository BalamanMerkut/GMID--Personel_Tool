//! GMID sunucusu. GMID Tool'un çevrim içi kısmı: Discord ile giriş, kim çevrim içi / ne oynuyor.
//! İleride: sohbet, ortak puan arşivi, oyun gecesi planlayıcı. GMID Tool bu sunucu olmadan da tam çalışır.
//!
//! Sunucuda yalnız 127.0.0.1'den dinler; dışarıya Caddy (https) açar. Kurulum: `kurulum/kur.sh`.

mod api;
mod ayar;
mod db;
mod dinle;
mod discord;
mod durum;
mod gece;
mod hafta;
mod klip;
mod oda;
mod oyna;
mod sayfa;
mod sosyal;
mod sosyal_api;
mod topluluk;
mod ws;
mod yedek;

use axum::routing::{delete, get, post, put};
use axum::Router;
use std::sync::{Arc, Mutex, MutexGuard};

pub struct Sunucu {
    pub ayar: ayar::Ayar,
    db: Mutex<db::Db>,
    pub durum: durum::Durum,
    /// Klip yüklemeleri sırayla: kişi başı ve toplam sınır aynı anda gelen yüklemelerle aşılmasın.
    pub klip_kilidi: tokio::sync::Mutex<()>,
    /// Birlikte dinleme odaları (bellekte, eski tek şarkılık).
    pub dinleme: dinle::Odalar,
    /// Dinleme odaları 2.0 (bellekte).
    pub odalar: oda::Odalar,
}

impl Sunucu {
    pub fn db(&self) -> MutexGuard<'_, db::Db> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn oturum(&self, anahtar: &str) -> Option<db::Kullanici> {
        self.db().oturum(anahtar)
    }

    pub fn yonetici_mi(&self, id: &str) -> bool {
        self.db().yonetici_mi(id, &self.ayar.yoneticiler)
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
        .route(
            "/api/dinle",
            get(dinle::liste).post(dinle::durum).delete(dinle::kapat),
        )
        .route(
            "/api/dinle/{sahip}/katil",
            post(dinle::katil).delete(dinle::ayril),
        )
        .route("/api/odalar", get(oda::liste))
        .route("/api/oda", post(oda::ac))
        .route(
            "/api/oda/{id}",
            axum::routing::patch(oda::ayarla).delete(oda::kapat),
        )
        .route("/api/oda/{id}/katil", post(oda::katil).delete(oda::ayril))
        .route("/api/oda/{id}/sira", post(oda::sira_ekle))
        .route("/api/oda/{id}/sira/{oge}", delete(oda::sira_sil))
        .route("/api/oda/{id}/oynat", post(oda::oynat))
        .route("/api/oda/{id}/sonraki", post(oda::sonraki))
        .route("/api/oda/{id}/at/{kisi}", post(oda::at))
        .route("/api/gece", post(gece::ac))
        .route("/api/sunucuya-girdi", post(oyna::girdi))
        .route("/api/geceler", get(gece::liste))
        .route("/api/gece/{id}", delete(gece::sil))
        .route("/api/gece/{id}/katil", post(gece::katil))
        .route("/api/akis/{id}/begen", post(sosyal_api::begen))
        .route("/api/akis/{id}/oy", post(topluluk::oy))
        .route("/api/akis/{id}/yorum", post(sosyal_api::yorum))
        .route("/api/yorum/{id}", delete(sosyal_api::yorum_sil))
        .route("/api/akis/{id}/sikayet", post(sosyal_api::sikayet))
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
        dinleme: dinle::Odalar::default(),
        odalar: oda::Odalar::default(),
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
    // Oyun gecesi hatırlatması: dakikada bir bakılır (başlamasına 15 dk kalanlar).
    let s7 = s.clone();
    tokio::spawn(async move {
        let mut t = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            t.tick().await;
            let s8 = s7.clone();
            let _ = tokio::task::spawn_blocking(move || {
                gece::hatirlat(&s8);
                // Haftanın klibi ve haftalık özet (pazartesi 12:00).
                hafta::kontrol(&s8);
            })
            .await;
        }
    });
    // Gece yedeği: açılışta ve 6 saatte bir bakılır; o günün yedeği yoksa alınır, en yeni 7 gün kalır.
    let s5 = s.clone();
    tokio::spawn(async move {
        let mut t = tokio::time::interval(std::time::Duration::from_secs(6 * 3600));
        loop {
            t.tick().await;
            let s6 = s5.clone();
            match tokio::task::spawn_blocking(move || yedek::al(&s6, db::simdi())).await {
                Ok(Ok(Some(yol))) => eprintln!("Yedek alındı: {}", yol.display()),
                Ok(Err(e)) => eprintln!("Yedek alınamadı: {e}"),
                _ => {}
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
                yoneticiler: Vec::new(),
            },
            db: Mutex::new(db::Db::bellekte()),
            durum: durum::Durum::default(),
            klip_kilidi: tokio::sync::Mutex::new(()),
            dinleme: dinle::Odalar::default(),
            odalar: oda::Odalar::default(),
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

    #[test]
    fn gece_yedegi() {
        let s = sunucu();
        let k = db::Kullanici {
            id: "1".into(),
            ad: "Ali".into(),
            avatar: None,
        };
        s.db().giris(&k).unwrap();
        let _ = std::fs::remove_dir_all(yedek::klasor(&s));
        let yol = yedek::al(&s, db::simdi())
            .unwrap()
            .expect("ilk yedek alınır");
        assert!(yol.exists());
        assert!(
            yedek::al(&s, db::simdi()).unwrap().is_none(),
            "aynı gün ikinci kez alınmaz"
        );
        // Yedek açılabilir ve içinde kullanıcı var.
        let c = rusqlite::Connection::open(&yol).unwrap();
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM kullanicilar", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn yonetici_siler_sikayet_bildirir() {
        let s = sunucu();
        let kisi = |id: &str, ad: &str| db::Kullanici {
            id: id.into(),
            ad: ad.into(),
            avatar: None,
        };
        // İlk giren (Ali) yönetici; Veli değil.
        let a = s.db().giris(&kisi("1", "Ali")).unwrap();
        let v = s.db().giris(&kisi("2", "Veli")).unwrap();
        let z = s.db().giris(&kisi("3", "Zeynep")).unwrap();
        assert!(s.yonetici_mi("1") && !s.yonetici_mi("2"));
        let yetki = |k: &str| format!("Bearer {k}");
        let (_, ben) = iste(
            &s,
            Request::get("/api/ben")
                .header("authorization", yetki(&a))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(ben["yonetici"], true);
        let g = s
            .db()
            .gonderi_yaz("2", "uygunsuz", None, None)
            .unwrap()
            .unwrap();
        // Zeynep şikâyet eder (ikinci kez sayılmaz), Zeynep silemez, yönetici siler.
        for beklenen in [true, false] {
            let (kod, c) = iste(
                &s,
                Request::post(format!("/api/akis/{g}/sikayet"))
                    .header("authorization", yetki(&z))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"neden":"hakaret"}"#))
                    .unwrap(),
            )
            .await;
            assert_eq!(kod, StatusCode::OK);
            assert_eq!(c["yeni"], beklenen);
        }
        assert_eq!(s.db().akis("1", None, None, 10).unwrap()[0].sikayet, 1);
        let sil = |k: &str| {
            Request::delete(format!("/api/akis/{g}"))
                .header("authorization", yetki(k))
                .body(Body::empty())
                .unwrap()
        };
        let (kod, _) = iste(&s, sil(&z)).await;
        assert_eq!(kod, StatusCode::FORBIDDEN);
        let (kod, _) = iste(&s, sil(&a)).await;
        assert_eq!(kod, StatusCode::OK);
        assert!(s.db().akis("1", None, None, 10).unwrap().is_empty());
        let _ = v;
    }

    #[tokio::test]
    async fn oda_ac_katil_ekle_yonet() {
        let s = sunucu();
        let kisi = |id: &str| db::Kullanici {
            id: id.into(),
            ad: format!("K{id}"),
            avatar: None,
        };
        let a = s.db().giris(&kisi("1")).unwrap();
        let b = s.db().giris(&kisi("2")).unwrap();
        let c = s.db().giris(&kisi("3")).unwrap();
        let istek = |yontem: &str, yol: String, k: &str, govde: &str| {
            Request::builder()
                .method(yontem)
                .uri(yol)
                .header("authorization", format!("Bearer {k}"))
                .header("content-type", "application/json")
                .body(Body::from(govde.to_string()))
                .unwrap()
        };
        let (kod, v) = iste(
            &s,
            istek(
                "POST",
                "/api/oda".into(),
                &a,
                r#"{"ad":"Akşam Rap","sinir":2,"yetki":"herkes"}"#,
            ),
        )
        .await;
        assert_eq!(kod, StatusCode::OK, "{v}");
        let id = v["oda"]["id"].as_str().unwrap().to_string();
        assert_eq!(v["oda"]["ad"], "Akşam Rap");
        // B katılır, C sınır yüzünden giremez.
        let (kod, _) = iste(&s, istek("POST", format!("/api/oda/{id}/katil"), &b, "")).await;
        assert_eq!(kod, StatusCode::OK);
        let (kod, _) = iste(&s, istek("POST", format!("/api/oda/{id}/katil"), &c, "")).await;
        assert_eq!(kod, StatusCode::CONFLICT);
        // B şarkı ekler (herkes ekleyebilir): hemen çalar.
        let (kod, v) = iste(
            &s,
            istek(
                "POST",
                format!("/api/oda/{id}/sira"),
                &b,
                r#"{"baslik":"Şarkı","url":"https://youtu.be/x","sure":200}"#,
            ),
        )
        .await;
        assert_eq!(kod, StatusCode::OK, "{v}");
        assert_eq!(v["oda"]["simdiki"]["ekleyen"]["id"], "2");
        assert_eq!(v["oda"]["caliyor"], true);
        // B duraklatamaz (DJ değil), A duraklatır.
        let dur = r#"{"konum":12.5,"caliyor":false}"#;
        let (kod, _) = iste(&s, istek("POST", format!("/api/oda/{id}/oynat"), &b, dur)).await;
        assert_eq!(kod, StatusCode::FORBIDDEN);
        let (kod, v) = iste(&s, istek("POST", format!("/api/oda/{id}/oynat"), &a, dur)).await;
        assert_eq!(kod, StatusCode::OK);
        assert_eq!(v["oda"]["caliyor"], false);
        // A, B'yi DJ yapar; B artık atlayabilir.
        let (kod, _) = iste(
            &s,
            istek(
                "PATCH",
                format!("/api/oda/{id}"),
                &a,
                r#"{"dj":{"kisi":"2","ver":true}}"#,
            ),
        )
        .await;
        assert_eq!(kod, StatusCode::OK);
        let (kod, v) = iste(
            &s,
            istek("POST", format!("/api/oda/{id}/sonraki"), &b, "{}"),
        )
        .await;
        assert_eq!(kod, StatusCode::OK);
        assert!(v["oda"]["simdiki"].is_null());
        // Sıradan seçilen şarkı hemen çalar (DJ).
        for b2 in ["Bir", "İki", "Üç"] {
            let govde = format!(r#"{{"baslik":"{b2}","url":"https://youtu.be/{b2}","sure":100}}"#);
            iste(&s, istek("POST", format!("/api/oda/{id}/sira"), &b, &govde)).await;
        }
        let (_, v) = iste(&s, istek("GET", "/api/odalar".into(), &a, "")).await;
        let uc = v["odalar"][0]["sira"][1]["id"].as_u64().unwrap();
        let (kod, v) = iste(
            &s,
            istek(
                "POST",
                format!("/api/oda/{id}/sonraki"),
                &b,
                &format!(r#"{{"sec":{uc}}}"#),
            ),
        )
        .await;
        assert_eq!(kod, StatusCode::OK);
        assert_eq!(v["oda"]["simdiki"]["sarki"]["baslik"], "Üç");
        assert_eq!(v["oda"]["sira"][0]["sarki"]["baslik"], "İki");
        // A ayrılınca oda B'ye geçer.
        let (kod, _) = iste(&s, istek("DELETE", format!("/api/oda/{id}/katil"), &a, "")).await;
        assert_eq!(kod, StatusCode::OK);
        let (_, v) = iste(&s, istek("GET", "/api/odalar".into(), &c, "")).await;
        assert_eq!(v["odalar"][0]["sahip"]["id"], "2");
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
