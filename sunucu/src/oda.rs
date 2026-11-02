//! Dinleme odaları (2.0): önce oda açılır (ad, kişi sınırı, kimler görebilir, kim şarkı ekleyebilir), sonra şarkılar
//! sıraya eklenir. Çalan şarkı, sıra, konum ve üyeler sunucuda (bellekte) durur; herkes kendi bilgisayarında aynı
//! şarkıyı aynı saniyeden çalar. Sahip ve DJ'ler oynatır/duraklatır/atlar; sahip ayarları değiştirir, DJ yapar,
//! birini çıkarır. Sahip çıkarsa yönetim en eski üyeye geçer; oda boşalınca kapanır.
//!
//! İstemciye: `{"tip":"oda","oda":{…},"simdi":ms}` ya da `{"tip":"oda","kapandi":"<id>"}`.

use crate::api::{hata, kim, Sonuc};
use crate::db::{simdi, Kullanici};
use crate::Sunucu;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Sırada en çok bu kadar şarkı.
const SIRA_SINIRI: usize = 50;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Sarki {
    pub baslik: String,
    /// Kaynak sayfa (YouTube vb.); herkesin GMID'i sesi/görüntüyü buradan alır.
    pub url: String,
    #[serde(default)]
    pub kapak: Option<String>,
    #[serde(default)]
    pub sure: Option<f64>,
    /// Birlikte izleme: görüntüsüyle oynatılır.
    #[serde(default)]
    pub video: bool,
    /// Eklentiden gelen videolarda videonun bulunduğu sayfa (site oynatmak için ister).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sayfa: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Oge {
    pub id: u64,
    pub sarki: Sarki,
    pub ekleyen: Kullanici,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Gizlilik {
    #[default]
    Herkes,
    /// Yalnız sahibin arkadaşları (ve üyeler) görür, katılır.
    Arkadaslar,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Yetki {
    /// Yalnız sahip şarkı ekler.
    Sahip,
    /// Sahip ve DJ'ler ekler.
    Dj,
    /// Odadaki herkes ekler.
    #[default]
    Herkes,
}

/// Oda türü: müzik (yalnız ses) ya da izleme (görüntülü, birlikte izle).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OdaTuru {
    #[default]
    Muzik,
    Video,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Tekrar {
    #[default]
    Yok,
    /// Biten şarkı baştan çalar.
    Sarki,
    /// Biten (ya da atlanan) şarkı sıranın sonuna döner.
    Liste,
}

#[derive(Serialize, Clone, Debug)]
pub struct Oda {
    pub id: String,
    pub ad: String,
    pub tur: OdaTuru,
    pub sahip: Kullanici,
    pub gizlilik: Gizlilik,
    /// Kişi sınırı (sahip dahil).
    pub sinir: Option<u32>,
    pub yetki: Yetki,
    pub djler: Vec<String>,
    /// Katılma sırasıyla (sahip dahil).
    pub uyeler: Vec<Kullanici>,
    pub simdiki: Option<Oge>,
    pub sira: Vec<Oge>,
    /// `guncelleme` anındaki konum (sn).
    pub konum: f64,
    pub caliyor: bool,
    pub guncelleme: i64,
    pub tekrar: Tekrar,
    /// Karıştır: sıradaki şarkı rastgele seçilir.
    pub karisik: bool,
    /// Şu an ekranını/sekmesini paylaşan üye (görüntü üyeler arasında doğrudan gider; sunucu yalnız aracı).
    pub yayinci: Option<String>,
}

/// [0, n) arası rastgele sayı.
fn rastgele(n: usize) -> usize {
    let mut b = [0u8; 4];
    let _ = getrandom::fill(&mut b);
    u32::from_le_bytes(b) as usize % n.max(1)
}

impl Oda {
    pub fn uye_mi(&self, id: &str) -> bool {
        self.uyeler.iter().any(|u| u.id == id)
    }

    /// Oynatmayı yönetir (oynat/duraklat/sar/atla): sahip ve DJ'ler.
    pub fn yonetir(&self, id: &str) -> bool {
        self.sahip.id == id || self.djler.iter().any(|d| d == id)
    }

    pub fn ekleyebilir(&self, id: &str) -> bool {
        match self.yetki {
            Yetki::Sahip => self.sahip.id == id,
            Yetki::Dj => self.yonetir(id),
            Yetki::Herkes => self.uye_mi(id),
        }
    }

    pub fn dolu(&self) -> bool {
        self.sinir.is_some_and(|s| self.uyeler.len() as u32 >= s)
    }

    /// Şu anki konum (çalıyorsa geçen süre eklenir).
    pub fn konum_simdi(&self, simdi: i64) -> f64 {
        if self.caliyor {
            self.konum + (simdi - self.guncelleme).max(0) as f64 / 1000.0
        } else {
            self.konum
        }
    }

    /// Sıraya ekler; hiçbir şey çalmıyorsa hemen başlar. Sıra doluysa `false`.
    pub fn ekle(&mut self, oge: Oge, simdi: i64) -> bool {
        if self.simdiki.is_none() {
            self.simdiki = Some(oge);
            self.konum = 0.0;
            self.caliyor = true;
            self.guncelleme = simdi;
            return true;
        }
        if self.sira.len() >= SIRA_SINIRI {
            return false;
        }
        self.sira.push(oge);
        true
    }

    /// Sıradaki şarkıya geç (sıra boşsa durur). `bitti`: şarkı kendiliğinden bitti (şarkı tekrarında baştan
    /// çalar); değilse atlandı. Liste tekrarında çalan şarkı sıranın sonuna döner; karışıkta sıradaki rastgele.
    pub fn sonraki(&mut self, simdi: i64, bitti: bool) {
        if bitti && self.tekrar == Tekrar::Sarki && self.simdiki.is_some() {
            self.konum = 0.0;
            self.caliyor = true;
            self.guncelleme = simdi;
            return;
        }
        let i = if self.karisik {
            rastgele(self.sira.len())
        } else {
            0
        };
        self.gec(i, simdi);
    }

    /// Sıranın `i`. öğesine geç (sıra boşsa durur).
    fn gec(&mut self, i: usize, simdi: i64) {
        let eski = self.simdiki.take();
        self.simdiki = (i < self.sira.len()).then(|| self.sira.remove(i));
        if self.tekrar == Tekrar::Liste {
            if let Some(e) = eski {
                // Sıra tek şarkıysa (ya da boşsa) aynı şarkı yeniden çalar.
                if self.simdiki.is_none() {
                    self.simdiki = Some(e);
                } else {
                    self.sira.push(e);
                }
            }
        }
        self.konum = 0.0;
        self.caliyor = self.simdiki.is_some();
        self.guncelleme = simdi;
    }

    /// Kişi ayrılır. Sahipse yönetim en eski üyeye geçer. Oda boşaldıysa `true` (kapanmalı).
    pub fn ayril(&mut self, id: &str) -> bool {
        if self.yayinci.as_deref() == Some(id) {
            self.yayinci = None;
        }
        self.uyeler.retain(|u| u.id != id);
        self.djler.retain(|d| d != id);
        if self.uyeler.is_empty() {
            return true;
        }
        if self.sahip.id == id {
            self.sahip = self.uyeler[0].clone();
        }
        false
    }
}

#[derive(Default)]
pub struct Odalar {
    ic: Mutex<HashMap<String, Oda>>,
    sayac: AtomicU64,
}

impl Odalar {
    fn kilit(&self) -> std::sync::MutexGuard<'_, HashMap<String, Oda>> {
        self.ic.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn yeni_oge(&self, sarki: Sarki, ekleyen: &Kullanici) -> Oge {
        Oge {
            id: self.sayac.fetch_add(1, Ordering::Relaxed) + 1,
            sarki,
            ekleyen: ekleyen.clone(),
        }
    }

    pub fn hepsi(&self) -> Vec<Oda> {
        self.kilit().values().cloned().collect()
    }

    pub fn al(&self, id: &str) -> Option<Oda> {
        self.kilit().get(id).cloned()
    }

    /// Odada değişiklik: kapanış fonksiyonu odayı değiştirir; dönen oda (değiştiyse) yayınlanır.
    fn degistir<T>(
        &self,
        id: &str,
        f: impl FnOnce(&mut Oda) -> Result<T, Hata>,
    ) -> Result<(T, Oda), Hata> {
        let mut k = self.kilit();
        let oda = k.get_mut(id).ok_or(Hata::Yok)?;
        let t = f(oda)?;
        Ok((t, oda.clone()))
    }

    /// Kişinin bulunduğu odalardan çıkarır. Dönen: (değişen odalar, kapanan odalar).
    pub fn her_yerden_cik(&self, kisi: &str) -> (Vec<Oda>, Vec<Oda>) {
        let mut k = self.kilit();
        let (mut degisen, mut kapanan) = (Vec::new(), Vec::new());
        let idler: Vec<String> = k
            .values()
            .filter(|o| o.uye_mi(kisi))
            .map(|o| o.id.clone())
            .collect();
        for id in idler {
            if let Some(o) = k.get_mut(&id) {
                if o.ayril(kisi) {
                    if let Some(o) = k.remove(&id) {
                        kapanan.push(o);
                    }
                } else {
                    degisen.push(o.clone());
                }
            }
        }
        (degisen, kapanan)
    }
}

#[derive(Debug, PartialEq)]
pub enum Hata {
    Yok,
    Yetki(&'static str),
    Dolu,
    Gecersiz(&'static str),
}

fn cevir(e: Hata) -> (StatusCode, Json<Value>) {
    match e {
        Hata::Yok => hata(StatusCode::NOT_FOUND, "Bu oda kapanmış"),
        Hata::Yetki(m) => hata(StatusCode::FORBIDDEN, m),
        Hata::Dolu => hata(StatusCode::CONFLICT, "Oda dolu"),
        Hata::Gecersiz(m) => hata(StatusCode::BAD_REQUEST, m),
    }
}

// ---------------- yayın ----------------

/// Odayı görebilecek herkese gönderir (herkese açıksa herkese; değilse üyeler + sahibin arkadaşları).
fn yay(s: &Sunucu, oda: &Oda) {
    let v = json!({ "tip": "oda", "oda": oda, "simdi": simdi() });
    yay_deger(s, oda, &v);
}

fn yay_kapandi(s: &Sunucu, oda: &Oda) {
    let v = json!({ "tip": "oda", "kapandi": oda.id });
    yay_deger(s, oda, &v);
}

fn yay_deger(s: &Sunucu, oda: &Oda, v: &Value) {
    match oda.gizlilik {
        Gizlilik::Herkes => s.durum.yay(v.clone()),
        Gizlilik::Arkadaslar => {
            let mut kime: Vec<String> = oda.uyeler.iter().map(|u| u.id.clone()).collect();
            if let Ok((ark, _, _)) = s.db().arkadaslar(&oda.sahip.id) {
                kime.extend(ark.into_iter().map(|k| k.id));
            }
            kime.push(oda.sahip.id.clone());
            kime.sort();
            kime.dedup();
            for k in kime {
                s.durum.kisiye(&k, v);
            }
        }
    }
}

fn gorur(s: &Sunucu, oda: &Oda, ben: &str) -> bool {
    oda.gizlilik == Gizlilik::Herkes
        || oda.uye_mi(ben)
        || oda.sahip.id == ben
        || s.db().arkadas_mi(ben, &oda.sahip.id)
}

/// Kişi tamamen çıktı (ws.rs çağırır): bulunduğu odalardan ayrılır.
pub fn kisi_gitti(s: &Sunucu, id: &str) {
    let (degisen, kapanan) = s.odalar.her_yerden_cik(id);
    for o in &degisen {
        yay(s, o);
    }
    for o in &kapanan {
        yay_kapandi(s, o);
    }
}

fn cik_ve_yay(s: &Sunucu, kisi: &str) {
    kisi_gitti(s, kisi);
}

// ---------------- uçlar ----------------

pub async fn liste(State(s): State<Arc<Sunucu>>, h: HeaderMap) -> Sonuc {
    let ben = kim(&s, &h)?;
    let l: Vec<Oda> = s
        .odalar
        .hepsi()
        .into_iter()
        .filter(|o| gorur(&s, o, &ben.id))
        .collect();
    Ok(Json(json!({ "odalar": l, "simdi": simdi() })))
}

#[derive(Deserialize)]
pub struct Ayar {
    #[serde(default)]
    tur: OdaTuru,
    ad: Option<String>,
    gizlilik: Option<Gizlilik>,
    /// 0: sınırsız.
    sinir: Option<u32>,
    yetki: Option<Yetki>,
}

fn ad_temizle(ad: Option<String>, varsayilan: &str) -> String {
    let a: String = ad.unwrap_or_default().trim().chars().take(40).collect();
    if a.is_empty() {
        varsayilan.to_string()
    } else {
        a
    }
}

fn sinir_temizle(s: Option<u32>) -> Option<u32> {
    s.filter(|n| *n > 0).map(|n| n.clamp(2, 50))
}

/// POST /api/oda: oda aç (başka odadaysan oradan çıkarsın).
pub async fn ac(State(s): State<Arc<Sunucu>>, h: HeaderMap, Json(a): Json<Ayar>) -> Sonuc {
    let ben = kim(&s, &h)?;
    if s.odalar.hepsi().iter().any(|o| o.sahip.id == ben.id) {
        return Err(hata(StatusCode::CONFLICT, "Zaten açık bir odan var"));
    }
    cik_ve_yay(&s, &ben.id);
    let mut b = [0u8; 5];
    getrandom::fill(&mut b).map_err(|e| hata(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let id: String = b.iter().map(|x| format!("{x:02x}")).collect();
    let oda = Oda {
        id: id.clone(),
        tur: a.tur,
        ad: ad_temizle(
            a.ad,
            &format!(
                "{} {}",
                ben.ad,
                if a.tur == OdaTuru::Video {
                    "izleme odası"
                } else {
                    "odası"
                }
            ),
        ),
        sahip: ben.clone(),
        gizlilik: a.gizlilik.unwrap_or_default(),
        sinir: sinir_temizle(a.sinir),
        yetki: a.yetki.unwrap_or_default(),
        djler: Vec::new(),
        uyeler: vec![ben.clone()],
        simdiki: None,
        sira: Vec::new(),
        konum: 0.0,
        caliyor: false,
        guncelleme: simdi(),
        tekrar: Tekrar::Yok,
        karisik: false,
        yayinci: None,
    };
    s.odalar.kilit().insert(id, oda.clone());
    yay(&s, &oda);
    Ok(Json(json!({ "oda": oda, "simdi": simdi() })))
}

#[derive(Deserialize)]
pub struct Degisiklik {
    ad: Option<String>,
    gizlilik: Option<Gizlilik>,
    sinir: Option<u32>,
    yetki: Option<Yetki>,
    /// DJ yap / DJ'liği al: {kisi, ver}.
    dj: Option<DjIstek>,
    /// Tekrar ve karıştır (sahip ve DJ'ler).
    tekrar: Option<Tekrar>,
    karisik: Option<bool>,
}

#[derive(Deserialize)]
pub struct DjIstek {
    kisi: String,
    ver: bool,
}

/// PATCH /api/oda/{id}: ayarlar (yalnız sahip).
pub async fn ayarla(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(d): Json<Degisiklik>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let ((), oda) = s
        .odalar
        .degistir(&id, |o| {
            let sahip_isi = d.ad.is_some()
                || d.gizlilik.is_some()
                || d.sinir.is_some()
                || d.yetki.is_some()
                || d.dj.is_some();
            if sahip_isi && o.sahip.id != ben.id {
                return Err(Hata::Yetki("Ayarları yalnız oda sahibi değiştirir"));
            }
            if (d.tekrar.is_some() || d.karisik.is_some()) && !o.yonetir(&ben.id) {
                return Err(Hata::Yetki(
                    "Tekrar ve karıştırmayı sahip ve DJ'ler değiştirir",
                ));
            }
            if let Some(t) = d.tekrar {
                o.tekrar = t;
            }
            if let Some(k) = d.karisik {
                o.karisik = k;
            }
            if let Some(a) = d.ad {
                o.ad = ad_temizle(Some(a), &o.ad.clone());
            }
            if let Some(g) = d.gizlilik {
                o.gizlilik = g;
            }
            if let Some(n) = d.sinir {
                o.sinir = sinir_temizle(Some(n));
            }
            if let Some(y) = d.yetki {
                o.yetki = y;
            }
            if let Some(dj) = d.dj {
                o.djler.retain(|x| *x != dj.kisi);
                if dj.ver && o.uye_mi(&dj.kisi) && dj.kisi != o.sahip.id {
                    o.djler.push(dj.kisi);
                }
            }
            Ok(())
        })
        .map_err(cevir)?;
    yay(&s, &oda);
    Ok(Json(json!({ "oda": oda })))
}

/// DELETE /api/oda/{id}: odayı kapat (sahip ya da yönetici).
pub async fn kapat(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(id): Path<String>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let oda = s.odalar.al(&id).ok_or_else(|| cevir(Hata::Yok))?;
    if oda.sahip.id != ben.id && !s.yonetici_mi(&ben.id) {
        return Err(cevir(Hata::Yetki("Odayı yalnız sahibi kapatır")));
    }
    s.odalar.kilit().remove(&id);
    yay_kapandi(&s, &oda);
    Ok(Json(json!({})))
}

/// POST /api/oda/{id}/katil
pub async fn katil(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(id): Path<String>) -> Sonuc {
    let ben = kim(&s, &h)?;
    let oda = s.odalar.al(&id).ok_or_else(|| cevir(Hata::Yok))?;
    if !gorur(&s, &oda, &ben.id) {
        return Err(cevir(Hata::Yok));
    }
    if oda.uye_mi(&ben.id) {
        return Ok(Json(json!({ "oda": oda, "simdi": simdi() })));
    }
    if oda.dolu() {
        return Err(cevir(Hata::Dolu));
    }
    // Aynı anda tek odada.
    cik_ve_yay(&s, &ben.id);
    let ((), oda) = s
        .odalar
        .degistir(&id, |o| {
            if o.dolu() {
                return Err(Hata::Dolu);
            }
            if !o.uye_mi(&ben.id) {
                o.uyeler.push(ben.clone());
            }
            Ok(())
        })
        .map_err(cevir)?;
    yay(&s, &oda);
    Ok(Json(json!({ "oda": oda, "simdi": simdi() })))
}

/// DELETE /api/oda/{id}/katil: ayrıl.
pub async fn ayril(State(s): State<Arc<Sunucu>>, h: HeaderMap, Path(_id): Path<String>) -> Sonuc {
    let ben = kim(&s, &h)?;
    cik_ve_yay(&s, &ben.id);
    Ok(Json(json!({})))
}

fn url_gecerli(u: &str) -> bool {
    (u.starts_with("https://") || u.starts_with("http://")) && u.len() <= 500
}

/// POST /api/oda/{id}/sira: şarkı (ya da video) ekle.
pub async fn sira_ekle(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(mut sarki): Json<Sarki>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    if !url_gecerli(&sarki.url) {
        return Err(cevir(Hata::Gecersiz("Geçersiz bağlantı")));
    }
    sarki.baslik = sarki.baslik.trim().chars().take(200).collect();
    sarki.kapak = sarki
        .kapak
        .filter(|k| k.starts_with("https://") && k.len() <= 500);
    sarki.sure = sarki.sure.filter(|x| x.is_finite() && *x > 0.0);
    sarki.sayfa = sarki.sayfa.filter(|k| url_gecerli(k));
    let oge = s.odalar.yeni_oge(sarki, &ben);
    let ((), oda) = s
        .odalar
        .degistir(&id, |o| {
            if !o.ekleyebilir(&ben.id) {
                return Err(Hata::Yetki("Bu odada şarkı ekleme yetkin yok"));
            }
            // Müzik odasında yalnız ses, izleme odasında görüntüyle.
            let mut oge = oge;
            oge.sarki.video = o.tur == OdaTuru::Video;
            if !o.ekle(oge, simdi()) {
                return Err(Hata::Gecersiz("Sıra dolu (en çok 50 şarkı)"));
            }
            Ok(())
        })
        .map_err(cevir)?;
    yay(&s, &oda);
    Ok(Json(json!({ "oda": oda, "simdi": simdi() })))
}

/// DELETE /api/oda/{id}/sira/{oge}: sıradan çıkar (sahip/DJ ya da ekleyen).
pub async fn sira_sil(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path((id, oge)): Path<(String, u64)>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let ((), oda) = s
        .odalar
        .degistir(&id, |o| {
            let i = o
                .sira
                .iter()
                .position(|x| x.id == oge)
                .ok_or(Hata::Gecersiz("Bu şarkı sırada değil"))?;
            if !o.yonetir(&ben.id) && o.sira[i].ekleyen.id != ben.id {
                return Err(Hata::Yetki("Yalnız ekleyen, sahip ya da DJ çıkarabilir"));
            }
            o.sira.remove(i);
            Ok(())
        })
        .map_err(cevir)?;
    yay(&s, &oda);
    Ok(Json(json!({ "oda": oda })))
}

#[derive(Deserialize)]
pub struct Oynat {
    konum: f64,
    caliyor: bool,
}

/// POST /api/oda/{id}/oynat: oynat/duraklat/sar (sahip ve DJ).
pub async fn oynat(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(i): Json<Oynat>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    if !i.konum.is_finite() {
        return Err(cevir(Hata::Gecersiz("Geçersiz konum")));
    }
    let ((), oda) = s
        .odalar
        .degistir(&id, |o| {
            if !o.yonetir(&ben.id) {
                return Err(Hata::Yetki("Oynatmayı sahip ve DJ'ler yönetir"));
            }
            o.konum = i.konum.max(0.0);
            o.caliyor = i.caliyor && o.simdiki.is_some();
            o.guncelleme = simdi();
            Ok(())
        })
        .map_err(cevir)?;
    yay(&s, &oda);
    Ok(Json(json!({ "oda": oda, "simdi": simdi() })))
}

#[derive(Deserialize)]
pub struct Sonraki {
    /// Bitti diyen dinleyici, hangi şarkının bittiğini söyler (iki kez atlanmasın).
    oge: Option<u64>,
    /// Sıradaki bu öğeye hemen geç (sahip/DJ; listede tıklanan şarkı).
    #[serde(default)]
    sec: Option<u64>,
    /// Şarkı kendiliğinden bitti (atlama değil): şarkı tekrarında baştan çalar.
    #[serde(default)]
    bitti: bool,
}

/// POST /api/oda/{id}/sonraki: atla (sahip/DJ) ya da "bu şarkı bitti" (herhangi bir üye, şarkı gerçekten
/// bittiyse).
pub async fn sonraki(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(i): Json<Sonraki>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let t = simdi();
    let ((), oda) = s
        .odalar
        .degistir(&id, |o| {
            if let Some(sec) = i.sec {
                if !o.yonetir(&ben.id) {
                    return Err(Hata::Yetki("Şarkı seçmeyi sahip ve DJ'ler yapar"));
                }
                if let Some(n) = o.sira.iter().position(|x| x.id == sec) {
                    o.gec(n, t);
                }
                return Ok(());
            }
            let simdiki = o.simdiki.as_ref().map(|x| x.id);
            if i.oge.is_some() && i.oge != simdiki {
                // Başkası zaten geçirmiş.
                return Ok(());
            }
            let bitti = o.uye_mi(&ben.id)
                && o.simdiki
                    .as_ref()
                    .and_then(|x| x.sarki.sure)
                    .is_some_and(|sure| o.konum_simdi(t) >= sure - 3.0);
            if !o.yonetir(&ben.id) && !bitti {
                return Err(Hata::Yetki("Atlamayı sahip ve DJ'ler yapar"));
            }
            o.sonraki(t, i.bitti);
            Ok(())
        })
        .map_err(cevir)?;
    yay(&s, &oda);
    Ok(Json(json!({ "oda": oda, "simdi": simdi() })))
}

#[derive(Deserialize)]
pub struct YayinIstek {
    acik: bool,
}

/// POST /api/oda/{id}/yayin {acik}: ekran/sekme paylaşımını başlat (sahip/DJ) ya da bitir (paylaşan ya da
/// sahip). Başlayınca sıradaki video duraklar.
pub async fn yayin(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path(id): Path<String>,
    Json(i): Json<YayinIstek>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let t = simdi();
    let ((), oda) = s
        .odalar
        .degistir(&id, |o| {
            if i.acik {
                if !o.yonetir(&ben.id) {
                    return Err(Hata::Yetki("Ekran paylaşımını sahip ve DJ'ler başlatır"));
                }
                if o.caliyor {
                    o.konum = o.konum_simdi(t);
                    o.caliyor = false;
                    o.guncelleme = t;
                }
                o.yayinci = Some(ben.id.clone());
            } else {
                if o.yayinci.as_deref() != Some(ben.id.as_str()) && o.sahip.id != ben.id {
                    return Err(Hata::Yetki("Paylaşımı paylaşan kişi ya da sahip bitirir"));
                }
                o.yayinci = None;
            }
            Ok(())
        })
        .map_err(cevir)?;
    yay(&s, &oda);
    Ok(Json(json!({ "oda": oda, "simdi": simdi() })))
}

/// Canlı bağlantıdan gelen ekran paylaşımı kurulum mesajını (teklif/cevap/aday) aynı odadaki kişiye iletir.
pub fn sinyal(s: &Sunucu, kimden: &str, v: &Value) {
    let (Some(oda), Some(kime)) = (v["oda"].as_str(), v["kime"].as_str()) else {
        return;
    };
    let Some(o) = s.odalar.al(oda) else { return };
    if !o.uye_mi(kimden) || !o.uye_mi(kime) || v["veri"].to_string().len() > 64 * 1024 {
        return;
    }
    s.durum.kisiye(
        kime,
        &json!({ "tip": "yayin-sinyal", "oda": oda, "kimden": kimden, "veri": v["veri"] }),
    );
}

/// POST /api/oda/{id}/at/{kisi}: odadan çıkar (sahip ya da yönetici).
pub async fn at(
    State(s): State<Arc<Sunucu>>,
    h: HeaderMap,
    Path((id, kisi)): Path<(String, String)>,
) -> Sonuc {
    let ben = kim(&s, &h)?;
    let yonetici = s.yonetici_mi(&ben.id);
    let (kapandi, oda) = s
        .odalar
        .degistir(&id, |o| {
            if o.sahip.id != ben.id && !yonetici {
                return Err(Hata::Yetki("Yalnız oda sahibi çıkarabilir"));
            }
            if kisi == o.sahip.id {
                return Err(Hata::Gecersiz("Sahip kendini çıkaramaz; odayı kapat"));
            }
            Ok(o.ayril(&kisi))
        })
        .map_err(cevir)?;
    if kapandi {
        s.odalar.kilit().remove(&id);
        yay_kapandi(&s, &oda);
    } else {
        yay(&s, &oda);
    }
    s.durum
        .kisiye(&kisi, &json!({ "tip": "oda", "atildi": id, "ad": oda.ad }));
    Ok(Json(json!({})))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(id: &str) -> Kullanici {
        Kullanici {
            id: id.into(),
            ad: id.into(),
            avatar: None,
        }
    }

    fn sarki(b: &str, sure: f64) -> Sarki {
        Sarki {
            baslik: b.into(),
            url: "https://youtu.be/x".into(),
            kapak: None,
            sure: Some(sure),
            video: false,
            sayfa: None,
        }
    }

    fn oda() -> Oda {
        Oda {
            id: "o".into(),
            tur: OdaTuru::Muzik,
            ad: "Akşam".into(),
            sahip: k("1"),
            gizlilik: Gizlilik::Herkes,
            sinir: Some(3),
            yetki: Yetki::Dj,
            djler: vec![],
            uyeler: vec![k("1"), k("2"), k("3")],
            simdiki: None,
            sira: vec![],
            konum: 0.0,
            caliyor: false,
            guncelleme: 0,
            tekrar: Tekrar::Yok,
            karisik: false,
            yayinci: None,
        }
    }

    #[test]
    fn yetkiler_ve_sinir() {
        let mut o = oda();
        assert!(o.dolu());
        assert!(o.ekleyebilir("1") && !o.ekleyebilir("2"));
        o.djler.push("2".into());
        assert!(o.ekleyebilir("2") && o.yonetir("2") && !o.yonetir("3"));
        o.yetki = Yetki::Herkes;
        assert!(o.ekleyebilir("3") && !o.ekleyebilir("9"));
        o.yetki = Yetki::Sahip;
        assert!(!o.ekleyebilir("2"));
    }

    #[test]
    fn sira_ve_gecis() {
        let o = Odalar::default();
        let mut x = oda();
        let a = o.yeni_oge(sarki("A", 100.0), &k("1"));
        let b = o.yeni_oge(sarki("B", 100.0), &k("2"));
        assert!(x.ekle(a.clone(), 1000));
        assert_eq!(x.simdiki.as_ref().unwrap().sarki.baslik, "A");
        assert!(x.caliyor);
        assert!(x.ekle(b, 2000));
        assert_eq!(x.sira.len(), 1);
        // 10 sn sonra konum 9 sn (1000 ms'de başladı).
        assert!((x.konum_simdi(10_000) - 9.0).abs() < 0.01);
        x.sonraki(20_000, true);
        assert_eq!(x.simdiki.as_ref().unwrap().sarki.baslik, "B");
        assert!(x.sira.is_empty());
        x.sonraki(30_000, true);
        assert!(x.simdiki.is_none() && !x.caliyor);
    }

    #[test]
    fn tekrar_ve_karisik() {
        let o = Odalar::default();
        let mut x = oda();
        for ad in ["A", "B", "C"] {
            x.ekle(o.yeni_oge(sarki(ad, 100.0), &k("1")), 0);
        }
        let ad = |x: &Oda| x.simdiki.as_ref().unwrap().sarki.baslik.clone();
        // Şarkı tekrarı: biten baştan; atlanan geçer.
        x.tekrar = Tekrar::Sarki;
        x.sonraki(1, true);
        assert_eq!(ad(&x), "A");
        x.sonraki(2, false);
        assert_eq!(ad(&x), "B");
        // Liste tekrarı: biten sona döner, sıra bitmez.
        x.tekrar = Tekrar::Liste;
        x.sonraki(3, true);
        x.sonraki(4, true);
        assert_eq!(ad(&x), "B");
        assert_eq!(x.sira.len(), 1);
        // Karışık: sıradakilerden biri; hiçbiri kaybolmaz.
        x.karisik = true;
        for t in 5..20 {
            x.sonraki(t, true);
            assert_eq!(x.sira.len() + 1, 2);
        }
    }

    #[test]
    fn sahip_ayrilinca_devreder_bosalinca_kapanir() {
        let mut x = oda();
        x.djler.push("2".into());
        assert!(!x.ayril("1"));
        assert_eq!(x.sahip.id, "2");
        assert!(!x.ayril("2"));
        assert_eq!(x.sahip.id, "3");
        assert!(x.ayril("3"));
    }

    #[test]
    fn her_yerden_cik() {
        let o = Odalar::default();
        let mut a = oda();
        a.id = "a".into();
        let mut b = oda();
        b.id = "b".into();
        b.uyeler = vec![k("5")];
        b.sahip = k("5");
        o.kilit().insert("a".into(), a);
        o.kilit().insert("b".into(), b);
        let (degisen, kapanan) = o.her_yerden_cik("5");
        assert!(degisen.is_empty());
        assert_eq!(kapanan.len(), 1);
        let (degisen, _) = o.her_yerden_cik("2");
        assert_eq!(degisen.len(), 1);
        assert_eq!(o.al("a").unwrap().uyeler.len(), 2);
    }
}
