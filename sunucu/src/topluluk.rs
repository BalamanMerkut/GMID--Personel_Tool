//! Topluluk ekleri: gönderide **anket** (2–4 seçenek, süreli, herkes bir oy) ve profilde **rozetler**
//! (bir kısmı hesaplanır: kurucu üye, sevilen, gece düzenleyicisi…; bir kısmı kalıcı olarak verilir:
//! haftanın klibi, turnuva şampiyonu).

use crate::api::{hata, kim, Sonuc};
use crate::db::{simdi, Db};
use crate::sosyal::Etkinlik;
use crate::Sunucu;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

pub fn kur(c: &Connection) -> rusqlite::Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS anket_oylar (
           etkinlik INTEGER NOT NULL REFERENCES etkinlikler(id) ON DELETE CASCADE,
           kullanici TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           secim INTEGER NOT NULL, zaman INTEGER NOT NULL,
           PRIMARY KEY (etkinlik, kullanici));
         CREATE TABLE IF NOT EXISTS profil_gorunum (
           kullanici TEXT PRIMARY KEY REFERENCES kullanicilar(id) ON DELETE CASCADE,
           veri TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS rozetler (
           kullanici TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           rozet TEXT NOT NULL, notu TEXT, zaman INTEGER NOT NULL,
           PRIMARY KEY (kullanici, rozet, zaman));",
    )
}

type R<T> = Result<T, String>;

fn e(x: rusqlite::Error) -> String {
    x.to_string()
}

// ---------------- anket ----------------

/// Anket süresi (saat) sınırları.
pub const ANKET_SAAT: (i64, i64) = (1, 168);

/// Gönderiye eklenecek anket verisi (doğrulanmış): `{"secenekler": [..], "bitis": ms}`.
pub fn anket_verisi(secenekler: &[String], saat: Option<i64>) -> Result<Value, &'static str> {
    let s: Vec<String> = secenekler
        .iter()
        .map(|x| x.trim().chars().take(60).collect::<String>())
        .filter(|x| !x.is_empty())
        .collect();
    if !(2..=4).contains(&s.len()) {
        return Err("Ankette 2 ile 4 arası seçenek olmalı");
    }
    let saat = saat.unwrap_or(24).clamp(ANKET_SAAT.0, ANKET_SAAT.1);
    Ok(json!({ "secenekler": s, "bitis": simdi() + saat * 3600 * 1000 }))
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct AnketDurum {
    /// Her seçeneğin oy sayısı.
    pub oylar: Vec<i64>,
    /// Benim seçimim.
    pub benim: Option<i64>,
}

#[derive(Debug, PartialEq)]
pub enum OySonuc {
    Tamam(AnketDurum),
    Yok,
    Bitti,
    Gecersiz,
}

impl Db {
    fn anket_durum(&self, ben: &str, etkinlik: i64, n: usize) -> R<AnketDurum> {
        let mut oylar = vec![0i64; n];
        let mut st = self
            .0
            .prepare("SELECT secim, COUNT(*) FROM anket_oylar WHERE etkinlik = ?1 GROUP BY secim")
            .map_err(e)?;
        for r in st
            .query_map([etkinlik], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
            })
            .map_err(e)?
            .flatten()
        {
            if let Some(x) = usize::try_from(r.0).ok().and_then(|i| oylar.get_mut(i)) {
                *x = r.1;
            }
        }
        let benim = self
            .0
            .query_row(
                "SELECT secim FROM anket_oylar WHERE etkinlik = ?1 AND kullanici = ?2",
                params![etkinlik, ben],
                |r| r.get(0),
            )
            .optional()
            .map_err(e)?;
        Ok(AnketDurum { oylar, benim })
    }

    /// Akıştaki anketli gönderilere oy sayılarını ve benim oyumu ekler (`veri.anket.durum`).
    pub fn anketleri_doldur(&self, ben: &str, l: &mut [Etkinlik]) -> R<()> {
        for et in l.iter_mut() {
            let n = et.veri["anket"]["secenekler"]
                .as_array()
                .map_or(0, Vec::len);
            if n == 0 {
                continue;
            }
            let d = self.anket_durum(ben, et.id, n)?;
            et.veri["anket"]["durum"] = serde_json::to_value(d).unwrap_or(Value::Null);
        }
        Ok(())
    }

    /// Oy ver (fikir değiştirilebilir; süre bitince kapanır).
    pub fn oy_ver(&self, ben: &str, etkinlik: i64, secim: i64) -> R<OySonuc> {
        let veri: Option<String> = self
            .0
            .query_row(
                "SELECT veri FROM etkinlikler WHERE id = ?1",
                [etkinlik],
                |r| r.get(0),
            )
            .optional()
            .map_err(e)?;
        let Some(veri) = veri else {
            return Ok(OySonuc::Yok);
        };
        let v: Value = serde_json::from_str(&veri).unwrap_or(Value::Null);
        let n = v["anket"]["secenekler"].as_array().map_or(0, Vec::len);
        if n == 0 {
            return Ok(OySonuc::Yok);
        }
        if v["anket"]["bitis"].as_i64().is_some_and(|b| b < simdi()) {
            return Ok(OySonuc::Bitti);
        }
        if secim < 0 || secim as usize >= n {
            return Ok(OySonuc::Gecersiz);
        }
        self.0
            .execute(
                "INSERT INTO anket_oylar (etkinlik, kullanici, secim, zaman) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(etkinlik, kullanici) DO UPDATE SET secim = ?3, zaman = ?4",
                params![etkinlik, ben, secim, simdi()],
            )
            .map_err(e)?;
        Ok(OySonuc::Tamam(self.anket_durum(ben, etkinlik, n)?))
    }
}

#[derive(Deserialize)]
pub struct OyIstek {
    secim: i64,
}

/// POST /api/akis/{id}/oy {secim}
pub async fn oy(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<OyIstek>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let r = s
        .db()
        .oy_ver(&ben.id, id, i.secim)
        .map_err(|x| hata(StatusCode::INTERNAL_SERVER_ERROR, x))?;
    match r {
        OySonuc::Tamam(d) => {
            s.durum.yay(json!({ "tip": "akis", "id": id }));
            Ok(Json(json!({ "durum": d })))
        }
        OySonuc::Yok => Err(hata(StatusCode::NOT_FOUND, "Anket bulunamadı")),
        OySonuc::Bitti => Err(hata(StatusCode::GONE, "Anket bitti")),
        OySonuc::Gecersiz => Err(hata(StatusCode::BAD_REQUEST, "Geçersiz seçim")),
    }
}

// ---------------- profil görünümü ----------------

/// Profil özelleştirme: kapak resmi (önce /api/resim ile yüklenir), vurgu rengi, kısa tanıtım, vitrin (en çok
/// 3 oyun adı).
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Gorunum {
    #[serde(default)]
    pub kapak: Option<String>,
    #[serde(default)]
    pub renk: Option<String>,
    #[serde(default)]
    pub biyografi: String,
    #[serde(default)]
    pub vitrin: Vec<String>,
}

impl Gorunum {
    /// Geçersiz alanları atar, uzunlukları sınırlar.
    pub fn temizle(mut self) -> Gorunum {
        self.kapak = self
            .kapak
            .filter(|k| k.starts_with("/resim/") && k.len() <= 200 && !k.contains(".."));
        self.renk = self.renk.filter(|r| {
            r.len() == 7 && r.starts_with('#') && r[1..].chars().all(|c| c.is_ascii_hexdigit())
        });
        self.biyografi = self.biyografi.trim().chars().take(160).collect();
        self.vitrin = self
            .vitrin
            .into_iter()
            .map(|v| v.trim().chars().take(80).collect::<String>())
            .filter(|v| !v.is_empty())
            .take(3)
            .collect();
        self
    }
}

impl Db {
    pub fn gorunum(&self, id: &str) -> Gorunum {
        self.0
            .query_row(
                "SELECT veri FROM profil_gorunum WHERE kullanici = ?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or_default()
    }

    pub fn gorunum_kaydet(&self, id: &str, g: &Gorunum) -> R<()> {
        self.0
            .execute(
                "INSERT INTO profil_gorunum (kullanici, veri) VALUES (?1, ?2)
                 ON CONFLICT(kullanici) DO UPDATE SET veri = ?2",
                params![id, serde_json::to_string(g).map_err(|x| x.to_string())?],
            )
            .map_err(e)?;
        Ok(())
    }
}

/// PUT /api/profil/gorunum {kapak?, renk?, biyografi?, vitrin?}
pub async fn gorunum_kaydet(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Json(g): Json<Gorunum>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let g = g.temizle();
    s.db()
        .gorunum_kaydet(&ben.id, &g)
        .map_err(|x| hata(StatusCode::INTERNAL_SERVER_ERROR, x))?;
    Ok(Json(json!({ "gorunum": g })))
}

// ---------------- rozetler ----------------

#[derive(Serialize, Clone, Debug)]
pub struct Rozet {
    /// kurucu | sevilen | yildiz | gececi | sosyal | yazar | haftanin-klibi | sampiyon
    pub id: String,
    pub ad: String,
    pub aciklama: String,
    /// Kalıcı rozetlerde ne zaman alındı.
    pub zaman: Option<i64>,
    /// Kaç kez alındı (haftanın klibi 3 kez gibi).
    pub kez: i64,
}

fn rozet(id: &str, ad: &str, aciklama: impl Into<String>) -> Rozet {
    Rozet {
        id: id.into(),
        ad: ad.into(),
        aciklama: aciklama.into(),
        zaman: None,
        kez: 1,
    }
}

/// İlk bu kadar üye "kurucu üye" rozeti alır.
const KURUCU_SAYISI: i64 = 25;

impl Db {
    fn say(&self, sql: &str, id: &str) -> i64 {
        self.0.query_row(sql, [id], |r| r.get(0)).unwrap_or(0)
    }

    /// Kalıcı rozet ver (haftanın klibi, şampiyon). `notu`: neyle kazanıldı.
    pub fn rozet_ver(&self, id: &str, rozet: &str, notu: Option<&str>) -> R<()> {
        self.0
            .execute(
                "INSERT OR IGNORE INTO rozetler (kullanici, rozet, notu, zaman) VALUES (?1, ?2, ?3, ?4)",
                params![id, rozet, notu, simdi()],
            )
            .map_err(e)?;
        Ok(())
    }

    /// Kişinin rozetleri: kalıcılar (en yenisi önce) + hesaplananlar.
    pub fn rozetler(&self, id: &str) -> Vec<Rozet> {
        let mut l = Vec::new();
        if let Ok(mut st) = self.0.prepare(
            "SELECT rozet, COUNT(*), MAX(zaman), (SELECT notu FROM rozetler r2 WHERE r2.kullanici = r.kullanici
               AND r2.rozet = r.rozet ORDER BY zaman DESC LIMIT 1)
             FROM rozetler r WHERE kullanici = ?1 GROUP BY rozet ORDER BY MAX(zaman) DESC",
        ) {
            let satirlar = st.query_map([id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            });
            for (rz, kez, zaman, notu) in satirlar.into_iter().flatten().flatten() {
                let (ad, aciklama) = match rz.as_str() {
                    "haftanin-klibi" => ("Haftanın Klibi", "Haftanın en çok beğenilen klibi"),
                    "sampiyon" => ("Şampiyon", "Topluluk turnuvası kazandı"),
                    _ => ("Rozet", ""),
                };
                let mut r = rozet(&rz, ad, notu.map_or(aciklama.to_string(), |n| format!("{aciklama}: {n}")));
                r.zaman = Some(zaman);
                r.kez = kez;
                l.push(r);
            }
        }
        let sira = self.say(
            "SELECT COUNT(*) FROM kullanicilar WHERE ilk <= (SELECT ilk FROM kullanicilar WHERE id = ?1)",
            id,
        );
        if (1..=KURUCU_SAYISI).contains(&sira) {
            l.push(rozet(
                "kurucu",
                "Kurucu Üye",
                format!("GMID Tool'a ilk katılan {KURUCU_SAYISI} kişiden"),
            ));
        }
        let (begeni, _) = self.toplam_puan(id);
        if begeni >= 100 {
            l.push(rozet("yildiz", "Yıldız", "Paylaşımları 100+ beğeni aldı"));
        } else if begeni >= 25 {
            l.push(rozet("sevilen", "Sevilen", "Paylaşımları 25+ beğeni aldı"));
        }
        let gece = self.say(
            "SELECT COUNT(*) FROM geceler WHERE olusturan = ?1 AND hemen = 0",
            id,
        );
        if gece >= 3 {
            l.push(rozet(
                "gececi",
                "Gece Düzenleyicisi",
                format!("{gece} oyun gecesi planladı"),
            ));
        }
        let ark = self.arkadas_sayisi(id);
        if ark >= 10 {
            l.push(rozet("sosyal", "Sosyal", format!("{ark} arkadaşı var")));
        }
        let yazi = self.say(
            "SELECT COUNT(*) FROM etkinlikler WHERE kullanici = ?1 AND tur = 'gonderi'",
            id,
        );
        if yazi >= 20 {
            l.push(rozet("yazar", "Yazar", format!("{yazi} gönderi paylaştı")));
        }
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Kullanici;

    fn kisi(db: &Db, id: &str) {
        db.giris(&Kullanici {
            id: id.into(),
            ad: format!("K{id}"),
            avatar: None,
        })
        .unwrap();
    }

    #[test]
    fn anket_oylama() {
        let db = Db::bellekte();
        kisi(&db, "1");
        kisi(&db, "2");
        assert!(anket_verisi(&["Tek".into()], None).is_err());
        assert!(anket_verisi(&["a".into(), " ".into()], None).is_err());
        let a = anket_verisi(&["CS2".into(), "Valorant".into(), "LoL".into()], Some(2)).unwrap();
        let id = db
            .gonderi_yaz("1", "Bu akşam ne oynayalım?", None, Some(&a))
            .unwrap()
            .unwrap();
        assert_eq!(db.oy_ver("2", id, 3).unwrap(), OySonuc::Gecersiz);
        assert!(matches!(db.oy_ver("2", id, 1).unwrap(), OySonuc::Tamam(_)));
        // Fikir değiştirilebilir; kişi başı tek oy.
        let OySonuc::Tamam(d) = db.oy_ver("2", id, 0).unwrap() else {
            panic!()
        };
        assert_eq!(d.oylar, vec![1, 0, 0]);
        assert_eq!(d.benim, Some(0));
        let mut akis = db.akis("1", None, None, 10).unwrap();
        db.anketleri_doldur("1", &mut akis).unwrap();
        assert_eq!(akis[0].veri["anket"]["durum"]["oylar"], json!([1, 0, 0]));
        assert_eq!(akis[0].veri["anket"]["durum"]["benim"], Value::Null);
        // Anketsiz gönderiye oy verilemez.
        let duz = db.gonderi_yaz("1", "selam", None, None).unwrap().unwrap();
        assert_eq!(db.oy_ver("2", duz, 0).unwrap(), OySonuc::Yok);
        // Süre bitince kapanır.
        db.0.execute(
            "UPDATE etkinlikler SET veri = json_set(veri, '$.anket.bitis', 1) WHERE id = ?1",
            [id],
        )
        .unwrap();
        assert_eq!(db.oy_ver("2", id, 1).unwrap(), OySonuc::Bitti);
    }

    #[test]
    fn gorunum_temizlenir() {
        let db = Db::bellekte();
        kisi(&db, "1");
        assert_eq!(db.gorunum("1"), Gorunum::default());
        let g = Gorunum {
            kapak: Some("https://kotu.com/x.png".into()),
            renk: Some("#E3C270".into()),
            biyografi: "  CS2 & Valorant  ".into(),
            vitrin: vec![
                "CS2".into(),
                "".into(),
                "LoL".into(),
                "Valorant".into(),
                "Dota 2".into(),
            ],
        }
        .temizle();
        assert_eq!(g.kapak, None);
        assert_eq!(g.renk.as_deref(), Some("#E3C270"));
        assert_eq!(g.biyografi, "CS2 & Valorant");
        assert_eq!(g.vitrin, vec!["CS2", "LoL", "Valorant"]);
        assert!(Gorunum {
            renk: Some("red".into()),
            ..Default::default()
        }
        .temizle()
        .renk
        .is_none());
        db.gorunum_kaydet("1", &g).unwrap();
        assert_eq!(db.gorunum("1"), g);
    }

    #[test]
    fn rozetler() {
        let db = Db::bellekte();
        kisi(&db, "1");
        let r = db.rozetler("1");
        assert!(r.iter().any(|x| x.id == "kurucu"));
        db.rozet_ver("1", "haftanin-klibi", Some("Ace")).unwrap();
        let r = db.rozetler("1");
        assert_eq!(r[0].id, "haftanin-klibi");
        assert!(r[0].aciklama.contains("Ace"));
    }
}
