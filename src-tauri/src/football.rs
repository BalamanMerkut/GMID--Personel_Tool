//! Futbol: lig maçları (saat, canlı dakika, skor, gol ve kırmızı kartlar) ve puan durumu.
//!
//! Veri ESPN'in sitesinin kullandığı açık, anahtarsız JSON servisinden gelir:
//! - maçlar: `site.api.espn.com/apis/site/v2/sports/soccer/<lig>/scoreboard?dates=YYYYMMDD` (gün gün)
//! - puan durumu: `site.api.espn.com/apis/v2/sports/soccer/<lig>/standings`
//!
//! Yanıtlar kısa süre bellekte tutulur (bugün/canlı 30 sn, diğer günler 10 dk); okuma esnek yapılır, eksik alan
//! hata değildir.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Ligler: (ESPN kodu, görünen ad).
pub const LEAGUES: [(&str, &str); 11] = [
    ("tur.1", "Süper Lig"),
    ("uefa.champions", "Şampiyonlar Ligi"),
    ("uefa.europa", "Avrupa Ligi"),
    ("uefa.europa.conf", "Konferans Ligi"),
    ("eng.1", "Premier Lig"),
    ("esp.1", "La Liga"),
    ("ita.1", "Serie A"),
    ("ger.1", "Bundesliga"),
    ("fra.1", "Ligue 1"),
    ("ned.1", "Eredivisie"),
    ("por.1", "Liga Portugal"),
];

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: String,
    pub name: String,
    pub short: String,
    pub logo: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Side {
    pub team: Team,
    pub score: Option<u32>,
    pub winner: bool,
}

/// Maçtaki olay: gol (penaltı, kendi kalesine) ya da kırmızı kart.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// goal | pen | own | red
    pub kind: String,
    pub minute: String,
    pub player: String,
    /// home | away
    pub side: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Match {
    pub id: String,
    pub league: String,
    /// Başlama zamanı (UTC, ISO 8601).
    pub date: String,
    /// pre (başlamadı) | in (oynanıyor) | post (bitti, ertelendi…)
    pub state: String,
    /// Kısa durum: "67'", "İY", "MS", "Ertelendi"; başlamamışsa boş.
    pub status: String,
    pub home: Side,
    pub away: Side,
    pub events: Vec<Event>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub rank: u32,
    pub team: Team,
    pub played: i64,
    pub won: i64,
    pub draw: i64,
    pub lost: i64,
    pub gf: i64,
    pub ga: i64,
    pub points: i64,
    /// Sıranın anlamı (ör. Avrupa kupası, küme düşme) ve rengi.
    pub note: String,
    pub color: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub name: String,
    pub rows: Vec<Row>,
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn team(v: &Value) -> Team {
    let name = text(&v["displayName"]);
    let short = text(&v["shortDisplayName"]);
    Team {
        id: text(&v["id"]),
        short: if short.is_empty() {
            name.clone()
        } else {
            short
        },
        name,
        logo: v["logo"]
            .as_str()
            .or_else(|| v["logos"][0]["href"].as_str())
            .unwrap_or_default()
            .to_string(),
    }
}

/// Durumun kısa Türkçe karşılığı (ESPN durum adı, oynanıyorsa saat).
pub fn status_text(name: &str, state: &str, clock: &str) -> String {
    let fixed = match name {
        "STATUS_HALFTIME" => "İY",
        "STATUS_FULL_TIME" | "STATUS_FINAL" => "MS",
        "STATUS_FINAL_AET" => "UZ",
        "STATUS_FINAL_PEN" => "PEN",
        "STATUS_POSTPONED" => "Ertelendi",
        "STATUS_CANCELED" => "İptal",
        "STATUS_SUSPENDED" | "STATUS_ABANDONED" => "Durduruldu",
        "STATUS_DELAYED" => "Gecikti",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.into();
    }
    match state {
        "in" if !clock.is_empty() => clock.into(),
        "in" => "Canlı".into(),
        "post" => "MS".into(),
        _ => String::new(),
    }
}

/// Olayın türü: ESPN'in işaretlerine, yoksa yazısına bakılır. Sarı kartlar listeye alınmaz.
fn event_kind(d: &Value) -> Option<&'static str> {
    let t = text(&d["type"]["text"]).to_lowercase();
    let flag = |k: &str| d[k].as_bool().unwrap_or(false);
    if flag("redCard") || t.contains("red card") {
        Some("red")
    } else if flag("ownGoal") || t.contains("own goal") {
        Some("own")
    } else if flag("scoringPlay") || t.contains("goal") || t.contains("penalty - scored") {
        if flag("penaltyKick") || t.contains("penalty") {
            Some("pen")
        } else {
            Some("goal")
        }
    } else {
        None
    }
}

/// Skor: "2", 2 ya da (takım fikstüründe) {"value": 2.0, "displayValue": "2"}.
fn score_of(v: &Value) -> Option<u32> {
    let t = if v.is_object() {
        let d = text(&v["displayValue"]);
        if d.is_empty() {
            text(&v["value"])
        } else {
            d
        }
    } else {
        text(v)
    };
    t.split('.').next()?.trim().parse().ok()
}

/// Olayın dakikası: "23'" ya da (yazı yoksa) saniyeden ("value": 1380 → "23'").
fn minute_of(clock: &Value) -> String {
    let d = text(&clock["displayClock"]);
    if !d.is_empty() {
        return d;
    }
    match clock["value"].as_f64() {
        Some(sec) if sec > 0.0 => format!("{}'", (sec / 60.0).ceil() as u32),
        _ => String::new(),
    }
}

/// Bir maç: ESPN `events[]` öğesinden.
pub fn parse_match(league: &str, ev: &Value) -> Option<Match> {
    let comp = &ev["competitions"][0];
    let sides = comp["competitors"].as_array()?;
    let side_of = |which: &str| sides.iter().find(|c| c["homeAway"] == which);
    let (h, a) = (side_of("home")?, side_of("away")?);
    let st = if comp["status"].is_object() {
        &comp["status"]
    } else {
        &ev["status"]
    };
    let state = text(&st["type"]["state"]);
    let make = |c: &Value| Side {
        team: team(&c["team"]),
        score: if state == "pre" {
            None
        } else {
            score_of(&c["score"])
        },
        winner: c["winner"].as_bool().unwrap_or(false),
    };
    let (home, away) = (make(h), make(a));
    let events = comp["details"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|d| {
                    let kind = event_kind(d)?;
                    let tid = text(&d["team"]["id"]);
                    Some(Event {
                        kind: kind.into(),
                        minute: minute_of(&d["clock"]),
                        player: text(&d["athletesInvolved"][0]["displayName"]),
                        side: if tid == home.team.id { "home" } else { "away" }.into(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Match {
        id: text(&ev["id"]),
        league: league.into(),
        date: text(&ev["date"]),
        status: status_text(
            &text(&st["type"]["name"]),
            &state,
            &text(&st["displayClock"]),
        ),
        state,
        home,
        away,
        events,
    })
}

pub fn parse_scoreboard(league: &str, v: &Value) -> Vec<Match> {
    v["events"]
        .as_array()
        .map(|l| l.iter().filter_map(|e| parse_match(league, e)).collect())
        .unwrap_or_default()
}

fn stat(stats: &Value, names: &[&str]) -> i64 {
    stats
        .as_array()
        .and_then(|l| {
            l.iter().find(|s| {
                names
                    .iter()
                    .any(|n| s["name"] == *n || s["abbreviation"] == *n)
            })
        })
        .and_then(|s| s["value"].as_f64())
        .map(|x| x as i64)
        .unwrap_or(0)
}

fn rows(entries: &Value) -> Vec<Row> {
    let mut out: Vec<Row> = entries
        .as_array()
        .map(|l| {
            l.iter()
                .map(|e| {
                    let s = &e["stats"];
                    Row {
                        rank: stat(s, &["rank"]) as u32,
                        team: team(&e["team"]),
                        played: stat(s, &["gamesPlayed", "GP"]),
                        won: stat(s, &["wins", "W"]),
                        draw: stat(s, &["ties", "D"]),
                        lost: stat(s, &["losses", "L"]),
                        gf: stat(s, &["pointsFor", "F"]),
                        ga: stat(s, &["pointsAgainst", "A"]),
                        points: stat(s, &["points", "P"]),
                        note: text(&e["note"]["description"]),
                        color: text(&e["note"]["color"]),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    // Sıra yoksa puana, sonra averaja göre.
    if out.iter().all(|r| r.rank == 0) {
        out.sort_by_key(|r| (-r.points, -(r.gf - r.ga), -r.gf));
        for (i, r) in out.iter_mut().enumerate() {
            r.rank = i as u32 + 1;
        }
    } else {
        out.sort_by_key(|r| r.rank);
    }
    out
}

/// Puan durumu: tek tablo ya da gruplar (`children`).
pub fn parse_standings(v: &Value) -> Vec<Group> {
    if let Some(children) = v["children"].as_array() {
        return children
            .iter()
            .map(|c| Group {
                name: text(&c["name"]),
                rows: rows(&c["standings"]["entries"]),
            })
            .filter(|g| !g.rows.is_empty())
            .collect();
    }
    let r = rows(&v["standings"]["entries"]);
    if r.is_empty() {
        Vec::new()
    } else {
        vec![Group {
            name: text(&v["name"]),
            rows: r,
        }]
    }
}

/// Kadrodaki oyuncu.
#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub id: String,
    pub name: String,
    pub jersey: String,
    /// G (kaleci) | D (defans) | M (orta saha) | F (forvet) | "" (bilinmiyor)
    pub pos: String,
    pub age: Option<u32>,
    pub nation: String,
    pub photo: String,
}

/// Takım sayfası: bilgi, kadro, son ve gelecek maçlar.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TeamPage {
    pub team: Team,
    pub league: String,
    /// "1st in Turkish Super Lig" gibi; yoksa boş.
    pub standing: String,
    pub venue: String,
    pub players: Vec<Player>,
    pub results: Vec<Match>,
    pub fixtures: Vec<Match>,
}

/// Krallık listesinde bir oyuncu (gol, asist).
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Leader {
    pub name: String,
    pub photo: String,
    pub team: Team,
    pub value: u32,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Leaders {
    pub goals: Vec<Leader>,
    pub assists: Vec<Leader>,
}

/// Mevki kısaltması: ESPN'in adı ya da kısaltmasından (G/D/M/F).
pub fn position(v: &Value) -> String {
    let a = text(&v["abbreviation"]).to_uppercase();
    let n = text(&v["name"]).to_lowercase();
    let hit = |w: &[&str]| w.iter().any(|x| n.contains(x));
    if a.starts_with('G') || hit(&["goal"]) {
        "G"
    } else if a.starts_with('D') || hit(&["defen", "back"]) {
        "D"
    } else if a.starts_with('M') || hit(&["midfield"]) {
        "M"
    } else if a.starts_with('F') || a == "ST" || hit(&["forward", "striker", "wing"]) {
        "F"
    } else {
        ""
    }
    .into()
}

fn player(v: &Value) -> Player {
    Player {
        id: text(&v["id"]),
        name: text(&v["displayName"]),
        jersey: text(&v["jersey"]),
        pos: position(&v["position"]),
        age: v["age"].as_u64().map(|a| a as u32),
        nation: {
            let c = text(&v["citizenship"]);
            if c.is_empty() {
                text(&v["birthPlace"]["country"])
            } else {
                c
            }
        },
        photo: text(&v["headshot"]["href"]),
    }
}

/// Kadro: düz liste ya da mevkilere göre gruplu (`[{ position, items: [...] }]`).
pub fn parse_roster(v: &Value) -> Vec<Player> {
    let mut out = Vec::new();
    for a in v["athletes"].as_array().into_iter().flatten() {
        if let Some(items) = a["items"].as_array() {
            out.extend(items.iter().map(player));
        } else {
            out.push(player(a));
        }
    }
    out.retain(|p| !p.name.is_empty());
    let order = |p: &str| match p {
        "G" => 0,
        "D" => 1,
        "M" => 2,
        "F" => 3,
        _ => 4,
    };
    out.sort_by_key(|p| (order(&p.pos), p.jersey.parse::<u32>().unwrap_or(999)));
    out
}

/// Takımın maçları (fikstür uçları): bitenler yeniden eskiye, gelecekler yakından uzağa.
pub fn split_schedule(league: &str, v: &Value) -> (Vec<Match>, Vec<Match>) {
    let all = parse_scoreboard(league, v);
    let (mut done, mut next): (Vec<Match>, Vec<Match>) =
        all.into_iter().partition(|m| m.state == "post");
    done.sort_by(|a, b| b.date.cmp(&a.date));
    next.sort_by(|a, b| a.date.cmp(&b.date));
    (done, next)
}

/// Krallık listesi: `categories[]` içinden adı tutan kategori; oyuncu ve takım çoğu zaman `$ref` bağlantısıdır.
pub fn leader_refs(v: &Value, want: &str) -> Vec<(u32, String, String)> {
    let cats = v["categories"].as_array().cloned().unwrap_or_default();
    let cat = cats.iter().find(|c| {
        let n = format!("{} {}", text(&c["name"]), text(&c["displayName"])).to_lowercase();
        n.contains(want)
    });
    cat.and_then(|c| c["leaders"].as_array())
        .map(|l| {
            l.iter()
                .take(10)
                .map(|x| {
                    (
                        x["value"].as_f64().unwrap_or(0.0) as u32,
                        text(&x["athlete"]["$ref"]),
                        text(&x["team"]["$ref"]),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// `$ref` bağlantısı yalnız ESPN'in çekirdek servisine gidebilir (https'e çevrilir).
pub fn safe_ref(r: &str) -> Option<String> {
    let r = r.replacen("http://", "https://", 1);
    r.starts_with("https://sports.core.api.espn.com/")
        .then_some(r)
}

const SITE: &str = "https://site.api.espn.com/apis/site/v2/sports/soccer";
const V2: &str = "https://site.api.espn.com/apis/v2/sports/soccer";
const CORE: &str = "https://sports.core.api.espn.com/v2/sports/soccer/leagues";
/// Bugün/yakın günler ve canlı maçlar.
const SHORT: Duration = Duration::from_secs(30);
/// Geçmiş ve ileri günler, puan durumu.
const LONG: Duration = Duration::from_secs(600);

#[derive(Default)]
pub struct Football {
    cache: Mutex<HashMap<String, (Instant, Duration, Value)>>,
}

fn known(league: &str) -> bool {
    LEAGUES.iter().any(|(s, _)| *s == league)
}

fn fetch(url: &str) -> Result<Value, String> {
    let mut r = crate::net::agent(Duration::from_secs(8), Duration::from_secs(20))
        .get(url)
        .header("Accept", "application/json")
        .call()
        .map_err(|e| format!("Bağlanılamadı ({e})"))?;
    if r.status() != 200 {
        return Err(format!("Sunucu {} döndü", r.status()));
    }
    let body = r
        .body_mut()
        .with_config()
        .limit(8 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

/// Gün bugüne bir günden yakınsa kısa önbellek (skorlar değişir).
fn near_today(day: &str) -> bool {
    let now = chrono::Utc::now().date_naive();
    chrono::NaiveDate::parse_from_str(day, "%Y%m%d")
        .map(|d| (d - now).num_days().abs() <= 1)
        .unwrap_or(true)
}

impl Football {
    fn cached(&self, url: &str) -> Option<Value> {
        let c = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        c.get(url)
            .filter(|(t, ttl, _)| t.elapsed() < *ttl)
            .map(|(_, _, v)| v.clone())
    }

    fn store(&self, url: &str, ttl: Duration, v: &Value) {
        let mut c = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        c.retain(|_, (t, ttl, _)| t.elapsed() < *ttl);
        c.insert(url.to_string(), (Instant::now(), ttl, v.clone()));
    }

    fn day(&self, league: &str, day: &str) -> Result<Vec<Match>, String> {
        let url = format!("{SITE}/{league}/scoreboard?dates={day}&limit=200");
        if let Some(v) = self.cached(&url) {
            return Ok(parse_scoreboard(league, &v));
        }
        let v = fetch(&url)?;
        let list = parse_scoreboard(league, &v);
        let live = list.iter().any(|m| m.state == "in");
        self.store(&url, if live || near_today(day) { SHORT } else { LONG }, &v);
        Ok(list)
    }

    /// Seçilen ligler ve günler (YYYYMMDD) için maçlar, başlama saatine göre. İstekler aynı anda gider;
    /// bazıları başarısız olursa gelenler gösterilir, hiçbiri gelmezse hata döner.
    pub fn matches(&self, leagues: &[String], days: &[String]) -> Result<Vec<Match>, String> {
        let jobs: Vec<(&str, &str)> = leagues
            .iter()
            .filter(|l| known(l))
            .flat_map(|l| {
                days.iter()
                    .filter(|d| d.len() == 8 && d.chars().all(|c| c.is_ascii_digit()))
                    .map(move |d| (l.as_str(), d.as_str()))
            })
            .take(60)
            .collect();
        if jobs.is_empty() {
            return Ok(Vec::new());
        }
        let results: Vec<Result<Vec<Match>, String>> = std::thread::scope(|s| {
            let handles: Vec<_> = jobs
                .iter()
                .map(|(l, d)| s.spawn(move || self.day(l, d)))
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|_| Err("İstek yarıda kaldı".into()))
                })
                .collect()
        });
        let mut out: Vec<Match> = Vec::new();
        let mut first_err = None;
        let mut ok = false;
        for r in results {
            match r {
                Ok(list) => {
                    ok = true;
                    for m in list {
                        if !out.iter().any(|x| x.id == m.id && x.league == m.league) {
                            out.push(m);
                        }
                    }
                }
                Err(e) => {
                    first_err.get_or_insert(e);
                }
            }
        }
        if !ok {
            return Err(first_err.unwrap_or_else(|| "Maçlar alınamadı".into()));
        }
        out.sort_by(|a, b| a.date.cmp(&b.date).then(a.league.cmp(&b.league)));
        Ok(out)
    }

    pub fn standings(&self, league: &str) -> Result<Vec<Group>, String> {
        if !known(league) {
            return Err("Bilinmeyen lig".into());
        }
        let url = format!("{V2}/{league}/standings");
        let v = match self.cached(&url) {
            Some(v) => v,
            None => {
                let v = fetch(&url)?;
                self.store(&url, LONG, &v);
                v
            }
        };
        Ok(parse_standings(&v))
    }

    fn get_cached(&self, url: &str, ttl: Duration) -> Result<Value, String> {
        if let Some(v) = self.cached(url) {
            return Ok(v);
        }
        let v = fetch(url)?;
        self.store(url, ttl, &v);
        Ok(v)
    }

    /// Takım sayfası: kadro, son maçlar ve fikstür aynı anda istenir; gelmeyen bölüm boş kalır.
    pub fn team(&self, league: &str, id: &str) -> Result<TeamPage, String> {
        if !known(league) || id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            return Err("Bilinmeyen takım".into());
        }
        let base = format!("{SITE}/{league}/teams/{id}");
        let urls = [
            base.clone(),
            format!("{base}/roster"),
            format!("{base}/schedule"),
            format!("{base}/schedule?fixture=true"),
        ];
        let got: Vec<Result<Value, String>> = std::thread::scope(|s| {
            let hs: Vec<_> = urls
                .iter()
                .map(|u| s.spawn(move || self.get_cached(u, LONG)))
                .collect();
            hs.into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|_| Err("İstek yarıda kaldı".into()))
                })
                .collect()
        });
        if got.iter().all(|r| r.is_err()) {
            return Err(got
                .into_iter()
                .find_map(|r| r.err())
                .unwrap_or_else(|| "Takım bilgisi alınamadı".into()));
        }
        let empty = Value::Null;
        let val = |i: usize| got[i].as_ref().unwrap_or(&empty);
        let info = &val(0)["team"];
        let (mut results, mut fixtures) = split_schedule(league, val(2));
        let (r2, f2) = split_schedule(league, val(3));
        for m in r2 {
            if !results.iter().any(|x| x.id == m.id) {
                results.push(m);
            }
        }
        for m in f2 {
            if !fixtures.iter().any(|x| x.id == m.id) {
                fixtures.push(m);
            }
        }
        results.sort_by(|a, b| b.date.cmp(&a.date));
        fixtures.sort_by(|a, b| a.date.cmp(&b.date));
        results.truncate(10);
        fixtures.truncate(8);
        let mut t = team(info);
        if t.id.is_empty() {
            t.id = id.to_string();
        }
        Ok(TeamPage {
            team: t,
            league: league.into(),
            standing: text(&info["standingSummary"]),
            venue: text(&info["venue"]["fullName"]),
            players: parse_roster(val(1)),
            results,
            fixtures,
        })
    }

    /// Gol ve asist krallığı (ilk 10). Oyuncu ve takım bilgileri ayrı bağlantılardan aynı anda alınır.
    pub fn leaders(&self, league: &str) -> Result<Leaders, String> {
        if !known(league) {
            return Err("Bilinmeyen lig".into());
        }
        let url = format!("{CORE}/{league}/leaders");
        let v = self.get_cached(&url, LONG)?;
        let lists = [leader_refs(&v, "goal"), leader_refs(&v, "assist")];
        let refs: Vec<String> = lists
            .iter()
            .flatten()
            .flat_map(|(_, a, t)| [a.clone(), t.clone()])
            .filter_map(|r| safe_ref(&r))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let resolved: HashMap<String, Value> = std::thread::scope(|s| {
            let hs: Vec<_> = refs
                .iter()
                .map(|r| s.spawn(move || (r.clone(), self.get_cached(r, LONG * 6))))
                .collect();
            hs.into_iter()
                .filter_map(|h| h.join().ok())
                .filter_map(|(r, v)| v.ok().map(|v| (r, v)))
                .collect()
        });
        let build = |list: &[(u32, String, String)]| -> Vec<Leader> {
            list.iter()
                .filter_map(|(value, a, t)| {
                    let ath = resolved.get(&safe_ref(a)?)?;
                    let tm = safe_ref(t)
                        .and_then(|t| resolved.get(&t))
                        .map(team)
                        .unwrap_or_default();
                    Some(Leader {
                        name: text(&ath["displayName"]),
                        photo: text(&ath["headshot"]["href"]),
                        team: tm,
                        value: *value,
                    })
                })
                .filter(|l| !l.name.is_empty())
                .collect()
        };
        Ok(Leaders {
            goals: build(&lists[0]),
            assists: build(&lists[1]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({ "events": [
          { "id": "701", "date": "2026-10-18T17:00Z", "competitions": [{
            "status": { "displayClock": "67'", "type": { "name": "STATUS_SECOND_HALF", "state": "in" } },
            "competitors": [
              { "homeAway": "home", "score": "2", "team": { "id": "1", "displayName": "Galatasaray", "shortDisplayName": "Galatasaray", "logo": "https://x/gs.png" } },
              { "homeAway": "away", "score": "1", "team": { "id": "2", "displayName": "Fenerbahce", "shortDisplayName": "Fenerbahce" } }
            ],
            "details": [
              { "type": { "text": "Goal" }, "scoringPlay": true, "clock": { "displayClock": "12'" }, "team": { "id": "1" }, "athletesInvolved": [{ "displayName": "Icardi" }] },
              { "type": { "text": "Yellow Card" }, "yellowCard": true, "clock": { "displayClock": "30'" }, "team": { "id": "2" } },
              { "type": { "text": "Penalty - Scored" }, "scoringPlay": true, "penaltyKick": true, "clock": { "displayClock": "45'+2'" }, "team": { "id": "2" }, "athletesInvolved": [{ "displayName": "Dzeko" }] },
              { "type": { "text": "Own Goal" }, "scoringPlay": true, "ownGoal": true, "clock": { "displayClock": "60'" }, "team": { "id": "1" }, "athletesInvolved": [{ "displayName": "Kim" }] },
              { "type": { "text": "Red Card" }, "redCard": true, "clock": { "displayClock": "61'" }, "team": { "id": "2" }, "athletesInvolved": [{ "displayName": "Fred" }] }
            ] }] },
          { "id": "702", "date": "2026-10-19T16:00Z", "competitions": [{
            "status": { "displayClock": "0'", "type": { "name": "STATUS_SCHEDULED", "state": "pre" } },
            "competitors": [
              { "homeAway": "away", "score": "0", "team": { "id": "3", "displayName": "Besiktas" } },
              { "homeAway": "home", "score": "0", "team": { "id": "4", "displayName": "Trabzonspor" } }
            ] }] }
        ]})
    }

    #[test]
    fn scoreboard_is_read() {
        let m = parse_scoreboard("tur.1", &sample());
        assert_eq!(m.len(), 2);
        let a = &m[0];
        assert_eq!((a.state.as_str(), a.status.as_str()), ("in", "67'"));
        assert_eq!(a.home.team.name, "Galatasaray");
        assert_eq!(a.home.team.logo, "https://x/gs.png");
        assert_eq!((a.home.score, a.away.score), (Some(2), Some(1)));
        // Sarı kart listeye girmez; gol, penaltı, kendi kalesine, kırmızı girer.
        let kinds: Vec<_> = a
            .events
            .iter()
            .map(|e| (e.kind.as_str(), e.side.as_str()))
            .collect();
        assert_eq!(
            kinds,
            [
                ("goal", "home"),
                ("pen", "away"),
                ("own", "home"),
                ("red", "away")
            ]
        );
        assert_eq!(a.events[1].minute, "45'+2'");
        assert_eq!(a.events[1].player, "Dzeko");
        // Başlamamış maçta skor yok; ev sahibi sırası "homeAway"e göre.
        let b = &m[1];
        assert_eq!(b.home.team.name, "Trabzonspor");
        assert_eq!((b.home.score, b.status.as_str()), (None, ""));
    }

    #[test]
    fn statuses() {
        assert_eq!(status_text("STATUS_HALFTIME", "in", "45'"), "İY");
        assert_eq!(status_text("STATUS_FULL_TIME", "post", "90'"), "MS");
        assert_eq!(status_text("STATUS_POSTPONED", "post", ""), "Ertelendi");
        assert_eq!(status_text("STATUS_FIRST_HALF", "in", "23'"), "23'");
        assert_eq!(status_text("STATUS_SCHEDULED", "pre", "0'"), "");
        assert_eq!(status_text("BILINMEYEN", "post", ""), "MS");
    }

    #[test]
    fn standings_are_read() {
        let entry = |id: &str, name: &str, rank: f64, pts: f64, note: Value| {
            json!({ "team": { "id": id, "displayName": name, "logos": [{ "href": format!("https://x/{id}.png") }] },
                    "note": note,
                    "stats": [
                      { "name": "rank", "value": rank }, { "name": "gamesPlayed", "value": 8.0 },
                      { "name": "wins", "value": 6.0 }, { "name": "ties", "value": 1.0 }, { "name": "losses", "value": 1.0 },
                      { "name": "pointsFor", "value": 18.0 }, { "name": "pointsAgainst", "value": 6.0 },
                      { "name": "points", "value": pts } ] })
        };
        let v = json!({ "children": [{ "name": "Super Lig", "standings": { "entries": [
            entry("2", "Fenerbahce", 2.0, 19.0, json!({})),
            entry("1", "Galatasaray", 1.0, 22.0, json!({ "description": "Champions League", "color": "#81D6AC" }))
        ] } }] });
        let g = parse_standings(&v);
        assert_eq!(g.len(), 1);
        let r = &g[0].rows;
        assert_eq!(r[0].team.name, "Galatasaray");
        assert_eq!(
            (r[0].rank, r[0].points, r[0].played, r[0].gf, r[0].ga),
            (1, 22, 8, 18, 6)
        );
        assert_eq!(r[0].team.logo, "https://x/1.png");
        assert_eq!(
            (r[0].note.as_str(), r[0].color.as_str()),
            ("Champions League", "#81D6AC")
        );
        assert_eq!(r[1].note, "");
        assert!(parse_standings(&json!({})).is_empty());
    }

    #[test]
    fn only_known_leagues_and_days() {
        let f = Football::default();
        // Bilinmeyen lig ve bozuk gün ağa hiç gitmez.
        assert!(f
            .matches(&["../evil".into()], &["20261018".into()])
            .unwrap()
            .is_empty());
        assert!(f
            .matches(&["tur.1".into()], &["2026-10".into()])
            .unwrap()
            .is_empty());
        assert!(f.standings("x").is_err());
    }

    #[test]
    fn minutes_and_object_scores() {
        assert_eq!(minute_of(&json!({ "value": 1380.0 })), "23'");
        assert_eq!(
            minute_of(&json!({ "value": 1380.0, "displayClock": "23'" })),
            "23'"
        );
        assert_eq!(minute_of(&json!({})), "");
        assert_eq!(
            score_of(&json!({ "value": 2.0, "displayValue": "2" })),
            Some(2)
        );
        assert_eq!(score_of(&json!({ "value": 3.0 })), Some(3));
        assert_eq!(score_of(&json!("1")), Some(1));
        assert_eq!(score_of(&json!(null)), None);
    }

    #[test]
    fn roster_flat_and_grouped() {
        let flat = json!({ "athletes": [
            { "id": "9", "displayName": "M. Icardi", "jersey": "9", "age": 32, "citizenship": "Argentina",
              "position": { "name": "Forward", "abbreviation": "F" }, "headshot": { "href": "https://x/9.png" } },
            { "id": "1", "displayName": "F. Muslera", "jersey": "1", "position": { "name": "Goalkeeper", "abbreviation": "G" } },
            { "id": "4", "displayName": "D. Sanchez", "jersey": "6", "position": { "name": "Center Back" } }
        ]});
        let r = parse_roster(&flat);
        let names: Vec<_> = r
            .iter()
            .map(|p| (p.pos.as_str(), p.name.as_str()))
            .collect();
        assert_eq!(
            names,
            [("G", "F. Muslera"), ("D", "D. Sanchez"), ("F", "M. Icardi")]
        );
        assert_eq!(r[2].age, Some(32));
        assert_eq!(r[2].nation, "Argentina");
        assert_eq!(r[2].photo, "https://x/9.png");
        let grouped = json!({ "athletes": [
            { "position": "Midfielders", "items": [ { "id": "8", "displayName": "L. Torreira", "jersey": "34", "position": { "abbreviation": "M" } } ] }
        ]});
        assert_eq!(parse_roster(&grouped)[0].pos, "M");
        assert!(parse_roster(&json!({})).is_empty());
    }

    #[test]
    fn schedule_is_split() {
        let ev = |id: &str, date: &str, state: &str| {
            json!({ "id": id, "date": date, "competitions": [{
            "status": { "type": { "name": "", "state": state } },
            "competitors": [
              { "homeAway": "home", "score": { "value": 1.0, "displayValue": "1" }, "team": { "id": "1", "displayName": "A" } },
              { "homeAway": "away", "score": { "value": 0.0, "displayValue": "0" }, "team": { "id": "2", "displayName": "B" } } ] }] })
        };
        let v = json!({ "events": [ev("a", "2026-09-01T18:00Z", "post"), ev("b", "2026-10-01T18:00Z", "post"), ev("c", "2026-11-01T18:00Z", "pre")] });
        let (done, next) = split_schedule("tur.1", &v);
        assert_eq!(
            done.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            ["b", "a"]
        );
        assert_eq!(done[0].home.score, Some(1));
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].home.score, None);
    }

    #[test]
    fn leaders_and_refs() {
        let v = json!({ "categories": [
            { "name": "goalsLeaders", "displayName": "Goals", "leaders": [
                { "value": 9.0, "athlete": { "$ref": "http://sports.core.api.espn.com/v2/sports/soccer/leagues/tur.1/seasons/2025/athletes/1" },
                  "team": { "$ref": "http://sports.core.api.espn.com/v2/sports/soccer/leagues/tur.1/seasons/2025/teams/2" } } ] },
            { "name": "assistsLeaders", "displayName": "Assists", "leaders": [] }
        ]});
        let g = leader_refs(&v, "goal");
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].0, 9);
        assert!(leader_refs(&v, "assist").is_empty());
        assert_eq!(
            safe_ref(&g[0].1).as_deref(),
            Some("https://sports.core.api.espn.com/v2/sports/soccer/leagues/tur.1/seasons/2025/athletes/1")
        );
        // Başka bir adrese asla gidilmez.
        assert_eq!(safe_ref("https://evil.example/x"), None);
        assert_eq!(safe_ref("file:///C:/x"), None);
        let f = Football::default();
        assert!(f.team("tur.1", "../1").is_err());
        assert!(f.leaders("x").is_err());
    }
}
