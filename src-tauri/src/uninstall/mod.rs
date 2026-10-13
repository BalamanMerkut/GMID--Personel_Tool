//! Program Kaldırıcı (Revo Uninstaller gibi): kurulu programları listeler, programın **kendi** kaldırıcısını
//! çalıştırır, bitince geride kalan dosya/klasör ve kayıt defteri girdilerini katı kurallarla bulur (bkz.
//! `rules.rs`) ve kullanıcı onaylarsa siler: dosyalar Geri Dönüşüm Kutusu'na gider, kayıtlar silinmeden önce
//! `.reg` dosyasına yedeklenir (çift tıklayınca geri yüklenir). Yalnız son taramada bulunan ve kullanıcının
//! seçtiği öğeler silinebilir.

pub mod icon;
pub mod reg;
pub mod rules;

use rules::{Candidate, RegView, Skipped, Target, World};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const UNINSTALL_ROOTS: [&str; 3] = [
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall",
];
const SOFTWARE_ROOTS: [&str; 3] = [
    r"HKCU\Software",
    r"HKLM\SOFTWARE",
    r"HKLM\SOFTWARE\WOW6432Node",
];
const RUN_KEYS: [&str; 3] = [
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
    r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run",
];

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Program {
    /// Kaldırma kaydının yolu (kimlik).
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub location: String,
    pub uninstall: String,
    /// Ana exe (simge yolundan).
    pub exe: String,
    pub size_kb: u64,
    /// "20240131"
    pub date: String,
    /// Kayıttaki ham `DisplayIcon` ("C:\x\app.exe,0").
    #[serde(skip)]
    pub icon: String,
}

impl Program {
    fn target(&self) -> Target {
        let path = |s: &str| {
            let p = s.trim().trim_matches('"');
            (!p.is_empty()).then(|| PathBuf::from(p))
        };
        Target {
            name: self.name.clone(),
            publisher: self.publisher.clone(),
            location: path(&self.location).filter(|p| p.components().count() > 2),
            exe: path(&self.exe)
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))),
        }
    }
}

/// "C:\x\app.exe,0" → "C:\x\app.exe"
fn icon_path(s: &str) -> String {
    let s = s.trim().trim_matches('"');
    match s.rfind(',') {
        Some(i)
            if s[i + 1..]
                .trim()
                .trim_start_matches('-')
                .chars()
                .all(|c| c.is_ascii_digit()) =>
        {
            s[..i].trim_matches('"').to_string()
        }
        _ => s.to_string(),
    }
}

/// Kurulu programlar (sistem bileşenleri ve güncellemeler hariç), ada göre.
pub fn list() -> Vec<Program> {
    let mut out: Vec<Program> = Vec::new();
    for root in UNINSTALL_ROOTS {
        for sub in reg::subkeys(root) {
            let id = format!("{root}\\{sub}");
            let v: std::collections::HashMap<String, String> =
                reg::values(&id).into_iter().collect();
            let get = |k: &str| v.get(k).cloned().unwrap_or_default();
            let name = get("DisplayName");
            let uninstall = get("UninstallString");
            if name.trim().is_empty()
                || uninstall.trim().is_empty()
                || get("SystemComponent") == "1"
                || !get("ParentKeyName").is_empty()
                || matches!(
                    get("ReleaseType").as_str(),
                    "Update" | "Hotfix" | "Security Update"
                )
            {
                continue;
            }
            if out
                .iter()
                .any(|p| p.name == name && p.version == get("DisplayVersion"))
            {
                continue;
            }
            out.push(Program {
                id,
                version: get("DisplayVersion"),
                publisher: get("Publisher"),
                location: get("InstallLocation"),
                uninstall,
                exe: icon_path(&get("DisplayIcon")),
                icon: get("DisplayIcon"),
                size_kb: get("EstimatedSize").parse().unwrap_or(0),
                date: get("InstallDate"),
                name,
            });
        }
    }
    out.sort_by_key(|p| p.name.to_lowercase());
    out
}

/// Kaldırma komutu: (program, argümanlar). MSI kurulumlarında `/I` yerine `/X` (kaldır).
pub fn split_command(cmd: &str) -> Option<(String, String)> {
    let c = cmd.trim();
    if c.is_empty() {
        return None;
    }
    let lower = c.to_lowercase();
    if lower.starts_with("msiexec") {
        let args = c
            .split_once(char::is_whitespace)
            .map(|x| x.1)
            .unwrap_or("")
            .trim();
        let args = args.replacen("/I", "/X", 1).replacen("/i", "/X", 1);
        return Some(("msiexec.exe".into(), args));
    }
    if let Some(rest) = c.strip_prefix('"') {
        let (exe, args) = rest.split_once('"')?;
        return Some((exe.to_string(), args.trim().to_string()));
    }
    match lower.find(".exe") {
        Some(i) => Some((c[..i + 4].to_string(), c[i + 4..].trim().to_string())),
        None => Some((c.to_string(), String::new())),
    }
}

/// Programın kaldırılmış sayılması: kaydı silinmiş ya da kurulum klasörü/exe artık yok.
fn removed(p: &Program) -> bool {
    if !reg::exists(&p.id) {
        return true;
    }
    let t = p.target();
    let gone = |x: &Option<PathBuf>| x.as_ref().is_some_and(|p| !p.exists());
    gone(&t.location) || (t.location.is_none() && gone(&t.exe))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct State {
    /// Kaldırıcı hâlâ açık mı.
    pub running: bool,
    /// Program kaldırılmış görünüyor mu.
    pub removed: bool,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
    pub id: String,
    pub name: String,
    /// Program hâlâ kurulu görünüyorsa tarama yapılmaz.
    pub removed: bool,
    pub items: Vec<Candidate>,
    pub skipped: Vec<Skipped>,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub done: Vec<String>,
    pub failed: Vec<(String, String)>,
    /// Kayıt yedeklerinin klasörü.
    pub backup: String,
}

#[derive(Default)]
pub struct Uninstaller {
    programs: Mutex<Vec<Program>>,
    running: Mutex<Option<(String, Arc<AtomicBool>)>>,
    last_scan: Mutex<Option<Scan>>,
    /// Simge önbelleği: "kimlik|DisplayIcon" → PNG data URL (yoksa None).
    icons: Mutex<std::collections::HashMap<String, Option<String>>>,
}

impl Uninstaller {
    pub fn list(&self) -> Vec<Program> {
        let l = list();
        *self.programs.lock().unwrap_or_else(|e| e.into_inner()) = l.clone();
        l
    }

    /// Program simgeleri (kimlik → PNG data URL). Her simge bir kez çıkarılır, sonra bellekten gelir.
    pub fn icons(&self) -> std::collections::HashMap<String, String> {
        use base64::Engine;
        let programs = self
            .programs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let mut out = std::collections::HashMap::new();
        for p in programs {
            let key = format!("{}|{}", p.id, p.icon);
            let cached = self
                .icons
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&key)
                .cloned();
            let v = cached.unwrap_or_else(|| {
                let v = icon::png_for(&p).map(|b| {
                    format!(
                        "data:image/png;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(b)
                    )
                });
                self.icons
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(key, v.clone());
                v
            });
            if let Some(v) = v {
                out.insert(p.id, v);
            }
        }
        out
    }

    fn find(&self, id: &str) -> Result<Program, String> {
        self.programs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| "Program bulunamadı; listeyi yenile.".to_string())
    }

    /// Programın kendi kaldırıcısını açar (gerekirse Windows yönetici izni ister); bitmesini arka planda bekler.
    pub fn run(&self, id: &str) -> Result<(), String> {
        let p = self.find(id)?;
        let (exe, args) = split_command(&p.uninstall).ok_or("Kaldırma komutu yok")?;
        let flag = Arc::new(AtomicBool::new(true));
        launch(&exe, &args, flag.clone())?;
        crate::log!("Kaldırıcı açıldı: {} ({exe} {args})", p.name);
        *self.running.lock().unwrap_or_else(|e| e.into_inner()) = Some((id.to_string(), flag));
        Ok(())
    }

    pub fn state(&self, id: &str) -> Result<State, String> {
        let p = self.find(id)?;
        let running = self
            .running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|(rid, f)| rid == id && f.load(Ordering::SeqCst));
        Ok(State {
            running,
            removed: removed(&p),
        })
    }

    /// Artıkları bulur (program kaldırılmış görünmüyorsa boş döner).
    pub fn scan(&self, id: &str) -> Result<Scan, String> {
        let p = self.find(id)?;
        let mut scan = Scan {
            id: p.id.clone(),
            name: p.name.clone(),
            removed: removed(&p),
            ..Default::default()
        };
        if !scan.removed {
            *self.last_scan.lock().unwrap_or_else(|e| e.into_inner()) = None;
            return Ok(scan);
        }
        let t = p.target();
        // Hâlâ kurulu programlar (taze liste): onlara ait hiçbir şeye dokunulmaz.
        let others: Vec<Program> = list().into_iter().filter(|o| o.id != p.id).collect();
        let mut other_paths = Vec::new();
        for o in &others {
            let ot = o.target();
            for x in [
                ot.location,
                ot.exe.and_then(|e| e.parent().map(Path::to_path_buf)),
            ]
            .into_iter()
            .flatten()
            {
                other_paths.push(x.to_string_lossy().to_lowercase());
            }
        }
        let shared_dlls = reg::values(r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\SharedDLLs")
            .into_iter()
            .chain(reg::values(
                r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\SharedDLLs",
            ))
            .map(|(n, _)| n.to_lowercase())
            .collect();
        let running = crate::games::procs::list()
            .into_iter()
            .map(|p| p.path)
            .filter(|p| !p.is_empty())
            .collect();
        let world = World {
            other_paths,
            shared_dlls,
            running,
        };
        let (roots, shortcuts) = roots();
        let (files, skipped) = rules::scan_files(&t, &roots, &shortcuts, &world);

        let mut view = RegView::default();
        let pubt = t.publisher_token();
        for root in SOFTWARE_ROOTS {
            let names = reg::subkeys(root);
            if let Some(pt) = &pubt {
                for n in names.iter().filter(|n| rules::norm(n) == *pt) {
                    let key = format!("{root}\\{n}");
                    let values = reg::values(&key)
                        .into_iter()
                        .filter(|(n, _)| !n.is_empty())
                        .count();
                    view.publisher
                        .push((key.clone(), reg::subkeys(&key), values));
                }
            }
            view.software.push((root.to_string(), names));
        }
        for key in RUN_KEYS {
            for (n, d) in reg::values(key) {
                view.run.push((key.to_string(), n, d));
            }
        }
        if reg::exists(&p.id) {
            view.uninstall_key = Some(p.id.clone());
        }
        scan.items = files;
        scan.items.extend(rules::scan_registry(&t, &view));
        scan.skipped = skipped;
        crate::log!(
            "Artık taraması: {} → {} aday, {} dokunulmadı",
            p.name,
            scan.items.len(),
            scan.skipped.len()
        );
        *self.last_scan.lock().unwrap_or_else(|e| e.into_inner()) = Some(scan.clone());
        Ok(scan)
    }

    /// Seçilenleri siler. Yalnız son taramada bulunanlar kabul edilir.
    pub fn clean(
        &self,
        backup_root: &Path,
        id: &str,
        keys: &[String],
    ) -> Result<CleanResult, String> {
        let scan = self
            .last_scan
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .filter(|s| s.id == id)
            .ok_or("Önce artıkları tara.")?;
        let chosen: Vec<Candidate> = scan
            .items
            .iter()
            .filter(|c| keys.contains(&item_key(c)))
            .cloned()
            .collect();
        if chosen.is_empty() {
            return Err("Silinecek bir şey seçilmedi.".into());
        }
        let stamp = chrono::Local::now().format("%Y-%m-%d %H.%M").to_string();
        let safe_name: String = scan
            .name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let backup = backup_root.join(format!("Kaldırıcı {stamp} {}", safe_name.trim()));
        let out = std::env::temp_dir().join(format!("gmid-kaldir-{}.json", crate::util::new_id()));
        let script = clean_script(&chosen, &backup, &out);
        let home = std::env::var("USERPROFILE")
            .unwrap_or_default()
            .to_lowercase();
        // HKLM ya da kullanıcı klasörü dışındaki dosyalar yönetici izni ister.
        let admin = chosen.iter().any(|c| {
            c.path.starts_with("HKLM")
                || (!c.path.starts_with("HKCU")
                    && (home.is_empty() || !c.path.to_lowercase().starts_with(&home)))
        });
        let code = if admin {
            crate::util::run_elevated_ps(&script)?
        } else {
            run_ps(&script)?
        };
        let text = std::fs::read(&out).map(|b| {
            String::from_utf8_lossy(&b)
                .trim_start_matches('\u{feff}')
                .to_string()
        });
        let _ = std::fs::remove_file(&out);
        let text = text.map_err(|_| format!("Silme betiği sonuç vermedi (kod {code})"))?;
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let mut res = CleanResult {
            backup: backup.to_string_lossy().into_owned(),
            ..Default::default()
        };
        for r in rows {
            let path = r["path"].as_str().unwrap_or_default().to_string();
            if r["ok"].as_bool() == Some(true) {
                res.done.push(path);
            } else {
                res.failed
                    .push((path, r["error"].as_str().unwrap_or("hata").to_string()));
            }
        }
        crate::log!(
            "Artıklar silindi: {} tamam, {} olmadı",
            res.done.len(),
            res.failed.len()
        );
        *self.last_scan.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(res)
    }
}

/// Arayüzle aday eşleştirme anahtarı.
pub fn item_key(c: &Candidate) -> String {
    format!("{}|{}|{}", c.kind, c.path, c.value.as_deref().unwrap_or(""))
}

/// Taranan kök klasörler ve yalnız kısayollarına bakılan klasörler.
fn roots() -> (Vec<PathBuf>, Vec<PathBuf>) {
    let env = |k: &str| {
        std::env::var_os(k)
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
    };
    let mut roots: Vec<PathBuf> = [
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramW6432",
        "ProgramData",
        "APPDATA",
        "LOCALAPPDATA",
    ]
    .iter()
    .filter_map(|k| env(k))
    .collect();
    if let Some(l) = env("LOCALAPPDATA") {
        roots.push(l.join("Programs"));
    }
    if let Some(a) = env("APPDATA") {
        roots.push(a.join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Some(p) = env("ProgramData") {
        roots.push(p.join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    roots.retain(|r| r.is_dir());
    roots.sort();
    roots.dedup();
    let mut shortcuts = Vec::new();
    if let Some(u) = env("USERPROFILE") {
        shortcuts.push(u.join("Desktop"));
    }
    if let Some(p) = env("PUBLIC") {
        shortcuts.push(p.join("Desktop"));
    }
    shortcuts.retain(|d| d.is_dir());
    (roots, shortcuts)
}

/// PowerShell tek tırnaklı metin.
fn ps(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Silme betiği: kayıtlar önce `reg export` ile yedeklenir, sonra `reg delete`; dosya/klasörler Geri Dönüşüm
/// Kutusu'na. Her öğenin sonucu JSON olarak `out`a yazılır.
fn clean_script(items: &[Candidate], backup: &Path, out: &Path) -> String {
    let mut s = String::new();
    s.push_str("$ErrorActionPreference = 'Stop'\nAdd-Type -AssemblyName Microsoft.VisualBasic\n");
    s.push_str(&format!(
        "$backup = {}\nNew-Item -ItemType Directory -Force -Path $backup | Out-Null\n",
        ps(&backup.to_string_lossy())
    ));
    s.push_str("$res = New-Object System.Collections.ArrayList\n");
    s.push_str("function Mark($p, $ok, $err) { [void]$res.Add([pscustomobject]@{ path = $p; ok = $ok; error = $err }) }\n");
    for (i, c) in items.iter().enumerate() {
        let p = ps(&c.path);
        let body = match c.kind {
            "key" => format!(
                "& reg.exe export {p} (Join-Path $backup '{i}.reg') /y | Out-Null; if ($LASTEXITCODE -ne 0) {{ throw 'Yedeklenemedi, silinmedi' }}\n  & reg.exe delete {p} /f | Out-Null; if ($LASTEXITCODE -ne 0) {{ throw 'Silinemedi' }}"
            ),
            "value" => format!(
                "& reg.exe export {p} (Join-Path $backup '{i}.reg') /y | Out-Null; if ($LASTEXITCODE -ne 0) {{ throw 'Yedeklenemedi, silinmedi' }}\n  & reg.exe delete {p} /v {} /f | Out-Null; if ($LASTEXITCODE -ne 0) {{ throw 'Silinemedi' }}",
                ps(c.value.as_deref().unwrap_or(""))
            ),
            "dir" => format!("[Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory({p}, 'OnlyErrorDialogs', 'SendToRecycleBin')"),
            _ => format!("[Microsoft.VisualBasic.FileIO.FileSystem]::DeleteFile({p}, 'OnlyErrorDialogs', 'SendToRecycleBin')"),
        };
        let label = ps(&match &c.value {
            Some(v) => format!("{} [{v}]", c.path),
            None => c.path.clone(),
        });
        s.push_str(&format!("try {{\n  {body}\n  Mark {label} $true $null\n}} catch {{ Mark {label} $false $_.Exception.Message }}\n"));
    }
    s.push_str(&format!(
        "[IO.File]::WriteAllText({}, (ConvertTo-Json -InputObject @($res) -Depth 3), [Text.UTF8Encoding]::new($false))\n",
        ps(&out.to_string_lossy())
    ));
    s
}

fn run_ps(script: &str) -> Result<u32, String> {
    use base64::Engine;
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut cmd = std::process::Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
        &base64::engine::general_purpose::STANDARD.encode(utf16),
    ])
    .stdin(std::process::Stdio::null());
    crate::util::hide_console(&mut cmd);
    let o = cmd
        .output()
        .map_err(|e| format!("PowerShell açılamadı: {e}"))?;
    Ok(o.status.code().unwrap_or(-1) as u32)
}

/// Kaldırıcıyı ShellExecute ile açar (kendi yönetici iznini kendisi ister); kapanınca `flag` düşer.
#[cfg(windows)]
fn launch(exe: &str, args: &str, flag: Arc<AtomicBool>) -> Result<(), String> {
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let file = HSTRING::from(exe);
    let params = HSTRING::from(args);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("open"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    unsafe { ShellExecuteExW(&mut info) }
        .map_err(|e| format!("Kaldırıcı açılamadı: {}", e.message()))?;
    let raw = info.hProcess.0 as isize;
    std::thread::spawn(move || {
        if raw != 0 {
            let h = HANDLE(raw as *mut _);
            unsafe {
                WaitForSingleObject(h, INFINITE);
                let _ = CloseHandle(h);
            }
        }
        flag.store(false, Ordering::SeqCst);
    });
    Ok(())
}

#[cfg(not(windows))]
fn launch(_exe: &str, _args: &str, flag: Arc<AtomicBool>) -> Result<(), String> {
    flag.store(false, Ordering::SeqCst);
    Err("Yalnızca Windows'ta".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uninstall_commands() {
        assert_eq!(
            split_command("MsiExec.exe /I{1234-ABCD}"),
            Some(("msiexec.exe".into(), "/X{1234-ABCD}".into()))
        );
        assert_eq!(
            split_command(r#""C:\Program Files\App\unins000.exe" /SILENT"#),
            Some((
                r"C:\Program Files\App\unins000.exe".into(),
                "/SILENT".into()
            ))
        );
        assert_eq!(
            split_command(r"C:\Program Files\App\uninstall.exe --remove"),
            Some((
                r"C:\Program Files\App\uninstall.exe".into(),
                "--remove".into()
            ))
        );
        assert_eq!(split_command("  "), None);
        assert_eq!(icon_path(r#""C:\App\app.exe",0"#), r"C:\App\app.exe");
        assert_eq!(icon_path(r"C:\App\app.ico"), r"C:\App\app.ico");
    }

    #[test]
    fn clean_script_backs_up_before_delete() {
        let items = vec![
            Candidate {
                kind: "key",
                path: r"HKCU\Software\Coolapp".into(),
                value: None,
                reason: String::new(),
                strong: true,
                size: 0,
            },
            Candidate {
                kind: "value",
                path: r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run".into(),
                value: Some("Cool'app".into()),
                reason: String::new(),
                strong: true,
                size: 0,
            },
            Candidate {
                kind: "dir",
                path: r"C:\Users\a\AppData\Roaming\Coolapp".into(),
                value: None,
                reason: String::new(),
                strong: true,
                size: 0,
            },
        ];
        let s = clean_script(&items, Path::new(r"C:\b"), Path::new(r"C:\o.json"));
        let export = s.find("reg.exe export 'HKCU\\Software\\Coolapp'").unwrap();
        let delete = s.find("reg.exe delete 'HKCU\\Software\\Coolapp'").unwrap();
        assert!(export < delete, "önce yedek");
        assert!(s.contains("throw 'Yedeklenemedi, silinmedi'"));
        assert!(s.contains("/v 'Cool''app' /f"), "tırnak kaçışı");
        assert!(s.contains("SendToRecycleBin"));
        assert!(!s.contains("Remove-Item"), "kalıcı silme yok");
        if let Ok(out) = std::env::var("GMID_DUMP_SCRIPT") {
            std::fs::write(out, &s).unwrap();
        }
    }
}
