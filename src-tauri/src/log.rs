//! Küçük günlük dosyası: `GMID Tool\gmid.log`. Sorun çıkınca (pencere açılamadı, dosya yazılamadı…)
//! ne olduğu buradan okunabilir. 512 KB'ı geçince eskisi `gmid.old.log` olur.

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const MAX_BYTES: u64 = 512 * 1024;

static FILE: OnceLock<PathBuf> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

pub fn init(path: PathBuf) {
    let _ = FILE.set(path);
}

pub fn write(msg: &str) {
    eprintln!("{msg}");
    let Some(path) = FILE.get() else { return };
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = std::fs::rename(path, path.with_file_name("gmid.old.log"));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(
            f,
            "{} {msg}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        );
    }
}

/// `log!("Bir şey oldu: {e}")`
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => { $crate::log::write(&format!($($arg)*)) };
}
