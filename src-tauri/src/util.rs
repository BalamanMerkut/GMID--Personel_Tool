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
