//! Oyun İçi Panel için anlık performans: FPS, işlemci, RAM, ekran kartı (yük, sıcaklık, bellek).
//! Oyuna hiçbir şey enjekte edilmez:
//! - İşlemci: `GetSystemTimes` farkı; RAM: `GlobalMemoryStatusEx`.
//! - Ekran kartı: NVIDIA'da sürücüyle gelen `nvidia-smi` (yük, sıcaklık, bellek); diğerlerinde Windows'un
//!   "GPU Engine" sayacı (yük; sıcaklık yok).
//! - FPS: Windows yönetici izni olmadan FPS ölçtürmez. RivaTuner Statistics Server (RTSS, MSI Afterburner ile gelir)
//!   açıksa onun paylaşılan belleğinden okunur.

// Windows dışında (geliştirme) yalnız ayrıştırıcılar testlerde kullanılır.
#![cfg_attr(not(windows), allow(dead_code))]

use serde::Serialize;
use std::sync::Mutex;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub cpu: Option<f32>,
    pub ram_used: Option<u64>,
    pub ram_total: Option<u64>,
    pub gpu: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub vram_used: Option<u64>,
    pub vram_total: Option<u64>,
    pub gpu_name: Option<String>,
    pub fps: Option<f32>,
    /// FPS'in ölçüldüğü programın adı (RTSS'in gördüğü).
    pub fps_app: Option<String>,
    /// RTSS çalışmıyorsa `false` (arayüz ipucu gösterir).
    pub rtss: bool,
    /// FPS nereden: rtss | presentmon
    pub fps_source: Option<&'static str>,
    /// PresentMon'un durumu (FPS yoksa arayüz ne yapılacağını söyler).
    pub fps_state: crate::fps::State,
    /// FPS neden yok (arayüzde görünür).
    pub fps_note: Option<String>,
    /// ACPI sıcaklık bölgesi (°C, yaklaşık; her bilgisayarda yok).
    pub cpu_temp: Option<f32>,
    /// İşlemcinin şu anki saat hızı (MHz).
    pub cpu_mhz: Option<f32>,
    /// Disk etkinliği (%).
    pub disk: Option<f32>,
    /// Ağ: indirme / yükleme (bayt/sn).
    pub net_rx: Option<f64>,
    pub net_tx: Option<f64>,
}

#[derive(Default)]
pub struct Perf {
    /// RTSS yokken FPS.
    pub presentmon: crate::fps::PresentMon,
    /// Önceki (boşta, çekirdek, kullanıcı) süreleri.
    cpu_prev: Mutex<Option<(u64, u64, u64)>>,
    /// nvidia-smi bulunamadıysa bir daha denenmez.
    no_nvidia: std::sync::atomic::AtomicBool,
    #[cfg(windows)]
    pdh: Mutex<Option<win::Pdh>>,
}

/// `nvidia-smi --query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total,name --format=csv,noheader,nounits`
pub fn parse_nvidia_smi(out: &str) -> Option<(f32, f32, u64, u64, String)> {
    let line = out.lines().find(|l| !l.trim().is_empty())?;
    let f: Vec<&str> = line.split(',').map(str::trim).collect();
    if f.len() < 5 {
        return None;
    }
    Some((
        f[0].parse().ok()?,
        f[1].parse().ok()?,
        f[2].parse::<u64>().ok()? * 1024 * 1024,
        f[3].parse::<u64>().ok()? * 1024 * 1024,
        f[4].to_string(),
    ))
}

/// RTSS paylaşılan belleğindeki bir uygulama kaydı.
#[derive(Debug, Clone, PartialEq)]
pub struct RtssApp {
    pub pid: u32,
    pub name: String,
    pub time0: u32,
    pub time1: u32,
    pub frames: u32,
}

impl RtssApp {
    pub fn fps(&self) -> Option<f32> {
        let dt = self.time1.wrapping_sub(self.time0);
        (dt > 0 && self.frames > 0).then(|| self.frames as f32 * 1000.0 / dt as f32)
    }
}

fn u32_at(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?))
}

/// RTSS_SHARED_MEMORY (RTSSSharedMemory.h): imza 'RTSS', sürüm ≥ 2.0, uygulama dizisi.
/// Uygulama kaydı: pid (0), ad (4, 260 bayt), bayraklar (264), time0 (268), time1 (272), kare (276).
pub fn parse_rtss(mem: &[u8]) -> Vec<RtssApp> {
    const SIGNATURE: u32 = 0x5254_5353; // "RTSS"
    let (Some(sig), Some(ver)) = (u32_at(mem, 0), u32_at(mem, 4)) else {
        return Vec::new();
    };
    if sig != SIGNATURE || ver < 0x0002_0000 {
        return Vec::new();
    }
    let (Some(size), Some(offset), Some(count)) =
        (u32_at(mem, 8), u32_at(mem, 12), u32_at(mem, 16))
    else {
        return Vec::new();
    };
    let (size, offset) = (size as usize, offset as usize);
    if size < 280 {
        return Vec::new();
    }
    (0..count.min(256) as usize)
        .filter_map(|i| {
            let b = mem.get(offset + i * size..offset + (i + 1) * size)?;
            let pid = u32_at(b, 0)?;
            if pid == 0 {
                return None;
            }
            let raw = b.get(4..264)?;
            let end = raw.iter().position(|c| *c == 0).unwrap_or(raw.len());
            let path = String::from_utf8_lossy(&raw[..end]).into_owned();
            let name = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string();
            Some(RtssApp {
                pid,
                name,
                time0: u32_at(b, 268)?,
                time1: u32_at(b, 272)?,
                frames: u32_at(b, 276)?,
            })
        })
        .collect()
}

/// Önce verilen sürecin (oyun) kaydı; yoksa en son kare çizen.
pub fn pick_rtss(apps: &[RtssApp], pid: Option<u32>) -> Option<&RtssApp> {
    pid.and_then(|p| apps.iter().find(|a| a.pid == p && a.fps().is_some()))
        .or_else(|| {
            apps.iter()
                .filter(|a| a.fps().is_some())
                .max_by_key(|a| a.time1)
        })
}

impl Perf {
    /// `game_pid`: panel açılmadan önceki ön plan penceresinin süreci.
    pub fn sample(&self, game_pid: Option<u32>, root: &std::path::Path) -> Sample {
        let mut s = Sample::default();
        #[cfg(windows)]
        {
            if let Some((idle, kernel, user)) = win::system_times() {
                let mut prev = self.cpu_prev.lock().unwrap_or_else(|e| e.into_inner());
                if let Some((pi, pk, pu)) = *prev {
                    let total = (kernel - pk) + (user - pu);
                    if total > 0 {
                        s.cpu = Some(
                            ((total - (idle - pi)) as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
                                as f32,
                        );
                    }
                }
                *prev = Some((idle, kernel, user));
            }
            if let Some((used, total)) = win::memory() {
                s.ram_used = Some(used);
                s.ram_total = Some(total);
            }
            let apps = win::rtss_apps();
            s.rtss = apps.is_some();
            if let Some(app) = apps.as_deref().and_then(|a| pick_rtss(a, game_pid)) {
                s.fps = app.fps();
                s.fps_app = Some(app.name.clone());
                s.fps_source = Some("rtss");
            }
        }
        if s.fps.is_none() {
            if let Some((fps, app)) = self.presentmon.sample(root, game_pid) {
                s.fps = Some(fps);
                s.fps_app = Some(app);
                s.fps_source = Some("presentmon");
            }
            let st = self.presentmon.status(root, false);
            s.fps_state = st.state;
            s.fps_note = st.message;
            if s.fps.is_none()
                && s.fps_note.is_none()
                && self
                    .presentmon
                    .running_for()
                    .is_some_and(|d| d > std::time::Duration::from_secs(4))
            {
                s.fps_note = Some(
                    "Oyundan kare gelmiyor: oyun küçülmüş ya da duraklatılmış olabilir".into(),
                );
            }
        }
        let _ = game_pid;
        use std::sync::atomic::Ordering;
        if !self.no_nvidia.load(Ordering::Relaxed) {
            match nvidia_smi() {
                Some((gpu, temp, vu, vt, name)) => {
                    s.gpu = Some(gpu);
                    s.gpu_temp = Some(temp);
                    s.vram_used = Some(vu);
                    s.vram_total = Some(vt);
                    s.gpu_name = Some(name);
                }
                None => self.no_nvidia.store(true, Ordering::Relaxed),
            }
        }
        #[cfg(windows)]
        {
            let mut q = self.pdh.lock().unwrap_or_else(|e| e.into_inner());
            if q.is_none() {
                *q = win::Pdh::open();
            }
            if let Some(x) = q.as_mut().map(|q| q.sample()) {
                if s.gpu.is_none() {
                    s.gpu = x.gpu;
                }
                s.cpu_temp = x.temp;
                s.cpu_mhz = x.mhz;
                s.disk = x.disk;
                s.net_rx = x.rx;
                s.net_tx = x.tx;
            }
        }
        s
    }
}

fn nvidia_smi() -> Option<(f32, f32, u64, u64, String)> {
    let mut cmd = std::process::Command::new("nvidia-smi");
    cmd.args([
        "--query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total,name",
        "--format=csv,noheader,nounits",
    ]);
    crate::util::hide_console(&mut cmd);
    let o = cmd.output().ok()?;
    if !o.status.success() {
        return None;
    }
    parse_nvidia_smi(&String::from_utf8_lossy(&o.stdout))
}

#[cfg(windows)]
mod win {
    use super::{parse_rtss, RtssApp};
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::Memory::{
        MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, VirtualQuery, FILE_MAP_READ,
        MEMORY_BASIC_INFORMATION,
    };
    use windows::Win32::System::Performance::{
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
    };
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows::Win32::System::Threading::GetSystemTimes;

    fn ft(f: FILETIME) -> u64 {
        ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
    }

    pub fn system_times() -> Option<(u64, u64, u64)> {
        let (mut i, mut k, mut u) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        unsafe { GetSystemTimes(Some(&mut i), Some(&mut k), Some(&mut u)).ok()? };
        // Çekirdek süresi boşta süresini de içerir.
        Some((ft(i), ft(k), ft(u)))
    }

    pub fn memory() -> Option<(u64, u64)> {
        let mut m = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        unsafe { GlobalMemoryStatusEx(&mut m).ok()? };
        Some((m.ullTotalPhys - m.ullAvailPhys, m.ullTotalPhys))
    }

    /// RTSS çalışmıyorsa `None`.
    pub fn rtss_apps() -> Option<Vec<RtssApp>> {
        unsafe {
            let h = OpenFileMappingW(FILE_MAP_READ.0, false, w!("RTSSSharedMemoryV2")).ok()?;
            let view = MapViewOfFile(h, FILE_MAP_READ, 0, 0, 0);
            let result = if view.Value.is_null() {
                None
            } else {
                let mut info = MEMORY_BASIC_INFORMATION::default();
                let n = VirtualQuery(
                    Some(view.Value),
                    &mut info,
                    std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
                );
                let len = if n == 0 { 0 } else { info.RegionSize };
                let bytes = std::slice::from_raw_parts(view.Value as *const u8, len);
                let apps = parse_rtss(bytes);
                let _ = UnmapViewOfFile(view);
                Some(apps)
            };
            let _ = CloseHandle(h);
            result
        }
    }

    /// Windows performans sayaçları (PDH): ekran kartı 3D kullanımı, ACPI sıcaklığı, işlemci saat hızı, disk ve
    /// ağ. Bulunamayan sayaç atlanır.
    pub struct Pdh {
        query: PDH_HQUERY,
        gpu: Option<PDH_HCOUNTER>,
        temp: Option<PDH_HCOUNTER>,
        perf: Option<PDH_HCOUNTER>,
        idle: Option<PDH_HCOUNTER>,
        rx: Option<PDH_HCOUNTER>,
        tx: Option<PDH_HCOUNTER>,
        /// İşlemcinin temel saat hızı (MHz, kayıt defterinden).
        base_mhz: Option<f64>,
    }

    // PDH tutamaçları iş parçacıkları arasında taşınabilir (Mutex içinde tek erişim).
    unsafe impl Send for Pdh {}

    #[derive(Default, Clone, Copy)]
    pub struct Extra {
        pub gpu: Option<f32>,
        pub temp: Option<f32>,
        pub mhz: Option<f32>,
        pub disk: Option<f32>,
        pub rx: Option<f64>,
        pub tx: Option<f64>,
    }

    impl Pdh {
        pub fn open() -> Option<Self> {
            unsafe {
                let mut query = PDH_HQUERY::default();
                if windows::Win32::System::Performance::PdhOpenQueryW(None, 0, &mut query) != 0 {
                    return None;
                }
                let add = |path: windows::core::PCWSTR| {
                    let mut c = PDH_HCOUNTER::default();
                    (PdhAddEnglishCounterW(query, path, 0, &mut c) == 0).then_some(c)
                };
                let me = Self {
                    gpu: add(w!("\\GPU Engine(*engtype_3D)\\Utilization Percentage")),
                    temp: add(w!("\\Thermal Zone Information(*)\\Temperature")),
                    perf: add(w!(
                        "\\Processor Information(_Total)\\% Processor Performance"
                    )),
                    idle: add(w!("\\PhysicalDisk(_Total)\\% Idle Time")),
                    rx: add(w!("\\Network Interface(*)\\Bytes Received/sec")),
                    tx: add(w!("\\Network Interface(*)\\Bytes Sent/sec")),
                    base_mhz: base_mhz(),
                    query,
                };
                let _ = PdhCollectQueryData(query);
                Some(me)
            }
        }

        fn values(&self, c: Option<PDH_HCOUNTER>) -> Option<Vec<f64>> {
            let c = c?;
            unsafe {
                let mut size = 0u32;
                let mut count = 0u32;
                let _ =
                    PdhGetFormattedCounterArrayW(c, PDH_FMT_DOUBLE, &mut size, &mut count, None);
                if size == 0 {
                    return None;
                }
                let mut buf = vec![0u8; size as usize];
                let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
                if PdhGetFormattedCounterArrayW(
                    c,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    Some(items),
                ) != 0
                {
                    return None;
                }
                Some(
                    (0..count as usize)
                        .map(|i| (*items.add(i)).FmtValue.Anonymous.doubleValue)
                        .collect(),
                )
            }
        }

        pub fn sample(&mut self) -> Extra {
            if unsafe { PdhCollectQueryData(self.query) } != 0 {
                return Extra::default();
            }
            let sum = |v: Vec<f64>| v.iter().sum::<f64>();
            Extra {
                gpu: self
                    .values(self.gpu)
                    .map(|v| sum(v).clamp(0.0, 100.0) as f32),
                // Kelvin → °C; en sıcak bölge. Uydurma sabit değerler (bazı anakartlar hep 27,8 der) ayıklanır.
                temp: self
                    .values(self.temp)
                    .and_then(|v| v.into_iter().map(|k| k - 273.15).reduce(f64::max))
                    .filter(|c| (30.0..115.0).contains(c))
                    .map(|c| c as f32),
                mhz: self
                    .values(self.perf)
                    .and_then(|v| v.first().copied())
                    .zip(self.base_mhz)
                    .map(|(p, base)| (p / 100.0 * base) as f32)
                    .filter(|m| *m > 100.0),
                disk: self
                    .values(self.idle)
                    .and_then(|v| v.first().copied())
                    .map(|idle| (100.0 - idle).clamp(0.0, 100.0) as f32),
                rx: self.values(self.rx).map(sum),
                tx: self.values(self.tx).map(sum),
            }
        }
    }

    impl Drop for Pdh {
        fn drop(&mut self) {
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
        }
    }

    /// İşlemcinin temel saat hızı: HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0 → ~MHz.
    fn base_mhz() -> Option<f64> {
        use windows::Win32::System::Registry::{
            RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD,
        };
        let mut v = 0u32;
        let mut size = 4u32;
        let r = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                w!("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0"),
                w!("~MHz"),
                RRF_RT_REG_DWORD,
                None,
                Some(&mut v as *mut u32 as *mut _),
                Some(&mut size),
            )
        };
        (r.is_ok() && v > 0).then_some(v as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nvidia_smi_line() {
        assert_eq!(
            parse_nvidia_smi("87, 67, 6120, 12282, NVIDIA GeForce RTX 4070\n"),
            Some((
                87.0,
                67.0,
                6120 * 1048576,
                12282 * 1048576,
                "NVIDIA GeForce RTX 4070".into()
            ))
        );
        assert_eq!(parse_nvidia_smi(""), None);
        assert_eq!(parse_nvidia_smi("[N/A], 40, 1, 2, X"), None);
    }

    #[test]
    fn rtss_shared_memory_is_read() {
        // Başlık + 2 kayıt (kayıt boyu 300).
        let entry = 300usize;
        let offset = 64usize;
        let mut mem = vec![0u8; offset + 2 * entry];
        let put = |m: &mut Vec<u8>, o: usize, v: u32| m[o..o + 4].copy_from_slice(&v.to_le_bytes());
        put(&mut mem, 0, 0x5254_5353);
        put(&mut mem, 4, 0x0002_0013);
        put(&mut mem, 8, entry as u32);
        put(&mut mem, 12, offset as u32);
        put(&mut mem, 16, 2);
        let app =
            |m: &mut Vec<u8>, i: usize, pid: u32, name: &str, t0: u32, t1: u32, frames: u32| {
                let b = offset + i * entry;
                put(m, b, pid);
                m[b + 4..b + 4 + name.len()].copy_from_slice(name.as_bytes());
                put(m, b + 268, t0);
                put(m, b + 272, t1);
                put(m, b + 276, frames);
            };
        app(
            &mut mem,
            0,
            1111,
            "C:\\Games\\VALORANT-Win64-Shipping.exe",
            10_000,
            11_000,
            240,
        );
        app(
            &mut mem,
            1,
            2222,
            "C:\\Windows\\explorer.exe",
            50_000,
            51_000,
            60,
        );
        let apps = parse_rtss(&mem);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].name, "VALORANT-Win64-Shipping.exe");
        assert_eq!(apps[0].fps(), Some(240.0));
        // Oyunun süreci biliniyorsa o; bilinmiyorsa en son kare çizen.
        assert_eq!(pick_rtss(&apps, Some(1111)).unwrap().pid, 1111);
        assert_eq!(pick_rtss(&apps, None).unwrap().pid, 2222);
        mem[0] = 0;
        assert!(parse_rtss(&mem).is_empty());
    }
}
