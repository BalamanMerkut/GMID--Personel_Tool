//! Canlı bağlantı (WebSocket): GMID açıkken bağlı kalır. İlk mesaj giriş anahtarıdır; sonra kişi
//! "şu oyunu oynuyorum" der, sunucu da herkesin durumunu anlık iletir.
//!
//! İstemci → sunucu: `{"tip":"merhaba","anahtar":"…"}`, `{"tip":"durum","oyun":"CS2"|null}`
//! Sunucu → istemci: `liste`, `geldi`, `gitti`, `durum`, `akis` (akış değişti), `mesaj` (özel mesaj),
//! `bildirim` (olayıma beğeni/yorum), `hata`

use crate::durum::oyun_adi;
use crate::Sunucu;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;

pub async fn baglan(ws: WebSocketUpgrade, State(s): State<Arc<Sunucu>>) -> Response {
    ws.max_message_size(16 * 1024)
        .on_upgrade(move |sock| oturum(sock, s))
}

async fn gonder(sock: &mut WebSocket, metin: String) -> bool {
    sock.send(Message::Text(metin.into())).await.is_ok()
}

async fn oturum(mut sock: WebSocket, s: Arc<Sunucu>) {
    // Giriş: 10 sn içinde anahtar gelmezse kapatılır.
    let anahtar = match tokio::time::timeout(Duration::from_secs(10), sock.recv()).await {
        Ok(Some(Ok(Message::Text(t)))) => serde_json::from_str::<Value>(&t)
            .ok()
            .filter(|v| v["tip"] == "merhaba")
            .and_then(|v| v["anahtar"].as_str().map(str::to_string)),
        _ => None,
    };
    let kisi = anahtar.and_then(|a| s.oturum(&a));
    let Some(kisi) = kisi else {
        gonder(
            &mut sock,
            json!({"tip": "hata", "mesaj": "Giriş geçersiz"}).to_string(),
        )
        .await;
        return;
    };

    // Önce "geldi" (başkalarına), sonra abone olunur: kişi kendi gelişini ayrıca almaz; arada olan her
    // değişiklik zaten hemen ardından gönderilen tam listede vardır.
    s.durum.geldi(&kisi);
    let mut rx = s.durum.abone();
    let (ozel_tx, mut ozel) = tokio::sync::mpsc::unbounded_channel::<String>();
    s.durum.kaydol(&kisi.id, ozel_tx.clone());
    let mut nabiz = tokio::time::interval(Duration::from_secs(30));
    nabiz.tick().await;
    if gonder(&mut sock, s.durum.liste_mesaji()).await {
        loop {
            tokio::select! {
                gelen = sock.recv() => match gelen {
                    Some(Ok(Message::Text(t))) => {
                        if let Ok(v) = serde_json::from_str::<Value>(&t) {
                            if v["tip"] == "durum" {
                                let oyun = oyun_adi(v["oyun"].as_str());
                                if s.durum.oyun(&kisi.id, oyun.clone()) {
                                    // Oyuna girdi: akışa "X oynuyor" (aynı oyun 12 saatte bir).
                                    if let Some(o) = oyun {
                                        if s.db().oyun_basladi(&kisi.id, &o).unwrap_or(false) {
                                            s.durum.yay(json!({ "tip": "akis" }));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                },
                giden = rx.recv() => match giden {
                    Ok(m) => if !gonder(&mut sock, m).await { break },
                    // Çok geride kaldıysa baştan tam liste.
                    Err(RecvError::Lagged(_)) => if !gonder(&mut sock, s.durum.liste_mesaji()).await { break },
                    Err(RecvError::Closed) => break,
                },
                Some(m) = ozel.recv() => if !gonder(&mut sock, m).await { break },
                // Arada modem/proxy bağlantıyı sessizce kesmesin.
                _ = nabiz.tick() => if sock.send(Message::Ping(Vec::new().into())).await.is_err() { break },
            }
        }
    }
    s.durum.birak(&kisi.id, &ozel_tx);
    s.durum.ayrildi(&kisi.id);
    // Son bağlantısı da kapandıysa dinleme odası kapanır, dinlediği odadan çıkar.
    if s.durum.kisi(&kisi.id).is_none() {
        crate::dinle::kisi_gitti(&s, &kisi.id);
    }
}
