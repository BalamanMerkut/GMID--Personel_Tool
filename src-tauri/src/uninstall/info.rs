//! Program hakkında bilgi: tür (oyun, sürücü/donanım, çalışma ortamı, Microsoft, uygulama), yükleyici, kapsam,
//! mimari ve gerçek kurulum klasörü. Yalnız gösterim ve süzgeç içindir; silme kuralları `rules.rs`'tedir.

/// Program türü (arayüzdeki süzgeçler).
pub fn kind(name: &str, publisher: &str, folder: &str, uninstall: &str, key: &str) -> &'static str {
    let n = name.to_lowercase();
    let p = publisher.to_lowercase();
    let f = format!("{}\\", folder.to_lowercase().replace('/', "\\"));
    let u = uninstall.to_lowercase();
    let k = key.to_lowercase();

    const GAME_DIRS: [&str; 10] = [
        "\\steamapps\\",
        "\\epic games\\",
        "\\riot games\\",
        "\\gog games\\",
        "\\gog galaxy\\games\\",
        "\\ubisoft game launcher\\games\\",
        "\\ea games\\",
        "\\rockstar games\\",
        "\\xboxgames\\",
        "\\games\\",
    ];
    if k.contains("steam app ")
        || u.starts_with("steam://")
        || u.starts_with("\"steam://")
        || u.contains("com.epicgames.launcher://")
        || u.contains("--uninstall-product=")
        || GAME_DIRS.iter().any(|d| f.contains(d))
    {
        return "game";
    }

    const RUNTIMES: [&str; 16] = [
        "visual c++",
        "redistributable",
        ".net",
        "runtime",
        "run time",
        "directx",
        "webview2",
        "java(tm)",
        "java 8",
        "openal",
        "physx",
        "xna framework",
        "vulkan",
        "vcredist",
        "windows sdk",
        "visual studio tools for office",
    ];
    if RUNTIMES.iter().any(|r| n.contains(r)) {
        return "runtime";
    }

    const HW_WORDS: [&str; 19] = [
        "driver",
        "sürücü",
        "chipset",
        "firmware",
        "bios",
        "graphics",
        "hd audio",
        "bluetooth",
        "wi-fi",
        "wireless",
        "touchpad",
        "management engine",
        "rapid storage",
        "serial io",
        "armoury crate",
        "icue",
        "g hub",
        "synapse",
        "control center",
    ];
    // Donanım üreticileri: adın ilk sözcüğü ya da başı.
    const VENDORS: [&str; 27] = [
        "nvidia",
        "advanced micro devices",
        "amd",
        "intel",
        "realtek",
        "asustek",
        "asus",
        "micro-star",
        "msi",
        "gigabyte",
        "logitech",
        "razer",
        "corsair",
        "steelseries",
        "hyperx",
        "synaptics",
        "elan",
        "qualcomm",
        "broadcom",
        "mediatek",
        "dell",
        "lenovo",
        "hp",
        "hewlett",
        "nahimic",
        "waves",
        "dolby",
    ];
    let first = p
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .find(|w| !w.is_empty())
        .unwrap_or("");
    let vendor = VENDORS
        .iter()
        .any(|v| first == *v || (v.contains(' ') && p.starts_with(v)));
    if HW_WORDS.iter().any(|w| n.contains(w)) || vendor {
        return "driver";
    }

    if p.starts_with("microsoft") {
        return "microsoft";
    }
    "app"
}

/// Türün uyarısı (boşsa uyarı yok).
pub fn note(kind: &str) -> &'static str {
    match kind {
        "runtime" => {
            "Çalışma ortamı: başka programlar ve oyunlar buna ihtiyaç duyabilir; kaldırırsan onlar açılmayabilir. Emin değilsen dokunma."
        }
        "driver" => {
            "Sürücü ya da donanım yazılımı: kaldırırsan ses, görüntü, ağ, RGB ya da fan kontrolü gibi şeyler çalışmayabilir."
        }
        "microsoft" => {
            "Microsoft programı: bazı Windows özellikleri buna bağlı olabilir; ne olduğunu biliyorsan kaldır."
        }
        "game" => "Oyun: kayıtların (save) buluta yedekli değilse kaldırınca kaybolabilir.",
        _ => "",
    }
}

/// Hangi yükleyiciyle kurulmuş.
pub fn installer(key: &str, uninstall: &str, windows_installer: bool) -> &'static str {
    let u = uninstall.to_lowercase();
    let k = key.to_lowercase();
    if windows_installer || u.trim_start_matches('"').starts_with("msiexec") {
        "MSI (Windows Installer)"
    } else if u.contains("steam://") {
        "Steam"
    } else if u.contains("com.epicgames.launcher://") {
        "Epic Games"
    } else if u.contains("riotclientservices") {
        "Riot Client"
    } else if k.ends_with("_is1") || u.contains("unins0") {
        "Inno Setup"
    } else if u.contains("update.exe") && u.contains("--uninstall") {
        "Squirrel"
    } else if u.contains("\\package cache\\") {
        "Kurulum paketi"
    } else {
        "Kendi kaldırıcısı"
    }
}

/// Programın gerçek klasörü: `InstallLocation`, yoksa ana exe'nin ya da kaldırıcının klasörü
/// (Windows\Installer ve Package Cache gibi ortak yerler değil). Bilinmiyorsa boş.
pub fn folder(location: &str, exe: &str, uninstaller: Option<&str>) -> String {
    let clean = |s: &str| {
        s.trim()
            .trim_matches('"')
            .trim_end_matches('\\')
            .to_string()
    };
    let loc = clean(location);
    if !loc.is_empty() {
        return loc;
    }
    let shared = |p: &str| {
        let l = p.to_lowercase();
        l.contains("\\windows\\installer")
            || l.contains("\\package cache\\")
            || l.contains("\\windows\\system32")
            || l.contains("\\windows\\syswow64")
    };
    for cand in [Some(exe), uninstaller].into_iter().flatten() {
        let c = clean(cand);
        if !c.to_lowercase().ends_with(".exe") || shared(&c) {
            continue;
        }
        if let Some(i) = c.rfind(['\\', '/']) {
            let parent = &c[..i];
            // "C:" gibi kök dizini klasör sayma.
            if parent.len() > 3 {
                return parent.to_string();
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        let k = |n: &str, p: &str, f: &str, u: &str, key: &str| kind(n, p, f, u, key);
        assert_eq!(
            k(
                "Counter-Strike 2",
                "Valve",
                r"D:\SteamLibrary\steamapps\common\Counter-Strike Global Offensive",
                "steam://uninstall/730",
                "Steam App 730"
            ),
            "game"
        );
        assert_eq!(
            k(
                "VALORANT",
                "Riot Games, Inc",
                r"C:\Riot Games\VALORANT\live",
                r#""C:\Riot Games\Riot Client\RiotClientServices.exe" --uninstall-product=valorant"#,
                "Riot Game valorant.live"
            ),
            "game"
        );
        assert_eq!(
            k(
                "Microsoft Visual C++ 2015-2022 Redistributable (x64)",
                "Microsoft Corporation",
                "",
                "",
                "{x}"
            ),
            "runtime"
        );
        assert_eq!(
            k(
                "Microsoft Edge WebView2 Runtime",
                "Microsoft Corporation",
                "",
                "",
                "x"
            ),
            "runtime"
        );
        assert_eq!(
            k(
                "AMD Chipset Software",
                "Advanced Micro Devices, Inc.",
                "",
                "",
                "x"
            ),
            "driver"
        );
        assert_eq!(
            k(
                "Armoury Crate Service",
                "ASUSTeK COMPUTER INC.",
                "",
                "",
                "x"
            ),
            "driver"
        );
        assert_eq!(
            k(
                "Realtek Audio Driver",
                "Realtek Semiconductor Corp.",
                "",
                "",
                "x"
            ),
            "driver"
        );
        assert_eq!(k("NVIDIA App", "NVIDIA Corporation", "", "", "x"), "driver");
        assert_eq!(
            k("Microsoft Teams", "Microsoft Corporation", "", "", "x"),
            "microsoft"
        );
        assert_eq!(k("Antigravity", "Google", "", "", "x"), "app");
        assert_eq!(k("Aseprite", "Igara Studio", "", "", "x"), "app");
        // "amd" sözcüğü başka adın içinde geçince donanım sayılmaz.
        assert_eq!(k("Hamdi Notes", "Hamdisoft", "", "", "x"), "app");
        assert!(note("runtime").contains("Çalışma ortamı"));
        assert_eq!(note("app"), "");
    }

    #[test]
    fn installers() {
        assert_eq!(
            installer("{G}", "MsiExec.exe /X{G}", false),
            "MSI (Windows Installer)"
        );
        assert_eq!(installer("x", "", true), "MSI (Windows Installer)");
        assert_eq!(
            installer("Steam App 1", "steam://uninstall/1", false),
            "Steam"
        );
        assert_eq!(
            installer("App_is1", r#""C:\A\unins000.exe""#, false),
            "Inno Setup"
        );
        assert_eq!(
            installer(
                "Discord",
                r#""C:\Users\a\AppData\Local\Discord\Update.exe" --uninstall"#,
                false
            ),
            "Squirrel"
        );
        assert_eq!(
            installer("x", r"C:\A\uninstall.exe", false),
            "Kendi kaldırıcısı"
        );
    }

    #[test]
    fn folders() {
        assert_eq!(
            folder(r#""C:\Program Files\App\""#, "", None),
            r"C:\Program Files\App"
        );
        assert_eq!(
            folder("", r"C:\Program Files\App\app.exe", None),
            r"C:\Program Files\App"
        );
        assert_eq!(
            folder("", "", Some(r"C:\Program Files (x86)\Tool\unins000.exe")),
            r"C:\Program Files (x86)\Tool"
        );
        // Ortak yerler programın klasörü değildir.
        assert_eq!(
            folder(
                "",
                r"C:\Windows\Installer\{G}\icon.exe",
                Some(r"C:\ProgramData\Package Cache\{G}\setup.exe")
            ),
            ""
        );
        assert_eq!(folder("", r"C:\app.exe", None), "");
    }
}
