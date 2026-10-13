//! Artık (kalıntı) kuralları: kaldırılan programdan geriye kalan dosya, klasör ve kayıt defteri girdilerinden
//! hangilerinin **yalnız o programa ait** olduğuna karar verir. Windows'u bozmamak için kurallar bilerek katıdır;
//! şüpheli her şey "dokunulmadı" listesine nedeniyle düşer, kullanıcıya gösterilir ama silinmez.
//!
//! 1. **Ad birebir tutmalı.** Programın adı (sürüm, bit, dil ekleri atılmış), kurulum klasörünün adı ya da
//!    "yayıncı adı atılmış" adı; harf/boşluk/noktalama farkı gözetmeden **tam eşit** olmalı. Benzerlik ya da
//!    içerme yetmez ("Discord" ≠ "DiscordPTB", "Steam" ≠ "SteamVR").
//! 2. **Yalnız bilinen yerler, yalnız birinci kat.** Dosyada: Program Files (iki tane), ProgramData, AppData
//!    (Roaming/Local), Local\Programs ve Başlat menüsünün doğrudan alt klasörleri; ya da `<yayıncı>\<program>`.
//!    Daha derine inilmez, başka yer taranmaz.
//! 3. **Korunanlar asla.** Windows, Microsoft, Common Files, WindowsApps, Packages, System32 adlı klasörler ve
//!    altları; sürücü kökleri ve taranan kök klasörlerin kendisi.
//! 4. **Başka bir programın parçasıysa asla.** Hâlâ kurulu bir programın klasörü adayın içindeyse (ya da aday onun
//!    içindeyse), adayda o programın dosyası varsa, ortak DLL kaydında (SharedDLLs) geçen bir dosya varsa, şu an
//!    çalışan bir program oradan açılmışsa: dokunulmaz.
//! 5. **Bağlantı (junction/symlink) asla.** Aday ya da içindeki herhangi bir şey bağlantıysa dokunulmaz.
//! 6. **Genel adlar asla.** "Update", "Common", "Microsoft", "Google" gibi birçok programın paylaştığı adlar ve
//!    4 harften kısa adlar eşleşme sayılmaz.
//! 7. **Kayıt defterinde** yalnız `Software\<Ad>` ve `Software\<Yayıncı>\<Ad>` (HKCU, HKLM, HKLM WOW6432Node),
//!    programın kendi kaldırma kaydı ve başlangıçta açılma (Run) değerleri. `Microsoft`, `Classes`, `Policies`,
//!    servisler ve COM kayıtlarına hiç bakılmaz.
//! 8. Yayıncı klasörü/anahtarı yalnız içinde başka hiçbir şey kalmayacaksa silinir.
//! 9. Silme geri alınabilir: dosyalar Geri Dönüşüm Kutusu'na, kayıtlar silinmeden önce `.reg` dosyasına yedeklenir.

use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Birçok programın paylaştığı ya da Windows'a ait adlar: asla eşleşme sayılmaz.
const GENERIC: &[&str] = &[
    "microsoft",
    "windows",
    "common",
    "commonfiles",
    "shared",
    "update",
    "updater",
    "updates",
    "system",
    "program",
    "programs",
    "programfiles",
    "data",
    "app",
    "apps",
    "application",
    "applications",
    "software",
    "classes",
    "policies",
    "clients",
    "intel",
    "nvidia",
    "nvidiacorporation",
    "amd",
    "realtek",
    "google",
    "mozilla",
    "apple",
    "adobe",
    "oracle",
    "java",
    "python",
    "packages",
    "temp",
    "cache",
    "logs",
    "config",
    "settings",
    "users",
    "user",
    "public",
    "default",
    "desktop",
    "documents",
    "downloads",
    "games",
    "launcher",
    "client",
    "helper",
    "service",
    "services",
    "setup",
    "install",
    "installer",
    "uninstall",
    "tools",
    "tool",
    "driver",
    "drivers",
    "runtime",
    "redistributable",
    "framework",
    "sdk",
    "plugins",
    "plugin",
    "steam",
    "epicgames",
    "origin",
    "ubisoft",
    "valve",
    "electron",
    "chromium",
    "crashpad",
    "crashreports",
    "webview2",
    "dotnet",
    "vcredist",
    "directx",
    "wow6432node",
];

/// Yolun hiçbir parçası bunlar olamaz (taranan kökün altında kalan kısımda).
const PROTECTED_PARTS: &[&str] = &[
    "windows",
    "microsoft",
    "common files",
    "windowsapps",
    "packages",
    "system32",
    "syswow64",
    "winsxs",
    "drivers",
    "$recycle.bin",
    "system volume information",
    "windows defender",
    "windows nt",
    "internet explorer",
];

/// "Mozilla Firefox (x64 tr)" → "mozillafirefox"; harf ve rakam dışı her şey atılır, Türkçe harfler korunur.
pub fn norm(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// Görünen addan sürüm, bit ve dil eklerini atar: "7-Zip 23.01 (x64)" → "7-Zip", "VLC media player 3.0.20" → "VLC media player".
pub fn strip_version(name: &str) -> String {
    let mut s = String::new();
    let mut depth = 0;
    for c in name.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth as i32 - 1).max(0) as usize,
            _ if depth == 0 => s.push(c),
            _ => {}
        }
    }
    let words: Vec<&str> = s
        .split_whitespace()
        .filter(|w| {
            let lw = w.to_lowercase();
            let versionish = lw
                .trim_start_matches('v')
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
                && lw.chars().any(|c| c.is_ascii_digit());
            !(versionish
                || ["x64", "x86", "64-bit", "32-bit", "64bit", "32bit", "-", "–"]
                    .contains(&lw.as_str()))
        })
        .collect();
    words.join(" ").trim().to_string()
}

/// Kaldırılan programın kimliği (kurallar bununla eşleştirir).
#[derive(Clone, Debug, Default)]
pub struct Target {
    pub name: String,
    pub publisher: String,
    /// Kurulum klasörü (biliniyorsa).
    pub location: Option<PathBuf>,
    /// Ana exe (simge ya da kaldırıcı yolundan; biliniyorsa).
    pub exe: Option<PathBuf>,
}

impl Target {
    /// Klasör/anahtar adı bunlardan biriyle **birebir** eşleşmeli.
    pub fn tokens(&self) -> HashSet<String> {
        let mut t = HashSet::new();
        let base = strip_version(&self.name);
        t.insert(norm(&base));
        let pubn = norm(&strip_version(&self.publisher));
        // "Mozilla Firefox" → "Firefox" (yayıncı adı başta yazılmışsa)
        let first = base.split_whitespace().next().unwrap_or_default();
        if !pubn.is_empty() && pubn.starts_with(&norm(first)) && base.split_whitespace().count() > 1
        {
            t.insert(norm(&base[first.len()..]));
        }
        if let Some(leaf) = self.location.as_ref().and_then(|l| l.file_name()) {
            t.insert(norm(&leaf.to_string_lossy()));
        }
        t.retain(|x| x.chars().count() >= 4 && !GENERIC.contains(&x.as_str()) && *x != pubn);
        t
    }

    /// Exe adı (ör. "firefox"): yalnız 5+ harf ve genel değilse, ayrıca zayıf eşleşme sayılır.
    pub fn exe_token(&self) -> Option<String> {
        let stem = norm(&self.exe.as_ref()?.file_stem()?.to_string_lossy());
        (stem.chars().count() >= 5 && !GENERIC.contains(&stem.as_str()) && !stem.contains("unins"))
            .then_some(stem)
    }

    pub fn publisher_token(&self) -> Option<String> {
        let p = norm(&strip_version(&self.publisher));
        (p.chars().count() >= 3).then_some(p)
    }
}

/// Bir aday: dosya/klasör ya da kayıt defteri anahtarı/değeri.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// "dir" | "file" | "key" | "value"
    pub kind: &'static str,
    /// Dosya yolu ya da kayıt yolu ("HKCU\Software\X"); değer için anahtar yolu.
    pub path: String,
    /// Değer adı (yalnız "value").
    pub value: Option<String>,
    /// Neden aday: "Adı programla birebir aynı" gibi.
    pub reason: String,
    /// Güçlü eşleşme: varsayılan seçili.
    pub strong: bool,
    pub size: u64,
}

/// Dokunulmayan ve nedeni (kullanıcıya şeffaflık için gösterilir).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Skipped {
    pub path: String,
    pub reason: String,
}

/// Diğer kurulu programlar ve sistem bilgisi: "başkasına ait mi?" denetimi için.
#[derive(Default)]
pub struct World {
    /// Hâlâ kurulu programların klasörleri ve exe yolları (küçük harf).
    pub other_paths: Vec<String>,
    /// SharedDLLs'teki dosya yolları (küçük harf).
    pub shared_dlls: HashSet<String>,
    /// Şu an çalışan programların yolları (küçük harf).
    pub running: Vec<String>,
}

fn lower(p: &Path) -> String {
    p.to_string_lossy().to_lowercase().replace('/', "\\")
}

/// `inner`, `outer`ın içinde mi (ya da aynısı)?
fn within(inner: &str, outer: &str) -> bool {
    let o = outer.trim_end_matches('\\');
    !o.is_empty() && (inner == o || inner.starts_with(&format!("{o}\\")))
}

fn is_link(p: &Path) -> bool {
    let Ok(m) = std::fs::symlink_metadata(p) else {
        return true; // okunamıyorsa dokunma
    };
    if m.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// Klasörü (bağlantılara girmeden) gezer: (boyut, içindeki dosyalar küçük harf, bağlantı var mı).
fn walk(dir: &Path, limit: usize) -> (u64, Vec<String>, bool) {
    let (mut size, mut files, mut link) = (0u64, Vec::new(), false);
    let mut stack = vec![dir.to_path_buf()];
    let mut seen = 0usize;
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            seen += 1;
            if seen > limit {
                return (size, files, link);
            }
            let p = e.path();
            if is_link(&p) {
                link = true;
                continue;
            }
            match e.metadata() {
                Ok(m) if m.is_dir() => stack.push(p),
                Ok(m) => {
                    size += m.len();
                    files.push(lower(&p));
                }
                Err(_) => {}
            }
        }
    }
    (size, files, link)
}

/// Yolun, taranan kökün altında kalan parçalarından biri korunan bir ad mı?
fn protected(root: &Path, p: &Path) -> bool {
    let Ok(rel) = p.strip_prefix(root) else {
        return true;
    };
    rel.components()
        .any(|c| PROTECTED_PARTS.contains(&c.as_os_str().to_string_lossy().to_lowercase().as_str()))
}

/// Bir aday klasör/dosya için güvenlik denetimi; sorun varsa nedeni.
fn unsafe_reason(root: &Path, p: &Path, w: &World, roots: &[PathBuf]) -> Option<String> {
    if protected(root, p) {
        return Some("Windows'a ya da Microsoft'a ait bir klasörün içinde".into());
    }
    if is_link(p) {
        return Some("Bağlantı (junction/symlink): hedefi başka yerde olabilir".into());
    }
    let lp = lower(p);
    if roots.iter().any(|r| within(&lower(r), &lp)) || p.parent().is_none() {
        return Some("Ana klasör ya da sürücü kökü".into());
    }
    for o in &w.other_paths {
        if within(o, &lp) || within(&lp, o) {
            return Some(format!("Hâlâ kurulu başka bir programla ilgili ({o})"));
        }
    }
    if w.running.iter().any(|r| within(r, &lp)) {
        return Some("Şu an buradan açılmış bir program çalışıyor".into());
    }
    if p.is_dir() {
        let (_, files, link) = walk(p, 200_000);
        if link {
            return Some("İçinde bağlantı (junction/symlink) var".into());
        }
        if let Some(f) = files.iter().find(|f| w.shared_dlls.contains(*f)) {
            return Some(format!("Ortak kullanılan bir dosya içeriyor ({f})"));
        }
    } else if w.shared_dlls.contains(&lp) {
        return Some("Ortak kullanılan bir dosya".into());
    }
    None
}

/// Dosya sistemi artıkları. `roots`: taranacak kök klasörler (yalnız birinci kat + `<yayıncı>\<program>`).
/// `shortcut_dirs`: Masaüstü gibi yalnız `.lnk` kısayollarına bakılan klasörler.
pub fn scan_files(
    t: &Target,
    roots: &[PathBuf],
    shortcut_dirs: &[PathBuf],
    w: &World,
) -> (Vec<Candidate>, Vec<Skipped>) {
    let tokens = t.tokens();
    let exe = t.exe_token();
    let pubt = t.publisher_token();
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    let mut seen = HashSet::new();
    let mut consider = |root: &Path,
                        p: PathBuf,
                        strong: bool,
                        reason: String,
                        out: &mut Vec<Candidate>,
                        skipped: &mut Vec<Skipped>| {
        if !seen.insert(lower(&p)) {
            return;
        }
        match unsafe_reason(root, &p, w, roots) {
            Some(r) => skipped.push(Skipped {
                path: p.to_string_lossy().into_owned(),
                reason: r,
            }),
            None => {
                let (size, kind) = if p.is_dir() {
                    (walk(&p, 200_000).0, "dir")
                } else {
                    (std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0), "file")
                };
                out.push(Candidate {
                    kind,
                    path: p.to_string_lossy().into_owned(),
                    value: None,
                    reason,
                    strong,
                    size,
                });
            }
        }
    };

    // Kurulum klasörü hâlâ duruyorsa (kaldırıcı her şeyi silmemişse).
    if let Some(loc) = t.location.as_ref().filter(|l| l.is_dir()) {
        if let Some(root) = roots.iter().find(|r| loc.starts_with(r)) {
            consider(
                root,
                loc.clone(),
                true,
                "Programın kurulum klasörü (kaldırıcı silmemiş)".into(),
                &mut out,
                &mut skipped,
            );
        } else {
            skipped.push(Skipped {
                path: loc.to_string_lossy().into_owned(),
                reason: "Kurulum klasörü bilinen bir yerde değil; elle bakmak gerekir".into(),
            });
        }
    }

    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let n = norm(name.trim_end_matches(".lnk"));
            let p = e.path();
            if tokens.contains(&n) {
                consider(
                    root,
                    p,
                    true,
                    "Adı programla birebir aynı".into(),
                    &mut out,
                    &mut skipped,
                );
            } else if exe.as_deref() == Some(n.as_str()) {
                consider(
                    root,
                    p,
                    false,
                    "Adı programın exe adıyla aynı (kontrol et)".into(),
                    &mut out,
                    &mut skipped,
                );
            } else if pubt.as_deref() == Some(n.as_str()) && p.is_dir() && !is_link(&p) {
                // Yayıncı klasörü: içinde yalnız programın adını taşıyanlar.
                let children: Vec<PathBuf> = std::fs::read_dir(&p)
                    .map(|r| r.flatten().map(|e| e.path()).collect())
                    .unwrap_or_default();
                let mut matched = 0;
                for c in &children {
                    let cn = norm(&c.file_name().unwrap_or_default().to_string_lossy());
                    if tokens.contains(&cn) {
                        matched += 1;
                        consider(
                            root,
                            c.clone(),
                            true,
                            format!("Yayıncı klasöründe ({name}), adı programla birebir aynı"),
                            &mut out,
                            &mut skipped,
                        );
                    }
                }
                if matched > 0 && matched == children.len() {
                    consider(
                        root,
                        p,
                        true,
                        "Yayıncı klasörü: içinde yalnız bu programın dosyaları var".into(),
                        &mut out,
                        &mut skipped,
                    );
                }
            }
        }
    }

    for dir in shortcut_dirs {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.to_lowercase().ends_with(".lnk")
                && tokens.contains(&norm(&name[..name.len() - 4]))
            {
                consider(
                    dir,
                    e.path(),
                    true,
                    "Programın kısayolu".into(),
                    &mut out,
                    &mut skipped,
                );
            }
        }
    }

    // Bir aday başka bir adayın içindeyse yalnız dıştaki kalsın.
    let all: Vec<String> = out.iter().map(|c| c.path.to_lowercase()).collect();
    out.retain(|c| {
        let lc = c.path.to_lowercase();
        !all.iter().any(|o| *o != lc && within(&lc, o))
    });
    (out, skipped)
}

/// Kayıt defterinden okunanlar (PowerShell ile; burada yalnız karar verilir).
#[derive(Default, Debug)]
pub struct RegView {
    /// (kök, alt anahtar adları): ör. ("HKCU\Software", ["7-Zip", "Mozilla", …])
    pub software: Vec<(String, Vec<String>)>,
    /// (yayıncı anahtarı yolu, alt anahtar adları, kendi değer sayısı)
    pub publisher: Vec<(String, Vec<String>, usize)>,
    /// (Run anahtarı, değer adı, değer verisi)
    pub run: Vec<(String, String, String)>,
    /// Programın kaldırma kaydı hâlâ duruyorsa yolu.
    pub uninstall_key: Option<String>,
}

/// Kayıt defteri artıkları.
pub fn scan_registry(t: &Target, r: &RegView) -> Vec<Candidate> {
    let tokens = t.tokens();
    let mut out = Vec::new();
    let mk = |kind, path: String, value: Option<String>, reason: &str| Candidate {
        kind,
        path,
        value,
        reason: reason.into(),
        strong: true,
        size: 0,
    };
    for (root, names) in &r.software {
        for n in names {
            if tokens.contains(&norm(n)) {
                out.push(mk(
                    "key",
                    format!("{root}\\{n}"),
                    None,
                    "Programın ayar anahtarı (adı birebir aynı)",
                ));
            }
        }
    }
    for (key, children, values) in &r.publisher {
        let matched: Vec<&String> = children
            .iter()
            .filter(|c| tokens.contains(&norm(c)))
            .collect();
        for c in &matched {
            out.push(mk(
                "key",
                format!("{key}\\{c}"),
                None,
                "Yayıncı anahtarında, adı programla birebir aynı",
            ));
        }
        if !matched.is_empty() && matched.len() == children.len() && *values == 0 {
            out.push(mk(
                "key",
                key.clone(),
                None,
                "Yayıncı anahtarı: içinde yalnız bu program var",
            ));
        }
    }
    if let Some(k) = &r.uninstall_key {
        out.push(mk(
            "key",
            k.clone(),
            None,
            "Programın kaldırma kaydı (program artık yok)",
        ));
    }
    // Başlangıçta açılma: yalnız verisi kurulum klasörünü ya da exe yolunu gösteriyorsa.
    let loc = t
        .location
        .as_ref()
        .map(|l| lower(l))
        .filter(|l| l.len() >= 10);
    let exe = t.exe.as_ref().map(|e| lower(e)).filter(|e| e.len() >= 10);
    for (key, name, data) in &r.run {
        let d = data.to_lowercase().replace('/', "\\");
        let hit = loc
            .as_ref()
            .is_some_and(|l| d.contains(&format!("{}\\", l.trim_end_matches('\\'))))
            || exe.as_ref().is_some_and(|e| d.contains(e.as_str()));
        if hit {
            out.push(mk(
                "value",
                key.clone(),
                Some(name.clone()),
                "Başlangıçta açılma kaydı (programın klasörünü gösteriyor)",
            ));
        }
    }
    // Bir anahtar başka bir adayın içindeyse dıştaki yeter.
    let keys: Vec<String> = out
        .iter()
        .filter(|c| c.kind == "key")
        .map(|c| c.path.to_lowercase())
        .collect();
    out.retain(|c| {
        let lc = c.path.to_lowercase();
        c.kind != "key" || !keys.iter().any(|o| *o != lc && within(&lc, o))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(name: &str, publisher: &str, loc: Option<&Path>) -> Target {
        Target {
            name: name.into(),
            publisher: publisher.into(),
            location: loc.map(Path::to_path_buf),
            exe: None,
        }
    }

    #[test]
    fn names_are_normalized_without_versions() {
        assert_eq!(strip_version("7-Zip 23.01 (x64)"), "7-Zip");
        assert_eq!(strip_version("VLC media player 3.0.20"), "VLC media player");
        assert_eq!(strip_version("Mozilla Firefox (x64 tr)"), "Mozilla Firefox");
        assert_eq!(strip_version("Notepad++ v8.6.2 (64-bit x64)"), "Notepad++");
        assert_eq!(norm("Notepad++"), "notepad");
        assert_eq!(norm("Çark Çekiliş"), "çarkçekiliş");
        let ff = t("Mozilla Firefox (x64 tr)", "Mozilla", None);
        let tk = ff.tokens();
        assert!(
            tk.contains("mozillafirefox") && tk.contains("firefox"),
            "{tk:?}"
        );
        assert!(
            !tk.contains("mozilla"),
            "yayıncı adı tek başına eşleşme değil"
        );
        // Genel ve kısa adlar asla.
        assert!(t("Update", "", None).tokens().is_empty());
        assert!(t("Steam", "Valve", None).tokens().is_empty());
        assert!(t("Git", "", None).tokens().is_empty());
    }

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }

    #[test]
    fn strict_file_rules() {
        let base = std::env::temp_dir().join(format!("gmid-unins-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let pf = base.join("Program Files");
        let appdata = base.join("AppData");
        let desktop = base.join("Desktop");
        // Kaldırılan: "Coolapp 2.1 (x64)", yayıncı "Acme"
        touch(&pf.join("Coolapp/kalan.dll"));
        touch(&appdata.join("Coolapp/ayar.json"));
        touch(&appdata.join("CoolappPro/x.txt")); // benzer ad: dokunulmaz
        touch(&appdata.join("Acme/Coolapp/log.txt")); // yayıncı/ program
        touch(&appdata.join("Acme/Other/data.bin")); // aynı yayıncının başka programı: yayıncı klasörü silinmez
        touch(&appdata.join("Microsoft/Coolapp/x")); // Microsoft altında: hiç bakılmaz (birinci kat değil)
        touch(&appdata.join("SharedThing/Coolapp.dll"));
        touch(&pf.join("Coolapp2/a")); // başka ad
        touch(&desktop.join("Coolapp.lnk"));
        touch(&desktop.join("Coolapp.txt"));
        // Başka kurulu bir program Coolapp klasörünün içinde (ör. eklenti) → dokunulmaz
        touch(&base.join("ProgramData/Coolapp/plugin/plugin.exe"));
        let target = t("Coolapp 2.1 (x64)", "Acme", None);
        let w = World {
            other_paths: vec![lower(&base.join("ProgramData/Coolapp/plugin"))],
            shared_dlls: HashSet::new(),
            running: vec![],
        };
        let roots = vec![pf.clone(), appdata.clone(), base.join("ProgramData")];
        let (c, s) = scan_files(&target, &roots, std::slice::from_ref(&desktop), &w);
        let paths: Vec<String> = c.iter().map(|x| x.path.replace('\\', "/")).collect();
        let has = |p: &Path| paths.contains(&p.to_string_lossy().replace('\\', "/"));
        assert!(has(&pf.join("Coolapp")), "{paths:?}");
        assert!(has(&appdata.join("Coolapp")));
        assert!(has(&appdata.join("Acme").join("Coolapp")));
        assert!(has(&desktop.join("Coolapp.lnk")));
        assert!(!has(&appdata.join("CoolappPro")), "benzer ad silinmez");
        assert!(
            !has(&appdata.join("Acme")),
            "içinde başka program olan yayıncı klasörü silinmez"
        );
        assert!(!has(&pf.join("Coolapp2")));
        assert!(!has(&desktop.join("Coolapp.txt")), "yalnız kısayol");
        assert!(!paths.iter().any(|p| p.contains("Microsoft")));
        assert!(
            !has(&base.join("ProgramData").join("Coolapp")),
            "içinde kurulu başka program var"
        );
        assert!(
            s.iter().any(|x| x.reason.contains("başka bir program")),
            "{s:?}"
        );
        assert!(c.iter().all(|x| x.strong));

        // Ortak DLL içeren klasör dokunulmaz.
        let w2 = World {
            shared_dlls: [lower(&appdata.join("Coolapp").join("ayar.json"))]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let (c2, s2) = scan_files(&target, &roots, &[], &w2);
        assert!(!c2
            .iter()
            .any(|x| x.path.replace('\\', "/").ends_with("AppData/Coolapp")));
        assert!(s2.iter().any(|x| x.reason.contains("Ortak")));

        // Kök klasörün kendisi asla (program adı kök adıyla aynı olsa bile).
        let w3 = World::default();
        let odd = t("AppData Cleaner", "", Some(&appdata));
        let (c3, _) = scan_files(&odd, &roots, &[], &w3);
        assert!(!c3
            .iter()
            .any(|x| x.path.replace('\\', "/").trim_end_matches('/')
                == appdata.to_string_lossy().replace('\\', "/")));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[cfg(unix)]
    #[test]
    fn links_are_never_touched() {
        let base = std::env::temp_dir().join(format!("gmid-unins-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("AppData");
        touch(&base.join("important/keep.txt"));
        std::fs::create_dir_all(&root).unwrap();
        std::os::unix::fs::symlink(base.join("important"), root.join("Coolapp")).unwrap();
        touch(&root.join("Linkyapp/real.txt"));
        std::os::unix::fs::symlink(base.join("important"), root.join("Linkyapp/inner")).unwrap();
        let (c, s) = scan_files(
            &t("Coolapp", "", None),
            std::slice::from_ref(&root),
            &[],
            &World::default(),
        );
        assert!(
            c.is_empty() && s.iter().any(|x| x.reason.contains("Bağlantı")),
            "{c:?} {s:?}"
        );
        let (c, s) = scan_files(
            &t("Linkyapp", "", None),
            std::slice::from_ref(&root),
            &[],
            &World::default(),
        );
        assert!(
            c.is_empty() && s.iter().any(|x| x.reason.contains("İçinde bağlantı")),
            "{c:?} {s:?}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn strict_registry_rules() {
        let target = Target {
            name: "Coolapp 2.1".into(),
            publisher: "Acme Inc.".into(),
            location: Some(PathBuf::from(r"C:\Program Files\Coolapp")),
            exe: Some(PathBuf::from(r"C:\Program Files\Coolapp\coolapp.exe")),
        };
        let view = RegView {
            software: vec![
                (
                    "HKCU\\Software".into(),
                    vec![
                        "Coolapp".into(),
                        "CoolappPro".into(),
                        "Microsoft".into(),
                        "Classes".into(),
                    ],
                ),
                (
                    "HKLM\\SOFTWARE".into(),
                    vec!["coolapp".into(), "Other".into()],
                ),
            ],
            publisher: vec![
                ("HKCU\\Software\\Acme Inc".into(), vec!["Coolapp".into()], 0),
                (
                    "HKLM\\SOFTWARE\\Acme Inc".into(),
                    vec!["Coolapp".into(), "Other".into()],
                    0,
                ),
            ],
            run: vec![
                (
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run".into(),
                    "Coolapp".into(),
                    r#""C:\Program Files\Coolapp\coolapp.exe" --tray"#.into(),
                ),
                (
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run".into(),
                    "Else".into(),
                    r"C:\Program Files\Coolapp2\x.exe".into(),
                ),
            ],
            uninstall_key: Some(
                "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Coolapp".into(),
            ),
        };
        let c = scan_registry(&target, &view);
        let paths: Vec<&str> = c.iter().map(|x| x.path.as_str()).collect();
        assert!(paths.contains(&"HKCU\\Software\\Coolapp"));
        assert!(paths.contains(&"HKLM\\SOFTWARE\\coolapp"));
        assert!(!paths.iter().any(|p| p.ends_with("CoolappPro")
            || p.ends_with("Microsoft")
            || p.ends_with("Classes")));
        // HKCU yayıncı anahtarı yalnız bu programı içeriyor: anahtarın kendisi (içindeki aday onun altında kalır).
        assert!(paths.contains(&"HKCU\\Software\\Acme Inc"));
        assert!(
            !paths.contains(&"HKCU\\Software\\Acme Inc\\Coolapp"),
            "dıştaki yeter"
        );
        // HKLM yayıncı anahtarında başka program var: yalnız alt anahtar.
        assert!(paths.contains(&"HKLM\\SOFTWARE\\Acme Inc\\Coolapp"));
        assert!(!paths.contains(&"HKLM\\SOFTWARE\\Acme Inc"));
        assert_eq!(
            c.iter().filter(|x| x.kind == "value").count(),
            1,
            "yalnız programın klasörünü gösteren Run değeri"
        );
        assert!(paths.iter().any(|p| p.ends_with("Uninstall\\Coolapp")));
    }
}
