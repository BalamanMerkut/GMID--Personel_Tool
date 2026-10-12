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
}

#[derive(Default)]
pub struct Perf {
    /// Önceki (boşta, çekirdek, kullanıcı) süreleri.
    cpu_prev: Mutex<Option<(u64, u64, u64)>>,
    /// nvidia-smi bulunamadıysa bir daha denenmez.
    no_nvidia: std::sync::atomic::AtomicBool,
    #[cfg(windows)]
    pdh: Mutex<Option<win::GpuQuery>>,
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
    pub fn sample(&self, game_pid: Option<u32>) -> Sample {
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
        if s.gpu.is_none() {
            let mut q = self.pdh.lock().unwrap_or_else(|e| e.into_inner());
            if q.is_none() {
                *q = win::GpuQuery::open();
            }
            s.gpu = q.as_mut().and_then(|q| q.utilization());
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

    /// Windows'un "GPU Engine(*engtype_3D)" sayacı: bütün süreçlerin 3D motoru kullanımı toplamı.
    pub struct GpuQuery {
        query: PDH_HQUERY,
        counter: PDH_HCOUNTER,
    }

    // PDH tutamaçları iş parçacıkları arasında taşınabilir (Mutex içinde tek erişim).
    unsafe impl Send for GpuQuery {}

    impl GpuQuery {
        pub fn open() -> Option<Self> {
            unsafe {
                let mut query = PDH_HQUERY::default();
                if windows::Win32::System::Performance::PdhOpenQueryW(None, 0, &mut query) != 0 {
                    return None;
                }
                let mut counter = PDH_HCOUNTER::default();
                if PdhAddEnglishCounterW(
                    query,
                    w!("\\GPU Engine(*engtype_3D)\\Utilization Percentage"),
                    0,
                    &mut counter,
                ) != 0
                {
                    let _ = PdhCloseQuery(query);
                    return None;
                }
                let _ = PdhCollectQueryData(query);
                Some(Self { query, counter })
            }
        }

        pub fn utilization(&mut self) -> Option<f32> {
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }
                let mut size = 0u32;
                let mut count = 0u32;
                let _ = PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    None,
                );
                if size == 0 {
                    return None;
                }
                let mut buf = vec![0u8; size as usize];
                let items = buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
                if PdhGetFormattedCounterArrayW(
                    self.counter,
                    PDH_FMT_DOUBLE,
                    &mut size,
                    &mut count,
                    Some(items),
                ) != 0
                {
                    return None;
                }
                let total: f64 = (0..count as usize)
                    .map(|i| (*items.add(i)).FmtValue.Anonymous.doubleValue)
                    .sum();
                Some(total.clamp(0.0, 100.0) as f32)
            }
        }
    }

    impl Drop for GpuQuery {
        fn drop(&mut self) {
            unsafe {
                let _ = PdhCloseQuery(self.query);
            }
        }
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
