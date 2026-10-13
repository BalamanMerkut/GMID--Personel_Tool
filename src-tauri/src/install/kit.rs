//! Format Kiti: USB'ye ya da buluta konan, GMID kurulu değilken de çalışan klasör.
//!
//! ```text
//! GMID Format Kiti\
//!   Kur.bat               ← çift tıkla
//!   kurulum.ps1           ← asıl iş (winget ile programlar, GMID'in kendisi, yedeğin geri yüklenmesi)
//!   GMID-Tool-Kurulum.exe ← isteğe bağlı
//!   GMID-Yedek.zip        ← isteğe bağlı (ayarlar, geçmiş, notlar, kapaklar…)
//!   BENİOKU.txt
//! ```

use super::{Item, Source, APP_INSTALLER_STORE};
use crate::restore::{self, Options};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const KIT_DIR: &str = "GMID Format Kiti";
pub const SETUP_NAME: &str = "GMID-Tool-Kurulum.exe";
pub const BACKUP_NAME: &str = "GMID-Yedek.zip";

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KitRequest {
    /// Kitin konacağı klasör (ör. USB bellek); içine `GMID Format Kiti` açılır.
    pub dir: String,
    pub items: Vec<Item>,
    /// GMID'in kurulum dosyası (isteğe bağlı).
    pub setup: Option<String>,
    pub backup: bool,
    #[serde(default)]
    pub backup_options: Options,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KitResult {
    pub path: String,
    pub programs: usize,
    pub setup: bool,
    pub backup_size: Option<u64>,
}

/// PowerShell tek tırnaklı metin: içindeki ' iki kez yazılır.
fn ps(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// `kurulum.ps1`. Windows PowerShell 5.1 BOM'suz dosyayı ANSI sanır; Türkçe harfler bozulmasın
/// diye dosya UTF-8 BOM ile yazılır.
pub fn script(items: &[Item], created: &str) -> String {
    let programs: Vec<String> = items
        .iter()
        .filter(|i| i.is_valid() && i.source != Source::Web)
        .map(|i| {
            format!(
                "  @{{ Id = {}; Name = {}; Source = {} }}",
                ps(&i.id),
                ps(&i.name),
                ps(i.source.as_str())
            )
        })
        .collect();
    let pages: Vec<String> = items
        .iter()
        .filter(|i| i.is_valid() && i.source == Source::Web)
        .map(|i| {
            format!(
                "  @{{ Name = {}; Url = {} }}",
                ps(&i.name),
                ps(i.url.as_deref().unwrap_or_default())
            )
        })
        .collect();
    let list = |v: &[String]| {
        if v.is_empty() {
            "@()".to_string()
        } else {
            format!("@(\r\n{}\r\n)", v.join(",\r\n"))
        }
    };
    SCRIPT
        .replace("{created}", created)
        .replace("{programs}", &list(&programs))
        .replace("{pages}", &list(&pages))
        .replace("{store}", APP_INSTALLER_STORE)
        .replace("{setup}", SETUP_NAME)
        .replace("{backup}", BACKUP_NAME)
        .replace('\n', "\r\n")
        .replace("\r\r\n", "\r\n")
}

const SCRIPT: &str = r#"# GMID Tool · Format Sonrası Kurulum (Merkut Studios)
# Bu dosyayı GMID Tool {created} tarihinde oluşturdu. Kur.bat ile çalıştır.
$ErrorActionPreference = 'Continue'
try { $Host.UI.RawUI.WindowTitle = 'GMID Format Kiti' } catch {}
$here = Split-Path -Parent $MyInvocation.MyCommand.Path

$programs = {programs}

$pages = {pages}

function Say($text, $color = 'Gray') { Write-Host $text -ForegroundColor $color }
function HasWinget { $null -ne (Get-Command winget -ErrorAction SilentlyContinue) }

Say ''
Say '  GMID Format Kiti' 'Yellow'
Say '  ----------------' 'Yellow'
Say ("  {0} program, {1} indirme sayfası" -f $programs.Count, $pages.Count)
Say ''

# 1) winget (Uygulama Yükleyicisi): yeni kurulmuş Windows'ta ilk dakikalarda hazır olmayabilir.
if ($programs.Count -gt 0 -and -not (HasWinget)) {
  Say 'winget hazırlanıyor...' 'Cyan'
  try { Add-AppxPackage -RegisterByFamilyName -MainPackage Microsoft.DesktopAppInstaller_8wekyb3d8bbwe -ErrorAction Stop } catch {}
  $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
}
$tries = 0
while ($programs.Count -gt 0 -and -not (HasWinget) -and $tries -lt 3) {
  $tries++
  Say ''
  Say 'winget (Uygulama Yükleyicisi) bulunamadı.' 'Red'
  Say 'Microsoft Store açılıyor: "Uygulama Yükleyicisi"ni güncelle ya da yükle.'
  Start-Process '{store}'
  $answer = Read-Host 'Bitince Enter''a bas (programları atlamak için "g" yaz)'
  if ($answer -eq 'g') { break }
  $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
}

# 2) Programlar, sırayla ve sessizce.
$ok = @(); $already = @(); $failed = @()
if (HasWinget) {
  $i = 0
  foreach ($p in $programs) {
    $i++
    Say ''
    Say ("[{0}/{1}] {2}" -f $i, $programs.Count, $p.Name) 'Cyan'
    $wargs = @('install', '--id', $p.Id, '-e', '--source', $p.Source, '--silent',
      '--accept-package-agreements', '--accept-source-agreements', '--disable-interactivity')
    & winget @wargs
    $code = $LASTEXITCODE
    if ($code -eq 0 -or $code -eq -1978334967 -or $code -eq -1978334966) { $ok += $p.Name; Say '  Kuruldu' 'Green' }
    elseif ($code -eq -1978335189 -or $code -eq -1978335135) { $already += $p.Name; Say '  Zaten kurulu' 'DarkGreen' }
    else { $failed += $p.Name; Say ("  Kurulamadı (kod {0})" -f $code) 'Red' }
  }
} elseif ($programs.Count -gt 0) {
  Say 'winget olmadan programlar kurulamadı; bu dosyayı sonra yeniden çalıştırabilirsin.' 'Red'
}

# 3) GMID'in verisi (ayarlar, geçmiş, notlar...). Uygulama ilk açılmadan önce yerine konur.
$zip = Join-Path $here '{backup}'
$data = Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'GMID Tool'
if (Test-Path $zip) {
  Say ''
  Say 'GMID yedeği geri yükleniyor...' 'Cyan'
  Get-Process -Name 'gmid-tool', 'GMID Tool' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
  Start-Sleep -Milliseconds 500
  if (Test-Path (Join-Path $data 'gmid.db')) {
    $old = 'GMID Tool (önceki ' + (Get-Date -Format 'yyyy-MM-dd HH-mm') + ')'
    Rename-Item -Path $data -NewName $old -ErrorAction SilentlyContinue
    if (Test-Path (Join-Path $data 'gmid.db')) { Say '  Mevcut GMID klasörü taşınamadı; yedek açılmadı.' 'Red'; $zip = $null }
    else { Say ("  Mevcut veriler '{0}' klasörüne taşındı." -f $old) }
  }
  if ($zip) {
    try {
      New-Item -ItemType Directory -Force -Path $data | Out-Null
      Expand-Archive -Path $zip -DestinationPath $data -Force -ErrorAction Stop
      Say '  Yedek yerine kondu.' 'Green'
    } catch { Say ('  Yedek açılamadı: ' + $_.Exception.Message) 'Red' }
  }
}

# 4) GMID Tool'un kendisi.
$setup = Join-Path $here '{setup}'
if (Test-Path $setup) {
  Say ''
  Say 'GMID Tool kuruluyor...' 'Cyan'
  try {
    Start-Process -FilePath $setup -ArgumentList '/S' -Wait -ErrorAction Stop
    $ok += 'GMID Tool'; Say '  Kuruldu' 'Green'
    $exe = Get-ChildItem -Path "$env:LOCALAPPDATA\GMID Tool", "$env:ProgramFiles\GMID Tool" -Filter '*.exe' -ErrorAction SilentlyContinue |
      Where-Object { $_.Name -in @('gmid-tool.exe', 'GMID Tool.exe') } | Select-Object -First 1
    if ($exe) { Start-Process $exe.FullName }
  } catch { $failed += 'GMID Tool'; Say ('  Kurulamadı: ' + $_.Exception.Message) 'Red' }
}

# 5) winget'te olmayanlar: resmî indirme sayfaları.
foreach ($p in $pages) {
  Say ''
  Say ($p.Name + ': indirme sayfası açılıyor') 'Cyan'
  Start-Process $p.Url
}

Say ''
Say '  Bitti' 'Yellow'
if ($ok.Count) { Say ('  Kuruldu: ' + ($ok -join ', ')) 'Green' }
if ($already.Count) { Say ('  Zaten vardı: ' + ($already -join ', ')) 'DarkGreen' }
if ($failed.Count) { Say ('  Kurulamadı: ' + ($failed -join ', ') + '  (bu dosyayı yeniden çalıştırmak yeterli)') 'Red' }
Say ''
Read-Host 'Kapatmak için Enter'
"#;

/// `Kur.bat`: yalnız ASCII (cmd.exe karakter sayfasından bağımsız).
pub const BAT: &str = "@echo off\r\ntitle GMID Format Kiti\r\ncd /d \"%~dp0\"\r\npowershell -NoProfile -ExecutionPolicy RemoteSigned -File \"%~dp0kurulum.ps1\"\r\n";

fn readme(items: &[Item], setup: bool, backup: bool) -> String {
    let mut s = String::from(
        "GMID Format Kiti (Merkut Studios)\r\n=================================\r\n\r\n\
         Formattan sonra bu klasördeki Kur.bat dosyasına çift tıkla. Gerisi kendiliğinden:\r\n\r\n",
    );
    let names: Vec<&str> = items
        .iter()
        .filter(|i| i.source != Source::Web)
        .map(|i| i.name.as_str())
        .collect();
    if !names.is_empty() {
        s += &format!(
            "  1. Programlar sırayla ve sessizce kurulur (winget ile):\r\n     {}\r\n",
            names.join(", ")
        );
    }
    if backup {
        s += "  2. GMID'in verisi geri yüklenir: ayarlar, notlar, hatırlatıcılar, oyun kütüphanesi,\r\n     \
              oturum geçmişi, puanlar, çalma listeleri, kapaklar ve arka planlar.\r\n";
    }
    if setup {
        s += "  3. GMID Tool kurulur ve açılır; formattan önceki haliyle karşına çıkar.\r\n";
    }
    let pages: Vec<&str> = items
        .iter()
        .filter(|i| i.source == Source::Web)
        .map(|i| i.name.as_str())
        .collect();
    if !pages.is_empty() {
        s += &format!(
            "  4. winget'te olmayanların indirme sayfası açılır: {}\r\n",
            pages.join(", ")
        );
    }
    s += "\r\nBazı programlar kurulurken Windows \"değişiklik yapılsın mı?\" diye sorabilir: Evet de.\r\n\
          Bir program kurulamazsa Kur.bat'ı yeniden çalıştırmak yeterli; kurulu olanlar atlanır.\r\n";
    s
}

/// Kurulum dosyasını İndirilenler ya da Masaüstü'nde arar (en yenisi).
pub fn find_setup(home: &Path) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for dir in ["Downloads", "Desktop", "İndirilenler", "Masaüstü"] {
        let Ok(rd) = std::fs::read_dir(home.join(dir)) else {
            continue;
        };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_lowercase();
            if n.contains("gmid")
                && n.ends_with(".exe")
                && (n.contains("setup") || n.contains("kurulum"))
            {
                if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                    if best.as_ref().map_or(true, |(b, _)| t > *b) {
                        best = Some((t, e.path()));
                    }
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Kiti oluşturur. `progress(aşama, yapılan, toplam)`.
pub fn build(
    db: &crate::db::Db,
    root: &Path,
    home: &Path,
    req: &KitRequest,
    progress: &mut dyn FnMut(&str, u64, u64),
) -> Result<KitResult, String> {
    let base = PathBuf::from(&req.dir);
    if !base.is_dir() {
        return Err("Seçilen klasör bulunamadı".into());
    }
    let kit = base.join(KIT_DIR);
    // Veri klasörünün içine kit kurulursa yedek kendini kopyalamaya çalışır.
    if kit.starts_with(root) {
        return Err("Kiti GMID'in kendi klasörü dışında bir yere koy (ör. USB bellek)".into());
    }
    std::fs::create_dir_all(&kit).map_err(|e| format!("{}: {e}", kit.display()))?;
    let items: Vec<Item> = req.items.iter().filter(|i| i.is_valid()).cloned().collect();
    let created = chrono::Local::now().format("%d.%m.%Y %H:%M").to_string();

    progress("script", 0, 1);
    let mut ps1 = vec![0xEF, 0xBB, 0xBF];
    ps1.extend_from_slice(script(&items, &created).as_bytes());
    std::fs::write(kit.join("kurulum.ps1"), ps1).map_err(|e| e.to_string())?;
    std::fs::write(kit.join("Kur.bat"), BAT).map_err(|e| e.to_string())?;

    let mut setup = false;
    if let Some(src) = req.setup.as_deref().filter(|s| !s.is_empty()) {
        progress("setup", 0, 1);
        std::fs::copy(src, kit.join(SETUP_NAME))
            .map_err(|e| format!("Kurulum dosyası kopyalanamadı: {e}"))?;
        setup = true;
    } else {
        let _ = std::fs::remove_file(kit.join(SETUP_NAME));
    }

    let mut backup_size = None;
    if req.backup {
        backup_size = Some(restore::archive(
            db,
            root,
            home,
            &kit.join(BACKUP_NAME),
            req.backup_options,
            &mut |a, b| progress("backup", a, b),
        )?);
    } else {
        let _ = std::fs::remove_file(kit.join(BACKUP_NAME));
    }

    let mut readme_bytes = vec![0xEF, 0xBB, 0xBF];
    readme_bytes.extend_from_slice(readme(&items, setup, req.backup).as_bytes());
    std::fs::write(kit.join("BENİOKU.txt"), readme_bytes).map_err(|e| e.to_string())?;
    progress("done", 1, 1);
    Ok(KitResult {
        path: kit.to_string_lossy().into_owned(),
        programs: items.len(),
        setup,
        backup_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, name: &str, source: Source, url: Option<&str>) -> Item {
        Item {
            id: id.into(),
            name: name.into(),
            source,
            category: None,
            url: url.map(Into::into),
            note: None,
        }
    }

    #[test]
    fn script_lists_programs_and_pages_safely() {
        let items = vec![
            item("Google.Chrome", "Google Chrome", Source::Winget, None),
            item("9NKSQGP7F2NH", "WhatsApp'ın", Source::Msstore, None),
            item(
                "nvidia-app",
                "NVIDIA App",
                Source::Web,
                Some("https://www.nvidia.com/x"),
            ),
            item("bad id;", "Kötü", Source::Winget, None),
        ];
        let s = script(&items, "11.10.2026 21:00");
        assert!(s.contains("@{ Id = 'Google.Chrome'; Name = 'Google Chrome'; Source = 'winget' }"));
        assert!(s.contains("Name = 'WhatsApp''ın'; Source = 'msstore'"));
        assert!(s.contains("@{ Name = 'NVIDIA App'; Url = 'https://www.nvidia.com/x' }"));
        assert!(!s.contains("Kötü"));
        assert!(!s.contains("{programs}") && !s.contains("{store}") && !s.contains("{backup}"));
        assert!(s.contains("ms-windows-store://pdp/?ProductId=9NBLGGH4NNS1"));
        // Satır sonları tutarlı CRLF.
        assert!(!s.replace("\r\n", "").contains('\n'));
        assert!(BAT.is_ascii());
        // Boş liste de geçerli PowerShell olsun.
        assert!(script(&[], "x").contains("$programs = @()"));
    }

    #[test]
    fn kit_is_built_outside_data_folder() {
        let base = std::env::temp_dir().join(format!("gmid-kit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("GMID Tool");
        let usb = base.join("USB");
        std::fs::create_dir_all(root.join("Notlar")).unwrap();
        std::fs::create_dir_all(&usb).unwrap();
        std::fs::write(root.join("Notlar").join("a.md"), "x").unwrap();
        let db = crate::db::Db::open(&root.join("gmid.db")).unwrap();
        let setup = base.join("GMID Tool_0.1.0_x64-setup.exe");
        std::fs::write(&setup, "exe").unwrap();
        let mut req = KitRequest {
            dir: usb.to_string_lossy().into_owned(),
            items: vec![item("Valve.Steam", "Steam", Source::Winget, None)],
            setup: Some(setup.to_string_lossy().into_owned()),
            backup: true,
            backup_options: Options::default(),
        };
        let mut stages = Vec::new();
        let r = build(&db, &root, &base, &req, &mut |s, _, _| {
            stages.push(s.to_string())
        })
        .unwrap();
        let kit = usb.join(KIT_DIR);
        for f in [
            "Kur.bat",
            "kurulum.ps1",
            SETUP_NAME,
            BACKUP_NAME,
            "BENİOKU.txt",
        ] {
            assert!(kit.join(f).is_file(), "{f}");
        }
        assert!(r.setup && r.backup_size.unwrap() > 0);
        assert_eq!(stages.last().map(String::as_str), Some("done"));
        let ps1 = std::fs::read(kit.join("kurulum.ps1")).unwrap();
        assert_eq!(&ps1[..3], &[0xEF, 0xBB, 0xBF]);

        // Kit veri klasörünün içine konamaz.
        req.dir = root.to_string_lossy().into_owned();
        assert!(build(&db, &root, &base, &req, &mut |_, _, _| {}).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn setup_is_found_in_downloads() {
        let home = std::env::temp_dir().join(format!("gmid-home-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("Downloads")).unwrap();
        assert!(find_setup(&home).is_none());
        std::fs::write(
            home.join("Downloads").join("GMID Tool_0.1.0_x64-setup.exe"),
            "x",
        )
        .unwrap();
        std::fs::write(home.join("Downloads").join("baska-setup.exe"), "x").unwrap();
        assert!(find_setup(&home)
            .unwrap()
            .ends_with("GMID Tool_0.1.0_x64-setup.exe"));
        let _ = std::fs::remove_dir_all(&home);
    }
}
