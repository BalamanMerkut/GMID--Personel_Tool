//! Sistem özellikleri (Kontrol Odası → Sistem özellikleri): işlemci, ekran kartı, bellek modülleri, anakart,
//! diskler ve bölümler, monitörler, fare/klavye, ses ve ağ aygıtları. Windows'un kendi kayıtlarından (CIM/WMI,
//! PnP, kayıt defteri) tek bir PowerShell çağrısıyla ham değerler olarak okunur; adlar burada sadeleştirilir
//! ("Intel(R) Core(TM) i7-10700K CPU @ 3.80GHz" → "Intel Core i7-10700K"). Sonuç bellekte tutulur, hiçbir yere
//! gönderilmez.

use crate::util::hide_console;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Specs {
    pub os: Os,
    pub cpu: Vec<Cpu>,
    pub gpu: Vec<Gpu>,
    pub ram: Ram,
    pub board: Board,
    pub disks: Vec<Disk>,
    pub volumes: Vec<Volume>,
    pub monitors: Vec<Monitor>,
    pub input: Vec<Device>,
    pub audio: Vec<Device>,
    pub net: Vec<Net>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Os {
    pub name: String,
    pub version: String,
    pub build: String,
    pub computer: String,
    pub maker: String,
    pub model: String,
    /// Açık kalma süresi (dakika).
    pub uptime_min: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Cpu {
    pub name: String,
    pub cores: u32,
    pub threads: u32,
    pub mhz: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Gpu {
    pub name: String,
    /// Bayt.
    pub vram: u64,
    pub driver: String,
    pub width: u32,
    pub height: u32,
    pub hz: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Ram {
    pub total: u64,
    pub modules: Vec<RamModule>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RamModule {
    pub slot: String,
    pub size: u64,
    /// MT/s (çalışan hız).
    pub speed: u32,
    pub maker: String,
    pub part: String,
    /// SMBIOS bellek türü: 24 DDR3, 26 DDR4, 34 DDR5.
    #[serde(rename = "type")]
    pub kind: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Board {
    pub maker: String,
    pub product: String,
    pub bios: String,
    pub bios_date: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Disk {
    pub name: String,
    pub size: u64,
    /// SSD | HDD | Unspecified
    pub media: String,
    /// NVMe | SATA | USB …
    pub bus: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Volume {
    pub letter: String,
    pub label: String,
    pub size: u64,
    pub free: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Monitor {
    pub name: String,
    /// EDID üretici kodu ("GSM") ya da çözülmüş adı ("LG").
    pub maker: String,
    pub year: u32,
    /// Ekran köşegeni (inç, fiziksel boyuttan).
    pub inches: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Device {
    /// Fare | Klavye | Çıkış | Giriş
    pub kind: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Net {
    pub name: String,
    pub desc: String,
    pub speed: String,
}

#[derive(Default)]
pub struct SysInfo {
    cache: Mutex<Option<Specs>>,
}

const SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
$ProgressPreference = 'SilentlyContinue'
# Windows PowerShell 5.1 dizileri JSON'da {"value":[…],"Count":n} diye yazar; bu tür eki kaldırılınca düz dizi olur.
Remove-TypeData System.Array

$os = Get-CimInstance Win32_OperatingSystem
$cs = Get-CimInstance Win32_ComputerSystem
$disp = (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').DisplayVersion
$up = [int]((Get-Date) - $os.LastBootUpTime).TotalMinutes
$r = [ordered]@{}
$r.os = [ordered]@{ name = [string]$os.Caption; version = [string]$disp; build = [string]$os.BuildNumber; computer = [string]$env:COMPUTERNAME; maker = [string]$cs.Manufacturer; model = [string]$cs.Model; uptimeMin = $up }

$r.cpu = @(Get-CimInstance Win32_Processor | ForEach-Object {
  [ordered]@{ name = [string]$_.Name; cores = [int]$_.NumberOfCores; threads = [int]$_.NumberOfLogicalProcessors; mhz = [int]$_.MaxClockSpeed }
})

$vram = @{}
Get-ChildItem 'HKLM:\SYSTEM\ControlSet001\Control\Class\{4d36e968-e325-11ce-bfc1-08002be10318}' | ForEach-Object {
  $p = Get-ItemProperty $_.PSPath
  $m = $p.'HardwareInformation.qwMemorySize'
  if ($p.DriverDesc -and $m) { if ($m -is [byte[]]) { $m = [BitConverter]::ToUInt64($m + [byte[]](0,0,0,0,0,0,0,0), 0) }; $vram[$p.DriverDesc] = [uint64]$m }
}
$r.gpu = @(Get-CimInstance Win32_VideoController | ForEach-Object {
  $mem = if ($vram[$_.Name]) { $vram[$_.Name] } else { [uint64]$_.AdapterRAM }
  [ordered]@{ name = [string]$_.Name; vram = $mem; driver = [string]$_.DriverVersion; width = [int]$_.CurrentHorizontalResolution; height = [int]$_.CurrentVerticalResolution; hz = [int]$_.CurrentRefreshRate }
})

$mods = @(Get-CimInstance Win32_PhysicalMemory)
$r.ram = [ordered]@{
  total = [uint64]$cs.TotalPhysicalMemory
  modules = @($mods | ForEach-Object {
    [ordered]@{ slot = [string]$_.DeviceLocator; size = [uint64]$_.Capacity; speed = [int]$(if ($_.ConfiguredClockSpeed) { $_.ConfiguredClockSpeed } else { $_.Speed }); maker = [string]$_.Manufacturer; part = ([string]$_.PartNumber).Trim(); type = [int]$_.SMBIOSMemoryType }
  })
}

$bb = Get-CimInstance Win32_BaseBoard
$bios = Get-CimInstance Win32_BIOS
$r.board = [ordered]@{ maker = [string]$bb.Manufacturer; product = [string]$bb.Product; bios = [string]$bios.SMBIOSBIOSVersion; biosDate = $(if ($bios.ReleaseDate) { $bios.ReleaseDate.ToString('yyyy-MM-dd') } else { '' }) }

$r.disks = @(Get-PhysicalDisk | ForEach-Object {
  [ordered]@{ name = [string]$_.FriendlyName; size = [uint64]$_.Size; media = [string]$_.MediaType; bus = [string]$_.BusType }
})
$r.volumes = @(Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3' | ForEach-Object {
  [ordered]@{ letter = [string]$_.DeviceID; label = [string]$_.VolumeName; size = [uint64]$_.Size; free = [uint64]$_.FreeSpace }
})

$params = @{}
Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorBasicDisplayParams | ForEach-Object { $params[$_.InstanceName] = $_ }
$r.monitors = @(Get-CimInstance -Namespace root\wmi -ClassName WmiMonitorID | ForEach-Object {
  $name = ($_.UserFriendlyName | Where-Object { $_ -gt 0 } | ForEach-Object { [char]$_ }) -join ''
  $make = ($_.ManufacturerName | Where-Object { $_ -gt 0 } | ForEach-Object { [char]$_ }) -join ''
  $p = $params[$_.InstanceName]
  $in = 0
  if ($p -and $p.MaxHorizontalImageSize -gt 0) { $in = [math]::Round([math]::Sqrt([math]::Pow($p.MaxHorizontalImageSize, 2) + [math]::Pow($p.MaxVerticalImageSize, 2)) / 2.54, 1) }
  [ordered]@{ name = $name; maker = $make; year = [int]$_.YearOfManufacture; inches = $in }
})

function Product($id) {
  foreach ($k in 0..2) {
    $d = (Get-PnpDeviceProperty -InstanceId $id -KeyName DEVPKEY_Device_BusReportedDeviceDesc).Data
    if ($d -and $d -notmatch '^(USB Input Device|HID|USB Composite Device)') { return $d }
    $id = (Get-PnpDeviceProperty -InstanceId $id -KeyName DEVPKEY_Device_Parent).Data
    if (-not $id) { break }
  }
  $null
}
$r.input = @(Get-PnpDevice -PresentOnly -Class Mouse, Keyboard | ForEach-Object {
  $name = Product $_.InstanceId
  if (-not $name) { $name = $_.FriendlyName }
  [ordered]@{ kind = $(if ($_.Class -eq 'Mouse') { 'Fare' } else { 'Klavye' }); name = [string]$name }
})
$r.audio = @(Get-PnpDevice -PresentOnly -Class AudioEndpoint | Where-Object { $_.Status -eq 'OK' } | ForEach-Object {
  [ordered]@{ kind = $(if ($_.InstanceId -like '*{0.0.0.00000000}*') { 'Çıkış' } else { 'Giriş' }); name = [string]$_.FriendlyName }
})
$r.net = @(Get-NetAdapter -Physical | Where-Object { $_.Status -eq 'Up' } | ForEach-Object {
  [ordered]@{ name = [string]$_.Name; desc = [string]$_.InterfaceDescription; speed = [string]$_.LinkSpeed }
})

# Sonuç dosyaya (konsol çıktısı Windows PowerShell'de bölünebiliyor, Türkçe harfler kod sayfasına takılabiliyor);
# dosya verilmediyse base64 olarak çıktıya.
$json = [pscustomobject]$r | ConvertTo-Json -Depth 6 -Compress
if ($env:GMID_OUT) { [System.IO.File]::WriteAllText($env:GMID_OUT, $json, (New-Object System.Text.UTF8Encoding $false)) }
else { [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($json)) }
"#;

impl SysInfo {
    /// Önbellekteki ya da (yoksa / `refresh`) yeniden okunan bilgiler.
    pub fn get(&self, refresh: bool) -> Result<Specs, String> {
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

fn read() -> Result<Specs, String> {
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

/// Windows PowerShell 5.1 biçimlerini düzeltir: `{"value": [...], "Count": n}` kutuları açılır, liste olması
/// gereken alanlardaki tek nesne listeye çevrilir, sayı yerine metin gelen değerler (ör. "16") sayıya.
fn normalize(v: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Object(mut o) if o.contains_key("value") && o.len() <= 2 => {
            normalize(o.remove("value").unwrap_or(Value::Null))
        }
        Value::Object(o) => Value::Object(
            o.into_iter()
                .map(|(k, v)| {
                    let v = normalize(v);
                    let list = matches!(
                        k.as_str(),
                        "cpu"
                            | "gpu"
                            | "modules"
                            | "disks"
                            | "volumes"
                            | "monitors"
                            | "input"
                            | "audio"
                            | "net"
                    );
                    let v = match v {
                        Value::Object(_) if list => Value::Array(vec![v]),
                        Value::Null if list => Value::Array(vec![]),
                        v => v,
                    };
                    (k, v)
                })
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.into_iter().map(normalize).collect()),
        v => v,
    }
}

/// Sayıların bazen metin, metinlerin bazen null geldiği esnek okuma.
fn lenient(v: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Object(o) => Value::Object(
            o.into_iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k, lenient(v)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.into_iter().map(lenient).collect()),
        v => v,
    }
}

pub fn parse(json: &str) -> Result<Specs, String> {
    let v: serde_json::Value =
        serde_json::from_str(json.trim()).map_err(|e| format!("Sistem bilgisi okunamadı: {e}"))?;
    let v = lenient(normalize(v));
    let mut s: Specs =
        serde_json::from_value(v).map_err(|e| format!("Sistem bilgisi okunamadı: {e}"))?;
    if s.os.name.is_empty() && s.cpu.is_empty() && s.gpu.is_empty() {
        return Err("Sistem bilgisi okunamadı".into());
    }
    tidy(&mut s);
    Ok(s)
}

/// "Intel(R) Core(TM) i7-10700K CPU @ 3.80GHz" → "Intel Core i7-10700K";
/// "AMD Ryzen 7 7800X3D 8-Core Processor" → "AMD Ryzen 7 7800X3D".
pub fn clean_cpu(name: &str) -> String {
    let mut s = name
        .replace("(R)", "")
        .replace("(TM)", "")
        .replace("(tm)", "");
    if let Some(i) = s.find(" @ ") {
        s.truncate(i);
    }
    // "8-Core", "16-Core", "Six-Core" gibi ekler.
    let mut s = s
        .split_whitespace()
        .filter(|w| !w.to_lowercase().ends_with("-core"))
        .collect::<Vec<_>>()
        .join(" ");
    for tail in [" Processor", " CPU"] {
        if let Some(t) = s.strip_suffix(tail) {
            s = t.to_string();
        }
    }
    s
}

/// "LG ULTRAGEAR" gibi adlarda marka zaten varsa tekrar yazılmaz.
pub fn monitor_title(m: &Monitor) -> String {
    if m.maker.is_empty() || m.name.to_lowercase().starts_with(&m.maker.to_lowercase()) {
        m.name.clone()
    } else {
        format!("{} {}", m.maker, m.name).trim().to_string()
    }
}

/// EDID üretici kodu → marka.
fn monitor_maker(code: &str) -> String {
    let name = match code.trim().to_uppercase().as_str() {
        "GSM" => "LG",
        "SAM" | "SEC" => "Samsung",
        "ACR" => "Acer",
        "AUS" => "ASUS",
        "DEL" => "Dell",
        "AOC" => "AOC",
        "BNQ" => "BenQ",
        "MSI" => "MSI",
        "HWP" | "HPN" => "HP",
        "LEN" => "Lenovo",
        "PHL" => "Philips",
        "VSC" => "ViewSonic",
        "GBT" => "Gigabyte",
        "AOR" => "AORUS",
        "HKC" => "HKC",
        "SNY" => "Sony",
        "IVM" => "iiyama",
        "XMI" => "Xiaomi",
        "BOE" => "BOE",
        "AUO" => "AU Optronics",
        "CMN" => "Innolux",
        "LGD" => "LG Display",
        "SHP" => "Sharp",
        "HSD" => "HannStar",
        "NEC" => "NEC",
        "EIZ" => "EIZO",
        "MEI" => "Panasonic",
        "TCL" => "TCL",
        "PNP" => "",
        _ => code.trim(),
    };
    name.to_string()
}

/// RAM üreticisi: bazı modüller ad yerine JEDEC kodu yazar ("80CE" → Samsung).
fn ram_maker(m: &str) -> String {
    let t = m.trim();
    let name = match t.to_uppercase().as_str() {
        "80CE" | "CE00000000000000" => "Samsung",
        "80AD" | "AD00000000000000" => "SK hynix",
        "802C" | "2C00000000000000" => "Micron",
        "859B" | "9B00000000000000" => "Crucial",
        "04CD" => "G.Skill",
        "029E" => "Corsair",
        "0198" | "9801000000000000" => "Kingston",
        "04CB" => "ADATA",
        "05CB" => "Crucial",
        "UNKNOWN" | "" => "",
        _ => t,
    };
    name.to_string()
}

fn generic(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with("hid ")
        || n.contains("hid-compliant")
        || n.contains("hid uyumlu")
        || n.starts_with("usb input")
        || n.starts_with("standard ps/2")
        || n.contains("keyboard device")
        || n.contains("klavye aygıtı")
        || n == "hid keyboard device"
}

/// Adları sadeleştirir, tekrarları ve anlamsız kayıtları atar, önemlisini öne alır.
fn tidy(s: &mut Specs) {
    for c in &mut s.cpu {
        c.name = clean_cpu(&c.name);
    }
    // Ayrık ekran kartı (en çok bellekli) önce; sanal/uzak ekran sürücüleri atılır.
    s.gpu.retain(|g| {
        let n = g.name.to_lowercase();
        !(n.contains("remote")
            || n.contains("virtual")
            || n.contains("basic display")
            || n.contains("parsec"))
    });
    s.gpu.sort_by_key(|g| std::cmp::Reverse(g.vram));
    for g in &mut s.gpu {
        g.name = g
            .name
            .replace("(R)", "")
            .replace("(TM)", "")
            .trim()
            .to_string();
    }
    for m in &mut s.ram.modules {
        m.maker = ram_maker(&m.maker);
    }
    if s.ram.total == 0 {
        s.ram.total = s.ram.modules.iter().map(|m| m.size).sum();
    }
    for m in &mut s.monitors {
        m.maker = monitor_maker(&m.maker);
    }
    s.monitors
        .retain(|m| !m.name.is_empty() || !m.maker.is_empty());
    // Fare/klavye: aynı ad bir kez; adı belli olan varsa genel "HID" kayıtları gösterilmez.
    let mut input: Vec<Device> = Vec::new();
    for d in std::mem::take(&mut s.input) {
        if d.name.trim().is_empty() || input.iter().any(|x| x.kind == d.kind && x.name == d.name) {
            continue;
        }
        input.push(d);
    }
    for kind in ["Fare", "Klavye"] {
        if input.iter().any(|d| d.kind == kind && !generic(&d.name)) {
            input.retain(|d| d.kind != kind || !generic(&d.name));
        }
    }
    s.input = input;
    let mut audio: Vec<Device> = Vec::new();
    for d in std::mem::take(&mut s.audio) {
        if !d.name.trim().is_empty() && !audio.contains(&d) {
            audio.push(d);
        }
    }
    s.audio = audio;
    s.volumes.retain(|v| v.size > 0);
    s.volumes.sort_by(|a, b| a.letter.cmp(&b.letter));
    s.disks.retain(|d| d.size > 0);
}

fn gb(b: u64) -> String {
    let g = b as f64 / 1_073_741_824.0;
    if g >= 1000.0 {
        format!("{:.1} TB", g / 1024.0).replace('.', ",")
    } else if g >= 10.0 {
        format!("{g:.0} GB")
    } else {
        format!("{g:.1} GB").replace('.', ",")
    }
}

/// Bellek boyutu: 17179869184 → "16 GB" (yuvarlanmış).
fn ram_gb(b: u64) -> String {
    format!("{:.0} GB", b as f64 / 1_073_741_824.0)
}

pub fn ram_type(t: u32) -> &'static str {
    match t {
        34 => "DDR5",
        26 => "DDR4",
        24 => "DDR3",
        _ => "",
    }
}

/// Discord'a yapıştırmak için düzenli özet.
pub fn as_text(s: &Specs) -> String {
    let mut t = String::from("🖥️ **Sistem özellikleri** · GMID Tool\n");
    let mut line = |k: &str, v: String| {
        if !v.trim().is_empty() {
            t += &format!("**{k}:** {v}\n");
        }
    };
    line(
        "Windows",
        format!("{} {}", s.os.name.replace("Microsoft ", ""), s.os.version),
    );
    for c in &s.cpu {
        line(
            "İşlemci",
            format!(
                "{} ({} çekirdek / {} iş parçacığı)",
                c.name, c.cores, c.threads
            ),
        );
    }
    for g in &s.gpu {
        let vram = if g.vram > 0 {
            format!(" ({})", ram_gb(g.vram))
        } else {
            String::new()
        };
        line("Ekran kartı", format!("{}{vram}", g.name));
    }
    if s.ram.total > 0 {
        let m = &s.ram.modules;
        let speed = m.iter().map(|x| x.speed).max().unwrap_or(0);
        let kind = m.first().map(|x| ram_type(x.kind)).unwrap_or("");
        let layout = if m.len() > 1 && m.iter().all(|x| x.size == m[0].size) {
            format!(" ({}×{})", m.len(), ram_gb(m[0].size))
        } else {
            String::new()
        };
        line(
            "Bellek",
            format!(
                "{}{layout} {kind}{}",
                ram_gb(s.ram.total),
                if speed > 0 {
                    format!(" {speed} MT/s")
                } else {
                    String::new()
                }
            ),
        );
    }
    line(
        "Anakart",
        format!("{} {}", s.board.maker, s.board.product)
            .trim()
            .to_string(),
    );
    for d in &s.disks {
        let kind = [d.bus.as_str(), d.media.as_str()]
            .iter()
            .filter(|x| !x.is_empty() && **x != "Unspecified")
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        line(
            "Disk",
            format!(
                "{} ({}{})",
                d.name,
                gb(d.size),
                if kind.is_empty() {
                    String::new()
                } else {
                    format!(", {kind}")
                }
            ),
        );
    }
    for m in &s.monitors {
        let inch = if m.inches > 0.0 {
            format!(" {:.0}\"", m.inches)
        } else {
            String::new()
        };
        line("Monitör", format!("{}{inch}", monitor_title(m)));
    }
    if let Some(g) = s.gpu.first().filter(|g| g.width > 0) {
        line(
            "Çözünürlük",
            format!("{}×{} @ {} Hz", g.width, g.height, g.hz),
        );
    }
    for d in &s.input {
        line(&d.kind, d.name.clone());
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"os":{"name":"Microsoft Windows 11 Pro","version":"24H2","build":"26100","computer":"MALIKANE","maker":"ASUS","model":"System Product Name","uptimeMin":1692},
      "cpu":[{"name":"AMD Ryzen 7 7800X3D 8-Core Processor","cores":8,"threads":16,"mhz":4201}],
      "gpu":[{"name":"Microsoft Remote Display Adapter","vram":0},{"name":"AMD Radeon(TM) Graphics","vram":536870912,"driver":"31.0","width":0},{"name":"NVIDIA GeForce RTX 4070","vram":12884901888,"driver":"32.0.15.6614","width":2560,"height":1440,"hz":165}],
      "ram":{"total":34359738368,"modules":[{"slot":"DIMM_A2","size":17179869184,"speed":6000,"maker":"04CD","part":"F5-6000J3038F16G","type":34},{"slot":"DIMM_B2","size":17179869184,"speed":6000,"maker":"G Skill Intl","part":"F5-6000J3038F16G","type":34}]},
      "board":{"maker":"ASUSTeK COMPUTER INC.","product":"TUF GAMING B650-PLUS WIFI","bios":"3024","biosDate":"2025-02-14"},
      "disks":[{"name":"Samsung SSD 990 PRO 2TB","size":2000398934016,"media":"SSD","bus":"NVMe"}],
      "volumes":[{"letter":"D:","label":"Oyunlar","size":1000202039296,"free":400000000000},{"letter":"C:","label":"","size":999000000000,"free":442000000000}],
      "monitors":[{"name":"LG ULTRAGEAR","maker":"GSM","year":2023,"inches":27.0}],
      "input":[{"kind":"Fare","name":"HID-compliant mouse"},{"kind":"Fare","name":"G502 HERO Gaming Mouse"},{"kind":"Klavye","name":"HID Keyboard Device"},{"kind":"Klavye","name":"HID Keyboard Device"}],
      "audio":[{"kind":"Çıkış","name":"Kulaklık (HyperX Cloud II)"},{"kind":"Çıkış","name":"Kulaklık (HyperX Cloud II)"}],
      "net":[{"name":"Ethernet","desc":"Realtek Gaming 2.5GbE Family Controller","speed":"1 Gbps"}]}"#;

    #[test]
    fn parses_and_tidies() {
        let s = parse(SAMPLE).unwrap();
        assert_eq!(s.cpu[0].name, "AMD Ryzen 7 7800X3D");
        assert_eq!(s.gpu.len(), 2, "uzak masaüstü sürücüsü atılır");
        assert_eq!(s.gpu[0].name, "NVIDIA GeForce RTX 4070");
        assert_eq!(s.gpu[1].name, "AMD Radeon Graphics");
        assert_eq!(s.ram.modules[0].maker, "G.Skill");
        assert_eq!(s.monitors[0].maker, "LG");
        // Fare: adı belli olan varsa "HID-compliant mouse" gizlenir; klavyenin tek kaydı genel, kalır.
        assert_eq!(
            s.input.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            ["G502 HERO Gaming Mouse", "HID Keyboard Device"]
        );
        assert_eq!(s.audio.len(), 1);
        assert_eq!(s.volumes[0].letter, "C:");
        let t = as_text(&s);
        assert!(
            t.contains("**İşlemci:** AMD Ryzen 7 7800X3D (8 çekirdek / 16 iş parçacığı)"),
            "{t}"
        );
        assert!(
            t.contains("**Bellek:** 32 GB (2×16 GB) DDR5 6000 MT/s"),
            "{t}"
        );
        assert!(t.contains("**Monitör:** LG ULTRAGEAR 27\""), "{t}");
        assert!(
            t.contains("**Ekran kartı:** NVIDIA GeForce RTX 4070 (12 GB)"),
            "{t}"
        );
    }

    #[test]
    fn reads_windows_powershell_51_shapes() {
        // 5.1: diziler {"value":[…],"Count":n}; tek elemanlı liste nesne olarak; null alanlar.
        let ps51 = r#"{"os":{"name":"Microsoft Windows 10 Home","version":null,"build":"19045","uptimeMin":5},
          "cpu":{"name":"Intel(R) Core(TM) i5-9400F CPU @ 2.90GHz","cores":6,"threads":6,"mhz":2904},
          "gpu":{"value":[{"name":"NVIDIA GeForce GTX 1660 SUPER","vram":6442450944}],"Count":1},
          "ram":{"total":17179869184,"modules":{"value":[{"slot":"ChannelA-DIMM0","size":8589934592,"speed":2666,"maker":"Kingston","part":"KHX2666C16/8G","type":26}],"Count":1}},
          "disks":null,"volumes":[],"monitors":{"value":[],"Count":0},"input":[],"audio":[],"net":[]}"#;
        let s = parse(ps51).unwrap();
        assert_eq!(s.cpu[0].name, "Intel Core i5-9400F");
        assert_eq!(s.gpu[0].vram, 6442450944);
        assert_eq!(s.ram.modules.len(), 1);
        assert!(s.disks.is_empty());
        assert!(parse("").is_err());
        assert!(parse("{}").is_err());
    }

    #[test]
    fn cpu_names() {
        assert_eq!(
            clean_cpu("Intel(R) Core(TM) i7-10700K CPU @ 3.80GHz"),
            "Intel Core i7-10700K"
        );
        assert_eq!(
            clean_cpu("AMD Ryzen 5 5600X 6-Core Processor             "),
            "AMD Ryzen 5 5600X"
        );
        assert_eq!(
            clean_cpu("13th Gen Intel(R) Core(TM) i5-13600KF"),
            "13th Gen Intel Core i5-13600KF"
        );
        assert_eq!(gb(2_000_398_934_016), "1,8 TB");
        assert_eq!(ram_type(34), "DDR5");
    }
}
