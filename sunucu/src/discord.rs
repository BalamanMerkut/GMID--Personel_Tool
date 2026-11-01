//! Discord ile giriş (OAuth2). Masaüstü uygulama kullanıcıyı Discord'a gönderir, dönen tek kullanımlık kodu
//! buraya yollar; kod burada (gizli anahtarla) değiştirilir. Gizli anahtar hiçbir zaman kullanıcıya gitmez.

use crate::ayar::DiscordAyar;
use crate::db::Kullanici;
use serde_json::Value;
use std::time::Duration;

const API: &str = "https://discord.com/api/v10";

fn ajan() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Dönüş adresi yalnız kullanıcının kendi bilgisayarı olabilir (masaüstü uygulamanın geçici dinleyicisi).
pub fn donus_gecerli(adres: &str) -> bool {
    ["http://127.0.0.1:", "http://localhost:"]
        .iter()
        .any(|p| adres.starts_with(p))
        && !adres.contains('@')
        && adres.len() < 200
}

/// Discord'un kullanıcı bilgisinden ad ve profil resmi.
pub fn kullanici_oku(v: &Value) -> Option<Kullanici> {
    let id = v["id"].as_str()?.to_string();
    let ad = v["global_name"]
        .as_str()
        .filter(|s| !s.is_empty())
        .or(v["username"].as_str())?
        .to_string();
    let avatar = v["avatar"]
        .as_str()
        .map(|h| format!("https://cdn.discordapp.com/avatars/{id}/{h}.png?size=128"));
    Some(Kullanici { id, ad, avatar })
}

fn json(mut r: ureq::http::Response<ureq::Body>, ne: &str) -> Result<Value, String> {
    let durum = r.status();
    let v: Value = r
        .body_mut()
        .read_json()
        .map_err(|e| format!("{ne}: yanıt okunamadı ({e})"))?;
    if !durum.is_success() {
        return Err(format!("{ne}: Discord {durum} döndü"));
    }
    Ok(v)
}

/// Kodu değiştirir, kullanıcıyı döner. Sunucu kimliği ayarlıysa üyelik de denetlenir.
pub fn giris(ayar: &DiscordAyar, kod: &str, donus: &str) -> Result<Kullanici, String> {
    if !donus_gecerli(donus) {
        return Err("Geçersiz dönüş adresi".into());
    }
    let a = ajan();
    let token = json(
        a.post(&format!("{API}/oauth2/token"))
            .send_form([
                ("client_id", ayar.client_id.as_str()),
                ("client_secret", ayar.client_secret.as_str()),
                ("grant_type", "authorization_code"),
                ("code", kod),
                ("redirect_uri", donus),
            ])
            .map_err(|e| format!("Discord'a ulaşılamadı: {e}"))?,
        "Giriş kodu",
    )?;
    let erisim = token["access_token"]
        .as_str()
        .ok_or("Discord erişim anahtarı vermedi")?
        .to_string();
    let yetki = format!("Bearer {erisim}");
    let ben = json(
        a.get(&format!("{API}/users/@me"))
            .header("Authorization", &yetki)
            .call()
            .map_err(|e| format!("Discord'a ulaşılamadı: {e}"))?,
        "Kullanıcı bilgisi",
    )?;
    let k = kullanici_oku(&ben).ok_or("Discord kullanıcı bilgisi eksik")?;
    if let Some(sunucu) = &ayar.sunucu_id {
        let liste = json(
            a.get(&format!("{API}/users/@me/guilds"))
                .header("Authorization", &yetki)
                .call()
                .map_err(|e| format!("Discord'a ulaşılamadı: {e}"))?,
            "Sunucu listesi",
        )?;
        let uye = liste
            .as_array()
            .is_some_and(|l| l.iter().any(|g| g["id"].as_str() == Some(sunucu.as_str())));
        if !uye {
            return Err("Yalnız GMID Discord sunucusunun üyeleri girebilir".into());
        }
    }
    Ok(k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kullanici_okunur() {
        let k = kullanici_oku(
            &json!({"id": "42", "username": "ali", "global_name": "Ali", "avatar": "abc"}),
        )
        .unwrap();
        assert_eq!(k.ad, "Ali");
        assert_eq!(
            k.avatar.as_deref(),
            Some("https://cdn.discordapp.com/avatars/42/abc.png?size=128")
        );
        let k = kullanici_oku(
            &json!({"id": "7", "username": "veli", "global_name": null, "avatar": null}),
        )
        .unwrap();
        assert_eq!((k.ad.as_str(), k.avatar), ("veli", None));
        assert!(kullanici_oku(&json!({"username": "kimliksiz"})).is_none());
    }

    #[test]
    fn donus_adresi_yalniz_yerel() {
        assert!(donus_gecerli("http://127.0.0.1:53682/giris"));
        assert!(donus_gecerli("http://localhost:53682/giris"));
        assert!(!donus_gecerli("https://kotu.site/giris"));
        assert!(!donus_gecerli("http://127.0.0.1:1@kotu.site/"));
    }
}
