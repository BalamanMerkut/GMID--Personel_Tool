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
