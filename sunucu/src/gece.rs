//! Oyun gecesi planlayıcı: biri "Cuma 21:00, CS2, 5 kişi" diye bir gece açar; akışa kart düşer, arkadaşlarına
//! davet bildirimi gider. Herkes Geliyorum / Belki / Gelemem der (kişi sınırı doluysa "Geliyorum" kapanır).
//! Başlamadan 15 dakika önce gelenlere ve belki diyenlere hatırlatma gider.

use crate::api::{hata, kim, Sonuc};
use crate::db::{simdi, Db, Kullanici};
use crate::Sunucu;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

/// Hatırlatma, başlangıçtan bu kadar önce.
pub const HATIRLAT_MS: i64 = 15 * 60 * 1000;
/// Başlamış gece bu kadar süre daha "yaklaşanlar"da görünür (o an oynanıyor).
const SURUYOR_MS: i64 = 3 * 3600 * 1000;
/// Bir kişinin aynı anda en çok bu kadar yaklaşan gecesi olabilir.
const KISI_BASI: i64 = 5;

pub fn kur(c: &Connection) -> rusqlite::Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS geceler (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           olusturan TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           oyun TEXT NOT NULL, zaman INTEGER NOT NULL, kontenjan INTEGER, notu TEXT,
           olusturma INTEGER NOT NULL, etkinlik INTEGER, hatirlatildi INTEGER NOT NULL DEFAULT 0);
         CREATE INDEX IF NOT EXISTS gece_zaman ON geceler (zaman);
         CREATE TABLE IF NOT EXISTS katilim (
           gece INTEGER NOT NULL REFERENCES geceler(id) ON DELETE CASCADE,
           kullanici TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           -- geliyor | belki | gelemem
           durum TEXT NOT NULL, zaman INTEGER NOT NULL,
           PRIMARY KEY (gece, kullanici));",
    )
}

#[derive(Serialize, Clone, Debug)]
pub struct Katilan {
    pub kisi: Kullanici,
    pub durum: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Gece {
    pub id: i64,
    pub olusturan: Kullanici,
    pub oyun: String,
    /// Başlangıç (ms).
    pub zaman: i64,
    pub kontenjan: Option<i64>,
    pub not: Option<String>,
    /// Akıştaki kartın kimliği.
    pub etkinlik: Option<i64>,
    pub katilanlar: Vec<Katilan>,
}

type R<T> = Result<T, String>;

fn e(x: rusqlite::Error) -> String {
    x.to_string()
}

#[derive(Debug, PartialEq)]
pub enum KatilSonuc {
    Tamam,
    Dolu,
    Yok,
    Bitti,
}

impl Db {
    pub fn gece_ac(
        &self,
        id: &str,
        oyun: &str,
        zaman: i64,
        kontenjan: Option<i64>,
        notu: Option<&str>,
    ) -> R<Option<i64>> {
        let t = simdi();
        let acik: i64 = self
            .0
            .query_row(
                "SELECT COUNT(*) FROM geceler WHERE olusturan = ?1 AND zaman > ?2",
                params![id, t],
                |r| r.get(0),
            )
            .map_err(e)?;
        if acik >= KISI_BASI {
            return Ok(None);
        }
        let gid: i64 = self
            .0
            .query_row(
                "INSERT INTO geceler (olusturan, oyun, zaman, kontenjan, notu, olusturma)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id",
                params![id, oyun, zaman, kontenjan, notu, t],
                |r| r.get(0),
            )
            .map_err(e)?;
        // Açan kişi gelir.
        self.0
            .execute(
                "INSERT INTO katilim (gece, kullanici, durum, zaman) VALUES (?1, ?2, 'geliyor', ?3)",
                params![gid, id, t],
            )
            .map_err(e)?;
        let veri = json!({ "ad": oyun, "gece": gid, "oyun": oyun, "zaman": zaman,
                           "kontenjan": kontenjan, "not": notu });
        let eid = self.etkinlik_yaz(id, "gece", &format!("gece:{gid}"), &veri, t)?;
        self.0
            .execute(
                "UPDATE geceler SET etkinlik = ?2 WHERE id = ?1",
                params![gid, eid],
            )
            .map_err(e)?;
        Ok(Some(gid))
    }

    pub fn gece(&self, gid: i64) -> R<Option<Gece>> {
        let g = self
            .0
            .query_row(
                "SELECT g.id, k.id, k.ad, k.avatar, g.oyun, g.zaman, g.kontenjan, g.notu, g.etkinlik
                 FROM geceler g JOIN kullanicilar k ON k.id = g.olusturan WHERE g.id = ?1",
                [gid],
                gece_satiri,
            )
            .optional()
            .map_err(e)?;
        match g {
            Some(mut g) => {
                g.katilanlar = self.katilanlar(g.id)?;
                Ok(Some(g))
            }
            None => Ok(None),
        }
    }

    fn katilanlar(&self, gid: i64) -> R<Vec<Katilan>> {
        let mut st = self
            .0
            .prepare(
                "SELECT k.id, k.ad, k.avatar, c.durum FROM katilim c JOIN kullanicilar k ON k.id = c.kullanici
                 WHERE c.gece = ?1 ORDER BY c.zaman",
            )
            .map_err(e)?;
        let l = st
            .query_map([gid], |r| {
                Ok(Katilan {
                    kisi: Kullanici {
                        id: r.get(0)?,
                        ad: r.get(1)?,
                        avatar: r.get(2)?,
                    },
                    durum: r.get(3)?,
                })
            })
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        Ok(l)
    }

    /// Yaklaşan (ve şu an süren) geceler, en yakını önce.
    pub fn yaklasan_geceler(&self) -> R<Vec<Gece>> {
        let mut st = self
            .0
            .prepare(
                "SELECT g.id, k.id, k.ad, k.avatar, g.oyun, g.zaman, g.kontenjan, g.notu, g.etkinlik
                 FROM geceler g JOIN kullanicilar k ON k.id = g.olusturan
                 WHERE g.zaman > ?1 ORDER BY g.zaman LIMIT 30",
            )
            .map_err(e)?;
        let mut l: Vec<Gece> = st
            .query_map([simdi() - SURUYOR_MS], gece_satiri)
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        for g in &mut l {
            g.katilanlar = self.katilanlar(g.id)?;
        }
        Ok(l)
    }

    pub fn katil(&self, gid: i64, id: &str, durum: &str) -> R<KatilSonuc> {
        let Some(g) = self.gece(gid)? else {
            return Ok(KatilSonuc::Yok);
        };
        if g.zaman < simdi() - SURUYOR_MS {
            return Ok(KatilSonuc::Bitti);
        }
        if durum == "geliyor" {
            let gelen = g
                .katilanlar
                .iter()
                .filter(|k| k.durum == "geliyor" && k.kisi.id != id)
                .count() as i64;
            if g.kontenjan.is_some_and(|k| gelen >= k) {
                return Ok(KatilSonuc::Dolu);
            }
        }
        self.0
            .execute(
                "INSERT INTO katilim (gece, kullanici, durum, zaman) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(gece, kullanici) DO UPDATE SET durum = ?3",
                params![gid, id, durum, simdi()],
            )
            .map_err(e)?;
        Ok(KatilSonuc::Tamam)
    }

    /// Geceyi siler (akıştaki kartı da). Yalnız açan kişi ya da yönetici.
    pub fn gece_sil(&self, gid: i64) -> R<bool> {
        let eid: Option<Option<i64>> = self
            .0
            .query_row("SELECT etkinlik FROM geceler WHERE id = ?1", [gid], |r| {
                r.get(0)
            })
            .optional()
            .map_err(e)?;
        let Some(eid) = eid else {
            return Ok(false);
        };
        self.0
            .execute("DELETE FROM geceler WHERE id = ?1", [gid])
            .map_err(e)?;
        if let Some(eid) = eid {
            self.0
                .execute("DELETE FROM etkinlikler WHERE id = ?1", [eid])
                .map_err(e)?;
        }
        Ok(true)
    }

    pub fn gece_sahibi(&self, gid: i64) -> Option<String> {
        self.0
            .query_row("SELECT olusturan FROM geceler WHERE id = ?1", [gid], |r| {
                r.get(0)
            })
            .optional()
            .ok()
            .flatten()
    }

    /// Hatırlatma zamanı gelen geceler: (gece, hatırlatılacak kişiler). Bir kez hatırlatılır.
    pub fn hatirlatilacaklar(&self, simdi: i64) -> R<Vec<(Gece, Vec<String>)>> {
        let ids: Vec<i64> = {
            let mut st = self
                .0
                .prepare(
                    "SELECT id FROM geceler WHERE hatirlatildi = 0 AND zaman <= ?1 AND zaman > ?2",
                )
                .map_err(e)?;
            let l = st
                .query_map(params![simdi + HATIRLAT_MS, simdi - 60_000], |r| r.get(0))
                .map_err(e)?
                .filter_map(Result::ok)
                .collect();
            l
        };
        let mut l = Vec::new();
        for gid in ids {
            self.0
                .execute("UPDATE geceler SET hatirlatildi = 1 WHERE id = ?1", [gid])
                .map_err(e)?;
            if let Some(g) = self.gece(gid)? {
                let kisiler = g
                    .katilanlar
                    .iter()
                    .filter(|k| k.durum != "gelemem")
                    .map(|k| k.kisi.id.clone())
                    .collect();
                l.push((g, kisiler));
            }
        }
        // Çok eski kaçırılmışlar bir daha denenmesin.
        self.0
            .execute(
                "UPDATE geceler SET hatirlatildi = 1 WHERE hatirlatildi = 0 AND zaman <= ?1",
                [simdi - 60_000],
            )
            .map_err(e)?;
        Ok(l)
    }
}

fn gece_satiri(r: &rusqlite::Row) -> rusqlite::Result<Gece> {
    Ok(Gece {
        id: r.get(0)?,
        olusturan: Kullanici {
            id: r.get(1)?,
            ad: r.get(2)?,
            avatar: r.get(3)?,
        },
        oyun: r.get(4)?,
        zaman: r.get(5)?,
        kontenjan: r.get(6)?,
        not: r.get(7)?,
        etkinlik: r.get(8)?,
        katilanlar: Vec::new(),
    })
}

fn ic(e: String) -> (StatusCode, Json<serde_json::Value>) {
    hata(StatusCode::INTERNAL_SERVER_ERROR, e)
}

#[derive(Deserialize)]
pub struct AcIstek {
    oyun: String,
    zaman: i64,
    kontenjan: Option<i64>,
    not: Option<String>,
}

/// POST /api/gece: yeni oyun gecesi; arkadaşlara davet bildirimi.
pub async fn ac(State(s): State<Arc<Sunucu>>, h: HeaderMap, Json(i): Json<AcIstek>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let oyun: String = i.oyun.trim().chars().take(80).collect();
    if oyun.is_empty() {
        return Err(hata(StatusCode::BAD_REQUEST, "Hangi oyun?"));
    }
    let t = simdi();
    if i.zaman < t - 5 * 60_000 || i.zaman > t + 60 * 24 * 3600 * 1000 {
        return Err(hata(
            StatusCode::BAD_REQUEST,
            "Zaman geçmişte ya da çok ileride (en çok 60 gün sonrası)",
        ));
    }
    let kontenjan = i.kontenjan.filter(|k| (2..=100).contains(k));
    let notu: Option<String> = i
        .not
        .map(|n| n.trim().chars().take(300).collect::<String>())
        .filter(|n| !n.is_empty());
    let (gid, arkadaslar) = {
        let db = s.db();
        let gid = db
            .gece_ac(&ben.id, &oyun, i.zaman, kontenjan, notu.as_deref())
            .map_err(ic)?
            .ok_or_else(|| {
                hata(
                    StatusCode::TOO_MANY_REQUESTS,
                    format!("Aynı anda en çok {KISI_BASI} oyun gecen olabilir"),
                )
            })?;
        let (ark, _, _) = db.arkadaslar(&ben.id).map_err(ic)?;
        (gid, ark)
    };
    let v = json!({ "tip": "bildirim", "tur": "davet", "kisi": ben, "gece": gid, "metin": oyun });
    for a in arkadaslar {
        s.durum.kisiye(&a.id, &v);
    }
    s.durum.yay(json!({ "tip": "akis" }));
    s.durum.yay(json!({ "tip": "gece", "id": gid }));
    Ok(Json(json!({ "id": gid })))
}

pub async fn liste(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> Sonuc {
    kim(&s, &h)?;
    let l = s.db().yaklasan_geceler().map_err(ic)?;
    Ok(Json(json!({ "geceler": l })))
}

#[derive(Deserialize)]
pub struct KatilIstek {
    durum: String,
}

/// POST /api/gece/{id}/katil {durum: geliyor|belki|gelemem}
pub async fn katil(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(gid): Path<i64>,
    Json(i): Json<KatilIstek>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    if !matches!(i.durum.as_str(), "geliyor" | "belki" | "gelemem") {
        return Err(hata(StatusCode::BAD_REQUEST, "Geçersiz seçim"));
    }
    let r = s.db().katil(gid, &ben.id, &i.durum).map_err(ic)?;
    match r {
        KatilSonuc::Tamam => {}
        KatilSonuc::Dolu => {
            return Err(hata(
                StatusCode::CONFLICT,
                "Kişi sınırı doldu; \"Belki\" diyebilirsin",
            ))
        }
        KatilSonuc::Yok => return Err(hata(StatusCode::NOT_FOUND, "Bu gece iptal edilmiş")),
        KatilSonuc::Bitti => return Err(hata(StatusCode::GONE, "Bu gece bitti")),
    }
    s.durum.yay(json!({ "tip": "gece", "id": gid }));
    let g = s.db().gece(gid).map_err(ic)?;
    Ok(Json(json!({ "gece": g })))
}

/// DELETE /api/gece/{id}: açan kişi ya da yönetici iptal eder.
pub async fn sil(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(gid): Path<i64>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let sahibi = s
        .db()
        .gece_sahibi(gid)
        .ok_or_else(|| hata(StatusCode::NOT_FOUND, "Bu gece zaten yok"))?;
    if sahibi != ben.id && !s.yonetici_mi(&ben.id) {
        return Err(hata(
            StatusCode::FORBIDDEN,
            "Yalnız açan kişi iptal edebilir",
        ));
    }
    s.db().gece_sil(gid).map_err(ic)?;
    s.durum.yay(json!({ "tip": "akis" }));
    s.durum.yay(json!({ "tip": "gece", "id": gid }));
    Ok(Json(json!({})))
}

/// Dakikada bir: başlamasına 15 dakika kalan gecelerin katılımcılarına hatırlatma.
pub fn hatirlat(s: &Sunucu) -> usize {
    let l = s.db().hatirlatilacaklar(simdi()).unwrap_or_default();
    for (g, kisiler) in &l {
        let v = json!({ "tip": "bildirim", "tur": "hatirlat", "kisi": g.olusturan, "gece": g.id,
                        "metin": g.oyun, "zaman": g.zaman });
        for k in kisiler {
            s.durum.kisiye(k, &v);
        }
    }
    l.len()
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
    fn gece_katilim_kontenjan_hatirlatma() {
        let db = Db::bellekte();
        for i in ["1", "2", "3"] {
            kisi(&db, i);
        }
        let t = simdi();
        let g = db
            .gece_ac("1", "CS2", t + 10 * 60_000, Some(2), Some("ranked"))
            .unwrap()
            .unwrap();
        // Akışa kart düştü.
        let akis = db.akis("2", None, None, 10).unwrap();
        assert_eq!(akis[0].tur, "gece");
        assert_eq!(akis[0].veri["gece"], g);
        // 2 kişilik: açan + 1 kişi; üçüncü "geliyor" diyemez, "belki" diyebilir.
        assert_eq!(db.katil(g, "2", "geliyor").unwrap(), KatilSonuc::Tamam);
        assert_eq!(db.katil(g, "3", "geliyor").unwrap(), KatilSonuc::Dolu);
        assert_eq!(db.katil(g, "3", "belki").unwrap(), KatilSonuc::Tamam);
        // Fikrini değiştiren aynı satırı günceller.
        assert_eq!(db.katil(g, "2", "gelemem").unwrap(), KatilSonuc::Tamam);
        assert_eq!(db.katil(g, "3", "geliyor").unwrap(), KatilSonuc::Tamam);
        let gece = db.gece(g).unwrap().unwrap();
        assert_eq!(gece.katilanlar.len(), 3);
        // 10 dk kala: hatırlatma bir kez; gelemem diyen hariç.
        let h = db.hatirlatilacaklar(t).unwrap();
        assert_eq!(h.len(), 1);
        let mut kisiler = h[0].1.clone();
        kisiler.sort();
        assert_eq!(kisiler, vec!["1", "3"]);
        assert!(db.hatirlatilacaklar(t).unwrap().is_empty());
        assert_eq!(db.yaklasan_geceler().unwrap().len(), 1);
        // İptal: kart da gider.
        assert!(db.gece_sil(g).unwrap());
        assert!(db.akis("2", None, None, 10).unwrap().is_empty());
        assert_eq!(db.katil(g, "2", "belki").unwrap(), KatilSonuc::Yok);
    }

    #[test]
    fn kisi_basi_sinir() {
        let db = Db::bellekte();
        kisi(&db, "1");
        let t = simdi() + 3_600_000;
        for _ in 0..KISI_BASI {
            assert!(db.gece_ac("1", "LoL", t, None, None).unwrap().is_some());
        }
        assert!(db.gece_ac("1", "LoL", t, None, None).unwrap().is_none());
    }
}
