//! Topluluk (GMID sunucusu) girişi: tarayıcıda Discord'un giriş sayfası açılır, kullanıcı izin verince Discord
//! bu bilgisayardaki geçici bir adrese (127.0.0.1:53682) tek kullanımlık bir kod gönderir; kod GMID sunucusuna
//! iletilir, sunucu Discord'la konuşup bir oturum anahtarı verir. Discord şifresi hiçbir zaman GMID'e gelmez.
//! Akış, profil, mesajlar gibi geri kalan her şeyi arayüz sunucuyla doğrudan konuşur.

use serde_json::{json, Value};
use std::hash::{BuildHasher, Hasher};
use std::time::{Duration, Instant};

/// Discord uygulamasında kayıtlı dönüş adresi bu porttadır (değişirse Developer Portal'da da değişmeli).
pub const DONUS_PORT: u16 = 53682;

/// Tahmin edilemez kısa bir değer (giriş isteğinin bu bilgisayardan başladığını doğrulamak için).
fn rastgele() -> String {
    let s = std::collections::hash_map::RandomState::new();
    let (mut a, mut b) = (s.build_hasher(), s.build_hasher());
    a.write_u64(crate::util::now_ms() as u64);
    b.write_u64(std::process::id() as u64 ^ 0x9e37_79b9_7f4a_7c15);
    format!("{:016x}{:016x}", a.finish(), b.finish())
}

fn kodla(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `?a=1&b=2` içinden bir değer (yüzde kodlaması çözülür).
pub fn sorgu(yol: &str, ad: &str) -> Option<String> {
    let q = yol.split_once('?')?.1;
    q.split('&').find_map(|p| {
        let (k, v) = p.split_once('=')?;
        (k == ad).then(|| coz(v))
    })
}

fn coz(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                Ok(x) => {
                    out.push(x);
                    i += 3;
                    continue;
                }
                Err(_) => out.push(b'%'),
            },
            b'+' => out.push(b' '),
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Sunucu adresi: https olmalı (yalnız deneme için kendi bilgisayarındaki http kabul edilir).
pub fn adres(s: &str) -> Result<String, String> {
    let s = s.trim().trim_end_matches('/');
    let ok = s.starts_with("https://")
        || s.starts_with("http://127.0.0.1:")
        || s.starts_with("http://localhost:");
    if !ok || s.len() < 10 {
        return Err("Sunucu adresi https:// ile başlamalı".into());
    }
    Ok(s.to_string())
}

fn json_al(
    r: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ne: &str,
) -> Result<(u16, Value), String> {
    let mut r = r.map_err(|e| format!("{ne}: sunucuya ulaşılamadı ({e})"))?;
    let kod = r.status().as_u16();
    let metin = r.body_mut().read_to_string().unwrap_or_default();
    Ok((kod, serde_json::from_str(&metin).unwrap_or(Value::Null)))
}

/// Paylaşılan klibin akışta görünen bilgileri.
pub struct KlipBilgi<'a> {
    pub ad: &'a str,
    pub oyun: Option<&'a str>,
    /// Kısa açıklama.
    pub not: Option<&'a str>,
    /// Süre (saniye).
    pub sure: Option<f64>,
}

/// Klibi GMID sunucusuna yükler (önce kapağı); akışta 7 gün görünür. Dönen: sunucunun cevabı (`id`, `url`, `bitis`).
pub fn klip_yukle(
    sunucu: &str,
    anahtar: &str,
    video: &std::path::Path,
    kapak: Option<&std::path::Path>,
    b: &KlipBilgi,
) -> Result<Value, String> {
    let KlipBilgi {
        ad,
        oyun,
        not,
        sure,
    } = *b;
    let sunucu = adres(sunucu)?;
    let yetki = format!("Bearer {anahtar}");
    let ajan = crate::net::agent_upload();
    let kapak_url = kapak
        .and_then(|k| std::fs::read(k).ok())
        .filter(|b| !b.is_empty() && b.len() <= 400 * 1024)
        .and_then(|b| {
            let r = ajan
                .post(&format!("{sunucu}/api/resim"))
                .header("Authorization", &yetki)
                .header("Content-Type", "image/jpeg")
                .send(&b[..]);
            json_al(r, "Kapak").ok()
        })
        .and_then(|(kod, v)| (kod == 200).then(|| v["url"].as_str().map(String::from))?);
    let govde = std::fs::read(video).map_err(|e| format!("Klip okunamadı: {e}"))?;
    let mut url = format!("{sunucu}/api/klip?ad={}", kodla(ad));
    if let Some(o) = oyun.filter(|o| !o.trim().is_empty()) {
        url += &format!("&oyun={}", kodla(o));
    }
    if let Some(n) = not.map(str::trim).filter(|n| !n.is_empty()) {
        url += &format!("&not={}", kodla(n));
    }
    if let Some(s) = sure.filter(|s| s.is_finite() && *s > 0.0) {
        url += &format!("&sure={s:.1}");
    }
    if let Some(k) = kapak_url {
        url += &format!("&kapak={}", kodla(&k));
    }
    let r = ajan
        .post(&url)
        .header("Authorization", &yetki)
        .header("Content-Type", "video/mp4")
        .send(&govde[..]);
    let (kod, v) = json_al(r, "Klip")?;
    match kod {
        200 => Ok(v),
        413 if v["hata"].is_null() => Err(
            "Sunucu klibi kabul etmedi (yükleme sınırı). Sunucuda bir kez gmid-guncelle çalıştırılmalı."
                .into(),
        ),
        401 => Err("Oturum kapanmış; Topluluk'tan yeniden giriş yap".into()),
        _ => Err(v["hata"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| format!("Klip yüklenemedi ({kod})"))),
    }
}

const SAYFA: &str = r#"<!doctype html><meta charset="utf-8"><title>GMID</title>
<body style="font-family:system-ui;background:#16181c;color:#eee;display:grid;place-items:center;height:100vh;margin:0">
<div style="text-align:center"><h2 style="color:#e3c270">{BASLIK}</h2><p>{METIN}</p></div></body>"#;

/// Discord ile giriş: `{ anahtar, kullanici }`.
pub fn giris(sunucu: &str) -> Result<Value, String> {
    let sunucu = adres(sunucu)?;
    let ajan = crate::net::agent(Duration::from_secs(10), Duration::from_secs(30));
    let (_, saglik) = json_al(ajan.get(&format!("{sunucu}/saglik")).call(), "Sunucu")?;
    let client = saglik["discord"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("Sunucuda Discord ile giriş henüz ayarlanmadı")?
        .to_string();

    let dinleyici = tiny_http::Server::http(("127.0.0.1", DONUS_PORT))
        .map_err(|_| format!("{DONUS_PORT} portu kullanımda; biraz sonra yeniden dene"))?;
    let durum = rastgele();
    let donus = format!("http://127.0.0.1:{DONUS_PORT}/giris");
    let url = format!(
        "https://discord.com/oauth2/authorize?client_id={}&response_type=code&redirect_uri={}&scope=identify%20guilds&state={durum}&prompt=none",
        kodla(&client),
        kodla(&donus)
    );
    tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|e| e.to_string())?;

    // Kullanıcı tarayıcıda izin verene kadar (en çok 5 dk).
    let son = Instant::now() + Duration::from_secs(300);
    let kod = loop {
        let kalan = son.saturating_duration_since(Instant::now());
        if kalan.is_zero() {
            return Err("Giriş zaman aşımına uğradı".into());
        }
        let Ok(Some(istek)) = dinleyici.recv_timeout(kalan.min(Duration::from_secs(1))) else {
            continue;
        };
        let yol = istek.url().to_string();
        if !yol.starts_with("/giris") {
            let _ = istek.respond(tiny_http::Response::empty(404));
            continue;
        }
        let sayfa = |b: &str, m: &str| {
            tiny_http::Response::from_string(SAYFA.replace("{BASLIK}", b).replace("{METIN}", m))
                .with_header(
                    "Content-Type: text/html; charset=utf-8"
                        .parse::<tiny_http::Header>()
                        .expect("başlık"),
                )
        };
        if sorgu(&yol, "state").as_deref() != Some(durum.as_str()) {
            let _ = istek.respond(sayfa("Geçersiz istek", "Bu giriş GMID'den başlatılmadı."));
            return Err("Giriş doğrulanamadı".into());
        }
        match sorgu(&yol, "code") {
            Some(k) => {
                let _ = istek.respond(sayfa(
                    "Giriş tamam",
                    "GMID'e dönebilirsin, bu sekmeyi kapatabilirsin.",
                ));
                break k;
            }
            None => {
                let _ = istek.respond(sayfa(
                    "Giriş iptal edildi",
                    "GMID'den yeniden deneyebilirsin.",
                ));
                return Err("Discord girişi iptal edildi".into());
            }
        }
    };
    drop(dinleyici);

    let (durum_kodu, v) = json_al(
        ajan.post(&format!("{sunucu}/api/giris"))
            .header("Content-Type", "application/json")
            .send(json!({ "kod": kod, "donus": donus }).to_string()),
        "Giriş",
    )?;
    if durum_kodu != 200 {
        return Err(v["hata"].as_str().unwrap_or("Giriş yapılamadı").to_string());
    }
    crate::log!(
        "Topluluk: {} olarak giriş yapıldı",
        v["kullanici"]["ad"].as_str().unwrap_or("?")
    );
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorgu_okunur() {
        let y = "/giris?code=abc%2B1&state=xyz&x=a+b";
        assert_eq!(sorgu(y, "code").as_deref(), Some("abc+1"));
        assert_eq!(sorgu(y, "state").as_deref(), Some("xyz"));
        assert_eq!(sorgu(y, "x").as_deref(), Some("a b"));
        assert_eq!(sorgu(y, "yok"), None);
        assert_eq!(sorgu("/giris", "code"), None);
        assert_eq!(sorgu("/g?c=%", "c").as_deref(), Some("%"));
    }

    #[test]
    fn adres_denetlenir() {
        assert_eq!(
            adres(" https://gmid.ornek.com/ ").unwrap(),
            "https://gmid.ornek.com"
        );
        assert!(adres("http://gmid.ornek.com").is_err());
        assert!(adres("http://127.0.0.1:8780").is_ok());
        assert!(adres("javascript:alert(1)").is_err());
    }

    #[test]
    fn kodlama() {
        assert_eq!(
            kodla("http://127.0.0.1:53682/giris"),
            "http%3A%2F%2F127.0.0.1%3A53682%2Fgiris"
        );
        assert_ne!(rastgele(), rastgele());
    }
}
