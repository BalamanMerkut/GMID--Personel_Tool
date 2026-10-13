//! Sistem özellikleri (Kontrol Odası → Sistem): işlemci, ekran kartı, bellek modülleri, anakart, diskler,
//! monitörler, fare/klavye, ses ve ağ aygıtları. Windows'un kendi kayıtlarından (CIM/WMI, PnP) tek bir
//! PowerShell çağrısıyla okunur; birkaç saniye sürer, sonuç bellekte tutulur. Hiçbir yere gönderilmez.

use crate::util::hide_console;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Section {
    pub title: String,
    pub icon: String,
    pub rows: Vec<(String, String)>,
}

#[derive(Default)]
pub struct SysInfo {
    cache: Mutex<Option<Vec<Section>>>,
}

const SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
$ProgressPreference = 'SilentlyContinue'
# Windows PowerShell 5.1 dizileri JSON'da {"value":[…],"Count":n} diye yazar; bu tür eki kaldırılınca düz dizi olur.
Remove-TypeData System.Array
function GB($b) { if ($b) { '{0:N1} GB' -f ($b / 1GB) } else { '' } }
function Sec($t, $i, $r) { [pscustomobject]@{ title = $t; icon = $i; rows = @($r | Where-Object { $_ -and $_[1] }) } }
$out = @()

$os = Get-CimInstance Win32_OperatingSystem
$cs = Get-CimInstance Win32_ComputerSystem
$disp = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').DisplayVersion
$up = (Get-Date) - $os.LastBootUpTime
$out += Sec 'Sistem' 'monitor' @(
  ,@('Windows', ("{0} {1} (derleme {2})" -f $os.Caption, $disp, $os.BuildNumber))
  ,@('Bilgisayar', ("{0} {1}" -f $cs.Manufacturer, $cs.Model))
  ,@('Bilgisayar adı', $env:COMPUTERNAME)
  ,@('Açık kalma süresi', ('{0} gün {1} saat {2} dk' -f $up.Days, $up.Hours, $up.Minutes))
)

$cpu = @(Get-CimInstance Win32_Processor)
$out += Sec 'İşlemci' 'bolt' @($cpu | ForEach-Object {
  ,@($_.Name.Trim(), ('{0} çekirdek / {1} iş parçacığı · {2:N2} GHz' -f $_.NumberOfCores, $_.NumberOfLogicalProcessors, ($_.MaxClockSpeed / 1000)))
})

$gpus = @()
$vram = @{}
Get-ChildItem 'HKLM:\SYSTEM\ControlSet001\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}' | ForEach-Object {
  $p = Get-ItemProperty $_.PSPath
  if ($p.DriverDesc -and $p.'HardwareInformation.qwMemorySize') { $vram[$p.DriverDesc] = [uint64]$p.'HardwareInformation.qwMemorySize' }
}
Get-CimInstance Win32_VideoController | ForEach-Object {
  $mem = if ($vram[$_.Name]) { $vram[$_.Name] } else { [uint64]$_.AdapterRAM }
  $res = if ($_.CurrentHorizontalResolution) { (' · {0}×{1} @ {2} Hz' -f $_.CurrentHorizontalResolution, $_.CurrentVerticalResolution, $_.CurrentRefreshRate) } else { '' }
  $gpus += ,@($_.Name, ('{0} bellek · sürücü {1}{2}' -f (GB $mem), $_.DriverVersion, $res))
}
$out += Sec 'Ekran kartı' 'monitor' $gpus

$mods = @(Get-CimInstance Win32_PhysicalMemory)
$total = ($mods | Measure-Object Capacity -Sum).Sum
$speed = ($mods | Measure-Object ConfiguredClockSpeed -Maximum).Maximum
$rows = @(,@('Toplam', ('{0} · {1} modül · {2} MT/s' -f (GB $total), $mods.Count, $speed)))
$mods | ForEach-Object {
  $rows += ,@($_.DeviceLocator, ('{0} · {1} · {2} {3}' -f (GB $_.Capacity), $_.ConfiguredClockSpeed, $_.Manufacturer, ($_.PartNumber -as [string]).Trim()))
}
$out += Sec 'Bellek (RAM)' 'lap' $rows

$bb = Get-CimInstance Win32_BaseBoard
$bios = Get-CimInstance Win32_BIOS
$out += Sec 'Anakart' 'server' @(
  ,@('Model', ('{0} {1}' -f $bb.Manufacturer, $bb.Product))
  ,@('BIOS', ('{0} ({1})' -f $bios.SMBIOSBIOSVersion, $bios.ReleaseDate.ToString('yyyy-MM-dd')))
)

$rows = @()
Get-PhysicalDisk | ForEach-Object {
  $rows += ,@($_.FriendlyName, ('{0} · {1} · {2}' -f (GB $_.Size), $_.MediaType, $_.BusType))
}
Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' | ForEach-Object {
  $rows += ,@(('Sürücü {0}' -f $_.DeviceID), ('{0} boş / {1}' -f (GB $_.FreeSpace), (GB $_.Size)))
}
$out += Sec 'Diskler' 'folder' $rows

$rows = @()
Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorID | ForEach-Object {
  $name = ($_.UserFriendlyName | Where-Object { $_ -gt 0 } | ForEach-Object { [char]$_ }) -join ''
  $make = ($_.ManufacturerName | Where-Object { $_ -gt 0 } | ForEach-Object { [char]$_ }) -join ''
  if ($name) { $rows += ,@($name, ('Üretici kodu {0} · {1}' -f $make, $_.YearOfManufacture)) }
}
$out += Sec 'Monitörler' 'monitor' $rows

function Product($id) {
  foreach ($k in 0..2) {
    $d = (Get-PnpDeviceProperty -InstanceId $id -KeyName DEVPKEY_Device_BusReportedDeviceDesc).Data
    if ($d -and $d -notmatch '^(USB Input Device|HID)') { return $d }
    $id = (Get-PnpDeviceProperty -InstanceId $id -KeyName DEVPKEY_Device_Parent).Data
    if (-not $id) { break }
  }
  $null
}
$rows = @()
$seen = @{}
Get-PnpDevice -PresentOnly -Class Mouse, Keyboard | ForEach-Object {
  $kind = if ($_.Class -eq 'Mouse') { 'Fare' } else { 'Klavye' }
  $name = Product $_.InstanceId
  if (-not $name) { $name = $_.FriendlyName }
  $key = "$kind|$name"
  if (-not $seen[$key]) { $seen[$key] = 1; $rows += ,@($kind, $name) }
}
$out += Sec 'Fare ve klavye' 'keyboard' $rows

$rows = @()
Get-PnpDevice -PresentOnly -Class AudioEndpoint | Where-Object { $_.Status -eq 'OK' } | ForEach-Object {
  $rows += ,@($(if ($_.InstanceId -like '*{0.0.0.00000000}*') { 'Çıkış' } else { 'Giriş' }), $_.FriendlyName)
}
$out += Sec 'Ses aygıtları' 'headphones' $rows

$rows = @()
Get-NetAdapter -Physical | Where-Object { $_.Status -eq 'Up' } | ForEach-Object {
  $rows += ,@($_.Name, ('{0} · {1}' -f $_.InterfaceDescription, $_.LinkSpeed))
}
$out += Sec 'Ağ' 'link' $rows

# Sonuç dosyaya (konsol çıktısı Windows PowerShell'de satır satır bölünebiliyor, Türkçe harfler de kod sayfasına
# takılabiliyor); dosya verilmediyse base64 olarak çıktıya.
$json = $out | ConvertTo-Json -Depth 5 -Compress
if ($env:GMID_OUT) { [System.IO.File]::WriteAllText($env:GMID_OUT, $json, (New-Object System.Text.UTF8Encoding $false)) }
else { [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($json)) }
"#;

impl SysInfo {
    /// Önbellekteki ya da (yoksa / `refresh`) yeniden okunan bilgiler.
    pub fn get(&self, refresh: bool) -> Result<Vec<Section>, String> {
        if !refresh {
            if let Some(v) = self.cache.lock().unwrap_or_else(|e| e.into_inner()).clone() {
                return Ok(v);
            }
        }
        let v = read()?;
        *self.cache.lock().unwrap_or_else(|e| e.into_inner()) = Some(v.clone());
        Ok(v)
    }
}

fn read() -> Result<Vec<Section>, String> {
    if !cfg!(windows) {
        return Err("Yalnızca Windows'ta".into());
    }
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    // Betik UTF-16 olarak kodlanıp verilir (tırnaklar ve Türkçe harfler komut satırında bozulmasın).
    let utf16: Vec<u8> = SCRIPT.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let out_file = std::env::temp_dir().join(format!("gmid-sysinfo-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&out_file);
    let mut cmd = std::process::Command::new("powershell");
    cmd.env("GMID_OUT", &out_file);
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-EncodedCommand",
        &b64.encode(utf16),
    ])
    .stdin(std::process::Stdio::null());
    hide_console(&mut cmd);
    let o = cmd
        .output()
        .map_err(|e| format!("PowerShell açılamadı: {e}"))?;
    let json = match std::fs::read(&out_file) {
        Ok(b) => {
            let _ = std::fs::remove_file(&out_file);
            String::from_utf8_lossy(&b)
                .trim_start_matches('\u{feff}')
                .to_string()
        }
        // Dosya yoksa çıktıdaki base64 (satır sonları ve boşluklar atılarak).
        Err(_) => {
            let text: String = String::from_utf8_lossy(&o.stdout)
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            b64.decode(text)
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .unwrap_or_default()
        }
    };
    let result = parse(&json);
    if let Err(e) = &result {
        let err = String::from_utf8_lossy(&o.stderr);
        crate::log!(
            "Sistem bilgisi okunamadı ({e}); çıkış {:?}, çıktı {} bayt, veri: {}, hata: {}",
            o.status.code(),
            o.stdout.len(),
            json.chars().take(300).collect::<String>(),
            err.trim().chars().take(400).collect::<String>()
        );
    }
    result
        .map_err(|e| format!("{e}. Ayrıntı günlükte: Hakkında → Sorun bildir ile gönderebilirsin."))
}

/// Windows PowerShell 5.1 bazı dizileri `{"value": [...], "Count": n}` olarak yazar: içindeki diziyi al.
fn unwrap(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(mut o) if o.contains_key("value") && o.len() <= 2 => {
            o.remove("value").unwrap_or(serde_json::Value::Null)
        }
        v => v,
    }
}

/// PowerShell'in JSON çıktısı (tek bölüm dizi değil nesne olarak, tek satır iç içe dizi olarak gelebilir).
pub fn parse(json: &str) -> Result<Vec<Section>, String> {
    let v: serde_json::Value =
        serde_json::from_str(json.trim()).map_err(|e| format!("Sistem bilgisi okunamadı: {e}"))?;
    let list = match unwrap(v) {
        serde_json::Value::Array(a) => a,
        o => vec![o],
    };
    let text = |v: &serde_json::Value| match v {
        serde_json::Value::String(s) => s.trim().to_string(),
        serde_json::Value::Null => String::new(),
        o => o.to_string(),
    };
    let mut out = Vec::new();
    for s in list {
        let s = unwrap(s);
        let rows_v = unwrap(s.get("rows").cloned().unwrap_or(serde_json::Value::Null));
        // Tek satırlı bölüm: ["a","b"]; çok satırlı: [["a","b"], …]
        let rows: Vec<serde_json::Value> = match rows_v {
            serde_json::Value::Array(a) if a.first().is_some_and(|x| x.is_string()) => {
                vec![serde_json::Value::Array(a)]
            }
            serde_json::Value::Array(a) => a.into_iter().map(unwrap).collect(),
            _ => Vec::new(),
        };
        let rows: Vec<(String, String)> = rows
            .iter()
            .filter_map(|r| {
                let r = r.as_array()?;
                let (k, v) = (text(r.first()?), text(r.get(1)?));
                (!k.is_empty() && !v.is_empty()).then(|| (k, v.replace("  ", " ")))
            })
            .collect();
        if rows.is_empty() {
            continue;
        }
        out.push(Section {
            title: s.get("title").map(text).unwrap_or_default(),
            icon: s.get("icon").map(text).unwrap_or_else(|| "info".into()),
            rows,
        });
    }
    if out.is_empty() {
        return Err("Sistem bilgisi okunamadı".into());
    }
    Ok(out)
}

/// Discord'a yapıştırmak için düz yazı.
pub fn as_text(sections: &[Section]) -> String {
    let mut t = String::from("🖥️ Sistem özellikleri (GMID Tool)\n");
    for s in sections {
        t += &format!("\n**{}**\n", s.title);
        for (k, v) in &s.rows {
            t += &format!("• {k}: {v}\n");
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_powershell_shapes() {
        let json = r#"[{"title":"İşlemci","icon":"bolt","rows":["AMD Ryzen 7 7800X3D","8 çekirdek / 16 iş parçacığı · 4,20 GHz"]},
            {"title":"Diskler","icon":"folder","rows":[["Samsung 990 PRO","1.863,0 GB · SSD · NVMe"],["Sürücü C:","412,3 GB boş / 1.862,0 GB"]]},
            {"title":"Ağ","icon":"link","rows":[]},
            {"title":"Ses","icon":"headphones","rows":[["Çıkış",null]]}]"#;
        let s = parse(json).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(
            s[0].rows,
            vec![(
                "AMD Ryzen 7 7800X3D".to_string(),
                "8 çekirdek / 16 iş parçacığı · 4,20 GHz".to_string()
            )]
        );
        assert_eq!(s[1].rows.len(), 2);
        // Tek bölüm nesne olarak.
        let one = parse(
            r#"{"title":"Sistem","icon":"monitor","rows":[["Windows","Windows 11 Pro 24H2"]]}"#,
        )
        .unwrap();
        assert_eq!(one[0].title, "Sistem");
        assert!(as_text(&one).contains("• Windows: Windows 11 Pro 24H2"));
        assert!(parse("").is_err());
        // Windows PowerShell 5.1 biçimi: diziler {"value": […], "Count": n} içinde.
        let ps51 = r#"{"value":[{"title":"İşlemci","icon":"bolt","rows":{"value":[{"value":["AMD Ryzen 5 5600","6 çekirdek"],"Count":2}],"Count":1}},
            {"title":"Ağ","icon":"link","rows":[{"value":["Ethernet","1 Gbps"],"Count":2},{"value":["Wi-Fi","866 Mbps"],"Count":2}]}],"Count":2}"#;
        let s = parse(ps51).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].rows[0].0, "AMD Ryzen 5 5600");
        assert_eq!(s[1].rows.len(), 2);
    }
}
