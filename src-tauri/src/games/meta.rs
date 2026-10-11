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
    let json = get_json(&search_url(name))?;
    let want = normalize(name);
    Ok(json["items"].as_array().and_then(|items| {
        items
            .iter()
            .find(|i| i["name"].as_str().is_some_and(|n| normalize(n) == want))
            .and_then(|i| i["id"].as_i64())
    }))
}

/// Steam kullanıcı etiketleri (İngilizce, en çok oylanandan aza), mağaza sayfasından.
/// Yaş doğrulaması çerezle geçilir. Sayfa okunamazsa boş liste: etiketsiz de olur.
pub fn store_tags(appid: i64) -> Vec<String> {
    let html = (|| -> Result<String, String> {
        let mut r = agent()
            .get(&format!(
                "https://store.steampowered.com/app/{appid}/?l=english"
            ))
            .header(
                "Cookie",
                "birthtime=0; lastagecheckage=1-0-1990; wants_mature_content=1",
            )
            .call()
            .map_err(|e| e.to_string())?;
        r.body_mut()
            .with_config()
            .limit(6 * 1024 * 1024)
            .read_to_string()
            .map_err(|e| e.to_string())
    })();
    html.map(|h| parse_store_tags(&h)).unwrap_or_default()
}

pub fn parse_store_tags(html: &str) -> Vec<String> {
    // 1) Sayfadaki betik: InitAppTagModal( 123, [{"tagid":1716,"name":"Roguelike",…}, …], …
    if let Some(i) = html.find("InitAppTagModal(") {
        let rest = &html[i..];
        if let Some(j) = rest.find('[') {
            let mut it =
                serde_json::Deserializer::from_str(&rest[j..]).into_iter::<serde_json::Value>();
            if let Some(Ok(serde_json::Value::Array(list))) = it.next() {
                let tags: Vec<String> = list
                    .iter()
                    .filter_map(|t| t["name"].as_str().map(str::to_string))
                    .collect();
                if !tags.is_empty() {
                    return tags;
                }
            }
        }
    }
    // 2) Yedek: görünen etiket bağlantıları.
    html.split(r#"class="app_tag""#)
        .skip(1)
        .filter_map(|part| {
            let text = part.split_once('>')?.1.split('<').next()?.trim();
            (!text.is_empty() && text != "+").then(|| text.to_string())
        })
        .collect()
}

/// Steam etiketleri ve kategorileri tek bir Türkçe listeye: "Rogue-like", "Roguelite",
/// "Action Roguelike" hepsi **Roguelike**. Bilinmeyen etiketler alınmaz (filtre kalabalıklaşmasın).
const TAGS: &[(&[&str], &str)] = &[
    (
        &[
            "roguelike",
            "roguelite",
            "action roguelike",
            "rogue-like",
            "rogue-lite",
            "traditional roguelike",
        ],
        "Roguelike",
    ),
    (&["souls-like", "soulslike"], "Soulslike"),
    (&["metroidvania"], "Metroidvania"),
    (
        &["co-op", "online co-op", "local co-op", "co-op campaign"],
        "Co-op",
    ),
    (&["open world", "open world survival craft"], "Açık dünya"),
    (&["survival"], "Hayatta kalma"),
    (&["story rich"], "Hikâye odaklı"),
    (
        &["multiplayer", "massively multiplayer", "online pvp"],
        "Çok oyunculu",
    ),
    (&["singleplayer"], "Tek oyunculu"),
    (&["fps"], "FPS"),
    (
        &[
            "shooter",
            "third-person shooter",
            "looter shooter",
            "hero shooter",
        ],
        "Nişancı",
    ),
    (
        &["horror", "survival horror", "psychological horror"],
        "Korku",
    ),
    (&["puzzle", "puzzle platformer"], "Bulmaca"),
    (
        &[
            "platformer",
            "2d platformer",
            "3d platformer",
            "precision platformer",
        ],
        "Platform",
    ),
    (
        &[
            "strategy",
            "rts",
            "turn-based strategy",
            "grand strategy",
            "4x",
        ],
        "Strateji",
    ),
    (
        &["rpg", "action rpg", "jrpg", "crpg", "party-based rpg"],
        "RPG",
    ),
    (&["simulation", "life sim"], "Simülasyon"),
    (&["racing", "driving"], "Yarış"),
    (&["sports", "football (soccer)"], "Spor"),
    (&["sandbox"], "Sandbox"),
    (&["crafting"], "Üretim"),
    (
        &["building", "base building", "city builder", "colony sim"],
        "İnşa",
    ),
    (&["battle royale"], "Battle Royale"),
    (&["competitive", "pvp", "esports"], "Rekabetçi"),
    (&["indie"], "Bağımsız"),
    (&["pixel graphics"], "Piksel grafik"),
    (&["anime"], "Anime"),
    (
        &["hack and slash", "character action game", "beat 'em up"],
        "Hack and Slash",
    ),
    (&["fighting", "2d fighter", "3d fighter"], "Dövüş"),
    (&["stealth"], "Gizlilik"),
    (&["deckbuilding", "card game", "card battler"], "Kart"),
    (&["tactical", "tactical rpg"], "Taktik"),
    (
        &["turn-based", "turn-based combat", "turn-based tactics"],
        "Sıra tabanlı",
    ),
    (&["exploration"], "Keşif"),
    (&["atmospheric"], "Atmosferik"),
    (&["difficult"], "Zor"),
    (&["relaxing", "cozy", "casual"], "Rahatlatıcı"),
    (&["short"], "Kısa"),
    (&["funny", "comedy"], "Komik"),
    (&["sci-fi", "space"], "Bilim kurgu"),
    (&["fantasy", "dark fantasy"], "Fantastik"),
    (&["post-apocalyptic"], "Kıyamet sonrası"),
    (&["zombies"], "Zombi"),
    (&["mystery", "detective"], "Gizem"),
    (&["visual novel"], "Görsel roman"),
    (&["party game", "party"], "Parti"),
    (&["isometric"], "İzometrik"),
    (&["war", "military", "world war ii"], "Savaş"),
    (&["historical"], "Tarihi"),
    (&["choices matter", "multiple endings"], "Seçimler önemli"),
];

pub fn tr_tags(steam_tags: &[String], categories: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |t: &str| {
        if !out.iter().any(|o| o == t) {
            out.push(t.to_string());
        }
    };
    // En çok oylanan 20 etiket yeter; sonrası oyunu pek anlatmıyor.
    for t in steam_tags.iter().take(20) {
        let low = t.to_lowercase();
        if let Some((_, tr)) = TAGS.iter().find(|(en, _)| en.contains(&low.as_str())) {
            push(tr);
        }
    }
    // Mağaza kategorileri (Türkçe gelir): "Çevrimiçi Co-op", "Çok oyunculu", "Tek oyunculu"…
    for c in categories {
        let low = c.to_lowercase();
        if low.contains("co-op") || low.contains("ortak") {
            push("Co-op");
        } else if low.contains("çok oyunculu") || low.contains("multi-player") {
            push("Çok oyunculu");
        } else if low.contains("tek oyunculu") || low.contains("single-player") {
            push("Tek oyunculu");
        }
    }
    out
}

/// Mağazada adıyla arama: Oyun Arşivi'ne kütüphanede olmayan bir oyun eklerken.
#[derive(serde::Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoreHit {
    pub appid: i64,
    pub name: String,
    pub image: Option<String>,
}

pub fn store_search(term: &str) -> Result<Vec<StoreHit>, String> {
    let json = get_json(&search_url(term))?;
    Ok(json["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|i| {
                    Some(StoreHit {
                        appid: i["id"].as_i64()?,
                        name: i["name"].as_str()?.to_string(),
                        image: i["tiny_image"].as_str().map(str::to_string),
                    })
                })
                .take(10)
                .collect()
        })
        .unwrap_or_default())
}

fn search_url(term: &str) -> String {
    let mut url =
        String::from("https://store.steampowered.com/api/storesearch/?l=turkish&cc=TR&term=");
    for b in term.trim().bytes() {
        if b.is_ascii_alphanumeric() {
            url.push(b as char);
        } else {
            url.push_str(&format!("%{b:02X}"));
        }
    }
    url
}

/// Microsoft Store (anahtar gerekmez): adıyla arar, oyunun dikey posterini (2:3) döndürür. Minecraft, VALORANT,
/// League of Legends ve Xbox uygulaması oyunları burada var; Wikipedia'daki resimler çoğu zaman yatay logo.
pub fn ms_store_image(name: &str) -> Result<Option<String>, String> {
    let mut q = String::new();
    for b in name.trim().bytes() {
        if b.is_ascii_alphanumeric() {
            q.push(b as char);
        } else {
            q.push_str(&format!("%{b:02X}"));
        }
    }
    let suggest = get_json(&format!(
        "https://displaycatalog.mp.microsoft.com/v7.0/productFamilies/autosuggest?market=US&languages=en-US\
         &productFamilyNames=Games,Apps&query={q}"
    ))?;
    let Some((id, fallback)) = pick_ms_product(&suggest, name).or_else(|| {
        // Minecraft'ın mağaza adı "Minecraft: Java & Bedrock Edition for PC".
        normalize(name)
            .starts_with("minecraft")
            .then(|| ("9NXP44L49SHJ".to_string(), None))
    }) else {
        return Ok(None);
    };
    let products = get_json(&format!(
        "https://displaycatalog.mp.microsoft.com/v7.0/products?bigIds={id}&market=US&languages=en-US"
    ))?;
    Ok(pick_ms_poster(&products).or(fallback))
}

/// Arama önerilerinden adı tutan ürünün kimliği (ve önerideki küçük resim).
pub fn pick_ms_product(json: &serde_json::Value, name: &str) -> Option<(String, Option<String>)> {
    let want = normalize(name);
    let suggests: Vec<&serde_json::Value> = json["ResultSets"]
        .as_array()?
        .iter()
        .flat_map(|r| r["Suggests"].as_array().into_iter().flatten())
        .collect();
    let id_of = |s: &serde_json::Value| -> Option<String> {
        s["Metas"].as_array()?.iter().find_map(|m| {
            (m["Key"].as_str() == Some("BigCatalogId"))
                .then(|| m["Value"].as_str().map(str::to_string))
                .flatten()
        })
    };
    let title = |s: &serde_json::Value| normalize(s["Title"].as_str().unwrap_or_default());
    // Önce birebir ad, sonra "Ad: …" ile başlayan (Minecraft: Java & Bedrock Edition).
    let found = suggests
        .iter()
        .find(|s| title(s) == want && id_of(s).is_some())
        .or_else(|| {
            suggests.iter().find(|s| {
                let raw = s["Title"].as_str().unwrap_or_default().to_lowercase();
                want.len() >= 4
                    && title(s).starts_with(&want)
                    && raw.contains(':')
                    && id_of(s).is_some()
            })
        })?;
    let image = found["ImageUrl"].as_str().map(fix_ms_url);
    Some((id_of(found)?, image))
}

fn fix_ms_url(u: &str) -> String {
    if u.starts_with("//") {
        format!("https:{u}")
    } else {
        u.to_string()
    }
}

/// Ürün bilgisinden dikey poster; yoksa kutu resmi.
pub fn pick_ms_poster(json: &serde_json::Value) -> Option<String> {
    let images: Vec<&serde_json::Value> = json["Products"]
        .as_array()?
        .iter()
        .flat_map(|p| p["LocalizedProperties"].as_array().into_iter().flatten())
        .flat_map(|l| l["Images"].as_array().into_iter().flatten())
        .collect();
    let by = |purpose: &str| {
        images
            .iter()
            .filter(|i| i["ImagePurpose"].as_str() == Some(purpose))
            .max_by_key(|i| i["Height"].as_i64().unwrap_or(0))
            .and_then(|i| i["Uri"].as_str())
            .map(fix_ms_url)
    };
    by("Poster").or_else(|| by("BoxArt"))
}

/// Steam'de olmayan oyunlar (VALORANT, League of Legends, Minecraft…) için yedek kapak: İngilizce Wikipedia'da
/// "<ad> video game" araması, kısa açıklaması "video game" olan ilk sayfanın kutu resmi (600 px önizleme).
pub fn wiki_image(name: &str) -> Result<Option<String>, String> {
    let mut url = String::from(
        "https://en.wikipedia.org/w/api.php?action=query&format=json&formatversion=2&generator=search\
         &gsrlimit=5&prop=pageimages%7Cpageprops&piprop=thumbnail&pithumbsize=600&pilicense=any\
         &ppprop=wikibase-shortdesc&gsrsearch=",
    );
    for b in format!("{} video game", name.trim()).bytes() {
        if b.is_ascii_alphanumeric() {
            url.push(b as char);
        } else {
            url.push_str(&format!("%{b:02X}"));
        }
    }
    let mut r = agent()
        .get(&url)
        .header("User-Agent", WIKI_UA)
        .call()
        .map_err(|e| e.to_string())?;
    let body = r
        .body_mut()
        .with_config()
        .limit(2 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(pick_wiki_image(&json, name))
}

const WIKI_UA: &str = "GMIDTool/0.1 (https://github.com/BalamanMerkut/GMID--Personel_Tool)";

pub fn pick_wiki_image(json: &serde_json::Value, name: &str) -> Option<String> {
    let mut pages: Vec<&serde_json::Value> = json["query"]["pages"].as_array()?.iter().collect();
    pages.sort_by_key(|p| p["index"].as_i64().unwrap_or(99));
    let want = normalize(name);
    let is_game = |p: &serde_json::Value| {
        p["pageprops"]["wikibase-shortdesc"]
            .as_str()
            .is_some_and(|d| d.to_lowercase().contains("video game"))
    };
    // Önce adı tutan oyun sayfası ("Valorant" ↔ "VALORANT"), yoksa ilk oyun sayfası.
    let title_ok = |p: &serde_json::Value| {
        p["title"].as_str().is_some_and(|t| {
            let t = normalize(t.split(" (").next().unwrap_or(t));
            t == want || (want.len() >= 4 && (t.contains(&want) || want.contains(&t)))
        })
    };
    pages
        .iter()
        .find(|p| is_game(p) && title_ok(p))
        .or_else(|| pages.iter().find(|p| is_game(p)))
        .and_then(|p| p["thumbnail"]["source"].as_str())
        .map(str::to_string)
}

/// Bir resmi indirip kaydeder (Wikipedia kapağı).
pub fn download_image(url: &str, dest: &Path) -> Result<(), String> {
    let mut r = agent()
        .get(url)
        .header("User-Agent", WIKI_UA)
        .call()
        .map_err(|e| e.to_string())?;
    let bytes = r
        .body_mut()
        .with_config()
        .limit(10 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    if bytes.len() < 1024 {
        return Err("resim çok küçük".into());
    }
    if let Some(d) = dest.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(dest, bytes).map_err(|e| e.to_string())
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

    #[test]
    fn ms_store_poster_is_picked() {
        let suggest: serde_json::Value = serde_json::from_str(
            r#"{"ResultSets":[{"Source":"Games","Suggests":[
              {"Source":"Games","Title":"Minecraft Dungeons","ImageUrl":"//store-images.s-microsoft.com/d","Metas":[{"Key":"BigCatalogId","Value":"9N8NJ74FVX8Q"}]},
              {"Source":"Games","Title":"Minecraft: Java & Bedrock Edition for PC","ImageUrl":"//store-images.s-microsoft.com/m","Metas":[{"Key":"ProductType","Value":"Game"},{"Key":"BigCatalogId","Value":"9NXP44L49SHJ"}]},
              {"Source":"Games","Title":"VALORANT","ImageUrl":"//store-images.s-microsoft.com/v","Metas":[{"Key":"BigCatalogId","Value":"9NBLWGW0CJMS"}]}
            ]}]}"#,
        )
        .unwrap();
        let (id, img) = pick_ms_product(&suggest, "Minecraft").unwrap();
        assert_eq!(id, "9NXP44L49SHJ");
        assert_eq!(
            img.as_deref(),
            Some("https://store-images.s-microsoft.com/m")
        );
        assert_eq!(
            pick_ms_product(&suggest, "VALORANT").unwrap().0,
            "9NBLWGW0CJMS"
        );
        assert!(pick_ms_product(&suggest, "Hades").is_none());
        let products: serde_json::Value = serde_json::from_str(
            r#"{"Products":[{"LocalizedProperties":[{"Images":[
              {"ImagePurpose":"BoxArt","Uri":"//store-images.s-microsoft.com/box","Height":1080},
              {"ImagePurpose":"Poster","Uri":"//store-images.s-microsoft.com/p-small","Height":600},
              {"ImagePurpose":"Poster","Uri":"//store-images.s-microsoft.com/p-big","Height":1440}
            ]}]}]}"#,
        )
        .unwrap();
        assert_eq!(
            pick_ms_poster(&products).as_deref(),
            Some("https://store-images.s-microsoft.com/p-big")
        );
    }

    #[test]
    fn wiki_image_prefers_matching_game_page() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"query":{"pages":[
              {"title":"Riot Games","index":2,"pageprops":{"wikibase-shortdesc":"American video game developer"},
               "thumbnail":{"source":"https://x/riot.png"}},
              {"title":"Valorant","index":1,"pageprops":{"wikibase-shortdesc":"2020 video game"},
               "thumbnail":{"source":"https://upload.wikimedia.org/valorant.png"}},
              {"title":"Valorant Champions Tour","index":3,"pageprops":{"wikibase-shortdesc":"Esports league"}}
            ]}}"#,
        )
        .unwrap();
        assert_eq!(
            pick_wiki_image(&json, "VALORANT").as_deref(),
            Some("https://upload.wikimedia.org/valorant.png")
        );
        // "video game developer" de "video game" içeriyor ama adı tutmuyor; adı tutan önce.
        assert!(pick_wiki_image(&serde_json::json!({}), "x").is_none());
    }

    #[test]
    fn store_tags_are_parsed_and_translated() {
        let html = r#"<script>InitAppTagModal( 1145350,
            [{"tagid":42804,"name":"Action Roguelike","count":900,"browseable":true},
             {"tagid":1716,"name":"Roguelike","count":800,"browseable":true},
             {"tagid":4166,"name":"Atmospheric","count":300,"browseable":true},
             {"tagid":1,"name":"Great Soundtrack","count":200,"browseable":true}],
            [], "x", "y", false );</script>"#;
        let tags = parse_store_tags(html);
        assert_eq!(tags[0], "Action Roguelike");
        assert_eq!(tags.len(), 4);
        let fallback = r#"<a href="/tags/en/Souls-like/" class="app_tag" style="">
            Souls-like </a><a class="app_tag" >Open World</a><div class="app_tag add_button">+</div>"#;
        assert_eq!(parse_store_tags(fallback), ["Souls-like", "Open World"]);
        let tr = tr_tags(&tags, &["Çevrimiçi Co-op".into(), "Tek oyunculu".into()]);
        assert_eq!(tr, ["Roguelike", "Atmosferik", "Co-op", "Tek oyunculu"]);
    }
}
