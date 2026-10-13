//! Çalışan programların listesi (oyun algılama için). Yalnızca program adı ve yolu okunur;
//! oyunlara hiçbir şey enjekte edilmez (anti-cheat güvenli).

#[derive(Clone, Debug, PartialEq)]
pub struct Proc {
    pub pid: u32,
    /// Küçük harfli exe adı ("hades2.exe").
    pub name: String,
    /// Küçük harfli tam yol (okunamazsa boş: korumalı süreçler).
    pub path: String,
}

#[cfg(windows)]
pub fn list() -> Vec<Proc> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return out;
        };
        let mut e = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut ok = Process32FirstW(snap, &mut e).is_ok();
        while ok {
            let len = e
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(e.szExeFile.len());
            let name = String::from_utf16_lossy(&e.szExeFile[..len]).to_lowercase();
            let pid = e.th32ProcessID;
            let mut path = String::new();
            if pid > 4 {
                if let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                    let mut buf = [0u16; 1024];
                    let mut size = buf.len() as u32;
                    if QueryFullProcessImageNameW(
                        h,
                        PROCESS_NAME_WIN32,
                        windows::core::PWSTR(buf.as_mut_ptr()),
                        &mut size,
                    )
                    .is_ok()
                    {
                        path = String::from_utf16_lossy(&buf[..size as usize]).to_lowercase();
                    }
                    let _ = CloseHandle(h);
                }
            }
            out.push(Proc { pid, name, path });
            ok = Process32NextW(snap, &mut e).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    out
}

/// Süreç → üst süreç eşlemesi.
#[cfg(windows)]
pub fn parents() -> std::collections::HashMap<u32, u32> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    let mut out = std::collections::HashMap::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return out;
        };
        let mut e = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut ok = Process32FirstW(snap, &mut e).is_ok();
        while ok {
            out.insert(e.th32ProcessID, e.th32ParentProcessID);
            ok = Process32NextW(snap, &mut e).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    out
}

/// `pid`, `ancestor`ın alt süreci mi (en çok 6 kuşak).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn descends_from(
    parents: &std::collections::HashMap<u32, u32>,
    mut pid: u32,
    ancestor: u32,
) -> bool {
    for _ in 0..6 {
        match parents.get(&pid) {
            Some(&p) if p == ancestor => return true,
            Some(&p) if p != 0 && p != pid => pid = p,
            _ => return false,
        }
    }
    false
}

/// Geliştirme ortamı (Linux): /proc'tan.
#[cfg(not(windows))]
pub fn list() -> Vec<Proc> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("/proc") else {
        return out;
    };
    for e in rd.flatten() {
        let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let path = std::fs::read_link(e.path().join("exe"))
            .map(|p| p.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let name = path.rsplit('/').next().unwrap_or_default().to_string();
        if !name.is_empty() {
            out.push(Proc { pid, name, path });
        }
    }
    out
}

/// Ekranda görünen bir penceresi olan programlar: "Çalışan uygulamalardan oyun ekle" listesi için.
/// (pid, pencere başlığı)
#[cfg(windows)]
pub fn windowed() -> Vec<(u32, String)> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible,
    };
    unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = &mut *(lparam.0 as *mut Vec<(u32, String)>);
        if IsWindowVisible(hwnd).as_bool() {
            let len = GetWindowTextLengthW(hwnd);
            if len > 0 {
                let mut buf = vec![0u16; len as usize + 1];
                let n = GetWindowTextW(hwnd, &mut buf);
                let mut pid = 0u32;
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
                let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
                if !title.trim().is_empty() && !out.iter().any(|(p, _)| *p == pid) {
                    out.push((pid, title));
                }
            }
        }
        BOOL(1)
    }
    let mut out: Vec<(u32, String)> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut out as *mut _ as isize));
    }
    out
}

#[cfg(not(windows))]
pub fn windowed() -> Vec<(u32, String)> {
    Vec::new()
}

/// Ön plandaki pencere bütün ekranı kaplıyorsa (tam ekran ya da kenarlıksız tam ekran): (pid, başlık).
#[cfg(windows)]
pub fn foreground_fullscreen() -> Option<(u32, String)> {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }
        // Masaüstünün kendisi de "tam ekran" görünür.
        let mut cls = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut cls);
        let class = String::from_utf16_lossy(&cls[..n.max(0) as usize]);
        if class == "Progman" || class == "WorkerW" {
            return None;
        }
        let mut r = RECT::default();
        GetWindowRect(hwnd, &mut r).ok()?;
        let mon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(mon, &mut mi).as_bool() {
            return None;
        }
        let m = mi.rcMonitor;
        if r.left > m.left || r.top > m.top || r.right < m.right || r.bottom < m.bottom {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let mut buf = [0u16; 256];
        let n = GetWindowTextW(hwnd, &mut buf);
        Some((pid, String::from_utf16_lossy(&buf[..n.max(0) as usize])))
    }
}

#[cfg(not(windows))]
pub fn foreground_fullscreen() -> Option<(u32, String)> {
    None
}

/// Sürecin exe adı (küçük harf, "discord.exe"); korumalı süreçlerde `None`.
#[cfg(windows)]
pub fn exe_of(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            h,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
        .is_ok();
        let _ = CloseHandle(h);
        ok.then(|| {
            let path = String::from_utf16_lossy(&buf[..size as usize]).to_lowercase();
            path.rsplit('\\').next().unwrap_or(&path).to_string()
        })
    }
}

#[cfg(not(windows))]
pub fn exe_of(pid: u32) -> Option<String> {
    list().into_iter().find(|p| p.pid == pid).map(|p| p.name)
}

#[cfg(test)]
mod tree_tests {
    #[test]
    fn descendants() {
        // gmid (10) → webview tarayıcı (20) → ses süreci (30); başka (40 → 1)
        let p = [(20, 10), (30, 20), (40, 1), (10, 1)].into_iter().collect();
        assert!(super::descends_from(&p, 30, 10));
        assert!(super::descends_from(&p, 20, 10));
        assert!(!super::descends_from(&p, 40, 10));
        assert!(!super::descends_from(&p, 99, 10));
    }
}
