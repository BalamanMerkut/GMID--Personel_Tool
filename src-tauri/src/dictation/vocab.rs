//! Öğrenen sözlük: Whisper'ın sık yanlış yazdığı sözcükler ("GMID" → "GMD", "VDS" → "videoyu") için
//! "şunu duyunca bunu yaz" kuralları.
//!
//! - Kurallar her diktede uygulanır (büyük/küçük harf fark etmez, sözcük sınırında; "GMD'ye" gibi eklerde ek
//!   korunur) ve doğru sözcükler Whisper'a ipucu olarak da verilir.
//! - Öğrenme: dikteden sonra yazıyı düzeltirsen (yapıştırılan yerde ya da Kontrol Odası'ndaki "Son dikteler"de)
//!   eski ve yeni yazı karşılaştırılır. Değişen kısa parça yanlış duyulmuşa benziyorsa (yazılışı yakın, ya da
//!   doğrusu kısaltma/marka/sayı) kendiliğinden kural olur ve birkaç saniye "Geri al" gösterilir; cümleyi
//!   başka türlü söylemek (sözcüğü bambaşka bir sözcükle değiştirmek) ve silmek öğrenilmez.

use crate::db::Db;
use serde::{Deserialize, Serialize};

pub const RULES_KEY: &str = "dictation.rules";
/// "Hayır" denen öneriler bir daha sorulmaz.
const REJECTED_KEY: &str = "dictation.rejected";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub from: String,
    pub to: String,
    /// Kaç kez düzeltti.
    #[serde(default)]
    pub hits: u32,
}

pub fn rules(db: &Db) -> Vec<Rule> {
    db.setting_json(RULES_KEY).unwrap_or_default()
}

fn save(db: &Db, rules: &[Rule]) -> Result<(), String> {
    let j = serde_json::to_string(rules).map_err(|e| e.to_string())?;
    db.set_setting(RULES_KEY, &j).map_err(|e| e.to_string())
}

pub fn add(db: &Db, from: &str, to: &str) -> Result<Vec<Rule>, String> {
    let (from, to) = (from.trim(), to.trim());
    if from.is_empty() || to.is_empty() || fold(from) == fold(to) && from == to {
        return Err("Boş ya da aynı".into());
    }
    if from.chars().count() > 60 || to.chars().count() > 60 {
        return Err("Çok uzun".into());
    }
    let mut v = rules(db);
    v.retain(|r| fold(&r.from) != fold(from));
    v.push(Rule {
        from: from.into(),
        to: to.into(),
        hits: 0,
    });
    save(db, &v)?;
    Ok(v)
}

pub fn remove(db: &Db, from: &str) -> Result<Vec<Rule>, String> {
    let mut v = rules(db);
    v.retain(|r| fold(&r.from) != fold(from));
    save(db, &v)?;
    Ok(v)
}

pub fn reject(db: &Db, from: &str, to: &str) {
    let mut v: Vec<(String, String)> = db.setting_json(REJECTED_KEY).unwrap_or_default();
    v.push((fold(from), to.to_string()));
    if v.len() > 300 {
        v.drain(..v.len() - 300);
    }
    if let Ok(j) = serde_json::to_string(&v) {
        let _ = db.set_setting(REJECTED_KEY, &j);
    }
}

/// Daha önce sorulup reddedilmiş ya da zaten kural olan öneriler çıkarılır.
pub fn fresh(db: &Db, pairs: Vec<(String, String)>) -> Vec<(String, String)> {
    let rejected: Vec<(String, String)> = db.setting_json(REJECTED_KEY).unwrap_or_default();
    let known = rules(db);
    pairs
        .into_iter()
        .filter(|(f, t)| {
            !rejected.iter().any(|(rf, rt)| *rf == fold(f) && rt == t)
                && !known.iter().any(|r| fold(&r.from) == fold(f) && r.to == *t)
        })
        .collect()
}

/// Uygulanan kuralların sayaçlarını artırır.
pub fn count_hits(db: &Db, used: &[usize]) {
    if used.is_empty() {
        return;
    }
    let mut v = rules(db);
    for &i in used {
        if let Some(r) = v.get_mut(i) {
            r.hits = r.hits.saturating_add(1);
        }
    }
    let _ = save(db, &v);
}

/// Whisper ipucu için doğru yazılışlar (en çok kullanılanlar önce).
pub fn hint_words(rules: &[Rule]) -> Vec<String> {
    let mut v: Vec<&Rule> = rules.iter().collect();
    v.sort_by_key(|r| std::cmp::Reverse(r.hits));
    let mut out: Vec<String> = Vec::new();
    for r in v {
        if !out.iter().any(|w| fold(w) == fold(&r.to)) {
            out.push(r.to.clone());
        }
        if out.len() >= 30 {
            break;
        }
    }
    out
}

/// Karşılaştırma için küçük harf. Noktalı/noktasız i ayrılmaz: Whisper kısaltmaları ("GMI") büyük harfle,
/// kullanıcı kuralı küçük harfle ("gmi") yazabilir.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'I' | 'İ' | 'ı' => vec!['i'],
            c => c.to_lowercase().collect(),
        })
        .collect()
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric()
}

/// Kuralları uygular; yeni yazı ve kullanılan kuralların sırası.
pub fn apply(text: &str, rules: &[Rule]) -> (String, Vec<usize>) {
    let mut out = text.to_string();
    let mut used = Vec::new();
    // Uzun kurallar önce: "gmd tool" "gmd"den önce.
    let mut order: Vec<usize> = (0..rules.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(rules[i].from.chars().count()));
    for i in order {
        let r = &rules[i];
        let (next, n) = replace_word(&out, &r.from, &r.to);
        if n > 0 {
            out = next;
            used.push(i);
        }
    }
    (out, used)
}

/// `from`u sözcük sınırlarında (büyük/küçük harf fark etmeden) `to` ile değiştirir. Sonrasında ek gelebilir
/// ("GMD'ye" → "GMID'ye", "GMD'nin"); sözcüğün içinde geçiyorsa ("videoyu" içindeki "video") dokunmaz.
fn replace_word(text: &str, from: &str, to: &str) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let low: Vec<String> = chars.iter().map(|c| fold(&c.to_string())).collect();
    let pat: Vec<String> = from.chars().map(|c| fold(&c.to_string())).collect();
    if pat.is_empty() {
        return (text.to_string(), 0);
    }
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let mut n = 0;
    while i < chars.len() {
        let fits = i + pat.len() <= chars.len() && low[i..i + pat.len()] == pat[..];
        let before_ok = i == 0 || !is_word(chars[i - 1]);
        let after = chars.get(i + pat.len()).copied();
        let after_ok = boundary_after(after);
        if fits && before_ok && after_ok {
            out.push_str(to);
            i += pat.len();
            n += 1;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    (out, n)
}

/// Eşleşmeden sonra sözcük bitmeli; kesme işaretli ek gelebilir.
fn boundary_after(c: Option<char>) -> bool {
    c.map_or(true, |c| !is_word(c) || c == '\'' || c == '’')
}

/// Noktalamadan arınmış sözcük (karşılaştırmada kullanılır) ve asıl hali.
fn tokens(s: &str) -> Vec<(String, String)> {
    s.split_whitespace()
        .map(|w| {
            let core = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '’');
            let core = core.trim_end_matches(['\'', '’']);
            (fold(core), core.to_string())
        })
        .filter(|(f, _)| !f.is_empty())
        .collect()
}

/// Yapıştırılan yazının sözcüklerinden kaçta kaçı `text` içinde duruyor (0..1). Yazı alanı boşaldıysa ya da
/// mesaj gönderildiyse düşer.
pub fn overlap(pasted: &str, text: &str) -> f32 {
    let a = tokens(pasted);
    if a.is_empty() {
        return 0.0;
    }
    let b: std::collections::HashSet<String> = tokens(text).into_iter().map(|t| t.0).collect();
    a.iter().filter(|t| b.contains(&t.0)).count() as f32 / a.len() as f32
}

/// Düzeltme yanlış duyulmuş bir sözcüğe benziyor mu? Yalnız harf düzeltmesi, kısaltma/marka ("videoyu" → "VDS",
/// "gmd" → "GMID"), sayı ya da yazılışı yakın bir sözcük ("Esad" → "Esat"). "nasılsın" → "naber" gibi cümleyi
/// başka türlü söylemek düzeltme sayılmaz.
pub fn plausible(from: &str, to: &str) -> bool {
    let (f, t) = (fold(from), fold(to));
    if f == t {
        return true;
    }
    if to.chars().filter(|c| c.is_uppercase()).count() >= 2
        || to.chars().any(|c| c.is_ascii_digit())
    {
        return true;
    }
    similarity(&f, &t) >= 0.5
}

/// 1 − (düzenleme uzaklığı / uzun olanın harf sayısı).
fn similarity(a: &str, b: &str) -> f32 {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let n = a.len().max(b.len());
    if n == 0 {
        return 1.0;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    1.0 - prev[b.len()] as f32 / n as f32
}

/// "GMD'ye" / "GMID'ye" → "GMD" / "GMID": aynı ek iki tarafta da varsa atılır.
fn strip_common_suffix(a: &str, b: &str) -> (String, String) {
    for sep in ['\'', '’'] {
        if let (Some((a0, a1)), Some((b0, b1))) = (a.split_once(sep), b.split_once(sep)) {
            if fold(a1) == fold(b1) && !a0.is_empty() && !b0.is_empty() {
                return (a0.to_string(), b0.to_string());
            }
        }
    }
    (a.to_string(), b.to_string())
}

/// Yapıştırılan yazı ile kullanıcının düzelttiği hali: değişen kısa parçalar (en çok 3'er sözcük) "şu → bu"
/// önerisi olur. `edited` başka yazı da içerebilir (yazı alanının tamamı): yalnız dikte edilen kısmın içindeki
/// değişikliklere bakılır. Yazının çoğu değiştiyse (baştan yazılmış) öneri yok.
pub fn suggest(pasted: &str, edited: &str) -> Vec<(String, String)> {
    let a = tokens(pasted);
    let b = tokens(edited);
    if a.is_empty() || b.is_empty() || a.len() > 400 || b.len() > 4000 {
        return Vec::new();
    }
    // Sözcük düzeyinde en uzun ortak alt dizi.
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0u16; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i].0 == b[j].0 {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let common = dp[0][0] as usize;
    // Dikte edilen yazının en az yarısı yerinde durmalı (yoksa bu alan başka bir yazı).
    if common * 2 < n || common == 0 {
        return Vec::new();
    }
    // Eşleşen çiftler.
    let mut pairs = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i].0 == b[j].0 {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    let mut out: Vec<(String, String)> = Vec::new();
    // Eşleşmeler arasındaki boşluklar = değişen parçalar (ilk eşleşmeden önce ve sondakinden sonrası yazı
    // alanındaki başka yazıdır, ama dikte yazısının kendi başı/sonu da değişmiş olabilir).
    let mut prev: Option<(usize, usize)> = None;
    let mut gaps: Vec<(std::ops::Range<usize>, std::ops::Range<usize>)> = Vec::new();
    for &(pi, pj) in &pairs {
        let (si, sj) = prev.map_or((0, pj.saturating_sub(pi)), |(x, y)| (x + 1, y + 1));
        if pi > si && pj > sj {
            gaps.push((si..pi, sj..pj));
        } else if pi > si && prev.is_none() {
            // Dikte yazısının başı değişmiş: alanın o kısmıyla aynı uzunlukta karşılaştır.
            let len = pi - si;
            if pj >= len && len > 0 {
                gaps.push((si..pi, pj - len..pj));
            }
        }
        prev = Some((pi, pj));
    }
    if let Some((li, lj)) = prev {
        let rest = n - li - 1;
        if rest > 0 && rest <= 3 && lj + 1 + rest <= m {
            gaps.push((li + 1..n, lj + 1..lj + 1 + rest));
        }
    }
    // Yalnız büyük/küçük harf düzeltmesi ("gmid" → "GMID"); cümle başını büyütmek öneri sayılmaz.
    for &(pi, pj) in &pairs {
        let (from, to) = (&a[pi].1, &b[pj].1);
        if from != to && to.chars().filter(|c| c.is_uppercase()).count() >= 2 {
            let pair = strip_common_suffix(from, to);
            if pair.0 != pair.1 && !out.contains(&pair) {
                out.push(pair);
            }
        }
    }
    let changed: usize = gaps.iter().map(|(x, _)| x.len()).sum();
    if changed * 5 > n * 3 {
        return Vec::new();
    }
    for (x, y) in gaps {
        if x.is_empty() || y.is_empty() || x.len() > 3 || y.len() > 3 {
            continue;
        }
        let from = a[x]
            .iter()
            .map(|t| t.1.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let to = b[y]
            .iter()
            .map(|t| t.1.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let (from, to) = strip_common_suffix(&from, &to);
        // Yalnız büyük/küçük harf farkı "gmid" → "GMID" de öğrenilir; tamamen aynıysa değil.
        if from == to || from.chars().count() > 40 || to.chars().count() > 40 {
            continue;
        }
        if !out.contains(&(from.clone(), to.clone())) {
            out.push((from, to));
        }
        if out.len() >= 3 {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(from: &str, to: &str) -> Rule {
        Rule {
            from: from.into(),
            to: to.into(),
            hits: 0,
        }
    }

    #[test]
    fn applies_at_word_boundaries_keeping_suffixes() {
        let r = [
            rule("GMD", "GMID"),
            rule("gmi", "GMID"),
            rule("videoyu", "VDS"),
        ];
        let (t, used) = apply(
            "gmd'ye gel, GMI sunucusu. Videoyu kiraladım, gmdx kalsın",
            &r,
        );
        assert_eq!(t, "GMID'ye gel, GMID sunucusu. VDS kiraladım, gmdx kalsın");
        assert_eq!(used.len(), 3);
        assert_eq!(fold("IŞIK"), fold("ışık"));
        assert!(overlap("akşam gmi sunucusu", "selam\nakşam GMID sunucusu") > 0.6);
        assert_eq!(overlap("akşam buluşalım", ""), 0.0);
        // Uzun kural önce.
        let (t, _) = apply(
            "gmd tool aç",
            &[rule("gmd", "GMID"), rule("gmd tool", "GMID Tool")],
        );
        assert_eq!(t, "GMID Tool aç");
    }

    #[test]
    fn only_mishearings_are_learned() {
        assert!(plausible("videoyu", "VDS"));
        assert!(plausible("gmd", "GMID"));
        assert!(plausible("Esad", "Esat"));
        assert!(plausible("gmid", "GMID"));
        assert!(plausible("iki", "2"));
        assert!(!plausible("nasılsın", "naber"));
        assert!(!plausible("elma", "armut"));
        assert!(!plausible("yarın gelirim", "bugün"));
    }

    #[test]
    fn suggests_small_substitutions() {
        assert_eq!(
            suggest(
                "Bugün GMD'ye girelim, videoyu da kiraladım.",
                "Bugün GMID'ye girelim, VDS da kiraladım."
            ),
            vec![
                ("GMD".to_string(), "GMID".to_string()),
                ("videoyu".into(), "VDS".into())
            ]
        );
        // Yazı alanında başka yazı da var (önceki mesaj, imza).
        assert_eq!(
            suggest(
                "akşam gmi sunucusunda buluşalım",
                "selam millet\nakşam GMID sunucusunda buluşalım\n-- Ali"
            ),
            vec![("gmi".to_string(), "GMID".to_string())]
        );
        // Yalnız büyük harf düzeltmesi.
        assert_eq!(
            suggest("gmid tool güzel", "GMID tool güzel"),
            vec![("gmid".to_string(), "GMID".to_string())]
        );
        // Sona eklenen sözcük, silinen sözcük: öneri değil.
        assert!(suggest("merhaba dünya", "merhaba dünya nasılsın").is_empty());
        assert!(suggest("merhaba güzel dünya", "merhaba dünya").is_empty());
        // Baştan yazılmış: öneri yok.
        assert!(suggest("bugün hava çok güzel", "yarın yağmur yağacak galiba").is_empty());
        // Son sözcük düzeltilmiş.
        assert_eq!(
            suggest("yeni vds aldık bugün gmd", "yeni vds aldık bugün GMID"),
            vec![("gmd".to_string(), "GMID".to_string())]
        );
    }

    #[test]
    fn hint_words_prefer_used_rules() {
        let mut a = rule("gmd", "GMID");
        a.hits = 1;
        let mut b = rule("videoyu", "VDS");
        b.hits = 5;
        let c = rule("gmi", "GMID");
        assert_eq!(
            hint_words(&[a, b, c]),
            vec!["VDS".to_string(), "GMID".to_string()]
        );
    }
}
