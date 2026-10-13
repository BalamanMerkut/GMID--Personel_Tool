//! Ekran kartı sürücüsü: kurulu sürüm ve (NVIDIA'da) en yeni Game Ready sürümü. Yenisi çıkınca bir kez
//! Windows bildirimi gelir; Sistem özelliklerinde "Yeni sürücü · İndir" görünür.
//!
//! NVIDIA'nın sürücü arama servisi: önce kartın adı NVIDIA'nın ürün listesinde bulunur (seri ve ürün
//! numarası), sonra o ürün için en yeni WHQL (DCH) sürücüsü sorulur. AMD ve Intel'in açık bir servisi yok:
//! kurulu sürüm ve resmî indirme sayfası gösterilir (AMD Software ve Intel uygulaması kendileri de bildirir).

use crate::db::Db;
use serde::Serialize;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const NV_PRODUCTS: &str = "https://www.nvidia.com/Download/API/lookupValueSearch.aspx?TypeID=3";
const NV_LOOKUP: &str = "https://gfwsl.geforce.com/services_toolkit/services/com/nvidia/services/AjaxDriverService.php?func=DriverManualLookup&osID=57&languageCode=1033&isWHQL=1&dch=1&sort1=0&numberOfResults=1";
const NV_PAGE: &str = "https://www.nvidia.com/tr-tr/geforce/drivers/";
const AMD_PAGE: &str = "https://www.amd.com/en/support/download/drivers.html";
const INTEL_PAGE: &str = "https://www.intel.com/content/www/us/en/download-center/home.html";
const NOTIFIED_KEY: &str = "drivers.notified";

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// "nvidia" | "amd" | "intel" | ""
    pub vendor: &'static str,
    pub gpu: String,
    /// Kurulu sürüm (NVIDIA'da 566.14 biçiminde).
    pub installed: String,
    /// En yeni sürüm (yalnızca NVIDIA).
    pub latest: Option<String>,
    pub released: Option<String>,
    /// Daha yeni sürüm var.
    pub newer: bool,
    /// İndirme sayfası.
    pub url: String,
    pub checked_at: i64,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct Drivers {
    last: Mutex<Option<Status>>,
}

/// Windows sürücü sürümü → NVIDIA sürümü: "32.0.15.6614" → "566.14" (son iki parçanın son 5 hanesi).
pub fn nvidia_version(windows: &str) -> Option<String> {
    let parts: Vec<&str> = windows.trim().split('.').collect();
    if parts.len() < 4 {
        return None;
    }
    let tail = format!("{}{}", parts[parts.len() - 2], parts[parts.len() - 1]);
    if tail.len() < 5 || !tail.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let d = &tail[tail.len() - 5..];
    Some(format!("{}.{}", &d[..3], &d[3..]))
}

/// "566.14" < "571.96"
pub fn older(installed: &str, latest: &str) -> bool {
    let parse =
        |v: &str| -> Vec<u32> { v.split('.').filter_map(|x| x.trim().parse().ok()).collect() };
    let (a, b) = (parse(installed), parse(latest));
    !a.is_empty() && !b.is_empty() && a < b
}

fn norm(name: &str) -> String {
    name.to_lowercase()
        .replace("nvidia", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// NVIDIA ürün listesinden (XML) kartın seri ve ürün numarası: `<LookupValue ParentID="127">
/// <Name>GeForce RTX 4090</Name><Value>995</Value></LookupValue>`.
pub fn find_product(xml: &str, gpu: &str) -> Option<(String, String)> {
    let want = norm(gpu);
    let mut loose = None;
    for block in xml.split("<LookupValue").skip(1) {
        let parent = block
            .split("ParentID=\"")
            .nth(1)
            .and_then(|r| r.split('"').next())
            .unwrap_or("");
        let tag = |t: &str| {
            block
                .split(&format!("<{t}>"))
                .nth(1)
                .and_then(|r| r.split(&format!("</{t}>")).next())
                .map(str::trim)
        };
        let (Some(name), Some(value)) = (tag("Name"), tag("Value")) else {
            continue;
        };
        if parent.is_empty() {
            continue;
        }
        let n = norm(name);
        if n == want {
            return Some((parent.to_string(), value.to_string()));
        }
        // "GeForce RTX 4060 Laptop GPU" ↔ "GeForce RTX 4060 Laptop GPU" farklı yazımları için yedek.
        if loose.is_none() && !n.is_empty() && (want.starts_with(&n) || n.starts_with(&want)) {
            loose = Some((parent.to_string(), value.to_string()));
        }
    }
    loose
}

/// Sürücü servisinin yanıtı: sürüm, yayın tarihi.
pub fn parse_lookup(json: &serde_json::Value) -> Option<(String, Option<String>)> {
    let info = &json["IDS"][0]["downloadInfo"];
    let version = info["Version"].as_str()?.trim().to_string();
    if version.is_empty() {
        return None;
    }
    let released = info["ReleaseDateTime"]
        .as_str()
        .map(|s| s.trim().to_string());
    Some((version, released))
}

fn fetch(url: &str) -> Result<String, String> {
    let agent = crate::net::agent(Duration::from_secs(8), Duration::from_secs(20));
    let mut r = agent.get(url).call().map_err(|e| e.to_string())?;
    if r.status() != 200 {
        return Err(format!("sunucu {} döndü", r.status()));
    }
    r.body_mut().read_to_string().map_err(|e| e.to_string())
}

fn latest_nvidia(gpu: &str) -> Result<(String, Option<String>), String> {
    let xml = fetch(NV_PRODUCTS)?;
    let (psid, pfid) = find_product(&xml, gpu).ok_or("kart NVIDIA'nın listesinde bulunamadı")?;
    let body = fetch(&format!("{NV_LOOKUP}&psid={psid}&pfid={pfid}"))?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    parse_lookup(&json).ok_or_else(|| "sürücü bilgisi okunamadı".into())
}

impl Drivers {
    pub fn last(&self) -> Option<Status> {
        self.last.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Kartı ve sürücüyü okur, NVIDIA ise en yeni sürümü sorar.
    pub fn check(&self, app: &AppHandle) -> Status {
        let specs = app.state::<crate::sysinfo::SysInfo>().get(false);
        let mut s = Status {
            checked_at: crate::util::now_ms(),
            ..Default::default()
        };
        match specs {
            Err(e) => s.error = Some(e),
            Ok(spec) => {
                // Asıl kart: listede belleği en büyük olan (sysinfo sıralıyor); NVIDIA varsa o.
                let gpu = spec
                    .gpu
                    .iter()
                    .find(|g| g.name.to_lowercase().contains("nvidia"))
                    .or(spec.gpu.first());
                if let Some(g) = gpu {
                    let lower = g.name.to_lowercase();
                    s.gpu = g.name.clone();
                    if lower.contains("nvidia") {
                        s.vendor = "nvidia";
                        s.url = NV_PAGE.into();
                        s.installed = nvidia_version(&g.driver).unwrap_or_else(|| g.driver.clone());
                        match latest_nvidia(&g.name) {
                            Ok((v, released)) => {
                                s.newer = older(&s.installed, &v);
                                s.latest = Some(v);
                                s.released = released;
                            }
                            Err(e) => s.error = Some(e),
                        }
                    } else {
                        s.vendor = if lower.contains("amd") || lower.contains("radeon") {
                            "amd"
                        } else if lower.contains("intel") {
                            "intel"
                        } else {
                            ""
                        };
                        s.url = match s.vendor {
                            "amd" => AMD_PAGE,
                            "intel" => INTEL_PAGE,
                            _ => "",
                        }
                        .into();
                        s.installed = g.driver.clone();
                    }
                }
            }
        }
        if let Some(e) = &s.error {
            crate::log!("Sürücü denetimi: {e}");
        }
        *self.last.lock().unwrap_or_else(|e| e.into_inner()) = Some(s.clone());
        notify(app, &s);
        s
    }
}

/// Yeni sürüm için bir kez bildirim (aynı sürüm için tekrar etmez).
fn notify(app: &AppHandle, s: &Status) {
    let (true, Some(latest)) = (s.newer, s.latest.as_ref()) else {
        return;
    };
    let db = app.state::<Db>();
    if db.setting_json::<String>(NOTIFIED_KEY).as_deref() == Some(latest.as_str()) {
        return;
    }
    let _ = db.set_setting(NOTIFIED_KEY, &serde_json::json!(latest).to_string());
    crate::overlay::system_notification(
        app,
        "drivers.notify",
        "Yeni NVIDIA sürücüsü",
        &format!(
            "{latest} çıktı (sende {}). Kontrol Odası → Sistem özellikleri'nden indirebilirsin.",
            s.installed
        ),
    );
}

/// Açılıştan 3 dk sonra ve 12 saatte bir (Windows'ta).
pub fn start(app: &AppHandle) {
    if !cfg!(windows) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(180));
        loop {
            app.state::<Drivers>().check(&app);
            std::thread::sleep(Duration::from_secs(12 * 3600));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(nvidia_version("32.0.15.6614").as_deref(), Some("566.14"));
        assert_eq!(nvidia_version("31.0.15.3623").as_deref(), Some("536.23"));
        assert_eq!(nvidia_version("32.0.15.7283").as_deref(), Some("572.83"));
        assert_eq!(nvidia_version("abc"), None);
        assert!(older("566.14", "572.83"));
        assert!(older("566.14", "566.36"));
        assert!(!older("572.83", "572.83"));
        assert!(!older("580.10", "572.83"));
    }

    #[test]
    fn products_and_lookup_are_parsed() {
        let xml = r#"<?xml version="1.0"?><LookupValueSearch><LookupValues>
          <LookupValue ParentID="127" RequiresProduct="True"><Name>GeForce RTX 4090</Name><Value>995</Value></LookupValue>
          <LookupValue ParentID="127" RequiresProduct="True"><Name>GeForce RTX 4070</Name><Value>1015</Value></LookupValue>
          <LookupValue ParentID="129" RequiresProduct="True"><Name>GeForce RTX 4060 Laptop GPU</Name><Value>1007</Value></LookupValue>
        </LookupValues></LookupValueSearch>"#;
        assert_eq!(
            find_product(xml, "NVIDIA GeForce RTX 4070"),
            Some(("127".into(), "1015".into()))
        );
        assert_eq!(
            find_product(xml, "NVIDIA GeForce RTX 4060 Laptop GPU"),
            Some(("129".into(), "1007".into()))
        );
        assert_eq!(find_product(xml, "NVIDIA GeForce GTX 1050"), None);
        let j: serde_json::Value = serde_json::from_str(
            r#"{"Success":"1","IDS":[{"downloadInfo":{"Version":"572.83","ReleaseDateTime":"Tue Mar 18, 2025","DownloadURL":"https://us.download.nvidia.com/x.exe"}}]}"#,
        )
        .unwrap();
        assert_eq!(
            parse_lookup(&j),
            Some(("572.83".into(), Some("Tue Mar 18, 2025".into())))
        );
        assert_eq!(parse_lookup(&serde_json::json!({"Success":"0"})), None);
    }
}
