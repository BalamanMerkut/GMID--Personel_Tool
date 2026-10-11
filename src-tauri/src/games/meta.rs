//! Oyun bilgileri ve kapaklar Steam mağazasından (anahtar gerekmez, türler Türkçe gelir).
//! Steam dışındaki oyunlar (Epic, elle eklenen) önce adıyla mağazada aranır.

use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Meta {
    pub name: Option<String>,
    pub description: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub year: Option<i64>,
    pub genres: Vec<String>,
    pub categories: Vec<String>,
    pub header: Option<String>,
}

#[derive(Deserialize)]
struct Desc {
    description: String,
}

#[derive(Deserialize)]
struct Details {
    name: Option<String>,
    short_description: Option<String>,
    #[serde(default)]
    developers: Vec<String>,
    #[serde(default)]
    publishers: Vec<String>,
    release_date: Option<serde_json::Value>,
    #[serde(default)]
    genres: Vec<Desc>,
    #[serde(default)]
    categories: Vec<Desc>,
    header_image: Option<String>,
}

fn agent() -> ureq::Agent {
    crate::net::agent(Duration::from_secs(10), Duration::from_secs(30))
}

fn get_json(url: &str) -> Result<serde_json::Value, String> {
    let mut r = agent()
        .get(url)
        .header("Accept-Language", "tr-TR,tr;q=0.9")
        .call()
        .map_err(|e| e.to_string())?;
    if r.status() != 200 {
        return Err(format!("Steam {} döndü", r.status()));
    }
    let body = r
        .body_mut()
        .with_config()
        .limit(8 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

/// "17 Eki 2024" / "Oct 17, 2024" → 2024
fn year_of(date: &str) -> Option<i64> {
    date.split(|c: char| !c.is_ascii_digit())
        .filter(|p| p.len() == 4)
        .filter_map(|p| p.parse::<i64>().ok())
        .find(|y| (1970..=2100).contains(y))
}

/// Mağaza yanıtını ayrıştırır.
pub fn parse_details(appid: i64, json: &serde_json::Value) -> Option<Meta> {
    let entry = &json[appid.to_string()];
    if !entry["success"].as_bool().unwrap_or(false) {
        return None;
    }
    let d: Details = serde_json::from_value(entry["data"].clone()).ok()?;
    let clean = |s: String| {
        // Açıklamada HTML varlıkları kalabiliyor.
        s.replace("&quot;", "\"")
            .replace("&amp;", "&")
            .replace("&#39;", "'")
            .trim()
            .to_string()
    };
    Some(Meta {
        name: d.name,
        description: d.short_description.map(clean).filter(|s| !s.is_empty()),
        developer: d.developers.into_iter().next(),
        publisher: d.publishers.into_iter().next(),
        year: d
            .release_date
            .as_ref()
            .and_then(|r| r["date"].as_str())
            .and_then(year_of),
        genres: d.genres.into_iter().map(|g| g.description).collect(),
        categories: d
            .categories
            .into_iter()
            .map(|c| c.description)
            .take(12)
            .collect(),
        header: d.header_image,
    })
}

pub fn details(appid: i64) -> Result<Option<Meta>, String> {
    let json = get_json(&format!(
        "https://store.steampowered.com/api/appdetails?appids={appid}&l=turkish&cc=tr"
    ))?;
    Ok(parse_details(appid, &json))
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Adıyla mağazada arar; adı birebir (harf/rakam olarak) tutan ilk sonucun kimliği.
pub fn search(name: &str) -> Result<Option<i64>, String> {
    let mut url =
        String::from("https://store.steampowered.com/api/storesearch/?l=turkish&cc=TR&term=");
    for b in name.trim().bytes() {
        if b.is_ascii_alphanumeric() {
            url.push(b as char);
        } else {
            url.push_str(&format!("%{b:02X}"));
        }
    }
    let json = get_json(&url)?;
    let want = normalize(name);
    Ok(json["items"].as_array().and_then(|items| {
        items
            .iter()
            .find(|i| i["name"].as_str().is_some_and(|n| normalize(n) == want))
            .and_then(|i| i["id"].as_i64())
    }))
}

/// Dikey kapak (600×900). Bulunamazsa yatay başlık resmi.
pub fn download_cover(appid: i64, header: Option<&str>, dest: &Path) -> Result<(), String> {
    let urls = [
        format!("https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{appid}/library_600x900.jpg"),
        format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_600x900.jpg"),
    ];
    let mut last = String::new();
    for url in urls.iter().map(String::as_str).chain(header) {
        match fetch_image(url) {
            Ok(bytes) => {
                if let Some(d) = dest.parent() {
                    let _ = std::fs::create_dir_all(d);
                }
                return std::fs::write(dest, bytes).map_err(|e| e.to_string());
            }
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn fetch_image(url: &str) -> Result<Vec<u8>, String> {
    let mut r = agent().get(url).call().map_err(|e| e.to_string())?;
    if r.status() != 200 {
        return Err(format!("{} döndü", r.status()));
    }
    let bytes = r
        .body_mut()
        .with_config()
        .limit(10 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    if bytes.len() < 1024 {
        return Err("resim çok küçük".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_details_are_parsed() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"1145350":{"success":true,"data":{"name":"Hades II","short_description":"Yeraltı dünyasının ötesine &quot;savaş&quot;.",
            "developers":["Supergiant Games"],"publishers":["Supergiant Games"],"release_date":{"coming_soon":false,"date":"25 Eyl 2025"},
            "genres":[{"id":"1","description":"Aksiyon"},{"id":"25","description":"Macera"}],
            "categories":[{"id":2,"description":"Tek oyunculu"}],"header_image":"https://x/h.jpg"}}}"#,
        )
        .unwrap();
        let m = parse_details(1145350, &json).unwrap();
        assert_eq!(m.year, Some(2025));
        assert_eq!(m.genres, ["Aksiyon", "Macera"]);
        assert_eq!(
            m.description.as_deref(),
            Some("Yeraltı dünyasının ötesine \"savaş\".")
        );
        assert_eq!(m.developer.as_deref(), Some("Supergiant Games"));
        let fail: serde_json::Value = serde_json::from_str(r#"{"1":{"success":false}}"#).unwrap();
        assert!(parse_details(1, &fail).is_none());
        assert_eq!(year_of("Oct 17, 2024"), Some(2024));
        assert_eq!(normalize("Hades II™"), "hadesii");
    }
}
