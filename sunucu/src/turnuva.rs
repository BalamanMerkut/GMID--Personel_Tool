//! Topluluk turnuvaları: biri "CS2 1v1, 8 kişi" diye turnuva açar, akışa kart düşer, herkes katılır.
//! Açan kişi başlatınca katılanlar karıştırılıp eleme ağacına yerleşir (kişi sayısı 2'nin kuvveti değilse
//! bazıları ilk turu "bay" geçer). Maç sonucunu maçın oyuncuları ya da açan kişi (veya yönetici) girer;
//! kazanan bir sonraki tura geçer. Finali kazanan şampiyon olur: kalıcı **Şampiyon** rozeti ve bildirim.

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

/// Bitmiş turnuva bu kadar süre listede kalır.
const BITMIS_MS: i64 = 7 * 24 * 3600 * 1000;

pub fn kur(c: &Connection) -> rusqlite::Result<()> {
    c.execute_batch(
        "CREATE TABLE IF NOT EXISTS turnuvalar (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           olusturan TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           ad TEXT NOT NULL, oyun TEXT NOT NULL, boyut INTEGER NOT NULL,
           -- kayit | suruyor | bitti
           durum TEXT NOT NULL DEFAULT 'kayit', sampiyon TEXT,
           etkinlik INTEGER, olusturma INTEGER NOT NULL, bitis INTEGER);
         CREATE TABLE IF NOT EXISTS turnuva_oyuncu (
           turnuva INTEGER NOT NULL REFERENCES turnuvalar(id) ON DELETE CASCADE,
           kullanici TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           zaman INTEGER NOT NULL, PRIMARY KEY (turnuva, kullanici));
         CREATE TABLE IF NOT EXISTS turnuva_mac (
           turnuva INTEGER NOT NULL REFERENCES turnuvalar(id) ON DELETE CASCADE,
           tur INTEGER NOT NULL, sira INTEGER NOT NULL,
           a TEXT, b TEXT, kazanan TEXT,
           PRIMARY KEY (turnuva, tur, sira));",
    )
}

type R<T> = Result<T, String>;

fn e(x: rusqlite::Error) -> String {
    x.to_string()
}

#[derive(Serialize, Clone, Debug)]
pub struct Mac {
    pub tur: i64,
    pub sira: i64,
    pub a: Option<Kullanici>,
    pub b: Option<Kullanici>,
    pub kazanan: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Turnuva {
    pub id: i64,
    pub olusturan: Kullanici,
    pub ad: String,
    pub oyun: String,
    pub boyut: i64,
    pub durum: String,
    pub sampiyon: Option<Kullanici>,
    pub etkinlik: Option<i64>,
    pub olusturma: i64,
    pub oyuncular: Vec<Kullanici>,
    /// Tur sırasıyla (0: ilk tur); her turda sıra sırasıyla.
    pub maclar: Vec<Mac>,
}

#[derive(Debug, PartialEq)]
pub enum TSonuc {
    Tamam,
    Yok,
    Dolu,
    Kapali,
    Az,
    Gecersiz,
}

/// En az `n` olan en küçük 2'nin kuvveti (en az 2).
fn ikinin_kuvveti(n: usize) -> usize {
    n.max(2).next_power_of_two()
}

fn karistir<T>(l: &mut [T]) {
    for i in (1..l.len()).rev() {
        let mut b = [0u8; 4];
        let _ = getrandom::fill(&mut b);
        let j = u32::from_le_bytes(b) as usize % (i + 1);
        l.swap(i, j);
    }
}

impl Db {
    pub fn turnuva_ac(&self, id: &str, ad: &str, oyun: &str, boyut: i64) -> R<Option<i64>> {
        let t = simdi();
        let acik: i64 = self
            .0
            .query_row(
                "SELECT COUNT(*) FROM turnuvalar WHERE olusturan = ?1 AND durum != 'bitti'",
                [id],
                |r| r.get(0),
            )
            .map_err(e)?;
        if acik >= 3 {
            return Ok(None);
        }
        let tid: i64 = self
            .0
            .query_row(
                "INSERT INTO turnuvalar (olusturan, ad, oyun, boyut, olusturma) VALUES (?1, ?2, ?3, ?4, ?5)
                 RETURNING id",
                params![id, ad, oyun, boyut, t],
                |r| r.get(0),
            )
            .map_err(e)?;
        self.0
            .execute(
                "INSERT INTO turnuva_oyuncu (turnuva, kullanici, zaman) VALUES (?1, ?2, ?3)",
                params![tid, id, t],
            )
            .map_err(e)?;
        let veri = json!({ "ad": ad, "turnuva": tid, "oyun": oyun, "boyut": boyut });
        let eid = self.etkinlik_yaz(id, "turnuva", &format!("turnuva:{tid}"), &veri, t)?;
        self.0
            .execute(
                "UPDATE turnuvalar SET etkinlik = ?2 WHERE id = ?1",
                params![tid, eid],
            )
            .map_err(e)?;
        Ok(Some(tid))
    }

    fn turnuva_satiri(&self, tid: i64) -> R<Option<Turnuva>> {
        self.0
            .query_row(
                "SELECT t.id, k.id, k.ad, k.avatar, t.ad, t.oyun, t.boyut, t.durum, t.sampiyon, t.etkinlik,
                        t.olusturma
                 FROM turnuvalar t JOIN kullanicilar k ON k.id = t.olusturan WHERE t.id = ?1",
                [tid],
                |r| {
                    Ok((
                        Turnuva {
                            id: r.get(0)?,
                            olusturan: Kullanici {
                                id: r.get(1)?,
                                ad: r.get(2)?,
                                avatar: r.get(3)?,
                            },
                            ad: r.get(4)?,
                            oyun: r.get(5)?,
                            boyut: r.get(6)?,
                            durum: r.get(7)?,
                            sampiyon: None,
                            etkinlik: r.get(9)?,
                            olusturma: r.get(10)?,
                            oyuncular: Vec::new(),
                            maclar: Vec::new(),
                        },
                        r.get::<_, Option<String>>(8)?,
                    ))
                },
            )
            .optional()
            .map_err(e)
            .map(|o| {
                o.map(|(mut t, s)| {
                    t.sampiyon = s.and_then(|s| self.kullanici(&s));
                    t
                })
            })
    }

    pub fn turnuva(&self, tid: i64) -> R<Option<Turnuva>> {
        let Some(mut t) = self.turnuva_satiri(tid)? else {
            return Ok(None);
        };
        let mut st = self
            .0
            .prepare(
                "SELECT k.id, k.ad, k.avatar FROM turnuva_oyuncu o JOIN kullanicilar k ON k.id = o.kullanici
                 WHERE o.turnuva = ?1 ORDER BY o.zaman",
            )
            .map_err(e)?;
        t.oyuncular = st
            .query_map([tid], |r| {
                Ok(Kullanici {
                    id: r.get(0)?,
                    ad: r.get(1)?,
                    avatar: r.get(2)?,
                })
            })
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        let mut st = self
            .0
            .prepare(
                "SELECT tur, sira, a, b, kazanan FROM turnuva_mac WHERE turnuva = ?1 ORDER BY tur, sira",
            )
            .map_err(e)?;
        // (tur, sıra, a, b, kazanan)
        type Satir = (i64, i64, Option<String>, Option<String>, Option<String>);
        let satirlar: Vec<Satir> = st
            .query_map([tid], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        let bul = |id: &Option<String>| {
            id.as_ref()
                .and_then(|i| t.oyuncular.iter().find(|k| &k.id == i).cloned())
        };
        let maclar = satirlar
            .iter()
            .map(|(tur, sira, a, b, k)| Mac {
                tur: *tur,
                sira: *sira,
                a: bul(a),
                b: bul(b),
                kazanan: k.clone(),
            })
            .collect();
        t.maclar = maclar;
        Ok(Some(t))
    }

    /// Kayıttakiler, sürenler ve son 7 günde bitenler.
    pub fn turnuvalar(&self) -> R<Vec<Turnuva>> {
        let ids: Vec<i64> = {
            let mut st = self
                .0
                .prepare(
                    "SELECT id FROM turnuvalar WHERE durum != 'bitti' OR bitis > ?1
                     ORDER BY durum = 'bitti', olusturma DESC LIMIT 30",
                )
                .map_err(e)?;
            let l = st
                .query_map([simdi() - BITMIS_MS], |r| r.get(0))
                .map_err(e)?
                .filter_map(Result::ok)
                .collect();
            l
        };
        let mut l = Vec::new();
        for id in ids {
            if let Some(t) = self.turnuva(id)? {
                l.push(t);
            }
        }
        Ok(l)
    }

    /// Katıl (`katil`) ya da ayrıl; yalnız kayıt sürerken.
    pub fn turnuva_kayit(&self, tid: i64, id: &str, katil: bool) -> R<TSonuc> {
        let Some(t) = self.turnuva(tid)? else {
            return Ok(TSonuc::Yok);
        };
        if t.durum != "kayit" {
            return Ok(TSonuc::Kapali);
        }
        if katil {
            if t.oyuncular.iter().any(|k| k.id == id) {
                return Ok(TSonuc::Tamam);
            }
            if t.oyuncular.len() as i64 >= t.boyut {
                return Ok(TSonuc::Dolu);
            }
            self.0
                .execute(
                    "INSERT OR IGNORE INTO turnuva_oyuncu (turnuva, kullanici, zaman) VALUES (?1, ?2, ?3)",
                    params![tid, id, simdi()],
                )
                .map_err(e)?;
        } else {
            self.0
                .execute(
                    "DELETE FROM turnuva_oyuncu WHERE turnuva = ?1 AND kullanici = ?2",
                    params![tid, id],
                )
                .map_err(e)?;
        }
        Ok(TSonuc::Tamam)
    }

    /// Karıştırıp eleme ağacını kurar. Boşluklar (bay) ilk turda kendiliğinden geçer.
    pub fn turnuva_baslat(&self, tid: i64) -> R<TSonuc> {
        let Some(t) = self.turnuva(tid)? else {
            return Ok(TSonuc::Yok);
        };
        if t.durum != "kayit" {
            return Ok(TSonuc::Kapali);
        }
        if t.oyuncular.len() < 2 {
            return Ok(TSonuc::Az);
        }
        let mut o: Vec<String> = t.oyuncular.iter().map(|k| k.id.clone()).collect();
        karistir(&mut o);
        let n = ikinin_kuvveti(o.len());
        let yari = n / 2;
        // İlk tur: i. maç = o[i] ve o[i + yarı]; o[i + yarı] yoksa bay. (n/2 < kişi sayısı olduğundan her
        // maçta en az bir kişi var.)
        let tx = self.0.unchecked_transaction().map_err(e)?;
        let mut tur = 0;
        let mut mac = yari;
        while mac >= 1 {
            for sira in 0..mac {
                let (a, b) = if tur == 0 {
                    (o.get(sira).cloned(), o.get(sira + yari).cloned())
                } else {
                    (None, None)
                };
                tx.execute(
                    "INSERT INTO turnuva_mac (turnuva, tur, sira, a, b) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![tid, tur, sira as i64, a, b],
                )
                .map_err(e)?;
            }
            tur += 1;
            mac /= 2;
        }
        tx.execute(
            "UPDATE turnuvalar SET durum = 'suruyor' WHERE id = ?1",
            [tid],
        )
        .map_err(e)?;
        tx.commit().map_err(e)?;
        // Baylar.
        for sira in 0..yari {
            if o.get(sira + yari).is_none() {
                self.mac_sonucu(tid, 0, sira as i64, &o[sira])?;
            }
        }
        self.etkinlik_guncelle(tid)?;
        Ok(TSonuc::Tamam)
    }

    /// Maçın kazananı; kazanan bir sonraki tura geçer, final ise turnuva biter. Dönen: şampiyon (bittiyse).
    pub fn mac_sonucu(&self, tid: i64, tur: i64, sira: i64, kazanan: &str) -> R<Option<String>> {
        self.0
            .execute(
                "UPDATE turnuva_mac SET kazanan = ?4 WHERE turnuva = ?1 AND tur = ?2 AND sira = ?3",
                params![tid, tur, sira, kazanan],
            )
            .map_err(e)?;
        let sonraki = self
            .0
            .execute(
                &format!(
                    "UPDATE turnuva_mac SET {} = ?4 WHERE turnuva = ?1 AND tur = ?2 AND sira = ?3",
                    if sira % 2 == 0 { "a" } else { "b" }
                ),
                params![tid, tur + 1, sira / 2, kazanan],
            )
            .map_err(e)?;
        if sonraki > 0 {
            return Ok(None);
        }
        // Final.
        let ad: String = self
            .0
            .query_row("SELECT ad FROM turnuvalar WHERE id = ?1", [tid], |r| {
                r.get(0)
            })
            .map_err(e)?;
        self.0
            .execute(
                "UPDATE turnuvalar SET durum = 'bitti', sampiyon = ?2, bitis = ?3 WHERE id = ?1",
                params![tid, kazanan, simdi()],
            )
            .map_err(e)?;
        self.rozet_ver(kazanan, "sampiyon", Some(&ad))?;
        self.etkinlik_guncelle(tid)?;
        Ok(Some(kazanan.to_string()))
    }

    /// Akıştaki kartın verisine durum ve şampiyonu yazar (akış kartı turnuva silinse de anlamlı kalsın).
    fn etkinlik_guncelle(&self, tid: i64) -> R<()> {
        let Some(t) = self.turnuva_satiri(tid)? else {
            return Ok(());
        };
        self.0
            .execute(
                "UPDATE etkinlikler SET veri = json_set(veri, '$.durum', ?2, '$.sampiyon', json(?3))
                 WHERE id = ?1",
                params![
                    t.etkinlik,
                    t.durum,
                    serde_json::to_string(&t.sampiyon).unwrap_or_else(|_| "null".into())
                ],
            )
            .map_err(e)?;
        Ok(())
    }

    /// Turnuvayı (ve akış kartını) siler.
    pub fn turnuva_sil(&self, tid: i64) -> R<bool> {
        let Some(t) = self.turnuva_satiri(tid)? else {
            return Ok(false);
        };
        self.0
            .execute("DELETE FROM turnuvalar WHERE id = ?1", [tid])
            .map_err(e)?;
        if let Some(eid) = t.etkinlik {
            self.0
                .execute("DELETE FROM etkinlikler WHERE id = ?1", [eid])
                .map_err(e)?;
        }
        Ok(true)
    }
}

// ---------------- uç noktalar ----------------

fn ic(x: String) -> (StatusCode, Json<serde_json::Value>) {
    hata(StatusCode::INTERNAL_SERVER_ERROR, x)
}

fn cevir(r: TSonuc) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    match r {
        TSonuc::Tamam => Ok(()),
        TSonuc::Yok => Err(hata(StatusCode::NOT_FOUND, "Turnuva bulunamadı")),
        TSonuc::Dolu => Err(hata(StatusCode::CONFLICT, "Turnuva dolu")),
        TSonuc::Kapali => Err(hata(StatusCode::CONFLICT, "Kayıtlar kapandı")),
        TSonuc::Az => Err(hata(
            StatusCode::BAD_REQUEST,
            "Başlatmak için en az 2 kişi gerek",
        )),
        TSonuc::Gecersiz => Err(hata(StatusCode::BAD_REQUEST, "Geçersiz seçim")),
    }
}

fn duyur(s: &Sunucu, tid: i64) {
    s.durum.yay(json!({ "tip": "turnuva", "id": tid }));
}

#[derive(Deserialize)]
pub struct AcIstek {
    ad: String,
    oyun: String,
    boyut: i64,
}

/// POST /api/turnuva {ad, oyun, boyut: 4|8|16|32}
pub async fn ac(State(s): State<Arc<Sunucu>>, h: HeaderMap, Json(i): Json<AcIstek>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let ad: String = i.ad.trim().chars().take(60).collect();
    let oyun: String = i.oyun.trim().chars().take(60).collect();
    if ad.is_empty() || oyun.is_empty() {
        return Err(hata(
            StatusCode::BAD_REQUEST,
            "Turnuvanın adını ve oyununu yaz",
        ));
    }
    if ![4, 8, 16, 32].contains(&i.boyut) {
        return Err(hata(
            StatusCode::BAD_REQUEST,
            "Kişi sayısı 4, 8, 16 ya da 32 olmalı",
        ));
    }
    let (tid, ark) = {
        let db = s.db();
        let tid = db
            .turnuva_ac(&ben.id, &ad, &oyun, i.boyut)
            .map_err(ic)?
            .ok_or_else(|| {
                hata(
                    StatusCode::TOO_MANY_REQUESTS,
                    "Aynı anda en çok 3 turnuvan olabilir",
                )
            })?;
        let (ark, _, _) = db.arkadaslar(&ben.id).map_err(ic)?;
        (tid, ark)
    };
    let v =
        json!({ "tip": "bildirim", "tur": "turnuva", "kisi": ben, "turnuva": tid, "metin": ad });
    for a in ark {
        s.durum.kisiye(&a.id, &v);
    }
    s.durum.yay(json!({ "tip": "akis" }));
    duyur(&s, tid);
    Ok(Json(json!({ "id": tid })))
}

pub async fn liste(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> Sonuc {
    kim(&s, &h)?;
    Ok(Json(
        json!({ "turnuvalar": s.db().turnuvalar().map_err(ic)? }),
    ))
}

pub async fn tek(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(tid): Path<i64>) -> Sonuc {
    kim(&s, &h)?;
    let t = s
        .db()
        .turnuva(tid)
        .map_err(ic)?
        .ok_or_else(|| hata(StatusCode::NOT_FOUND, "Turnuva bulunamadı"))?;
    Ok(Json(json!({ "turnuva": t })))
}

/// POST /api/turnuva/{id}/katil · DELETE /api/turnuva/{id}/katil
pub async fn katil(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(tid): Path<i64>) -> Sonuc {
    kayit(s, h, tid, true)
}
pub async fn ayril(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(tid): Path<i64>) -> Sonuc {
    kayit(s, h, tid, false)
}
fn kayit(s: Arc<Sunucu>, h: HeaderMap, tid: i64, katil: bool) -> Sonuc {
    let ben = kim(&s, &h)?;
    cevir(s.db().turnuva_kayit(tid, &ben.id, katil).map_err(ic)?)?;
    duyur(&s, tid);
    Ok(Json(json!({ "turnuva": s.db().turnuva(tid).map_err(ic)? })))
}

fn yetkili(s: &Sunucu, t: &Turnuva, id: &str) -> bool {
    t.olusturan.id == id || s.yonetici_mi(id)
}

/// POST /api/turnuva/{id}/baslat (açan ya da yönetici)
pub async fn baslat(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(tid): Path<i64>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let t = s
        .db()
        .turnuva(tid)
        .map_err(ic)?
        .ok_or_else(|| hata(StatusCode::NOT_FOUND, "Turnuva bulunamadı"))?;
    if !yetkili(&s, &t, &ben.id) {
        return Err(hata(StatusCode::FORBIDDEN, "Turnuvayı açan kişi başlatır"));
    }
    cevir(s.db().turnuva_baslat(tid).map_err(ic)?)?;
    let v = json!({ "tip": "bildirim", "tur": "turnuva-basladi", "kisi": ben, "turnuva": tid, "metin": t.ad });
    for k in t.oyuncular.iter().filter(|k| k.id != ben.id) {
        s.durum.kisiye(&k.id, &v);
    }
    s.durum.yay(json!({ "tip": "akis", "id": t.etkinlik }));
    duyur(&s, tid);
    Ok(Json(json!({ "turnuva": s.db().turnuva(tid).map_err(ic)? })))
}

#[derive(Deserialize)]
pub struct MacIstek {
    tur: i64,
    sira: i64,
    kazanan: String,
}

/// POST /api/turnuva/{id}/mac {tur, sira, kazanan}: maçın oyuncuları, açan kişi ya da yönetici. Açan/yönetici,
/// sonraki maç oynanmadıysa sonucu değiştirebilir.
pub async fn mac(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(tid): Path<i64>,
    Json(i): Json<MacIstek>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let t = s
        .db()
        .turnuva(tid)
        .map_err(ic)?
        .ok_or_else(|| hata(StatusCode::NOT_FOUND, "Turnuva bulunamadı"))?;
    if t.durum != "suruyor" {
        return Err(hata(StatusCode::CONFLICT, "Turnuva sürmüyor"));
    }
    let m = t
        .maclar
        .iter()
        .find(|m| m.tur == i.tur && m.sira == i.sira)
        .ok_or_else(|| hata(StatusCode::NOT_FOUND, "Maç bulunamadı"))?;
    let (Some(a), Some(b)) = (&m.a, &m.b) else {
        return Err(hata(
            StatusCode::CONFLICT,
            "Bu maçın rakipleri henüz belli değil",
        ));
    };
    if i.kazanan != a.id && i.kazanan != b.id {
        cevir(TSonuc::Gecersiz)?;
    }
    let yetki = yetkili(&s, &t, &ben.id);
    if !yetki && ben.id != a.id && ben.id != b.id {
        return Err(hata(
            StatusCode::FORBIDDEN,
            "Sonucu maçın oyuncuları ya da turnuvayı açan girer",
        ));
    }
    if m.kazanan.is_some() {
        let sonraki_oynandi = t
            .maclar
            .iter()
            .any(|x| x.tur == i.tur + 1 && x.sira == i.sira / 2 && x.kazanan.is_some());
        if !yetki || sonraki_oynandi {
            return Err(hata(StatusCode::CONFLICT, "Sonuç zaten girildi"));
        }
    }
    let sampiyon = s
        .db()
        .mac_sonucu(tid, i.tur, i.sira, &i.kazanan)
        .map_err(ic)?;
    if let Some(sp) = sampiyon {
        let kisi = if sp == a.id { a } else { b };
        s.durum.kisiye(
            &sp,
            &json!({ "tip": "bildirim", "tur": "sampiyon", "kisi": kisi, "turnuva": tid, "metin": t.ad }),
        );
        s.durum.yay(json!({ "tip": "akis", "id": t.etkinlik }));
    }
    duyur(&s, tid);
    Ok(Json(json!({ "turnuva": s.db().turnuva(tid).map_err(ic)? })))
}

/// DELETE /api/turnuva/{id} (açan ya da yönetici)
pub async fn sil(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(tid): Path<i64>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let t = s
        .db()
        .turnuva(tid)
        .map_err(ic)?
        .ok_or_else(|| hata(StatusCode::NOT_FOUND, "Turnuva bulunamadı"))?;
    if !yetkili(&s, &t, &ben.id) {
        return Err(hata(
            StatusCode::FORBIDDEN,
            "Turnuvayı yalnız açan kişi siler",
        ));
    }
    s.db().turnuva_sil(tid).map_err(ic)?;
    s.durum.yay(json!({ "tip": "akis" }));
    s.durum
        .yay(json!({ "tip": "turnuva", "id": tid, "silindi": true }));
    Ok(Json(json!({})))
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
    fn ikinin_kuvvetleri() {
        assert_eq!(ikinin_kuvveti(2), 2);
        assert_eq!(ikinin_kuvveti(3), 4);
        assert_eq!(ikinin_kuvveti(5), 8);
        assert_eq!(ikinin_kuvveti(8), 8);
    }

    #[test]
    fn bes_kisilik_turnuva() {
        let db = Db::bellekte();
        for i in 1..=6 {
            kisi(&db, &i.to_string());
        }
        let t = db.turnuva_ac("1", "CS2 1v1", "CS2", 8).unwrap().unwrap();
        // Akışa kart.
        assert_eq!(db.akis("2", None, None, 5).unwrap()[0].tur, "turnuva");
        for i in 2..=5 {
            assert_eq!(
                db.turnuva_kayit(t, &i.to_string(), true).unwrap(),
                TSonuc::Tamam
            );
        }
        assert_eq!(db.turnuva_kayit(t, "6", false).unwrap(), TSonuc::Tamam);
        assert_eq!(db.turnuva_baslat(t).unwrap(), TSonuc::Tamam);
        // Kayıt kapandı.
        assert_eq!(db.turnuva_kayit(t, "6", true).unwrap(), TSonuc::Kapali);
        let tu = db.turnuva(t).unwrap().unwrap();
        assert_eq!(tu.durum, "suruyor");
        // 5 kişi → 8'lik ağaç: 4 + 2 + 1 maç; 3 bay kendiliğinden geçti.
        assert_eq!(tu.maclar.len(), 7);
        let ilk: Vec<&Mac> = tu.maclar.iter().filter(|m| m.tur == 0).collect();
        assert_eq!(ilk.iter().filter(|m| m.b.is_none()).count(), 3);
        assert_eq!(ilk.iter().filter(|m| m.kazanan.is_some()).count(), 3);
        // Kalan maçları hep "a" kazansın; sonunda şampiyon.
        let mut sampiyon = None;
        for _ in 0..10 {
            let tu = db.turnuva(t).unwrap().unwrap();
            let Some(m) = tu
                .maclar
                .iter()
                .find(|m| m.kazanan.is_none() && m.a.is_some() && m.b.is_some())
            else {
                break;
            };
            sampiyon = db
                .mac_sonucu(t, m.tur, m.sira, &m.a.as_ref().unwrap().id)
                .unwrap();
        }
        let sp = sampiyon.expect("şampiyon çıkmalı");
        let tu = db.turnuva(t).unwrap().unwrap();
        assert_eq!(tu.durum, "bitti");
        assert_eq!(tu.sampiyon.unwrap().id, sp);
        assert!(db.rozetler(&sp).iter().any(|r| r.id == "sampiyon"));
        let akis = db.akis("2", None, None, 5).unwrap();
        assert_eq!(akis[0].veri["durum"], "bitti");
        assert_eq!(akis[0].veri["sampiyon"]["id"], sp.as_str());
        assert_eq!(db.turnuvalar().unwrap().len(), 1);
        assert!(db.turnuva_sil(t).unwrap());
        assert!(db.akis("2", None, None, 5).unwrap().is_empty());
    }

    #[test]
    fn iki_kisi_ve_az() {
        let db = Db::bellekte();
        kisi(&db, "1");
        kisi(&db, "2");
        let t = db
            .turnuva_ac("1", "Düello", "Valorant", 4)
            .unwrap()
            .unwrap();
        assert_eq!(db.turnuva_baslat(t).unwrap(), TSonuc::Az);
        db.turnuva_kayit(t, "2", true).unwrap();
        assert_eq!(db.turnuva_baslat(t).unwrap(), TSonuc::Tamam);
        let tu = db.turnuva(t).unwrap().unwrap();
        assert_eq!(tu.maclar.len(), 1);
        let a = tu.maclar[0].a.clone().unwrap().id;
        assert_eq!(db.mac_sonucu(t, 0, 0, &a).unwrap(), Some(a));
    }
}
