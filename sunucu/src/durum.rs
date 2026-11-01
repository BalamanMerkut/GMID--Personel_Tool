//! Kim çevrim içi, kim ne oynuyor. Bellekte tutulur; değişiklikler bağlı herkese yayınlanır.
//! Aynı kişi birden çok yerden bağlanabilir (iki bilgisayar): son bağlantı kapanınca "gitti" sayılır.

use crate::db::Kullanici;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::{broadcast, mpsc};

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Kisi {
    pub id: String,
    pub ad: String,
    pub avatar: Option<String>,
    /// Şu an oynadığı oyunun adı.
    pub oyun: Option<String>,
}

struct Kayit {
    kisi: Kisi,
    baglanti: usize,
}

pub struct Durum {
    ic: Mutex<HashMap<String, Kayit>>,
    yayin: broadcast::Sender<String>,
    /// Kişiye özel iletiler (özel mesaj, bildirim): kişinin açık bağlantılarının kanalları.
    kanallar: Mutex<HashMap<String, Vec<mpsc::UnboundedSender<String>>>>,
}

/// Oyun adı temizlenir: boşsa yok, çok uzunsa kısaltılır.
pub fn oyun_adi(s: Option<&str>) -> Option<String> {
    let s = s?.trim();
    (!s.is_empty()).then(|| s.chars().take(80).collect())
}

impl Default for Durum {
    fn default() -> Self {
        Durum {
            ic: Mutex::new(HashMap::new()),
            yayin: broadcast::channel(256).0,
            kanallar: Mutex::new(HashMap::new()),
        }
    }
}

impl Durum {
    pub fn abone(&self) -> broadcast::Receiver<String> {
        self.yayin.subscribe()
    }

    /// Bağlı herkese.
    pub fn yay(&self, v: serde_json::Value) {
        let _ = self.yayin.send(v.to_string());
    }

    pub fn kaydol(&self, id: &str, tx: mpsc::UnboundedSender<String>) {
        let mut k = self.kanallar.lock().unwrap_or_else(|e| e.into_inner());
        k.entry(id.to_string()).or_default().push(tx);
    }

    pub fn birak(&self, id: &str, tx: &mpsc::UnboundedSender<String>) {
        let mut k = self.kanallar.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(l) = k.get_mut(id) {
            l.retain(|x| !x.same_channel(tx));
            if l.is_empty() {
                k.remove(id);
            }
        }
    }

    /// Yalnız o kişiye (açık tüm bağlantılarına).
    pub fn kisiye(&self, id: &str, v: &serde_json::Value) {
        let k = self.kanallar.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(l) = k.get(id) {
            let m = v.to_string();
            for tx in l {
                let _ = tx.send(m.clone());
            }
        }
    }

    pub fn kisi(&self, id: &str) -> Option<Kisi> {
        let ic = self.ic.lock().unwrap_or_else(|e| e.into_inner());
        ic.get(id).map(|k| k.kisi.clone())
    }

    pub fn liste(&self) -> Vec<Kisi> {
        let ic = self.ic.lock().unwrap_or_else(|e| e.into_inner());
        let mut l: Vec<Kisi> = ic.values().map(|k| k.kisi.clone()).collect();
        l.sort_by_key(|a| a.ad.to_lowercase());
        l
    }

    pub fn liste_mesaji(&self) -> String {
        json!({ "tip": "liste", "kisiler": self.liste() }).to_string()
    }

    pub fn geldi(&self, k: &Kullanici) {
        let mut ic = self.ic.lock().unwrap_or_else(|e| e.into_inner());
        let kayit = ic.entry(k.id.clone()).or_insert_with(|| Kayit {
            kisi: Kisi {
                id: k.id.clone(),
                ad: k.ad.clone(),
                avatar: k.avatar.clone(),
                oyun: None,
            },
            baglanti: 0,
        });
        kayit.baglanti += 1;
        if kayit.baglanti == 1 {
            let kisi = kayit.kisi.clone();
            drop(ic);
            self.yay(json!({ "tip": "geldi", "kisi": kisi }));
        }
    }

    pub fn ayrildi(&self, id: &str) {
        let mut ic = self.ic.lock().unwrap_or_else(|e| e.into_inner());
        let Some(kayit) = ic.get_mut(id) else { return };
        kayit.baglanti = kayit.baglanti.saturating_sub(1);
        if kayit.baglanti == 0 {
            ic.remove(id);
            drop(ic);
            self.yay(json!({ "tip": "gitti", "id": id }));
        }
    }

    /// Oynadığı oyun değiştiyse yayınlar ve `true` döner.
    pub fn oyun(&self, id: &str, oyun: Option<String>) -> bool {
        let mut ic = self.ic.lock().unwrap_or_else(|e| e.into_inner());
        let Some(kayit) = ic.get_mut(id) else {
            return false;
        };
        if kayit.kisi.oyun == oyun {
            return false;
        }
        kayit.kisi.oyun = oyun.clone();
        drop(ic);
        self.yay(json!({ "tip": "durum", "id": id, "oyun": oyun }));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(id: &str, ad: &str) -> Kullanici {
        Kullanici {
            id: id.into(),
            ad: ad.into(),
            avatar: None,
        }
    }

    #[test]
    fn gelir_oynar_gider() {
        let d = Durum::default();
        let mut rx = d.abone();
        d.geldi(&k("1", "Veli"));
        d.geldi(&k("2", "ali"));
        assert!(rx.try_recv().unwrap().contains("\"geldi\""));
        assert!(rx.try_recv().unwrap().contains("\"ali\""));
        // Ada göre sıralı (büyük/küçük harf fark etmez).
        assert_eq!(
            d.liste().iter().map(|x| x.ad.as_str()).collect::<Vec<_>>(),
            ["ali", "Veli"]
        );
        d.oyun("1", oyun_adi(Some("  CS2 ")));
        assert!(rx.try_recv().unwrap().contains("\"CS2\""));
        // Aynı oyun yeniden gelince yayınlanmaz.
        d.oyun("1", Some("CS2".into()));
        assert!(rx.try_recv().is_err());
        d.ayrildi("1");
        assert!(rx.try_recv().unwrap().contains("\"gitti\""));
        assert_eq!(d.liste().len(), 1);
    }

    #[test]
    fn iki_baglanti_tek_kisi() {
        let d = Durum::default();
        let mut rx = d.abone();
        d.geldi(&k("1", "Ali"));
        d.geldi(&k("1", "Ali"));
        assert!(rx.try_recv().is_ok());
        assert!(
            rx.try_recv().is_err(),
            "ikinci bağlantı yeniden 'geldi' demez"
        );
        d.ayrildi("1");
        assert_eq!(d.liste().len(), 1, "bir bağlantısı hâlâ açık");
        d.ayrildi("1");
        assert!(d.liste().is_empty());
    }

    #[test]
    fn kisiye_ozel() {
        let d = Durum::default();
        let (a1, mut r1) = mpsc::unbounded_channel();
        let (a2, mut r2) = mpsc::unbounded_channel();
        let (b, mut rb) = mpsc::unbounded_channel();
        d.kaydol("1", a1.clone());
        d.kaydol("1", a2);
        d.kaydol("2", b);
        d.kisiye("1", &json!({"tip": "mesaj"}));
        assert!(r1.try_recv().is_ok() && r2.try_recv().is_ok());
        assert!(rb.try_recv().is_err());
        d.birak("1", &a1);
        d.kisiye("1", &json!({"tip": "mesaj"}));
        assert!(r1.try_recv().is_err() && r2.try_recv().is_ok());
    }

    #[test]
    fn oyun_adi_temizlenir() {
        assert_eq!(oyun_adi(None), None);
        assert_eq!(oyun_adi(Some("   ")), None);
        assert_eq!(oyun_adi(Some(&"x".repeat(200))).unwrap().len(), 80);
    }
}
