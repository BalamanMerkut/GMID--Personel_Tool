//! Ayarlar ortam değişkenlerinden okunur (sunucuda `/etc/gmid/sunucu.env`, yalnız root okuyabilir).

use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clone)]
pub struct DiscordAyar {
    pub client_id: String,
    pub client_secret: String,
    /// Doluysa yalnız bu Discord sunucusunun (GMID) üyeleri girebilir.
    pub sunucu_id: Option<String>,
}

#[derive(Clone)]
pub struct Ayar {
    pub adres: SocketAddr,
    pub veritabani: PathBuf,
    /// Discord uygulaması henüz açılmadıysa `None`: sunucu çalışır ama giriş kapalıdır.
    pub discord: Option<DiscordAyar>,
    /// Yöneticilerin Discord kimlikleri (`GMID_YONETICILER=123,456`). Boşsa ilk giriş yapan kişi yöneticidir.
    pub yoneticiler: Vec<String>,
}

fn oku(ad: &str) -> Option<String> {
    std::env::var(ad)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl Ayar {
    pub fn ortamdan() -> Ayar {
        let discord = match (oku("DISCORD_CLIENT_ID"), oku("DISCORD_CLIENT_SECRET")) {
            (Some(client_id), Some(client_secret)) => Some(DiscordAyar {
                client_id,
                client_secret,
                sunucu_id: oku("DISCORD_SUNUCU_ID"),
            }),
            _ => None,
        };
        Ayar {
            // Dışarıya Caddy (https) açılır; program yalnız sunucunun içinden dinler.
            adres: oku("GMID_ADRES")
                .and_then(|a| a.parse().ok())
                .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8780))),
            veritabani: oku("GMID_VERITABANI")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("gmid.db")),
            discord,
            yoneticiler: oku("GMID_YONETICILER")
                .map(|v| {
                    v.split([',', ' '])
                        .map(str::trim)
                        .filter(|x| !x.is_empty())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}
