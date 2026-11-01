//! İnternet bağlantıları için ortak ayar.
//!
//! Sertifikalar Windows'un kendi deposundan doğrulanır: antivirüslerin "HTTPS tarama" özelliği ya da
//! kurumsal ağlar bağlantıyı kendi sertifikalarıyla açtığında da indirmeler çalışsın. Windows'ta
//! tanımlı vekil sunucu (proxy) varsa o da kullanılır.

use std::time::Duration;
use ureq::tls::{RootCerts, TlsConfig};

pub fn agent(connect: Duration, body: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .timeout_connect(Some(connect))
        .timeout_recv_body(Some(body))
        .http_status_as_error(false)
        .user_agent(concat!("GMID-Tool/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// Büyük dosya yükleme (Sosyal'de klip paylaşma): gönderme yavaş bağlantıda uzun sürebilir, ama cevap
/// gelmezse (bağlantı yolda koptu) sonsuza dek beklenmez.
pub fn agent_upload() -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(120)))
        .timeout_recv_body(Some(Duration::from_secs(120)))
        .timeout_global(Some(Duration::from_secs(30 * 60)))
        .http_status_as_error(false)
        .user_agent(concat!("GMID-Tool/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// Yönlendirmeleri izlemeyen bağlantı: kısa bağlantının (GamerPower "open" adresleri) gittiği yeri öğrenmek için.
pub fn agent_no_redirect(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .timeout_global(Some(timeout))
        .max_redirects(0)
        .http_status_as_error(false)
        .user_agent(concat!("GMID-Tool/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}
