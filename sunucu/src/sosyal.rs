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
/// Paylaşılan klip bu kadar sonra (video da, paylaşım da) silinir; aldığı beğeniler profilde kalır.
pub const KLIP_OMRU_MS: i64 = 7 * 24 * 3600 * 1000;

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
         CREATE INDEX IF NOT EXISTS mesaj_kimden ON mesajlar (kimden, kime, id);
         CREATE TABLE IF NOT EXISTS arkadaslar (
           kimden TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           kime TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           -- istek | arkadas
           durum TEXT NOT NULL, zaman INTEGER NOT NULL,
           PRIMARY KEY (kimden, kime));
         CREATE INDEX IF NOT EXISTS arkadas_kime ON arkadaslar (kime);
         -- Silinen paylaşımların (süresi dolan klipler dahil) aldığı beğeni ve yorumlar: profilde kalır.
         CREATE TABLE IF NOT EXISTS sikayetler (
           etkinlik INTEGER NOT NULL REFERENCES etkinlikler(id) ON DELETE CASCADE,
           kullanici TEXT NOT NULL REFERENCES kullanicilar(id) ON DELETE CASCADE,
           neden TEXT, zaman INTEGER NOT NULL, PRIMARY KEY (etkinlik, kullanici));
         CREATE TABLE IF NOT EXISTS kalici_puan (
           kullanici TEXT PRIMARY KEY REFERENCES kullanicilar(id) ON DELETE CASCADE,
           begeni INTEGER NOT NULL DEFAULT 0, yorum INTEGER NOT NULL DEFAULT 0);",
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
    /// puan | oyun | klip | gonderi
    pub tur: String,
    pub veri: Value,
    pub zaman: i64,
    pub begeni: i64,
    pub begendim: bool,
    pub yorumlar: Vec<Yorum>,
    /// Kaç kişi şikâyet etti (arayüz yalnız yöneticiye gösterir).
    pub sikayet: i64,
}

/// Akış süzgeci.
#[derive(Default, Clone, Copy)]
pub struct Suzgec<'a> {
    /// Bu zamandan eskiler (sayfalama).
    pub once: Option<i64>,
    /// Yalnız bu kişinin olayları.
    pub kisi: Option<&'a str>,
    /// Yalnız arkadaşlarımın ve benim olaylarım.
    pub arkadaslar: bool,
    /// "#etiket" ya da oyun/film/dizi adı.
    pub konu: Option<&'a str>,
    /// Yalnız bu türden olaylar (gonderi, klip, puan, oyun, gece).
    pub tur: Option<&'a str>,
    /// Yalnız bu kişinin beğendikleri.
    pub begenen: Option<&'a str>,
    pub sayi: i64,
}

/// Gündemdeki bir konu.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Gundem {
    /// Oyun/film/dizi adı ya da "#etiket".
    pub ad: String,
    pub etiket: bool,
    /// Son 7 günde kaç paylaşımda geçti.
    pub paylasim: i64,
    /// Bu paylaşımların aldığı beğeni.
    pub begeni: i64,
}

/// Yazıdaki #etiketler ("#CS2", "#OyunGecesi"): harf/rakam/alt çizgi, 2–30 karakter.
pub fn etiketler(metin: &str) -> Vec<String> {
    let mut l = Vec::new();
    let mut onceki: Option<char> = None;
    for (i, c) in metin.char_indices() {
        // "a#b" etiket sayılmaz; önce boşluk ya da noktalama olmalı.
        let basta = onceki.is_none_or(|p| !(p.is_alphanumeric() || p == '_'));
        onceki = Some(c);
        if c != '#' || !basta {
            continue;
        }
        let govde: String = metin[i + 1..]
            .chars()
            .take_while(|x| x.is_alphanumeric() || *x == '_')
            .collect();
        let n = govde.chars().count();
        if (2..=30).contains(&n) && !govde.chars().all(|x| x.is_ascii_digit()) {
            l.push(format!("#{govde}"));
        }
    }
    l
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

/// İki kişi arasındaki ilişki (benim gözümden).
#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Iliski {
    Arkadas,
    /// Ben istek gönderdim, bekliyor.
    Giden,
    /// Bana istek geldi.
    Gelen,
}

/// Arkadaş isteği sonucu.
#[derive(Debug, PartialEq)]
pub enum Istek {
    Gonderildi,
    /// Karşı taraf zaten istek göndermişti: artık arkadaşız.
    Kabul,
    ZatenArkadas,
}

impl Db {
    pub fn iliski(&self, ben: &str, diger: &str) -> Option<Iliski> {
        let r: Option<(String, String)> = self
            .0
            .query_row(
                "SELECT kimden, durum FROM arkadaslar
                 WHERE (kimden = ?1 AND kime = ?2) OR (kimden = ?2 AND kime = ?1)",
                params![ben, diger],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .ok()
            .flatten();
        r.map(|(kimden, durum)| match durum.as_str() {
            "arkadas" => Iliski::Arkadas,
            _ if kimden == ben => Iliski::Giden,
            _ => Iliski::Gelen,
        })
    }

    pub fn arkadas_mi(&self, a: &str, b: &str) -> bool {
        self.iliski(a, b) == Some(Iliski::Arkadas)
    }

    /// Arkadaş ekle: karşıdan istek varsa kabul edilir, yoksa istek gönderilir.
    pub fn arkadas_ekle(&self, ben: &str, diger: &str) -> R<Istek> {
        match self.iliski(ben, diger) {
            Some(Iliski::Arkadas) => Ok(Istek::ZatenArkadas),
            Some(Iliski::Giden) => Ok(Istek::Gonderildi),
            Some(Iliski::Gelen) => {
                self.0
                    .execute(
                        "UPDATE arkadaslar SET durum = 'arkadas', zaman = ?3 WHERE kimden = ?2 AND kime = ?1",
                        params![ben, diger, simdi()],
                    )
                    .map_err(e)?;
                Ok(Istek::Kabul)
            }
            None => {
                self.0
                    .execute(
                        "INSERT INTO arkadaslar (kimden, kime, durum, zaman) VALUES (?1, ?2, 'istek', ?3)",
                        params![ben, diger, simdi()],
                    )
                    .map_err(e)?;
                Ok(Istek::Gonderildi)
            }
        }
    }

    /// Arkadaşlıktan çıkar / isteği reddet / isteği geri al.
    pub fn arkadas_sil(&self, ben: &str, diger: &str) -> R<bool> {
        let n = self
            .0
            .execute(
                "DELETE FROM arkadaslar WHERE (kimden = ?1 AND kime = ?2) OR (kimden = ?2 AND kime = ?1)",
                params![ben, diger],
            )
            .map_err(e)?;
        Ok(n > 0)
    }

    /// (arkadaşlar, gelen istekler, giden istekler)
    pub fn arkadaslar(&self, ben: &str) -> R<(Vec<Kullanici>, Vec<Kullanici>, Vec<Kullanici>)> {
        let mut st = self
            .0
            .prepare(
                "SELECT k.id, k.ad, k.avatar, a.durum, a.kimden = ?1 FROM arkadaslar a
                 JOIN kullanicilar k ON k.id = CASE WHEN a.kimden = ?1 THEN a.kime ELSE a.kimden END
                 WHERE a.kimden = ?1 OR a.kime = ?1 ORDER BY lower(k.ad)",
            )
            .map_err(e)?;
        let (mut ark, mut gelen, mut giden) = (Vec::new(), Vec::new(), Vec::new());
        let satirlar = st
            .query_map([ben], |r| {
                Ok((
                    kullanici(r, 0)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, bool>(4)?,
                ))
            })
            .map_err(e)?;
        for (k, durum, benden) in satirlar.filter_map(Result::ok) {
            match (durum.as_str(), benden) {
                ("arkadas", _) => ark.push(k),
                (_, true) => giden.push(k),
                _ => gelen.push(k),
            }
        }
        Ok((ark, gelen, giden))
    }

    pub fn arkadas_sayisi(&self, id: &str) -> i64 {
        self.0
            .query_row(
                "SELECT COUNT(*) FROM arkadaslar WHERE durum = 'arkadas' AND (kimden = ?1 OR kime = ?1)",
                [id],
                |r| r.get(0),
            )
            .unwrap_or(0)
    }

    pub fn kisiler(&self) -> R<Vec<(Kullanici, i64)>> {
        let mut st = self
            .0
            .prepare("SELECT id, ad, avatar, son FROM kullanicilar WHERE id != 'gmid' ORDER BY lower(ad)")
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
    pub(crate) fn etkinlik_yaz(
        &self,
        id: &str,
        tur: &str,
        anahtar: &str,
        veri: &Value,
        zaman: i64,
    ) -> R<i64> {
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
        self.akis_suz(ben, once, kisi, false, sayi)
    }

    /// `arkadaslar`: yalnız arkadaşlarımın ve benim olaylarım.
    pub fn akis_suz(
        &self,
        ben: &str,
        once: Option<i64>,
        kisi: Option<&str>,
        arkadaslar: bool,
        sayi: i64,
    ) -> R<Vec<Etkinlik>> {
        self.akis_konu(ben, once, kisi, arkadaslar, None, sayi)
    }

    /// `konu`: "#etiket" (gönderide geçen) ya da oyun/film/dizi adı (gündemden tıklanan).
    pub fn akis_konu(
        &self,
        ben: &str,
        once: Option<i64>,
        kisi: Option<&str>,
        arkadaslar: bool,
        konu: Option<&str>,
        sayi: i64,
    ) -> R<Vec<Etkinlik>> {
        self.akis_suzgec(
            ben,
            &Suzgec {
                once,
                kisi,
                arkadaslar,
                konu,
                sayi,
                ..Suzgec::default()
            },
        )
    }

    /// Akış, bütün süzgeçlerle (profil sekmeleri: tür ve beğendikleri).
    pub fn akis_suzgec(&self, ben: &str, f: &Suzgec) -> R<Vec<Etkinlik>> {
        let Suzgec {
            once,
            kisi,
            arkadaslar,
            konu,
            tur,
            begenen,
            sayi,
        } = *f;
        // "#cs2" → gönderide "#cs2" geçenler (büyük/küçük harf fark etmez); ad → o oyunun/filmin olayları
        // ve "#adı" (boşluksuz) etiketli gönderiler.
        let konu = konu.map(str::trim).filter(|k| !k.is_empty());
        let etiket = konu.map(|k| {
            let k = k.trim_start_matches('#');
            format!("#{}", k.split_whitespace().collect::<String>()).to_lowercase()
        });
        let ad = konu.filter(|k| !k.starts_with('#'));
        let mut st = self
            .0
            .prepare(
                "SELECT e.id, k.id, k.ad, k.avatar, e.tur, e.veri, e.zaman,
                   (SELECT COUNT(*) FROM begeniler b WHERE b.etkinlik = e.id),
                   EXISTS (SELECT 1 FROM begeniler b WHERE b.etkinlik = e.id AND b.kullanici = ?1),
                   (SELECT COUNT(*) FROM sikayetler s WHERE s.etkinlik = e.id)
                 FROM etkinlikler e JOIN kullanicilar k ON k.id = e.kullanici
                 WHERE (?2 IS NULL OR e.zaman < ?2) AND (?3 IS NULL OR e.kullanici = ?3)
                   AND (e.tur != 'klip' OR e.zaman > ?6)
                   AND (?7 IS NULL OR instr(lower(COALESCE(json_extract(e.veri, '$.metin'), '') || ' '
                          || COALESCE(json_extract(e.veri, '$.not'), '')), ?7) > 0
                        OR (?8 IS NOT NULL AND (json_extract(e.veri, '$.ad') = ?8 OR json_extract(e.veri, '$.oyun') = ?8)))
                   AND (?9 IS NULL OR e.tur = ?9)
                   AND (?10 IS NULL OR EXISTS (SELECT 1 FROM begeniler b WHERE b.etkinlik = e.id AND b.kullanici = ?10))
                   AND (?5 = 0 OR e.kullanici = ?1 OR EXISTS (SELECT 1 FROM arkadaslar a WHERE a.durum = 'arkadas'
                        AND ((a.kimden = ?1 AND a.kime = e.kullanici) OR (a.kime = ?1 AND a.kimden = e.kullanici))))
                 ORDER BY e.zaman DESC, e.id DESC LIMIT ?4",
            )
            .map_err(e)?;
        let mut l: Vec<Etkinlik> = st
            .query_map(
                params![
                    ben,
                    once,
                    kisi,
                    sayi.clamp(1, 100),
                    arkadaslar as i64,
                    simdi() - KLIP_OMRU_MS,
                    etiket,
                    ad,
                    tur,
                    begenen
                ],
                |r| {
                    Ok(Etkinlik {
                        id: r.get(0)?,
                        kisi: kullanici(r, 1)?,
                        tur: r.get(4)?,
                        veri: serde_json::from_str(&r.get::<_, String>(5)?).unwrap_or(Value::Null),
                        zaman: r.get(6)?,
                        begeni: r.get(7)?,
                        begendim: r.get(8)?,
                        yorumlar: Vec::new(),
                        sikayet: r.get(9)?,
                    })
                },
            )
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
    /// `yonetici`: herkesin yorumunu silebilir.
    pub fn yorum_sil(&self, ben: &str, id: i64, yonetici: bool) -> R<bool> {
        let n = self
            .0
            .execute(
                "DELETE FROM yorumlar WHERE id = ?1 AND (?3 = 1 OR kullanici = ?2 OR etkinlik IN
                   (SELECT id FROM etkinlikler WHERE kullanici = ?2))",
                params![id, ben, yonetici as i64],
            )
            .map_err(e)?;
        Ok(n > 0)
    }

    /// Yönetici mi? Liste boşsa ilk giriş yapan kişi (sunucuyu kuran) yöneticidir.
    pub fn yonetici_mi(&self, id: &str, liste: &[String]) -> bool {
        if !liste.is_empty() {
            return liste.iter().any(|x| x == id);
        }
        self.0
            .query_row(
                "SELECT id FROM kullanicilar ORDER BY ilk, rowid LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .is_ok_and(|ilk| ilk == id)
    }

    /// Şikâyet: aynı kişi aynı paylaşımı bir kez şikâyet eder. Dönen: yeni mi.
    pub fn sikayet_et(&self, ben: &str, etkinlik: i64, neden: Option<&str>) -> R<bool> {
        let n = self
            .0
            .execute(
                "INSERT OR IGNORE INTO sikayetler (etkinlik, kullanici, neden, zaman) VALUES (?1, ?2, ?3, ?4)",
                params![etkinlik, ben, neden, simdi()],
            )
            .map_err(e)?;
        Ok(n > 0)
    }

    /// Yönetici: herkesin paylaşımını siler (aynı kurallar: puanlar sahibinde kalır). Dönen: tür ve veri.
    pub fn etkinlik_sil_yonetici(&self, id: i64) -> R<Option<(String, Value)>> {
        let sahibi = self.etkinlik_sahibi(id);
        match sahibi {
            Some(k) => self.etkinlik_sil(&k, id),
            None => Ok(None),
        }
    }

    /// Kendi paylaşımını siler (aldığı beğeni ve yorumlar profilde kalır). Silindiyse olayın türü ve verisi.
    pub fn etkinlik_sil(&self, ben: &str, id: i64) -> R<Option<(String, Value)>> {
        let olay: Option<(String, String)> = self
            .0
            .query_row(
                "SELECT tur, veri FROM etkinlikler WHERE id = ?1 AND kullanici = ?2",
                params![id, ben],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(e)?;
        let Some((tur, veri)) = olay else {
            return Ok(None);
        };
        self.kaldir(id)?;
        Ok(Some((
            tur,
            serde_json::from_str(&veri).unwrap_or(Value::Null),
        )))
    }

    /// Olayı siler; başkalarından aldığı beğeni ve yorumlar önce sahibinin kalıcı puanına eklenir.
    fn kaldir(&self, id: i64) -> R<()> {
        self.0
            .execute(
                "INSERT INTO kalici_puan (kullanici, begeni, yorum)
                 SELECT e.kullanici,
                   (SELECT COUNT(*) FROM begeniler b WHERE b.etkinlik = e.id AND b.kullanici != e.kullanici),
                   (SELECT COUNT(*) FROM yorumlar y WHERE y.etkinlik = e.id AND y.kullanici != e.kullanici)
                 FROM etkinlikler e WHERE e.id = ?1
                 ON CONFLICT(kullanici) DO UPDATE SET begeni = begeni + excluded.begeni,
                   yorum = yorum + excluded.yorum",
                [id],
            )
            .map_err(e)?;
        self.0
            .execute("DELETE FROM etkinlikler WHERE id = ?1", [id])
            .map_err(e)?;
        Ok(())
    }

    /// Kişinin başkalarından aldığı toplam beğeni ve yorum (silinen paylaşımlarınki dahil).
    pub fn toplam_puan(&self, id: &str) -> (i64, i64) {
        self.0
            .query_row(
                "SELECT
                   COALESCE((SELECT begeni FROM kalici_puan WHERE kullanici = ?1), 0)
                   + (SELECT COUNT(*) FROM begeniler b JOIN etkinlikler e ON e.id = b.etkinlik
                      WHERE e.kullanici = ?1 AND b.kullanici != ?1),
                   COALESCE((SELECT yorum FROM kalici_puan WHERE kullanici = ?1), 0)
                   + (SELECT COUNT(*) FROM yorumlar y JOIN etkinlikler e ON e.id = y.etkinlik
                      WHERE e.kullanici = ?1 AND y.kullanici != ?1)",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((0, 0))
    }

    /// Gündem: son 7 günün olaylarında en çok geçen oyun/film/dizi adları ve #etiketler. Her paylaşım 1,
    /// aldığı her beğeni 1 puan daha getirir. `arkadaslar`: yalnız arkadaşlarımın ve benim olaylarım.
    pub fn gundem(&self, ben: &str, arkadaslar: bool, sayi: usize) -> R<Vec<Gundem>> {
        let t = simdi();
        let mut st = self
            .0
            .prepare(
                "SELECT e.tur, e.veri, (SELECT COUNT(*) FROM begeniler b WHERE b.etkinlik = e.id)
                 FROM etkinlikler e
                 WHERE e.zaman > ?2 AND (e.tur != 'klip' OR e.zaman > ?3)
                   AND (?4 = 0 OR e.kullanici = ?1 OR EXISTS (SELECT 1 FROM arkadaslar a WHERE a.durum = 'arkadas'
                        AND ((a.kimden = ?1 AND a.kime = e.kullanici) OR (a.kime = ?1 AND a.kimden = e.kullanici))))",
            )
            .map_err(e)?;
        let satirlar: Vec<(String, String, i64)> = st
            .query_map(
                params![
                    ben,
                    t - 7 * 24 * 3600 * 1000,
                    t - KLIP_OMRU_MS,
                    arkadaslar as i64
                ],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        // anahtar (küçük harf) → (görünen ad, etiket mi, paylaşım, beğeni)
        let mut say: HashMap<String, (String, bool, i64, i64)> = HashMap::new();
        for (tur, veri, begeni) in satirlar {
            let v: Value = serde_json::from_str(&veri).unwrap_or(Value::Null);
            let mut konular: Vec<(String, bool)> = Vec::new();
            match tur.as_str() {
                "gonderi" => {}
                "klip" => konular.extend(v["oyun"].as_str().map(|s| (s.to_string(), false))),
                _ => konular.extend(v["ad"].as_str().map(|s| (s.to_string(), false))),
            }
            for alan in ["metin", "not"] {
                for et in etiketler(v[alan].as_str().unwrap_or("")) {
                    konular.push((et, true));
                }
            }
            konular.retain(|(k, _)| !k.trim().is_empty());
            konular.sort();
            konular.dedup();
            for (ad, etiket) in konular {
                let g = say
                    .entry(ad.to_lowercase())
                    .or_insert_with(|| (ad.clone(), etiket, 0, 0));
                g.2 += 1;
                g.3 += begeni;
            }
        }
        let mut l: Vec<Gundem> = say
            .into_values()
            .map(|(ad, etiket, paylasim, begeni)| Gundem {
                ad,
                etiket,
                paylasim,
                begeni,
            })
            .collect();
        l.sort_by(|a, b| {
            (b.paylasim + b.begeni)
                .cmp(&(a.paylasim + a.begeni))
                .then(b.paylasim.cmp(&a.paylasim))
                .then(a.ad.cmp(&b.ad))
        });
        l.truncate(sayi);
        Ok(l)
    }

    /// Yazıda @Ad ile anılan üyeler (kendisi hariç). Ad birden çok kelime olabilir ("@Ali Veli"); en uzun
    /// eşleşen ad seçilir, adın hemen arkasında harf/rakam olmamalı.
    pub fn anilanlar(&self, metin: &str, ben: &str) -> Vec<Kullanici> {
        if !metin.contains('@') {
            return Vec::new();
        }
        let kucuk = metin.to_lowercase();
        let mut l: Vec<Kullanici> = self
            .kisiler()
            .unwrap_or_default()
            .into_iter()
            .map(|(k, _)| k)
            .filter(|k| k.id != ben && !k.ad.trim().is_empty())
            .filter(|k| {
                let aranan = format!("@{}", k.ad.to_lowercase());
                kucuk.match_indices(&aranan).any(|(i, m)| {
                    kucuk[i + m.len()..]
                        .chars()
                        .next()
                        .is_none_or(|c| !(c.is_alphanumeric() || c == '_'))
                })
            })
            .collect();
        l.truncate(10);
        l
    }

    /// Serbest gönderi (Twitter'daki gibi: yazı, isteğe bağlı resim). Son 10 dakikada en çok 10 gönderi.
    pub fn gonderi_yaz(
        &self,
        id: &str,
        metin: &str,
        resim: Option<&str>,
        anket: Option<&Value>,
    ) -> R<Option<i64>> {
        let t = simdi();
        let son: i64 = self
            .0
            .query_row(
                "SELECT COUNT(*) FROM etkinlikler WHERE kullanici = ?1 AND tur = 'gonderi' AND zaman > ?2",
                params![id, t - 10 * 60 * 1000],
                |r| r.get(0),
            )
            .map_err(e)?;
        if son >= 10 {
            return Ok(None);
        }
        let mut b = [0u8; 6];
        getrandom::fill(&mut b).map_err(|x| x.to_string())?;
        let anahtar = format!(
            "gonderi:{t}:{}",
            b.iter().map(|x| format!("{x:02x}")).collect::<String>()
        );
        let mut veri = json!({ "ad": "", "metin": metin, "resim": resim });
        if let Some(a) = anket {
            veri["anket"] = a.clone();
        }
        self.etkinlik_yaz(id, "gonderi", &anahtar, &veri, t)
            .map(Some)
    }

    /// Klip paylaşımı (aynı klip yeniden paylaşılırsa başa alınır, süresi yeniden başlar).
    pub fn klip_paylas(&self, id: &str, ozet: &str, veri: &Value) -> R<i64> {
        self.etkinlik_yaz(id, "klip", &format!("klip:{ozet}"), veri, simdi())
    }

    /// Kişinin şu an yayında olan klip sayısı.
    pub fn klip_sayisi(&self, id: &str) -> i64 {
        self.0
            .query_row(
                "SELECT COUNT(*) FROM etkinlikler WHERE kullanici = ?1 AND tur = 'klip' AND zaman > ?2",
                params![id, simdi() - KLIP_OMRU_MS],
                |r| r.get(0),
            )
            .unwrap_or(0)
    }

    /// Bu video dosyasını hâlâ bir paylaşım kullanıyor mu (aynı klibi iki kişi paylaşmış olabilir)?
    pub fn klip_kullaniliyor(&self, ozet: &str) -> bool {
        self.0
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM etkinlikler WHERE tur = 'klip' AND anahtar = ?1)",
                [format!("klip:{ozet}")],
                |r| r.get(0),
            )
            .unwrap_or(true)
    }

    /// Süresi dolan klip paylaşımlarını siler (puanları kalır). Dönen: silinen videoların özetleri.
    pub fn eski_klipleri_sil(&self, simdi: i64) -> R<Vec<String>> {
        let mut st = self
            .0
            .prepare("SELECT id, anahtar FROM etkinlikler WHERE tur = 'klip' AND zaman <= ?1")
            .map_err(e)?;
        let l: Vec<(i64, String)> = st
            .query_map([simdi - KLIP_OMRU_MS], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(e)?
            .filter_map(Result::ok)
            .collect();
        let mut ozetler = Vec::new();
        for (id, anahtar) in l {
            self.kaldir(id)?;
            if let Some(o) = anahtar.strip_prefix("klip:") {
                ozetler.push(o.to_string());
            }
        }
        Ok(ozetler)
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

    #[test]
    fn etiket_ayikla() {
        assert_eq!(
            etiketler("Bu akşam #CS2 var! #OyunGecesi, #ağır_çekim a#b #1 #x"),
            vec!["#CS2", "#OyunGecesi", "#ağır_çekim"]
        );
        assert!(etiketler("renk #fff değil ama ##").contains(&"#fff".to_string()));
    }

    #[test]
    fn profil_sekmeleri_ve_anma() {
        let db = Db::bellekte();
        kisi(&db, "1", "Ali");
        kisi(&db, "2", "Ali Veli");
        kisi(&db, "3", "Zeynep");
        let g = db
            .gonderi_yaz("1", "selam @zeynep ve @Ali Veli!", None, None)
            .unwrap()
            .unwrap();
        db.oyun_basladi("1", "CS2").unwrap();
        db.begen("3", g).unwrap();
        let f = |tur: Option<&'static str>, begenen: Option<&'static str>| Suzgec {
            kisi: if begenen.is_some() { None } else { Some("1") },
            tur,
            begenen,
            sayi: 30,
            ..Suzgec::default()
        };
        assert_eq!(db.akis_suzgec("3", &f(None, None)).unwrap().len(), 2);
        assert_eq!(
            db.akis_suzgec("3", &f(Some("gonderi"), None))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            db.akis_suzgec("3", &f(Some("klip"), None)).unwrap().len(),
            0
        );
        let begendi = db.akis_suzgec("1", &f(None, Some("3"))).unwrap();
        assert_eq!(begendi.len(), 1);
        assert_eq!(begendi[0].id, g);
        // "@Ali Veli" Ali Veli'yi anar; "@Ali" kendisi (yazan) sayılmaz; "@zeynep" büyük/küçük harf fark etmez.
        let mut anilan: Vec<String> = db
            .anilanlar("selam @zeynep ve @Ali Veli!", "1")
            .into_iter()
            .map(|k| k.id)
            .collect();
        anilan.sort();
        assert_eq!(anilan, vec!["2", "3"]);
        assert!(db.anilanlar("@Zeynepçik", "1").is_empty());
    }

    #[test]
    fn gundem_ve_konu() {
        let db = Db::bellekte();
        kisi(&db, "1", "Ali");
        kisi(&db, "2", "Veli");
        db.gonderi_yaz("1", "Bu akşam #CS2 kim var?", None, None)
            .unwrap();
        let g2 = db
            .gonderi_yaz("2", "#cs2 olur, #OyunGecesi", None, None)
            .unwrap()
            .unwrap();
        db.oyun_basladi("2", "Hades II").unwrap();
        db.begen("1", g2).unwrap();
        let g = db.gundem("1", false, 10).unwrap();
        assert_eq!(g[0].ad, "#CS2");
        assert_eq!((g[0].paylasim, g[0].begeni), (2, 1));
        assert!(g.iter().any(|x| x.ad == "Hades II" && !x.etiket));
        // "#cs2" süzgeci iki gönderiyi de bulur; ad süzgeci oyuna girişi.
        assert_eq!(
            db.akis_konu("1", None, None, false, Some("#cs2"), 30)
                .unwrap()
                .len(),
            2
        );
        let h = db
            .akis_konu("1", None, None, false, Some("Hades II"), 30)
            .unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].tur, "oyun");
        assert_eq!(
            db.akis_konu("1", None, None, false, Some("#yok"), 30)
                .unwrap()
                .len(),
            0
        );
    }

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
        assert!(!db.yorum_sil("3", y, false).unwrap());
        assert!(db.yorum_sil("1", y, false).unwrap());

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
    fn arkadaslik() {
        let db = Db::bellekte();
        kisi(&db, "1", "Ali");
        kisi(&db, "2", "Veli");
        kisi(&db, "3", "Can");
        assert_eq!(db.arkadas_ekle("1", "2").unwrap(), Istek::Gonderildi);
        assert_eq!(db.iliski("1", "2"), Some(Iliski::Giden));
        assert_eq!(db.iliski("2", "1"), Some(Iliski::Gelen));
        let (a, gelen, _) = db.arkadaslar("2").unwrap();
        assert!(a.is_empty());
        assert_eq!(gelen[0].ad, "Ali");
        // Karşı taraf "ekle" deyince kabul.
        assert_eq!(db.arkadas_ekle("2", "1").unwrap(), Istek::Kabul);
        assert!(db.arkadas_mi("1", "2") && db.arkadas_mi("2", "1"));
        assert_eq!(db.arkadas_ekle("1", "2").unwrap(), Istek::ZatenArkadas);
        assert_eq!(db.arkadas_sayisi("1"), 1);
        // Akış: yalnız arkadaşlar süzgeci.
        let t = simdi();
        for id in ["2", "3"] {
            let p = Profil {
                oyunlar: vec![],
                puanlar: vec![puan("game:1", "Oyun", 7.0, t)],
            };
            db.profil_kaydet(id, p).unwrap();
        }
        assert_eq!(db.akis_suz("1", None, None, false, 30).unwrap().len(), 2);
        let yalniz = db.akis_suz("1", None, None, true, 30).unwrap();
        assert_eq!(yalniz.len(), 1);
        assert_eq!(yalniz[0].kisi.ad, "Veli");
        // Çıkarma.
        assert!(db.arkadas_sil("2", "1").unwrap());
        assert_eq!(db.iliski("1", "2"), None);
    }

    #[test]
    fn metin_temizlenir() {
        assert_eq!(metin("  merhaba "), Some("merhaba".into()));
        assert_eq!(metin("   "), None);
        assert_eq!(metin(&"a".repeat(METIN_SINIRI + 1)), None);
    }
}
