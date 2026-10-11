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
