//! Fırsatlar: ücretsiz oyunlar (her platform) ve istek listesindeki oyunların indirimleri. Hepsi anahtarsız.
//!
//! - **GamerPower** (`/api/giveaways`): Steam, Epic, GOG, PlayStation, Xbox, Ubisoft, EA, Prime Gaming, itch…
//!   dağıtımlarını tek listede toplar.
//! - **Epic** (`freeGamesPromotions`): Epic'in haftalık ücretsiz oyunları ve gelecek haftanınki (kesin tarihler).
//! - **Steam** (`appdetails`, Türkiye fiyatı) ve **CheapShark** (GOG, Humble, EA, Ubisoft, Epic, Fanatical…):
//!   Oyun Arşivi'nde "İstiyorum" dediğin oyunların indirimleri.
//!
//! 3 saatte bir bakılır; yeni ücretsiz oyun ve istek listesinde yeni/daha büyük indirim Windows bildirimiyle gelir.
//! İlk bakışta (henüz hiçbir şey görülmemişken) bildirim yağmuru olmasın diye mevcutlar sessizce "görüldü" sayılır.

use crate::db::Db;
use crate::util::now_ms;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const CACHE_KEY: &str = "deals.cache";
const SEEN_KEY: &str = "deals.seen";
pub const PREFS_KEY: &str = "deals.prefs";

const GAMERPOWER: &str = "https://www.gamerpower.com/api/giveaways?type=game";
const EPIC: &str = "https://store-site-backend-static.ak.epicgames.com/freeGamesPromotions?locale=tr&country=TR&allowCountries=TR";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Giveaway {
    pub id: String,
    pub title: String,
    /// epic | steam | gog | playstation | xbox | ubisoft | ea | prime | itch | battlenet | mobile | other
    pub store: String,
    /// Kaynağın yazdığı platformlar ("PC, Steam").
    pub platforms: String,
    /// "$29.99" gibi normal fiyatı (biliniyorsa).
    pub worth: Option<String>,
    pub image: Option<String>,
    pub url: String,
    /// ms; bilinmiyorsa `None`.
    pub starts: Option<i64>,
    pub ends: Option<i64>,
    pub upcoming: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WishDeal {
    pub game_id: String,
    pub name: String,
    pub cover: Option<String>,
    /// Mağazanın adı ("Steam", "GOG"…).
    pub store: String,
    pub price: String,
    pub regular: Option<String>,
    pub discount: u32,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Snapshot {
    pub free: Vec<Giveaway>,
    pub upcoming: Vec<Giveaway>,
    pub wishlist: Vec<WishDeal>,
    /// İstek listesindeki oyun sayısı (indirimde olmasa da).
    pub wish_count: usize,
    pub fetched_at: Option<i64>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Prefs {
    pub notify_free: bool,
    /// Bildirim gelecek mağazalar (boşsa hepsi).
    pub stores: Vec<String>,
    pub notify_wish: bool,
    pub min_discount: u32,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            notify_free: true,
            stores: Vec::new(),
            notify_wish: true,
            min_discount: 20,
        }
    }
}

#[derive(Default)]
pub struct Deals {
    snapshot: Mutex<Option<Snapshot>>,
    busy: std::sync::atomic::AtomicBool,
}

fn agent() -> ureq::Agent {
    crate::net::agent(Duration::from_secs(10), Duration::from_secs(30))
}

fn get_json(url: &str) -> Result<Value, String> {
    let mut r = agent()
        .get(url)
        .header("Accept", "application/json")
        .call()
        .map_err(|e| e.to_string())?;
    if r.status() != 200 {
        return Err(format!("{} döndü", r.status()));
    }
    let text = r
        .body_mut()
        .with_config()
        .limit(16 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// "PC, Epic Games Store" → epic. Önce mağaza, sonra konsol; yalnız telefon → mobile.
pub fn store_of(platforms: &str, title: &str) -> &'static str {
    let p = format!("{platforms} {title}").to_lowercase();
    let has = |s: &str| p.contains(s);
    if has("epic") {
        "epic"
    } else if has("steam") {
        "steam"
    } else if has("gog") {
        "gog"
    } else if has("ubisoft") || has("uplay") {
        "ubisoft"
    } else if has("origin") || has("ea app") || has("(ea)") {
        "ea"
    } else if has("prime") || has("amazon") {
        "prime"
    } else if has("itch") {
        "itch"
    } else if has("battle.net") || has("battlenet") {
        "battlenet"
    } else if has("ps4") || has("ps5") || has("playstation") {
        "playstation"
    } else if has("xbox") || has("microsoft") {
        "xbox"
    } else if (has("android") || has("ios")) && !has("pc") {
        "mobile"
    } else {
        "other"
    }
}

/// "Sniper Elite 4 (Epic Games) Giveaway" → "Sniper Elite 4"
pub fn clean_title(t: &str) -> String {
    let mut s = t.trim().to_string();
    for suffix in [" Giveaway", " giveaway", " Key Giveaway", " (Steam Key)"] {
        if let Some(x) = s.strip_suffix(suffix) {
            s = x.trim().to_string();
        }
    }
    // Sondaki parantez içinde mağaza adı varsa at.
    if s.ends_with(')') {
        if let Some(i) = s.rfind(" (") {
            let inner = s[i + 2..s.len() - 1].to_lowercase();
            let stores = [
                "epic",
                "steam",
                "gog",
                "ubisoft",
                "origin",
                "itch",
                "prime",
                "xbox",
                "ps4",
                "ps5",
                "playstation",
                "battle.net",
                "ea",
            ];
            if stores.iter().any(|k| inner.contains(k)) {
                s.truncate(i);
            }
        }
    }
    s
}

/// "2025-10-16 23:59:00" (GamerPower, UTC) ya da ISO 8601 → ms.
fn parse_time(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("n/a") {
        return None;
    }
    if let Ok(t) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(t.timestamp_millis());
    }
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|t| t.and_utc().timestamp_millis())
}

pub fn parse_gamerpower(json: &Value) -> Vec<Giveaway> {
    json.as_array()
        .into_iter()
        .flatten()
        .filter(|g| {
            g["status"]
                .as_str()
                .map_or(true, |s| s.eq_ignore_ascii_case("active"))
                && g["type"]
                    .as_str()
                    .map_or(true, |t| t.eq_ignore_ascii_case("game"))
        })
        .filter_map(|g| {
            let raw = g["title"].as_str()?;
            let platforms = g["platforms"].as_str().unwrap_or_default().to_string();
            Some(Giveaway {
                id: format!(
                    "gp-{}",
                    g["id"]
                        .as_i64()
                        .map(|i| i.to_string())
                        .or_else(|| g["id"].as_str().map(String::from))?
                ),
                title: clean_title(raw),
                store: store_of(&platforms, raw).to_string(),
                worth: g["worth"]
                    .as_str()
                    .filter(|w| !w.is_empty() && !w.eq_ignore_ascii_case("n/a"))
                    .map(String::from),
                image: g["image"]
                    .as_str()
                    .or(g["thumbnail"].as_str())
                    .map(String::from),
                url: g["open_giveaway_url"]
                    .as_str()
                    .or(g["gamerpower_url"].as_str())?
                    .to_string(),
                starts: g["published_date"].as_str().and_then(parse_time),
                ends: g["end_date"].as_str().and_then(parse_time),
                upcoming: false,
                platforms,
            })
        })
        .collect()
}

/// Epic: şu an ücretsiz olanlar ve sıradaki ücretsizler.
pub fn parse_epic(json: &Value, now: i64) -> (Vec<Giveaway>, Vec<Giveaway>) {
    let mut free = Vec::new();
    let mut upcoming = Vec::new();
    let elements = json["data"]["Catalog"]["searchStore"]["elements"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for e in &elements {
        let Some(title) = e["title"].as_str() else {
            continue;
        };
        // Ücretsiz dönemler: indirim yüzdesi 0 = fiyatın %0'ı.
        let offers = |key: &str| -> Vec<(Option<i64>, Option<i64>)> {
            e["promotions"][key]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|o| o["promotionalOffers"].as_array().into_iter().flatten())
                .filter(|o| o["discountSetting"]["discountPercentage"].as_i64() == Some(0))
                .map(|o| {
                    (
                        o["startDate"].as_str().and_then(parse_time),
                        o["endDate"].as_str().and_then(parse_time),
                    )
                })
                .collect()
        };
        let slug = e["offerMappings"][0]["pageSlug"]
            .as_str()
            .or(e["catalogNs"]["mappings"][0]["pageSlug"].as_str())
            .or(e["productSlug"].as_str())
            .map(|s| s.trim_end_matches("/home").to_string())
            .filter(|s| !s.is_empty());
        let url = match slug {
            Some(s) => format!("https://store.epicgames.com/tr/p/{s}"),
            None => "https://store.epicgames.com/tr/free-games".into(),
        };
        let image = [
            "OfferImageWide",
            "DieselStoreFrontWide",
            "Thumbnail",
            "OfferImageTall",
        ]
        .iter()
        .find_map(|k| {
            e["keyImages"]
                .as_array()?
                .iter()
                .find(|i| i["type"].as_str() == Some(k))?["url"]
                .as_str()
                .map(String::from)
        });
        let worth = e["price"]["totalPrice"]["fmtPrice"]["originalPrice"]
            .as_str()
            .filter(|p| *p != "0" && !p.is_empty())
            .map(String::from);
        let id = e["id"].as_str().unwrap_or(title).to_string();
        let make = |starts, ends, upcoming| Giveaway {
            id: format!("epic-{id}"),
            title: title.to_string(),
            store: "epic".into(),
            platforms: "PC, Epic Games Store".into(),
            worth: worth.clone(),
            image: image.clone(),
            url: url.clone(),
            starts,
            ends,
            upcoming,
        };
        for (s, en) in offers("promotionalOffers") {
            let live = s.map_or(true, |s| s <= now) && en.map_or(true, |en| en > now);
            if live {
                free.push(make(s, en, false));
            }
        }
        for (s, en) in offers("upcomingPromotionalOffers") {
            upcoming.push(make(s, en, true));
        }
    }
    (free, upcoming)
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Epic'in kendi kaydı GamerPower'dakinden önce gelir; aynı oyun (aynı mağazada) bir kez.
pub fn merge(epic: Vec<Giveaway>, others: Vec<Giveaway>) -> Vec<Giveaway> {
    let mut out: Vec<Giveaway> = Vec::new();
    for g in epic.into_iter().chain(others) {
        if !out
            .iter()
            .any(|x| x.store == g.store && norm(&x.title) == norm(&g.title))
        {
            out.push(g);
        }
    }
    // Yakında bitenler önce; bitişi bilinmeyenler sonda.
    out.sort_by_key(|g| g.ends.unwrap_or(i64::MAX));
    out
}

/// Steam `appdetails?filters=price_overview` → (fiyat, normal fiyat, indirim %).
pub fn parse_steam_price(json: &Value, appid: i64) -> Option<(String, Option<String>, u32)> {
    let p = &json[appid.to_string()]["data"]["price_overview"];
    let discount = p["discount_percent"].as_u64()? as u32;
    Some((
        p["final_formatted"].as_str()?.to_string(),
        p["initial_formatted"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from),
        discount,
    ))
}

const CHEAPSHARK_STORES: &[(&str, &str)] = &[
    ("1", "Steam"),
    ("3", "GreenManGaming"),
    ("7", "GOG"),
    ("8", "EA"),
    ("11", "Humble"),
    ("13", "Ubisoft"),
    ("15", "Fanatical"),
    ("25", "Epic"),
    ("27", "Gamesplanet"),
    ("31", "Blizzard"),
];

/// CheapShark `games?id=` → en büyük indirimli mağaza.
pub fn parse_cheapshark(json: &Value) -> Option<(String, String, Option<String>, u32, String)> {
    json["deals"]
        .as_array()?
        .iter()
        .filter_map(|d| {
            let store = CHEAPSHARK_STORES
                .iter()
                .find(|(id, _)| d["storeID"].as_str() == Some(id))?
                .1;
            let savings: f64 = d["savings"].as_str()?.parse().ok()?;
            let price = d["price"].as_str()?;
            let retail = d["retailPrice"].as_str();
            let deal = d["dealID"].as_str()?;
            Some((
                store.to_string(),
                format!("${price}"),
                retail.map(|r| format!("${r}")),
                savings.round() as u32,
                format!("https://www.cheapshark.com/redirect?dealID={deal}"),
            ))
        })
        .max_by_key(|d| d.3)
}

struct Wish {
    id: String,
    name: String,
    cover: Option<String>,
    steam: Option<i64>,
}

fn wishlist(db: &Db) -> Vec<Wish> {
    let conn = db.conn();
    let Ok(mut st) = conn.prepare(
        "SELECT g.id, g.name, g.cover, g.steam_appid FROM reviews r JOIN games g ON g.id = r.subject_id
         WHERE r.subject_kind = 'game' AND r.status = 'wishlist' ORDER BY g.name",
    ) else {
        return Vec::new();
    };
    st.query_map([], |r| {
        Ok(Wish {
            id: r.get(0)?,
            name: r.get(1)?,
            cover: r.get(2)?,
            steam: r.get(3)?,
        })
    })
    .map(|rows| rows.filter_map(Result::ok).collect())
    .unwrap_or_default()
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn wish_deal(w: &Wish) -> Option<WishDeal> {
    let mut best: Option<WishDeal> = None;
    let mut consider = |d: WishDeal| {
        if d.discount > 0 && best.as_ref().map_or(true, |b| d.discount > b.discount) {
            best = Some(d);
        }
    };
    if let Some(appid) = w.steam {
        if let Some((price, regular, discount)) = get_json(&format!(
            "https://store.steampowered.com/api/appdetails?appids={appid}&cc=tr&filters=price_overview"
        ))
        .ok()
        .and_then(|j| parse_steam_price(&j, appid))
        {
            consider(WishDeal {
                game_id: w.id.clone(),
                name: w.name.clone(),
                cover: w.cover.clone(),
                store: "Steam".into(),
                price,
                regular,
                discount,
                url: format!("https://store.steampowered.com/app/{appid}"),
            });
        }
        std::thread::sleep(Duration::from_millis(400));
    }
    // Diğer mağazalar: Steam kimliğiyle, yoksa adıyla.
    let search = match w.steam {
        Some(a) => format!("https://www.cheapshark.com/api/1.0/games?steamAppID={a}&limit=1"),
        None => format!(
            "https://www.cheapshark.com/api/1.0/games?title={}&limit=5&exact=0",
            enc(&w.name)
        ),
    };
    let game_id = get_json(&search).ok().and_then(|j| {
        j.as_array()?.iter().find(|g| {
            w.steam.is_some()
                || g["external"]
                    .as_str()
                    .is_some_and(|e| norm(e) == norm(&w.name))
        })?["gameID"]
            .as_str()
            .map(String::from)
    });
    if let Some(gid) = game_id {
        std::thread::sleep(Duration::from_millis(400));
        if let Some((store, price, regular, discount, url)) = get_json(&format!(
            "https://www.cheapshark.com/api/1.0/games?id={gid}"
        ))
        .ok()
        .and_then(|j| parse_cheapshark(&j))
        {
            consider(WishDeal {
                game_id: w.id.clone(),
                name: w.name.clone(),
                cover: w.cover.clone(),
                store,
                price,
                regular,
                discount,
                url,
            });
        }
    }
    best
}

impl Deals {
    pub fn snapshot(&self, db: &Db) -> Snapshot {
        let mut s = self.snapshot.lock().unwrap_or_else(|e| e.into_inner());
        if s.is_none() {
            *s = db.setting_json(CACHE_KEY);
        }
        s.clone().unwrap_or_default()
    }

    pub fn refresh(&self, app: &AppHandle) -> Snapshot {
        use std::sync::atomic::Ordering;
        if self.busy.swap(true, Ordering::SeqCst) {
            return self.snapshot(&app.state::<Db>());
        }
        let db = app.state::<Db>();
        let now = now_ms();
        let mut errors = Vec::new();
        let (epic_free, upcoming) = match get_json(EPIC) {
            Ok(j) => parse_epic(&j, now),
            Err(e) => {
                errors.push(format!("Epic: {e}"));
                (Vec::new(), Vec::new())
            }
        };
        let others = match get_json(GAMERPOWER) {
            Ok(j) => parse_gamerpower(&j),
            Err(e) => {
                errors.push(format!("GamerPower: {e}"));
                Vec::new()
            }
        };
        let free: Vec<Giveaway> = merge(epic_free, others)
            .into_iter()
            .filter(|g| g.ends.map_or(true, |e| e > now))
            .collect();
        let wishes = wishlist(&db);
        let mut deals: Vec<WishDeal> = wishes.iter().filter_map(wish_deal).collect();
        deals.sort_by_key(|d| std::cmp::Reverse(d.discount));
        let previous = self.snapshot(&db);
        let snap = Snapshot {
            // Kaynak yanıt vermediyse eldekini koru.
            free: if free.is_empty() && !errors.is_empty() {
                previous.free
            } else {
                free
            },
            upcoming: if upcoming.is_empty() && !errors.is_empty() {
                previous.upcoming
            } else {
                upcoming
            },
            wishlist: deals,
            wish_count: wishes.len(),
            fetched_at: Some(now),
            error: (!errors.is_empty()).then(|| errors.join(" · ")),
        };
        if let Some(e) = &snap.error {
            crate::log!("Fırsatlar: {e}");
        }
        notify(app, &db, &snap);
        if let Ok(j) = serde_json::to_string(&snap) {
            let _ = db.set_setting(CACHE_KEY, &j);
        }
        *self.snapshot.lock().unwrap_or_else(|e| e.into_inner()) = Some(snap.clone());
        self.busy.store(false, Ordering::SeqCst);
        let _ = app.emit("deals", &snap);
        snap
    }
}

pub fn store_label(store: &str) -> &'static str {
    match store {
        "epic" => "Epic",
        "steam" => "Steam",
        "gog" => "GOG",
        "playstation" => "PlayStation",
        "xbox" => "Xbox / Microsoft",
        "ubisoft" => "Ubisoft",
        "ea" => "EA",
        "prime" => "Prime Gaming",
        "itch" => "itch.io",
        "battlenet" => "Battle.net",
        "mobile" => "Telefon",
        _ => "Diğer",
    }
}

/// Yeni ücretsiz oyun ve yeni / daha büyük istek listesi indirimi için bildirim.
fn notify(app: &AppHandle, db: &Db, snap: &Snapshot) {
    let prefs: Prefs = db.setting_json(PREFS_KEY).unwrap_or_default();
    let first = db.setting(SEEN_KEY).ok().flatten().is_none();
    let mut seen: HashMap<String, u32> = db.setting_json(SEEN_KEY).unwrap_or_default();
    let mut lines = Vec::new();
    for g in &snap.free {
        if seen.insert(g.id.clone(), 0).is_none()
            && !first
            && prefs.notify_free
            && g.store != "mobile"
            && (prefs.stores.is_empty() || prefs.stores.contains(&g.store))
        {
            lines.push(format!("Ücretsiz: {} ({})", g.title, store_label(&g.store)));
        }
    }
    for d in &snap.wishlist {
        let key = format!("wish-{}", d.game_id);
        let before = seen.get(&key).copied().unwrap_or(0);
        if d.discount > before {
            seen.insert(key, d.discount);
            if !first && prefs.notify_wish && d.discount >= prefs.min_discount {
                lines.push(format!(
                    "İndirim: {} %{} · {} ({})",
                    d.name, d.discount, d.price, d.store
                ));
            }
        }
    }
    // İndirimi bitenler unutulur (yeniden indirime girerse yine haber verilsin).
    seen.retain(|k, _| {
        !k.starts_with("wish-")
            || snap
                .wishlist
                .iter()
                .any(|d| format!("wish-{}", d.game_id) == *k)
    });
    if let Ok(j) = serde_json::to_string(&seen) {
        let _ = db.set_setting(SEEN_KEY, &j);
    }
    if lines.is_empty() {
        return;
    }
    let title = if lines.len() == 1 {
        "GMID Fırsatlar".to_string()
    } else {
        format!("GMID Fırsatlar · {} yeni", lines.len())
    };
    let body = lines.iter().take(4).cloned().collect::<Vec<_>>().join("\n");
    crate::overlay::system_notification(app, "deals.toast", &title, &body);
}

/// Açılıştan 1 dk sonra ve 3 saatte bir.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(60));
        loop {
            app.state::<Deals>().refresh(&app);
            std::thread::sleep(Duration::from_secs(3 * 3600));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gamerpower_giveaways_are_cleaned() {
        let j: Value = serde_json::from_str(
            r#"[
              {"id":3082,"title":"Sniper Elite 4 (Epic Games) Giveaway","worth":"$59.99","thumbnail":"https://t/1.jpg","image":"https://i/1.jpg",
               "open_giveaway_url":"https://www.gamerpower.com/open/sniper","published_date":"2025-10-09 10:00:00","type":"Game",
               "platforms":"PC, Epic Games Store","end_date":"2025-10-16 23:59:00","status":"Active"},
              {"id":3083,"title":"Rogue Lords (Steam) Giveaway","worth":"N/A","image":"https://i/2.jpg","open_giveaway_url":"https://x/2",
               "type":"Game","platforms":"PC, Steam","end_date":"N/A","status":"Active"},
              {"id":3084,"title":"God of War DLC","worth":"$4.99","open_giveaway_url":"https://x/3","type":"DLC","platforms":"PS4","status":"Active"},
              {"id":3085,"title":"Mobile Game Giveaway","open_giveaway_url":"https://x/4","type":"Game","platforms":"Android","status":"Active"},
              {"id":3086,"title":"Halo Free Weekend","open_giveaway_url":"https://x/5","type":"Game","platforms":"Xbox Series X|S, Xbox One","status":"Expired"}
            ]"#,
        )
        .unwrap();
        let g = parse_gamerpower(&j);
        assert_eq!(g.len(), 3, "{g:?}");
        assert_eq!(g[0].title, "Sniper Elite 4");
        assert_eq!(g[0].store, "epic");
        assert_eq!(g[0].worth.as_deref(), Some("$59.99"));
        assert!(g[0].ends.is_some());
        assert_eq!(
            (g[1].title.as_str(), g[1].store.as_str()),
            ("Rogue Lords", "steam")
        );
        assert_eq!(g[1].worth, None);
        assert_eq!(g[1].ends, None);
        assert_eq!(g[2].store, "mobile");
    }

    #[test]
    fn stores_are_recognised() {
        assert_eq!(store_of("PC, GOG", ""), "gog");
        assert_eq!(store_of("PS4, PS5", ""), "playstation");
        assert_eq!(store_of("Xbox One, Xbox Series X|S", ""), "xbox");
        assert_eq!(store_of("PC, Ubisoft", ""), "ubisoft");
        assert_eq!(store_of("PC, DRM-Free", "Game (Prime Gaming)"), "prime");
        assert_eq!(store_of("PC, Itch.io", ""), "itch");
        assert_eq!(store_of("PC, Battle.net", ""), "battlenet");
        assert_eq!(clean_title("Hades (Steam Key) Giveaway"), "Hades");
        assert_eq!(clean_title("Alan Wake (GOG) Giveaway"), "Alan Wake");
        assert_eq!(
            clean_title("Half-Life (Remastered)"),
            "Half-Life (Remastered)"
        );
    }

    #[test]
    fn epic_free_now_and_upcoming() {
        let j: Value = serde_json::from_str(
            r#"{"data":{"Catalog":{"searchStore":{"elements":[
              {"title":"Hogwarts Legacy","id":"a1","productSlug":"hogwarts-legacy",
               "keyImages":[{"type":"Thumbnail","url":"https://e/t.jpg"},{"type":"OfferImageWide","url":"https://e/w.jpg"}],
               "price":{"totalPrice":{"discountPrice":0,"originalPrice":179900,"fmtPrice":{"originalPrice":"₺1.799,00"}}},
               "promotions":{"promotionalOffers":[{"promotionalOffers":[{"startDate":"2025-10-09T15:00:00.000Z","endDate":"2025-10-16T15:00:00.000Z","discountSetting":{"discountType":"PERCENTAGE","discountPercentage":0}}]}],"upcomingPromotionalOffers":[]}},
              {"title":"Gelecek Oyun","id":"b2","offerMappings":[{"pageSlug":"gelecek-oyun"}],"keyImages":[],
               "price":{"totalPrice":{"fmtPrice":{"originalPrice":"0"}}},
               "promotions":{"promotionalOffers":[],"upcomingPromotionalOffers":[{"promotionalOffers":[{"startDate":"2025-10-16T15:00:00.000Z","endDate":"2025-10-23T15:00:00.000Z","discountSetting":{"discountPercentage":0}}]}]}},
              {"title":"Yalnız indirimde","id":"c3","promotions":{"promotionalOffers":[{"promotionalOffers":[{"startDate":"2025-10-09T15:00:00.000Z","endDate":"2025-10-16T15:00:00.000Z","discountSetting":{"discountPercentage":50}}]}]}},
              {"title":"Promosyonsuz","id":"d4","promotions":null}
            ]}}}}"#,
        )
        .unwrap();
        let now = parse_time("2025-10-12T12:00:00Z").unwrap();
        let (free, up) = parse_epic(&j, now);
        assert_eq!(free.len(), 1);
        assert_eq!(free[0].title, "Hogwarts Legacy");
        assert_eq!(
            free[0].url,
            "https://store.epicgames.com/tr/p/hogwarts-legacy"
        );
        assert_eq!(free[0].image.as_deref(), Some("https://e/w.jpg"));
        assert_eq!(free[0].worth.as_deref(), Some("₺1.799,00"));
        assert_eq!(up.len(), 1);
        assert!(up[0].upcoming);
        assert_eq!(up[0].url, "https://store.epicgames.com/tr/p/gelecek-oyun");
        assert_eq!(up[0].worth, None);
    }

    #[test]
    fn epic_entry_wins_over_duplicate() {
        let mk = |id: &str, title: &str, store: &str, ends: Option<i64>| Giveaway {
            id: id.into(),
            title: title.into(),
            store: store.into(),
            platforms: String::new(),
            worth: None,
            image: None,
            url: String::new(),
            starts: None,
            ends,
            upcoming: false,
        };
        let m = merge(
            vec![mk("epic-1", "Hogwarts Legacy", "epic", Some(10))],
            vec![
                mk("gp-1", "Hogwarts Legacy", "epic", Some(10)),
                mk("gp-2", "Hogwarts Legacy", "steam", Some(5)),
                mk("gp-3", "Başka", "gog", None),
            ],
        );
        let ids: Vec<&str> = m.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids, ["gp-2", "epic-1", "gp-3"]);
    }

    #[test]
    fn prices_are_parsed() {
        let s: Value = serde_json::from_str(
            r#"{"1145360":{"success":true,"data":{"price_overview":{"currency":"USD","initial":2499,"final":999,"discount_percent":60,"initial_formatted":"$24.99","final_formatted":"$9.99"}}}}"#,
        )
        .unwrap();
        assert_eq!(
            parse_steam_price(&s, 1145360),
            Some(("$9.99".into(), Some("$24.99".into()), 60))
        );
        let free: Value = serde_json::from_str(r#"{"570":{"success":true,"data":[]}}"#).unwrap();
        assert_eq!(parse_steam_price(&free, 570), None);
        let c: Value = serde_json::from_str(
            r#"{"info":{"title":"Hades"},"deals":[
              {"storeID":"1","dealID":"s1","price":"12.49","retailPrice":"24.99","savings":"50.020008"},
              {"storeID":"7","dealID":"g7","price":"9.99","retailPrice":"24.99","savings":"60.024010"},
              {"storeID":"999","dealID":"x","price":"1.00","retailPrice":"24.99","savings":"96.0"}
            ]}"#,
        )
        .unwrap();
        let best = parse_cheapshark(&c).unwrap();
        assert_eq!(
            (best.0.as_str(), best.1.as_str(), best.3),
            ("GOG", "$9.99", 60)
        );
        assert!(best.4.ends_with("dealID=g7"));
    }
}
