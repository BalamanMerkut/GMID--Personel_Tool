//! Film & Dizi Arşivi'nin film/dizi bilgileri. Anahtar gerektirmeyen kaynaklar:
//! - **Arama:** İngilizce Wikipedia (kısa açıklaması "film" / "series" olan sayfalar, afiş önizlemesiyle).
//! - **Bilgiler:** Wikidata — Türkçe ad, yıl, süre, sezon/bölüm sayısı, yönetmen, oyuncular, türler.
//! - **Özet:** Türkçe Vikipedi (yoksa İngilizce).
//!
//! Bilgiler bir kez çekilip yerelde saklanır; afiş `Kapaklar\Filmler` içine indirilir.

use crate::db::Db;
use crate::paths::DataDir;
use crate::util::{new_id, now_ms};
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const UA: &str = "GMIDTool/0.1 (https://github.com/BalamanMerkut/GMID--Personel_Tool)";

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Title {
    pub id: String,
    /// movie | series
    pub kind: String,
    pub name: String,
    pub original: Option<String>,
    pub year: Option<i64>,
    pub poster: Option<String>,
    pub summary: Option<String>,
    pub directors: Vec<String>,
    pub cast: Vec<String>,
    pub genres: Vec<String>,
    pub runtime: Option<i64>,
    pub seasons: Option<i64>,
    pub episodes: Option<i64>,
    pub wiki_title: Option<String>,
    pub added_at: i64,
}

/// Arama sonucu.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub wiki_title: String,
    /// Sayfa adından ayıklanmış görünen ad ("Inception (film)" → "Inception").
    pub name: String,
    pub description: String,
    pub image: Option<String>,
    pub wikidata: Option<String>,
    pub kind: String,
    pub year: Option<i64>,
}

fn agent() -> ureq::Agent {
    crate::net::agent(Duration::from_secs(10), Duration::from_secs(30))
}

fn get_json(url: &str) -> Result<Value, String> {
    let mut r = agent()
        .get(url)
        .header("User-Agent", UA)
        .call()
        .map_err(|e| e.to_string())?;
    let body = r
        .body_mut()
        .with_config()
        .limit(8 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn year_in(s: &str) -> Option<i64> {
    s.split(|c: char| !c.is_ascii_digit())
        .filter(|p| p.len() == 4)
        .filter_map(|p| p.parse().ok())
        .find(|y| (1880..=2100).contains(y))
}

/// Kısa açıklamadan tür: dizi mi film mi; ikisi de değilse (kişi, film serisi, stüdyo) `None`.
pub fn kind_of(desc: &str) -> Option<&'static str> {
    let d = desc.to_lowercase();
    // Sözcük olarak: "filmmaker" film değildir.
    let words: Vec<&str> = d
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let has = |w: &str| words.contains(&w);
    if d.contains("film series") || d.contains("franchise") || d.contains("character") {
        None
    } else if has("series") || has("miniseries") || has("sitcom") || has("telenovela") {
        Some("series")
    } else if has("film") || has("movie") {
        Some("movie")
    } else {
        None
    }
}

/// "Inception (film)", "Dark (TV series)" → "Inception", "Dark".
fn display_name(title: &str) -> String {
    match title.rfind(" (") {
        Some(i) if title.ends_with(')') => title[..i].to_string(),
        _ => title.to_string(),
    }
}

pub fn parse_search(json: &Value) -> Vec<Hit> {
    let mut pages: Vec<&Value> = json["query"]["pages"]
        .as_array()
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    pages.sort_by_key(|p| p["index"].as_i64().unwrap_or(99));
    pages
        .into_iter()
        .filter_map(|p| {
            let title = p["title"].as_str()?;
            let desc = p["pageprops"]["wikibase-shortdesc"]
                .as_str()
                .unwrap_or_default();
            let kind = kind_of(desc)?;
            Some(Hit {
                wiki_title: title.to_string(),
                name: display_name(title),
                description: desc.to_string(),
                image: p["thumbnail"]["source"].as_str().map(str::to_string),
                wikidata: p["pageprops"]["wikibase_item"].as_str().map(str::to_string),
                kind: kind.to_string(),
                year: year_in(desc),
            })
        })
        .collect()
}

pub fn search(term: &str) -> Result<Vec<Hit>, String> {
    let url = format!(
        "https://en.wikipedia.org/w/api.php?action=query&format=json&formatversion=2&generator=search\
         &gsrlimit=12&prop=pageimages%7Cpageprops&piprop=thumbnail&pithumbsize=300&pilicense=any\
         &ppprop=wikibase-shortdesc%7Cwikibase_item&gsrsearch={}",
        enc(term.trim())
    );
    Ok(parse_search(&get_json(&url)?))
}

// ---- Wikidata ----

/// Wikidata'dan çıkan bilgiler.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Facts {
    pub label_tr: Option<String>,
    pub label_en: Option<String>,
    pub year: Option<i64>,
    pub runtime: Option<i64>,
    pub seasons: Option<i64>,
    pub episodes: Option<i64>,
    pub directors: Vec<String>,
    pub cast: Vec<String>,
    pub genres: Vec<String>,
    pub tr_wiki: Option<String>,
}

fn claim_ids(entity: &Value, prop: &str, max: usize) -> Vec<String> {
    entity["claims"][prop]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|c| c["mainsnak"]["datavalue"]["value"]["id"].as_str())
                .map(str::to_string)
                .take(max)
                .collect()
        })
        .unwrap_or_default()
}

fn claim_amount(entity: &Value, prop: &str) -> Option<i64> {
    let v = &entity["claims"][prop].as_array()?.first()?["mainsnak"]["datavalue"]["value"];
    v["amount"]
        .as_str()?
        .trim_start_matches('+')
        .parse::<f64>()
        .ok()
        .map(|f| f.round() as i64)
}

fn claim_year(entity: &Value, prop: &str) -> Option<i64> {
    entity["claims"][prop]
        .as_array()?
        .iter()
        .filter_map(|c| c["mainsnak"]["datavalue"]["value"]["time"].as_str())
        .filter_map(year_in)
        .min()
}

fn label(entity: &Value, lang: &str) -> Option<String> {
    entity["labels"][lang]["value"].as_str().map(str::to_string)
}

/// Ana varlığın iddiaları; adlar sonra ayrı bir istekle çözülür (kimlik listeleri döner).
pub fn parse_entity(json: &Value, id: &str) -> (Facts, Vec<String>, Vec<String>, Vec<String>) {
    let e = &json["entities"][id];
    let mut f = Facts {
        label_tr: label(e, "tr"),
        label_en: label(e, "en"),
        year: claim_year(e, "P577").or_else(|| claim_year(e, "P580")),
        runtime: claim_amount(e, "P2047"),
        seasons: claim_amount(e, "P2437"),
        episodes: claim_amount(e, "P1113"),
        tr_wiki: e["sitelinks"]["trwiki"]["title"]
            .as_str()
            .map(str::to_string),
        ..Default::default()
    };
    let mut directors = claim_ids(e, "P57", 3);
    if directors.is_empty() {
        // Dizilerde yönetmen yerine yaratıcı.
        directors = claim_ids(e, "P170", 3);
    }
    let cast = claim_ids(e, "P161", 8);
    let genres = claim_ids(e, "P136", 5);
    f.directors.clear();
    (f, directors, cast, genres)
}

/// Kimlik → ad (Türkçesi yoksa İngilizcesi).
pub fn parse_labels(json: &Value) -> std::collections::HashMap<String, String> {
    json["entities"]
        .as_object()
        .map(|m| {
            m.iter()
                .filter_map(|(id, e)| {
                    Some((id.clone(), label(e, "tr").or_else(|| label(e, "en"))?))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// "bilimkurgu filmi" → "Bilimkurgu", "drama television series" → "Drama".
pub fn clean_genre(g: &str) -> String {
    let mut s = g.trim().to_string();
    for suffix in [
        " filmi",
        " film",
        " dizisi",
        " television series",
        " television program",
        " film genre",
        " fiction",
    ] {
        if s.to_lowercase().ends_with(suffix) && s.len() > suffix.len() {
            s.truncate(s.len() - suffix.len());
        }
    }
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => s,
    }
}

fn facts(qid: &str) -> Result<Facts, String> {
    let json = get_json(&format!(
        "https://www.wikidata.org/w/api.php?action=wbgetentities&format=json&ids={qid}\
         &props=labels%7Cclaims%7Csitelinks&languages=tr%7Cen&sitefilter=trwiki%7Cenwiki"
    ))?;
    let (mut f, directors, cast, genres) = parse_entity(&json, qid);
    let all: Vec<&String> = directors.iter().chain(&cast).chain(&genres).collect();
    if !all.is_empty() {
        let ids: Vec<&str> = all.iter().map(|s| s.as_str()).take(50).collect();
        let names = get_json(&format!(
            "https://www.wikidata.org/w/api.php?action=wbgetentities&format=json&ids={}\
             &props=labels&languages=tr%7Cen",
            ids.join("%7C")
        ))
        .map(|j| parse_labels(&j))
        .unwrap_or_default();
        let pick = |list: &[String]| -> Vec<String> {
            list.iter()
                .filter_map(|id| names.get(id).cloned())
                .collect()
        };
        f.directors = pick(&directors);
        f.cast = pick(&cast);
        let mut g: Vec<String> = Vec::new();
        for x in pick(&genres).iter().map(|x| clean_genre(x)) {
            if !g.contains(&x) {
                g.push(x);
            }
        }
        f.genres = g;
    }
    Ok(f)
}

fn summary(lang: &str, title: &str) -> Option<String> {
    let json = get_json(&format!(
        "https://{lang}.wikipedia.org/api/rest_v1/page/summary/{}",
        enc(&title.replace(' ', "_"))
    ))
    .ok()?;
    json["extract"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn poster_url(wiki_title: &str) -> Option<String> {
    let json = get_json(&format!(
        "https://en.wikipedia.org/w/api.php?action=query&format=json&formatversion=2&prop=pageimages\
         &piprop=thumbnail&pithumbsize=600&pilicense=any&titles={}",
        enc(wiki_title)
    ))
    .ok()?;
    json["query"]["pages"][0]["thumbnail"]["source"]
        .as_str()
        .map(str::to_string)
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    let mut r = agent()
        .get(url)
        .header("User-Agent", UA)
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

// ---- veritabanı ----

const COLUMNS: &str =
    "id, kind, name, original, year, poster, summary, directors, cast_list, genres,
    runtime, seasons, episodes, wiki_title, added_at";

fn json_list(s: Option<String>) -> Vec<String> {
    s.and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn from_row(r: &Row) -> rusqlite::Result<Title> {
    Ok(Title {
        id: r.get(0)?,
        kind: r.get(1)?,
        name: r.get(2)?,
        original: r.get(3)?,
        year: r.get(4)?,
        poster: r.get(5)?,
        summary: r.get(6)?,
        directors: json_list(r.get(7)?),
        cast: json_list(r.get(8)?),
        genres: json_list(r.get(9)?),
        runtime: r.get(10)?,
        seasons: r.get(11)?,
        episodes: r.get(12)?,
        wiki_title: r.get(13)?,
        added_at: r.get(14)?,
    })
}

pub fn list(db: &Db) -> Result<Vec<Title>, String> {
    let conn = db.conn();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM titles ORDER BY added_at DESC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], from_row)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

pub fn get(db: &Db, id: &str) -> Result<Option<Title>, String> {
    db.conn()
        .query_row(
            &format!("SELECT {COLUMNS} FROM titles WHERE id = ?1"),
            [id],
            from_row,
        )
        .optional()
        .map_err(|e| e.to_string())
}

/// Arama sonucundan ekler (aynı Wikipedia sayfası zaten varsa onu döndürür); bilgiler arka planda gelir.
/// `hit` yoksa yalnızca adıyla (elle) eklenir.
pub fn add(app: &AppHandle, hit: Option<Hit>, name: &str, kind: &str) -> Result<Title, String> {
    let db = app.state::<Db>();
    if let Some(h) = &hit {
        let existing: Option<String> = db
            .conn()
            .query_row(
                "SELECT id FROM titles WHERE wiki_title = ?1",
                [&h.wiki_title],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(id) = existing {
            return get(&db, &id)?.ok_or_else(|| "Bulunamadı".into());
        }
    }
    let name = hit
        .as_ref()
        .map(|h| h.name.clone())
        .unwrap_or_else(|| name.trim().to_string());
    if name.is_empty() {
        return Err("Adını yaz".into());
    }
    let kind = hit.as_ref().map(|h| h.kind.as_str()).unwrap_or(kind);
    let kind = if kind == "series" { "series" } else { "movie" };
    let id = new_id();
    db.conn()
        .execute(
            "INSERT INTO titles (id, kind, name, year, wikidata, wiki_title, meta_at, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                kind,
                name,
                hit.as_ref().and_then(|h| h.year),
                hit.as_ref().and_then(|h| h.wikidata.clone()),
                hit.as_ref().map(|h| h.wiki_title.clone()),
                // Elle eklenenin aranacak bilgisi yok.
                hit.is_none().then(now_ms),
                now_ms()
            ],
        )
        .map_err(|e| e.to_string())?;
    let title = get(&db, &id)?.ok_or("Eklenemedi")?;
    fetch_meta(app);
    Ok(title)
}

/// Puanlanmadan vazgeçilen film/dizi (puanı yoksa) silinir.
pub fn remove_if_unrated(db: &Db, id: &str) -> Result<(), String> {
    db.conn()
        .execute(
            "DELETE FROM titles WHERE id = ?1 AND NOT EXISTS
               (SELECT 1 FROM reviews WHERE subject_kind = 'title' AND subject_id = ?1)",
            [id],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn refresh(app: &AppHandle, id: &str) -> Result<(), String> {
    app.state::<Db>()
        .conn()
        .execute(
            "UPDATE titles SET meta_at = NULL WHERE id = ?1 AND wiki_title IS NOT NULL",
            [id],
        )
        .map_err(|e| e.to_string())?;
    fetch_meta(app);
    Ok(())
}

static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Bilgisi alınmamış film/dizileri arka planda doldurur.
pub fn fetch_meta(app: &AppHandle) {
    use std::sync::atomic::Ordering;
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let db = app.state::<Db>();
        let posters = app
            .state::<DataDir>()
            .folder(Some("Kapaklar"))
            .join("Filmler");
        loop {
            let next: Option<(String, Option<String>, Option<String>)> = db
                .conn()
                .query_row(
                    "SELECT id, wikidata, wiki_title FROM titles WHERE meta_at IS NULL LIMIT 1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
                .ok()
                .flatten();
            let Some((id, qid, wiki)) = next else { break };
            if let Err(e) = fill(&db, &posters, &id, qid.as_deref(), wiki.as_deref()) {
                crate::log!("Film bilgisi alınamadı: {e}");
                break;
            }
            let _ = app.emit("titles-changed", ());
            std::thread::sleep(Duration::from_millis(500));
        }
        BUSY.store(false, Ordering::SeqCst);
    });
}

fn fill(
    db: &Db,
    posters: &Path,
    id: &str,
    qid: Option<&str>,
    wiki: Option<&str>,
) -> Result<(), String> {
    let f = match qid {
        Some(q) => facts(q)?,
        None => Facts::default(),
    };
    let summary = f
        .tr_wiki
        .as_deref()
        .and_then(|t| summary("tr", t))
        .or_else(|| wiki.and_then(|t| summary("en", t)));
    let poster = wiki.and_then(poster_url).and_then(|url| {
        let dest = posters.join(format!("{id}.jpg"));
        download(&url, &dest)
            .map(|_| dest.to_string_lossy().into_owned())
            .map_err(|e| crate::log!("Afiş indirilemedi: {e}"))
            .ok()
    });
    let name = f.label_tr.clone().or(f.label_en.clone());
    let original = f.label_en.clone().filter(|en| Some(en) != name.as_ref());
    let j = |v: &Vec<String>| serde_json::to_string(v).unwrap_or_else(|_| "[]".into());
    db.conn()
        .execute(
            "UPDATE titles SET name = COALESCE(?2, name), original = ?3, year = COALESCE(?4, year),
                poster = COALESCE(?5, poster), summary = COALESCE(?6, summary), directors = ?7,
                cast_list = ?8, genres = ?9, runtime = ?10, seasons = ?11, episodes = ?12, meta_at = ?13
             WHERE id = ?1",
            params![
                id,
                name,
                original,
                f.year,
                poster,
                summary,
                j(&f.directors),
                j(&f.cast),
                j(&f.genres),
                f.runtime,
                f.seasons,
                f.episodes,
                now_ms()
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_results_are_films_and_series_only() {
        let json: Value = serde_json::from_str(
            r#"{"query":{"pages":[
              {"title":"Inception","index":1,"pageprops":{"wikibase-shortdesc":"2010 film by Christopher Nolan","wikibase_item":"Q25188"},"thumbnail":{"source":"https://upload.wikimedia.org/inception.jpg"}},
              {"title":"Christopher Nolan","index":2,"pageprops":{"wikibase-shortdesc":"British-American filmmaker (born 1970)","wikibase_item":"Q25191"}},
              {"title":"Dark (TV series)","index":3,"pageprops":{"wikibase-shortdesc":"German science fiction television series","wikibase_item":"Q27877"}},
              {"title":"The Dark Knight Trilogy","index":4,"pageprops":{"wikibase-shortdesc":"Film series"}}
            ]}}"#,
        )
        .unwrap();
        let hits = parse_search(&json);
        let names: Vec<_> = hits
            .iter()
            .map(|h| (h.name.as_str(), h.kind.as_str(), h.year))
            .collect();
        // Kişi sayfası ("filmmaker") ve film serisi elenir.
        assert_eq!(names.len(), 2, "{names:?}");
        assert_eq!(names[0], ("Inception", "movie", Some(2010)));
        assert!(names.iter().any(|n| n.0 == "Dark" && n.1 == "series"));
        assert_eq!(hits[0].wikidata.as_deref(), Some("Q25188"));
    }

    #[test]
    fn wikidata_facts_are_parsed() {
        let json: Value = serde_json::from_str(
            r#"{"entities":{"Q25188":{
              "labels":{"tr":{"value":"Başlangıç"},"en":{"value":"Inception"}},
              "sitelinks":{"trwiki":{"title":"Başlangıç (film, 2010)"}},
              "claims":{
                "P577":[{"mainsnak":{"datavalue":{"value":{"time":"+2010-07-16T00:00:00Z"}}}},{"mainsnak":{"datavalue":{"value":{"time":"+2010-07-08T00:00:00Z"}}}}],
                "P2047":[{"mainsnak":{"datavalue":{"value":{"amount":"+148","unit":"http://www.wikidata.org/entity/Q7727"}}}}],
                "P57":[{"mainsnak":{"datavalue":{"value":{"id":"Q25191"}}}}],
                "P161":[{"mainsnak":{"datavalue":{"value":{"id":"Q38111"}}}}],
                "P136":[{"mainsnak":{"datavalue":{"value":{"id":"Q471839"}}}}]
              }}}}"#,
        )
        .unwrap();
        let (f, d, c, g) = parse_entity(&json, "Q25188");
        assert_eq!(f.label_tr.as_deref(), Some("Başlangıç"));
        assert_eq!((f.year, f.runtime), (Some(2010), Some(148)));
        assert_eq!(f.tr_wiki.as_deref(), Some("Başlangıç (film, 2010)"));
        assert_eq!(
            (d, c, g),
            (
                vec!["Q25191".to_string()],
                vec!["Q38111".to_string()],
                vec!["Q471839".to_string()]
            )
        );
        let labels: Value = serde_json::from_str(
            r#"{"entities":{"Q25191":{"labels":{"en":{"value":"Christopher Nolan"}}},"Q471839":{"labels":{"tr":{"value":"bilimkurgu filmi"},"en":{"value":"science fiction film"}}}}}"#,
        )
        .unwrap();
        let m = parse_labels(&labels);
        assert_eq!(m["Q25191"], "Christopher Nolan");
        assert_eq!(clean_genre(&m["Q471839"]), "Bilimkurgu");
        assert_eq!(clean_genre("drama television series"), "Drama");
    }

    #[test]
    fn kinds_and_names() {
        assert_eq!(
            kind_of("2008 American crime drama television series"),
            Some("series")
        );
        assert_eq!(
            kind_of("1994 film directed by Frank Darabont"),
            Some("movie")
        );
        assert_eq!(kind_of("Turkish actor"), None);
        assert_eq!(display_name("Dark (TV series)"), "Dark");
        assert_eq!(display_name("Up"), "Up");
    }
}
