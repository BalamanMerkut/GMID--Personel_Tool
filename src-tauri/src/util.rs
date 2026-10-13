use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Şu an, Unix zamanı (milisaniye).
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Zamana göre sıralanabilen, çakışmayan kimlik (ileride eşitleme için de uygun).
pub fn new_id() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!(
        "{:011x}{:04x}{:04x}",
        now_ms(),
        COUNTER.fetch_add(1, Ordering::Relaxed) & 0xffff,
        nanos & 0xffff
    )
}

/// Windows'ta arka planda çalışan programlar için siyah konsol penceresi açılmasın.
pub fn hide_console(cmd: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}

/// Bir programı ve başlattığı alt programları (yt-dlp → ffmpeg) kapatır.
pub fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        let mut cmd = std::process::Command::new("taskkill");
        cmd.args(["/PID", &pid.to_string(), "/T", "/F"]);
        hide_console(&mut cmd);
        let _ = cmd.status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("pkill")
            .args(["-TERM", "-P", &pid.to_string()])
            .status();
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
    }
}

/// GMID'in PowerShell betiklerinin durduğu klasör (%LOCALAPPDATA%\com.merkutstudios.gmidtool\Betikler).
/// Betikler gizli ya da şifreli komut olarak verilmez: burada okunabilir `.ps1` dosyaları olarak durur.
pub fn scripts_dir() -> std::path::PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("com.merkutstudios.gmidtool")
        .join("Betikler")
}

/// Betiği `Betikler\<ad>.ps1` olarak yazar (UTF-8 BOM'lu: Windows PowerShell 5.1 Türkçe harfleri doğru okusun).
pub fn write_script(name: &str, body: &str) -> Result<std::path::PathBuf, String> {
    let dir = scripts_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Betik klasörü açılamadı: {e}"))?;
    let path = dir.join(format!("{name}.ps1"));
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(body.replace("\r\n", "\n").replace('\n', "\r\n").as_bytes());
    std::fs::write(&path, bytes).map_err(|e| format!("Betik yazılamadı: {e}"))?;
    Ok(path)
}

/// Betik dosyasını çalıştıran PowerShell argümanları (yerel dosya: RemoteSigned yeterli).
pub fn ps_file_args(path: &std::path::Path) -> Vec<std::ffi::OsString> {
    let mut v: Vec<std::ffi::OsString> = [
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "RemoteSigned",
        "-File",
    ]
    .iter()
    .map(Into::into)
    .collect();
    v.push(path.as_os_str().to_owned());
    v
}

/// Betiği dosyaya yazıp PowerShell ile çalıştırır, bitmesini bekler; çıkış kodu.
pub fn run_ps(name: &str, body: &str) -> Result<u32, String> {
    let path = write_script(name, body)?;
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(ps_file_args(&path))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    hide_console(&mut cmd);
    let st = cmd
        .status()
        .map_err(|e| format!("PowerShell açılamadı: {e}"))?;
    Ok(st.code().unwrap_or(-1) as u32)
}

/// Yönetici izniyle (UAC) betik çalıştırır: betik `Betikler` klasörüne yazılır, Windows izin penceresi
/// onu çalıştıracak komutu gösterir; bitmesi beklenir, çıkış kodu döner.
#[cfg(windows)]
pub fn run_elevated_ps(name: &str, body: &str) -> Result<u32, String> {
    use windows::core::{w, HSTRING};
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    let path = write_script(name, body)?;
    let params = HSTRING::from(format!(
        "-NoProfile -NonInteractive -ExecutionPolicy RemoteSigned -File \"{}\"",
        path.display()
    ));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: w!("powershell.exe"),
        lpParameters: windows::core::PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    let mut code = 0u32;
    unsafe {
        ShellExecuteExW(&mut info).map_err(|_| "Yönetici izni verilmedi".to_string())?;
        if !info.hProcess.is_invalid() {
            WaitForSingleObject(info.hProcess, INFINITE);
            let _ = GetExitCodeProcess(info.hProcess, &mut code);
            let _ = CloseHandle(info.hProcess);
        }
    }
    Ok(code)
}

#[cfg(not(windows))]
pub fn run_elevated_ps(_name: &str, _body: &str) -> Result<u32, String> {
    Err("Yalnızca Windows'ta".into())
}

#[cfg(test)]
mod script_tests {
    use super::*;

    #[test]
    fn scripts_are_readable_files() {
        let p = write_script("gmid-test-betik", "Write-Output 'Türkçe şğı'\n$x = 1\n").unwrap();
        let b = std::fs::read(&p).unwrap();
        assert_eq!(&b[..3], &[0xEF, 0xBB, 0xBF]);
        let t = String::from_utf8(b[3..].to_vec()).unwrap();
        assert_eq!(t, "Write-Output 'Türkçe şğı'\r\n$x = 1\r\n");
        let args = ps_file_args(&p);
        assert_eq!(args[3], "RemoteSigned");
        assert_eq!(args[5], p.as_os_str());
        let _ = std::fs::remove_file(p);
    }
}
