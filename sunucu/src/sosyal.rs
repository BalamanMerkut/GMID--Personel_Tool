//! Sosyal veriler: profiller (kişinin paylaştığı oyunlar ve puanlar), son olaylar akışı (beğeni, yorum),
//! özel mesajlar. Neyin paylaşılacağını herkes GMID Tool'da kendisi seçer; buraya yalnız seçtikleri gelir.

use crate::db::{simdi, Db, Kullanici};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Yeni (ya da değişen) puan bu kadar eskiyse akışa düşmez (eski puanlar ilk paylaşımda akışı doldurmasın).
const TAZE_MS: i64 = 7 * 24 * 3600 * 1000;
/// Aynı oyunu yeniden açmak bu süre geçmeden akışa yeni olay olarak düşmez.
const OYUN_ARA_MS: i64 = 12 * 3600 * 1000;
pub const METIN_SINIRI: usize = 1000;

pub fn kur(c: &Connection) -> rusqlite::Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS profiller (
           kullanici TEXT PRIMARY KEY REFERENCES kullanicilar(id) ON DELETE CASCADE,
           veri TEXT NOT NULL, guncelleme INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS etkinlikler (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           kullanici TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           tur TEXT NOT NULL, anahtar TEXT NOT NULL, veri TEXT NOT NULL, zaman INTEGER NOT NULL,
           UNIQUE (kullanici, anahtar));
         CREATE INDEX IF NOT EXISTS etkinlik_zaman ON etkinlikler (zaman);
         CREATE TABLE IF NOT EXISTS begeniler (
           etkinlik INTEGER NOT NULL REFERENCES etkinlikler(id) ON DELETE CASCADE,
           kullanici TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           zaman INTEGER NOT NULL, PRIMARY KEY (etkinlik, kullanici));
         CREATE TABLE IF NOT EXISTS yorumlar (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           etkinlik INTEGER NOT NULL REFERENCES etkinlikler(id) ON DELETE CASCADE,
           kullanici TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           metin TEXT NOT NULL, zaman INTEGER NOT NULL);
         CREATE INDEX IF NOT EXISTS yorum_etkinlik ON yorumlar (etkinlik, id);
         CREATE TABLE IF NOT EXISTS mesajlar (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           kimden TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           kime TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           metin TEXT NOT NULL, zaman INTEGER NOT NULL, okundu INTEGER NOT NULL DEFAULT 0);
         CREATE INDEX IF NOT EXISTS mesaj_kime ON mesajlar (kime, okundu);
         CREATE INDEX IF NOT EXISTS mesaj_kimden ON mesajlar (kimden, kime, id);",
    )
}

/// Kişinin paylaştığı oyunlardan biri.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Oyun {
    pub ad: String,
    #[serde(default)]
    pub kapak: Option<String>,
    /// Toplam oynama (dakika).
    #[serde(default)]
    pub dakika: i64,
    /// Son oynama (ms).
    #[serde(default)]
    pub son: Option<i64>,
}

/// Paylaşılan bir puan (oyun, film ya da dizi).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Puan {
    /// GMID Tool'daki kimliği ("game:…", "title:…"): aynı şeyin güncellenmesini tanımak için.
    pub anahtar: String,
    /// game | movie | series
    pub tur: String,
    pub ad: String,
    #[serde(default)]
    pub kapak: Option<String>,
    #[serde(default)]
    pub puan: Option<f64>,
    /// Tek cümlelik yorum.
    #[serde(default)]
    pub yorum: Option<String>,
    /// playing | finished | dropped | wishlist … (GMID Tool'daki durum)
    #[serde(default)]
    pub durum: Option<String>,
    #[serde(default)]
    pub yil: Option<i64>,
    /// Son değişiklik (ms).
    #[serde(default)]
    pub guncelleme: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Profil {
    #[serde(default)]
    pub oyunlar: Vec<Oyun>,
    #[serde(default)]
    pub puanlar: Vec<Puan>,
}

impl Profil {
    /// Boyut sınırları (kötüye kullanım ya da hatalı istemci sunucuyu şişirmesin).
    pub fn sinirla(mut self) -> Profil {
        self.oyunlar.truncate(2000);
        self.puanlar.truncate(3000);
        let kisa = |s: &mut String, n: usize| {
            if s.chars().count() > n {
                *s = s.chars().take(n).collect();
            }
        };
        for o in &mut self.oyunlar {
            kisa(&mut o.ad, 120);
        }
        for p in &mut self.puanlar {
            kisa(&mut p.ad, 160);
            kisa(&mut p.anahtar, 120);
            if let Some(y) = &mut p.yorum {
                kisa(y, 500);
            }
        }
        self
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Yorum {
    pub id: i64,
    pub kisi: Kullanici,
    pub metin: String,
    pub zaman: i64,
}

#[derive(Serialize, Clone, Debug)]
pub struct Etkinlik {
    pub id: i64,
    pub kisi: Kullanici,
    /// puan | oyun
    pub tur: String,
    pub veri: Value,
    pub zaman: i64,
    pub begeni: i64,
    pub begendim: bool,
    pub yorumlar: Vec<Yorum>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Mesaj {
    pub id: i64,
    pub kimden: String,
    pub kime: String,
    pub metin: String,
    pub zaman: i64,
    pub okundu: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Konusma {
    pub kisi: Kullanici,
    pub son: Mesaj,
    pub okunmamis: i64,
}

fn kullanici(r: &rusqlite::Row, i: usize) -> rusqlite::Result<Kullanici> {
    Ok(Kullanici {
        id: r.get(i)?,
        ad: r.get(i + 1)?,
        avatar: r.get(i + 2)?,
    })
}

fn mesaj(r: &rusqlite::Row) -> rusqlite::Result<Mesaj> {
    Ok(Mesaj {
        id: r.get(0)?,
        kimden: r.get(1)?,
        kime: r.get(2)?,
        metin: r.get(3)?,
        zaman: r.get(4)?,
        okundu: r.get::<_, i64>(5)? != 0,
    })
}

/// Metni temizler: baştaki/sondaki boşluk atılır, boşsa ya da çok uzunsa `None`.
pub fn metin(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty() && s.chars().count() <= METIN_SINIRI).then(|| s.to_string())
}

type R<T> = Result<T, String>;

fn e(x: rusqlite::Error) -> String {
    x.to_string()
}

impl Db {
    pub fn kisiler(&self) -> R<Vec<(Kullanici, i64)>> {
        let mut st = self
            .0
            .prepare("SELECT id, ad, avatar, son FROM kullanicilar ORDER BY lower(ad)")
            .map_err(e)?;
        let l = st
            .query_map([], |r| Ok((kullanici(r, 0)?, r.get(3)?)))
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        Ok(l)
    }

    pub fn kullanici(&self, id: &str) -> Option<Kullanici> {
        self.0
            .query_row(
                "SELECT id, ad, avatar FROM kullanicilar WHERE id = ?1",
                [id],
                |r| kullanici(r, 0),
            )
            .optional()
            .ok()
            .flatten()
    }

    pub fn profil(&self, id: &str) -> Option<(Profil, i64)> {
        self.0
            .query_row(
                "SELECT veri, guncelleme FROM profiller WHERE kullanici = ?1",
                [id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
            )
            .optional()
            .ok()
            .flatten()
            .map(|(v, t)| (serde_json::from_str(&v).unwrap_or_default(), t))
    }

    /// Profili kaydeder; yeni ya da değişen taze puanlar akışa düşer. Dönen: akışa eklenen olay sayısı.
    pub fn profil_kaydet(&self, id: &str, yeni: Profil) -> R<usize> {
        let t = simdi();
        let eski = self.profil(id);
        let onceki: HashMap<&str, &Puan> = eski
            .as_ref()
            .map(|(p, _)| p.puanlar.iter().map(|x| (x.anahtar.as_str(), x)).collect())
            .unwrap_or_default();
        // İlk paylaşımda yalnız son 2 günün puanları (en çok 5); sonra son 7 günde değişenler.
        let sinir = if eski.is_some() {
            TAZE_MS
        } else {
            2 * 24 * 3600 * 1000
        };
        let mut olaylar: Vec<&Puan> = yeni
            .puanlar
            .iter()
            .filter(|p| p.guncelleme > t - sinir)
            .filter(|p| match onceki.get(p.anahtar.as_str()) {
                None => true,
                Some(o) => o.puan != p.puan || o.yorum != p.yorum || o.durum != p.durum,
            })
            .collect();
        olaylar.sort_by_key(|p| std::cmp::Reverse(p.guncelleme));
        if eski.is_none() {
            olaylar.truncate(5);
        }
        for p in &olaylar {
            self.etkinlik_yaz(
                id,
                "puan",
                &format!("puan:{}", p.anahtar),
                &json!(p),
                p.guncelleme.min(t),
            )?;
        }
        let veri = serde_json::to_string(&yeni).map_err(|x| x.to_string())?;
        self.0
            .execute(
                "INSERT INTO profiller (kullanici, veri, guncelleme) VALUES (?1, ?2, ?3)
                 ON CONFLICT(kullanici) DO UPDATE SET veri = ?2, guncelleme = ?3",
                params![id, veri, t],
            )
            .map_err(e)?;
        Ok(olaylar.len())
    }

    /// Aynı anahtarla olay varsa güncellenip başa alınır (beğeni ve yorumlar korunur).
    fn etkinlik_yaz(&self, id: &str, tur: &str, anahtar: &str, veri: &Value, zaman: i64) -> R<i64> {
        self.0
            .query_row(
                "INSERT INTO etkinlikler (kullanici, tur, anahtar, veri, zaman) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(kullanici, anahtar) DO UPDATE SET veri = ?4, zaman = ?5
                 RETURNING id",
                params![id, tur, anahtar, veri.to_string(), zaman],
                |r| r.get(0),
            )
            .map_err(e)
    }

    /// Oyuna girdi: aynı oyun 12 saat içinde yeniden akışa düşmez. Olay eklendiyse `true`.
    pub fn oyun_basladi(&self, id: &str, oyun: &str) -> R<bool> {
        let t = simdi();
        let anahtar = format!("oyun:{}", oyun.to_lowercase());
        let son: Option<i64> = self
            .0
            .query_row(
                "SELECT zaman FROM etkinlikler WHERE kullanici = ?1 AND anahtar = ?2",
                params![id, anahtar],
                |r| r.get(0),
            )
            .optional()
            .map_err(e)?;
        if son.is_some_and(|s| t - s < OYUN_ARA_MS) {
            return Ok(false);
        }
        let kapak = self.profil(id).and_then(|(p, _)| {
            p.oyunlar
                .into_iter()
                .find(|o| o.ad.eq_ignore_ascii_case(oyun))
                .and_then(|o| o.kapak)
        });
        self.etkinlik_yaz(
            id,
            "oyun",
            &anahtar,
            &json!({ "ad": oyun, "kapak": kapak }),
            t,
        )?;
        Ok(true)
    }

    pub fn etkinlik_sahibi(&self, etkinlik: i64) -> Option<String> {
        self.0
            .query_row(
                "SELECT kullanici FROM etkinlikler WHERE id = ?1",
                [etkinlik],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten()
    }

    /// Akış (en yeni önce). `once`: bu zamandan eskiler (sayfalama); `kisi`: yalnız bir kişinin olayları.
    pub fn akis(
        &self,
        ben: &str,
        once: Option<i64>,
        kisi: Option<&str>,
        sayi: i64,
    ) -> R<Vec<Etkinlik>> {
        let mut st = self
            .0
            .prepare(
                "SELECT e.id, k.id, k.ad, k.avatar, e.tur, e.veri, e.zaman,
                   (SELECT COUNT(*) FROM begeniler b WHERE b.etkinlik = e.id),
                   EXISTS (SELECT 1 FROM begeniler b WHERE b.etkinlik = e.id AND b.kullanici = ?1)
                 FROM etkinlikler e JOIN kullanicilar k ON k.id = e.kullanici
                 WHERE (?2 IS NULL OR e.zaman < ?2) AND (?3 IS NULL OR e.kullanici = ?3)
                 ORDER BY e.zaman DESC, e.id DESC LIMIT ?4",
            )
            .map_err(e)?;
        let mut l: Vec<Etkinlik> = st
            .query_map(params![ben, once, kisi, sayi.clamp(1, 100)], |r| {
                Ok(Etkinlik {
                    id: r.get(0)?,
                    kisi: kullanici(r, 1)?,
                    tur: r.get(4)?,
                    veri: serde_json::from_str(&r.get::<_, String>(5)?).unwrap_or(Value::Null),
                    zaman: r.get(6)?,
                    begeni: r.get(7)?,
                    begendim: r.get(8)?,
                    yorumlar: Vec::new(),
                })
            })
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        let mut yst = self
            .0
            .prepare(
                "SELECT y.id, k.id, k.ad, k.avatar, y.metin, y.zaman FROM yorumlar y
                 JOIN kullanicilar k ON k.id = y.kullanici WHERE y.etkinlik = ?1 ORDER BY y.id",
            )
            .map_err(e)?;
        for et in &mut l {
            et.yorumlar = yst
                .query_map([et.id], |r| {
                    Ok(Yorum {
                        id: r.get(0)?,
                        kisi: kullanici(r, 1)?,
                        metin: r.get(4)?,
                        zaman: r.get(5)?,
                    })
                })
                .map_err(e)?
                .filter_map(Result::ok)
                .collect();
        }
        Ok(l)
    }

    /// Beğeniyi açar/kapatır: (şimdi beğenildi mi, toplam beğeni).
    pub fn begen(&self, ben: &str, etkinlik: i64) -> R<(bool, i64)> {
        let eklendi = self
            .0
            .execute(
                "INSERT OR IGNORE INTO begeniler (etkinlik, kullanici, zaman) VALUES (?1, ?2, ?3)",
                params![etkinlik, ben, simdi()],
            )
            .map_err(e)?
            > 0;
        if !eklendi {
            self.0
                .execute(
                    "DELETE FROM begeniler WHERE etkinlik = ?1 AND kullanici = ?2",
                    params![etkinlik, ben],
                )
                .map_err(e)?;
        }
        let n = self
            .0
            .query_row(
                "SELECT COUNT(*) FROM begeniler WHERE etkinlik = ?1",
                [etkinlik],
                |r| r.get(0),
            )
            .map_err(e)?;
        Ok((eklendi, n))
    }

    pub fn yorum_yaz(&self, ben: &str, etkinlik: i64, metin: &str) -> R<i64> {
        self.0
            .query_row(
                "INSERT INTO yorumlar (etkinlik, kullanici, metin, zaman) VALUES (?1, ?2, ?3, ?4) RETURNING id",
                params![etkinlik, ben, metin, simdi()],
                |r| r.get(0),
            )
            .map_err(e)
    }

    /// Yalnız kendi yorumu ya da kendi olayının altındaki yorum silinebilir.
    pub fn yorum_sil(&self, ben: &str, id: i64) -> R<bool> {
        let n = self
            .0
            .execute(
                "DELETE FROM yorumlar WHERE id = ?1 AND (kullanici = ?2 OR etkinlik IN
                   (SELECT id FROM etkinlikler WHERE kullanici = ?2))",
                params![id, ben],
            )
            .map_err(e)?;
        Ok(n > 0)
    }

    pub fn etkinlik_sil(&self, ben: &str, id: i64) -> R<bool> {
        let n = self
            .0
            .execute(
                "DELETE FROM etkinlikler WHERE id = ?1 AND kullanici = ?2",
                params![id, ben],
            )
            .map_err(e)?;
        Ok(n > 0)
    }

    pub fn mesaj_yaz(&self, kimden: &str, kime: &str, metin: &str) -> R<Mesaj> {
        let t = simdi();
        let id = self
            .0
            .query_row(
                "INSERT INTO mesajlar (kimden, kime, metin, zaman) VALUES (?1, ?2, ?3, ?4) RETURNING id",
                params![kimden, kime, metin, t],
                |r| r.get(0),
            )
            .map_err(e)?;
        Ok(Mesaj {
            id,
            kimden: kimden.into(),
            kime: kime.into(),
            metin: metin.into(),
            zaman: t,
            okundu: false,
        })
    }

    /// İki kişi arasındaki mesajlar (eskiden yeniye); `once`: bu kimlikten eskiler.
    pub fn mesajlar(&self, ben: &str, diger: &str, once: Option<i64>, sayi: i64) -> R<Vec<Mesaj>> {
        let mut st = self
            .0
            .prepare(
                "SELECT id, kimden, kime, metin, zaman, okundu FROM mesajlar
                 WHERE ((kimden = ?1 AND kime = ?2) OR (kimden = ?2 AND kime = ?1)) AND (?3 IS NULL OR id < ?3)
                 ORDER BY id DESC LIMIT ?4",
            )
            .map_err(e)?;
        let mut l: Vec<Mesaj> = st
            .query_map(params![ben, diger, once, sayi.clamp(1, 200)], mesaj)
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        l.reverse();
        Ok(l)
    }

    /// Bana gelen mesajları okundu yapar; kaç tane.
    pub fn okundu(&self, ben: &str, diger: &str) -> R<usize> {
        self.0
            .execute(
                "UPDATE mesajlar SET okundu = 1 WHERE kime = ?1 AND kimden = ?2 AND okundu = 0",
                params![ben, diger],
            )
            .map_err(e)
    }

    pub fn konusmalar(&self, ben: &str) -> R<Vec<Konusma>> {
        let mut st = self
            .0
            .prepare(
                "SELECT CASE WHEN kimden = ?1 THEN kime ELSE kimden END AS diger, MAX(id) AS son
                 FROM mesajlar WHERE kimden = ?1 OR kime = ?1 GROUP BY diger ORDER BY son DESC",
            )
            .map_err(e)?;
        let ciftler: Vec<(String, i64)> = st
            .query_map([ben], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        let mut l = Vec::new();
        for (diger, son) in ciftler {
            let Some(kisi) = self.kullanici(&diger) else {
                continue;
            };
            let son = self
                .0
                .query_row(
                    "SELECT id, kimden, kime, metin, zaman, okundu FROM mesajlar WHERE id = ?1",
                    [son],
                    mesaj,
                )
                .map_err(e)?;
            let okunmamis = self
                .0
                .query_row(
                    "SELECT COUNT(*) FROM mesajlar WHERE kime = ?1 AND kimden = ?2 AND okundu = 0",
                    params![ben, diger],
                    |r| r.get(0),
                )
                .map_err(e)?;
            l.push(Konusma {
                kisi,
                son,
                okunmamis,
            });
        }
        Ok(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kisi(db: &Db, id: &str, ad: &str) -> Kullanici {
        let k = Kullanici {
            id: id.into(),
            ad: ad.into(),
            avatar: None,
        };
        db.giris(&k).unwrap();
        k
    }

    fn puan(anahtar: &str, ad: &str, p: f64, ne_zaman: i64) -> Puan {
        Puan {
            anahtar: anahtar.into(),
            tur: "series".into(),
            ad: ad.into(),
            kapak: None,
            puan: Some(p),
            yorum: Some("Efsane".into()),
            durum: Some("finished".into()),
            yil: None,
            guncelleme: ne_zaman,
        }
    }

    #[test]
    fn profil_ve_akis() {
        let db = Db::bellekte();
        kisi(&db, "1", "Emirhan");
        kisi(&db, "2", "Ali");
        let t = simdi();
        // İlk paylaşım: eski puan akışa düşmez, yeni olan düşer.
        let p = Profil {
            oyunlar: vec![Oyun {
                ad: "CS2".into(),
                kapak: Some("k.jpg".into()),
                dakika: 600,
                son: None,
            }],
            puanlar: vec![
                puan("title:a", "Aykut Enişte", 8.5, t - 1000),
                puan("title:b", "Eski Film", 7.0, t - 30 * 24 * 3600 * 1000),
            ],
        };
        assert_eq!(db.profil_kaydet("1", p.clone()).unwrap(), 1);
        // Aynı profil yeniden: yeni olay yok.
        assert_eq!(db.profil_kaydet("1", p.clone()).unwrap(), 0);
        // Puan değişti: aynı olay güncellenir (yeni satır değil).
        let mut p2 = p.clone();
        p2.puanlar[0].puan = Some(9.0);
        assert_eq!(db.profil_kaydet("1", p2).unwrap(), 1);
        let akis = db.akis("2", None, None, 30).unwrap();
        assert_eq!(akis.len(), 1);
        assert_eq!(akis[0].kisi.ad, "Emirhan");
        assert_eq!(akis[0].veri["ad"], "Aykut Enişte");
        assert_eq!(akis[0].veri["puan"], 9.0);

        // Beğeni aç/kapa, yorum.
        let id = akis[0].id;
        assert_eq!(db.begen("2", id).unwrap(), (true, 1));
        assert_eq!(db.begen("2", id).unwrap(), (false, 0));
        db.begen("2", id).unwrap();
        let y = db.yorum_yaz("2", id, "Katılıyorum").unwrap();
        let akis = db.akis("2", None, None, 30).unwrap();
        assert!(akis[0].begendim);
        assert_eq!(akis[0].yorumlar[0].metin, "Katılıyorum");
        assert!(!db.akis("1", None, None, 30).unwrap()[0].begendim);
        // Başkasının yorumunu silemez; olay sahibi silebilir.
        kisi(&db, "3", "Veli");
        assert!(!db.yorum_sil("3", y).unwrap());
        assert!(db.yorum_sil("1", y).unwrap());

        // Oyun: 12 saatte bir kez; kapak profilden.
        assert!(db.oyun_basladi("1", "cs2").unwrap());
        assert!(!db.oyun_basladi("1", "CS2").unwrap());
        let akis = db.akis("2", None, Some("1"), 30).unwrap();
        assert_eq!(akis[0].tur, "oyun");
        assert_eq!(akis[0].veri["kapak"], "k.jpg");
        // Sayfalama.
        let ikinci = db.akis("2", Some(akis[0].zaman), None, 30).unwrap();
        assert!(ikinci.iter().all(|x| x.zaman < akis[0].zaman));
    }

    #[test]
    fn ilk_paylasim_en_cok_bes_olay() {
        let db = Db::bellekte();
        kisi(&db, "1", "A");
        let t = simdi();
        let p = Profil {
            oyunlar: vec![],
            puanlar: (0..12)
                .map(|i| puan(&format!("game:{i}"), "x", 5.0, t - i * 1000))
                .collect(),
        };
        assert_eq!(db.profil_kaydet("1", p).unwrap(), 5);
    }

    #[test]
    fn ozel_mesajlar() {
        let db = Db::bellekte();
        kisi(&db, "1", "Ali");
        kisi(&db, "2", "Veli");
        kisi(&db, "3", "Can");
        db.mesaj_yaz("1", "2", "selam").unwrap();
        db.mesaj_yaz("2", "1", "as").unwrap();
        db.mesaj_yaz("3", "2", "naber").unwrap();
        let k = db.konusmalar("2").unwrap();
        assert_eq!(k.len(), 2);
        assert_eq!(k[0].kisi.ad, "Can");
        assert_eq!(k[0].okunmamis, 1);
        assert_eq!(k[1].okunmamis, 1);
        let m = db.mesajlar("2", "1", None, 50).unwrap();
        assert_eq!(
            m.iter().map(|x| x.metin.as_str()).collect::<Vec<_>>(),
            ["selam", "as"]
        );
        // Üçüncü kişi başkalarının konuşmasını göremez.
        assert!(db.mesajlar("3", "1", None, 50).unwrap().is_empty());
        assert_eq!(db.okundu("2", "1").unwrap(), 1);
        assert_eq!(db.konusmalar("2").unwrap()[1].okunmamis, 0);
    }

    #[test]
    fn metin_temizlenir() {
        assert_eq!(metin("  merhaba "), Some("merhaba".into()));
        assert_eq!(metin("   "), None);
        assert_eq!(metin(&"a".repeat(METIN_SINIRI + 1)), None);
    }
}
