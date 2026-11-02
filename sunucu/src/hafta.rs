//! Haftalık işler (her pazartesi 12:00, Türkiye saati): **haftanın klibi** (geçen hafta paylaşılıp en çok beğeni
//! alan klip; sahibine kalıcı rozet ve bildirim) ve **haftalık GMID özeti** (akışa "GMID" adına bir kart:
//! kaç kişi aktifti, en çok oynanan oyunlar, haftanın klibi, en beğenilen gönderi…). `GMID_DISCORD_WEBHOOK`
//! verilmişse özet Discord kanalına da yazılır.

use crate::db::{simdi, Db, Kullanici};
use crate::Sunucu;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

/// Özeti yazan sistem hesabı (akışta "GMID" olarak görünür; üye listelerinde yoktur).
pub const SISTEM: &str = "gmid";
const GUN: i64 = 24 * 3600 * 1000;
/// Türkiye saati (UTC+3, yaz saati yok).
const TR: i64 = 3 * 3600 * 1000;

pub fn kur(c: &Connection) -> rusqlite::Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS haftalik (
           hafta INTEGER PRIMARY KEY, zaman INTEGER NOT NULL, veri TEXT NOT NULL);",
    )
}

type R<T> = Result<T, String>;

fn e(x: rusqlite::Error) -> String {
    x.to_string()
}

/// `t` anının içinde bulunduğu haftanın pazartesi 12:00'si (TR) ve haftanın numarası (o pazartesinin gün sırası).
pub fn bu_haftanin_ogleni(t: i64) -> (i64, i64) {
    let gun = (t + TR).div_euclid(GUN);
    // 1970-01-01 perşembe: pazartesi = 0.
    let hafta_gunu = (gun + 3).rem_euclid(7);
    let pazartesi = gun - hafta_gunu;
    (pazartesi * GUN - TR + 12 * 3600 * 1000, pazartesi)
}

impl Db {
    /// Sistem hesabını (yoksa) açar. `ilk` çok büyük: "ilk üye" (yönetici, kurucu) sayılmaz.
    fn sistem_hesabi(&self) -> R<()> {
        self.0
            .execute(
                "INSERT OR IGNORE INTO kullanicilar (id, ad, avatar, ilk, son) VALUES (?1, 'GMID', NULL, ?2, ?2)",
                params![SISTEM, i64::MAX / 2],
            )
            .map_err(e)?;
        Ok(())
    }

    /// [bas, son) aralığının özeti. Haftanın klibi varsa `klip` alanında.
    pub fn hafta_ozeti(&self, bas: i64, son: i64) -> R<Value> {
        let say = |sql: &str| -> R<i64> {
            self.0
                .query_row(sql, params![bas, son], |r| r.get(0))
                .map_err(e)
        };
        let aktif = say(
            "SELECT COUNT(DISTINCT kullanici) FROM etkinlikler WHERE zaman >= ?1 AND zaman < ?2 AND kullanici != 'gmid'",
        )?;
        let gonderi = say(
            "SELECT COUNT(*) FROM etkinlikler WHERE tur = 'gonderi' AND zaman >= ?1 AND zaman < ?2",
        )?;
        let klip = say(
            "SELECT COUNT(*) FROM etkinlikler WHERE tur = 'klip' AND zaman >= ?1 AND zaman < ?2",
        )?;
        let gece = say("SELECT COUNT(*) FROM geceler WHERE olusturma >= ?1 AND olusturma < ?2")?;
        // En çok kişinin oynadığı oyunlar.
        let mut st = self
            .0
            .prepare(
                "SELECT json_extract(veri, '$.ad') AS ad, COUNT(DISTINCT kullanici) AS n FROM etkinlikler
                 WHERE tur = 'oyun' AND zaman >= ?1 AND zaman < ?2 AND ad IS NOT NULL
                 GROUP BY lower(ad) ORDER BY n DESC, ad LIMIT 3",
            )
            .map_err(e)?;
        let oyunlar: Vec<Value> = st
            .query_map(params![bas, son], |r| {
                Ok(json!({ "ad": r.get::<_, String>(0)?, "kisi": r.get::<_, i64>(1)? }))
            })
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        let en = |tur: &str| -> R<Option<Value>> {
            self.0
                .query_row(
                    "SELECT e.id, k.id, k.ad, k.avatar, e.veri,
                       (SELECT COUNT(*) FROM begeniler b WHERE b.etkinlik = e.id) AS n
                     FROM etkinlikler e JOIN kullanicilar k ON k.id = e.kullanici
                     WHERE e.tur = ?3 AND e.zaman >= ?1 AND e.zaman < ?2
                     ORDER BY n DESC, e.zaman LIMIT 1",
                    params![bas, son, tur],
                    |r| {
                        let veri: Value =
                            serde_json::from_str(&r.get::<_, String>(4)?).unwrap_or(Value::Null);
                        Ok((
                            r.get::<_, i64>(0)?,
                            Kullanici {
                                id: r.get(1)?,
                                ad: r.get(2)?,
                                avatar: r.get(3)?,
                            },
                            veri,
                            r.get::<_, i64>(5)?,
                        ))
                    },
                )
                .optional()
                .map_err(e)
                .map(|o| {
                    o.filter(|x| x.3 > 0).map(|(id, kisi, veri, n)| {
                        json!({ "etkinlik": id, "kisi": kisi, "begeni": n,
                                "ad": veri["oyun"].as_str().or(veri["ad"].as_str()).unwrap_or(""),
                                "metin": veri["metin"].as_str().or(veri["not"].as_str()).unwrap_or("") })
                    })
                })
        };
        Ok(json!({
            "ad": "Haftalık GMID özeti",
            "baslangic": bas, "bitis": son,
            "aktif": aktif, "gonderi": gonderi, "klip_sayisi": klip, "gece": gece,
            "oyunlar": oyunlar,
            "klip": en("klip")?,
            "gonderi_en": en("gonderi")?,
        }))
    }

    /// Bu haftanın özeti yazılmadıysa yazar; yazdıysa `None`. Dönen: (özet, akış kimliği).
    pub fn haftalik_calis(&self, t: i64) -> R<Option<(Value, i64)>> {
        let (oglen, hafta) = bu_haftanin_ogleni(t);
        if t < oglen {
            return Ok(None);
        }
        let var: bool = self
            .0
            .query_row("SELECT 1 FROM haftalik WHERE hafta = ?1", [hafta], |_| {
                Ok(())
            })
            .optional()
            .map_err(e)?
            .is_some();
        if var {
            return Ok(None);
        }
        let ozet = self.hafta_ozeti(oglen - 7 * GUN, oglen)?;
        self.0
            .execute(
                "INSERT INTO haftalik (hafta, zaman, veri) VALUES (?1, ?2, ?3)",
                params![hafta, t, ozet.to_string()],
            )
            .map_err(e)?;
        // Hiç hareket olmayan haftada akışa boş kart düşmesin.
        if ozet["aktif"].as_i64().unwrap_or(0) == 0 {
            return Ok(None);
        }
        if let Some(k) = ozet["klip"].as_object() {
            if let Some(kid) = k.get("kisi").and_then(|x| x["id"].as_str()) {
                let ad = k.get("ad").and_then(Value::as_str).unwrap_or("");
                self.rozet_ver(kid, "haftanin-klibi", (!ad.is_empty()).then_some(ad))?;
            }
        }
        self.sistem_hesabi()?;
        let id = self.etkinlik_yaz(SISTEM, "ozet", &format!("ozet:{hafta}"), &ozet, t)?;
        Ok(Some((ozet, id)))
    }
}

/// Discord'a yazılacak düz metin.
fn discord_metni(o: &Value) -> String {
    let mut s = String::from("**Haftalık GMID özeti** 🏰\n");
    s += &format!(
        "Bu hafta {} kişi aktifti · {} gönderi · {} klip · {} oyun gecesi\n",
        o["aktif"], o["gonderi"], o["klip_sayisi"], o["gece"]
    );
    if let Some(l) = o["oyunlar"].as_array().filter(|l| !l.is_empty()) {
        let ad: Vec<String> = l
            .iter()
            .map(|x| format!("{} ({} kişi)", x["ad"].as_str().unwrap_or(""), x["kisi"]))
            .collect();
        s += &format!("En çok oynanan: {}\n", ad.join(", "));
    }
    if let Some(k) = o["klip"].as_object() {
        s += &format!(
            "Haftanın klibi: {} · {} ({} beğeni)\n",
            k.get("kisi").and_then(|x| x["ad"].as_str()).unwrap_or(""),
            k.get("ad").and_then(Value::as_str).unwrap_or(""),
            k.get("begeni").cloned().unwrap_or_default()
        );
    }
    s + "Ayrıntılar GMID Tool → Sosyal'de."
}

/// Dakikada bir çağrılır; zamanı geldiyse haftalık işi yapar.
pub fn kontrol(s: &Sunucu) {
    let r = s.db().haftalik_calis(simdi());
    match r {
        Ok(Some((ozet, _))) => {
            s.durum.yay(json!({ "tip": "akis" }));
            if let Some(k) = ozet["klip"].as_object() {
                if let Some(kisi) = k.get("kisi") {
                    if let Some(kid) = kisi["id"].as_str() {
                        s.durum.kisiye(
                            kid,
                            &json!({ "tip": "bildirim", "tur": "haftanin-klibi", "kisi": kisi,
                                     "metin": k.get("ad").cloned().unwrap_or_default() }),
                        );
                    }
                }
            }
            if let Ok(url) = std::env::var("GMID_DISCORD_WEBHOOK") {
                if url.starts_with("https://discord.com/api/webhooks/")
                    || url.starts_with("https://discordapp.com/api/webhooks/")
                {
                    let _ = ureq::post(&url).send_json(json!({ "content": discord_metni(&ozet) }));
                }
            }
        }
        Ok(None) => {}
        Err(x) => eprintln!("haftalık özet: {x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kisi(db: &Db, id: &str) {
        db.giris(&Kullanici {
            id: id.into(),
            ad: format!("K{id}"),
            avatar: None,
        })
        .unwrap();
    }

    #[test]
    fn pazartesi_oglen() {
        // 2026-11-02 pazartesi 12:00 TR = 09:00 UTC = 1793610000000.
        let oglen = 1_793_610_000_000;
        assert_eq!(bu_haftanin_ogleni(oglen).0, oglen);
        assert_eq!(bu_haftanin_ogleni(oglen + 3 * GUN).0, oglen);
        // Pazartesi sabahı hâlâ bu haftanın öğleni (henüz gelmedi).
        assert_eq!(bu_haftanin_ogleni(oglen - 3_600_000).0, oglen);
        // Pazar gecesi bir önceki hafta.
        assert_eq!(
            bu_haftanin_ogleni(oglen - 13 * 3_600_000).0,
            oglen - 7 * GUN
        );
    }

    #[test]
    fn haftalik_ozet_ve_klip() {
        let db = Db::bellekte();
        kisi(&db, "1");
        kisi(&db, "2");
        let (oglen, _) = bu_haftanin_ogleni(simdi());
        let t = oglen - GUN;
        let k = db
            .klip_paylas(
                "1",
                "abc",
                &json!({ "ad": "Ace", "oyun": "CS2", "url": "/klip/a.mp4" }),
            )
            .unwrap();
        db.0.execute(
            "UPDATE etkinlikler SET zaman = ?1 WHERE id = ?2",
            params![t, k],
        )
        .unwrap();
        db.begen("2", k).unwrap();
        let g = db.gonderi_yaz("2", "selam", None, None).unwrap().unwrap();
        db.0.execute(
            "UPDATE etkinlikler SET zaman = ?1 WHERE id = ?2",
            params![t, g],
        )
        .unwrap();
        // Öğlenden önce çalışmaz.
        assert!(db.haftalik_calis(oglen - 1).unwrap().is_none());
        let (o, _) = db.haftalik_calis(oglen + 1).unwrap().unwrap();
        assert_eq!(o["aktif"], 2);
        assert_eq!(o["klip"]["kisi"]["id"], "1");
        assert_eq!(o["klip"]["ad"], "CS2");
        // Beğeni almamış gönderi "en beğenilen" olmaz.
        assert!(o["gonderi_en"].is_null());
        // Bir hafta bir kez.
        assert!(db.haftalik_calis(oglen + 2).unwrap().is_none());
        assert!(db.rozetler("1").iter().any(|r| r.id == "haftanin-klibi"));
        // Akışta "GMID" kartı; sistem hesabı kurucu sayılmaz, üye listesinde yok.
        let akis = db.akis("1", None, None, 10).unwrap();
        assert_eq!(akis[0].tur, "ozet");
        assert!(!db.kisiler().unwrap().iter().any(|(k, _)| k.id == SISTEM));
        assert!(discord_metni(&o).contains("Haftanın klibi"));
    }
}
