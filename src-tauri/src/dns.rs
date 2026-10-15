//! DNS seçici: bilinen hızlı/güvenilir DNS'ler arasında seçim, yanıt sürelerinin ölçülmesi ve tek tıkla
//! "otomatik"e (internet sağlayıcısının DNS'i) dönüş. Ayar bütün bilgisayara, internete çıkan bağdaştırıcılara
//! uygulanır (Windows'un kendi DNS ayarı; yeniden başlatınca da kalır). Yönetici izni ister.

use serde::Serialize;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Serialize, Debug)]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub detail: &'static str,
    pub servers: &'static [&'static str],
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "cloudflare",
        name: "Cloudflare",
        detail: "Çoğu yerde en hızlılardan; sorgu kaydı tutmaz.",
        servers: &[
            "1.1.1.1",
            "1.0.0.1",
            "2606:4700:4700::1111",
            "2606:4700:4700::1001",
        ],
    },
    Preset {
        id: "google",
        name: "Google",
        detail: "Hızlı ve çok kararlı.",
        servers: &[
            "8.8.8.8",
            "8.8.4.4",
            "2001:4860:4860::8888",
            "2001:4860:4860::8844",
        ],
    },
    Preset {
        id: "quad9",
        name: "Quad9",
        detail: "Bilinen zararlı (virüs, dolandırıcılık) siteleri açmaz.",
        servers: &["9.9.9.9", "149.112.112.112", "2620:fe::fe", "2620:fe::9"],
    },
    Preset {
        id: "adguard",
        name: "AdGuard",
        detail: "Reklam ve izleyici sitelerini engeller.",
        servers: &[
            "94.140.14.14",
            "94.140.15.15",
            "2a10:50c0::ad1:ff",
            "2a10:50c0::ad2:ff",
        ],
    },
    Preset {
        id: "cloudflare-family",
        name: "Cloudflare Aile",
        detail: "Zararlı ve yetişkin içerikli siteleri açmaz.",
        servers: &[
            "1.1.1.3",
            "1.0.0.3",
            "2606:4700:4700::1113",
            "2606:4700:4700::1003",
        ],
    },
];

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Şu an kullanılan DNS'ler (internete çıkan bağdaştırıcılarınki).
    pub servers: Vec<String>,
    /// Bağdaştırıcı adları ("Ethernet", "Wi-Fi").
    pub adapters: Vec<String>,
    /// Bilinen bir DNS ise kimliği; değilse `None` (otomatik ya da başka).
    pub preset: Option<String>,
    /// Seçenekler (arayüz listesi).
    pub presets: &'static [Preset],
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bench {
    pub id: String,
    /// En iyi yanıt süresi (ms); yanıt gelmediyse `None`.
    pub ms: Option<u32>,
}

/// Kullanılan DNS'lerden bilinen bir seçeneği tanır (yalnız IPv4 adreslerine bakılır).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn match_preset(servers: &[String]) -> Option<&'static str> {
    let v4: Vec<&str> = servers
        .iter()
        .map(String::as_str)
        .filter(|s| s.parse::<std::net::Ipv4Addr>().is_ok())
        .collect();
    let first = *v4.first()?;
    PRESETS
        .iter()
        .find(|p| p.servers.iter().take(2).any(|s| *s == first))
        .map(|p| p.id)
}

/// "www.google.com" için A kaydı soran en küçük DNS sorgusu.
fn query(id: u16, name: &str) -> Vec<u8> {
    let mut q = Vec::with_capacity(32);
    q.extend_from_slice(&id.to_be_bytes());
    q.extend_from_slice(&[0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
    for part in name.split('.') {
        q.push(part.len() as u8);
        q.extend_from_slice(part.as_bytes());
    }
    q.extend_from_slice(&[0, 0, 1, 0, 1]);
    q
}

/// Bir DNS sunucusunun yanıt süresi: birkaç sorgunun en iyisi (ms).
pub fn measure(server: &str) -> Option<u32> {
    let ip: IpAddr = server.parse().ok()?;
    let bind = if ip.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
    let sock = UdpSocket::bind(bind).ok()?;
    sock.set_read_timeout(Some(Duration::from_millis(1200)))
        .ok()?;
    sock.connect(SocketAddr::new(ip, 53)).ok()?;
    let mut best: Option<u32> = None;
    let mut buf = [0u8; 512];
    for (i, name) in [
        "www.google.com",
        "discord.com",
        "store.steampowered.com",
        "www.google.com",
    ]
    .iter()
    .enumerate()
    {
        let id = 0x4d00 + i as u16;
        let t = Instant::now();
        if sock.send(&query(id, name)).is_err() {
            continue;
        }
        // Gecikmiş eski yanıtlar atlanır.
        while let Ok(n) = sock.recv(&mut buf) {
            if n >= 2 && u16::from_be_bytes([buf[0], buf[1]]) == id {
                let ms = t.elapsed().as_millis() as u32;
                best = Some(best.map_or(ms, |b| b.min(ms)));
                break;
            }
        }
    }
    best
}

/// Bütün seçeneklerin ve şu anki DNS'in yanıt süresi (aynı anda ölçülür).
pub fn bench(current: Option<String>) -> Vec<Bench> {
    let mut jobs: Vec<(String, String)> = PRESETS
        .iter()
        .map(|p| (p.id.to_string(), p.servers[0].to_string()))
        .collect();
    if let Some(c) = current {
        jobs.push(("current".into(), c));
    }
    std::thread::scope(|s| {
        let handles: Vec<_> = jobs
            .iter()
            .map(|(id, ip)| {
                s.spawn(move || Bench {
                    id: id.clone(),
                    ms: measure(ip),
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    })
}

/// Seçilen DNS'i internete çıkan bağdaştırıcılara uygulayan betik (`None`: otomatiğe dön).
pub fn apply_script(preset: Option<&Preset>) -> String {
    let set = match preset {
        Some(p) => format!(
            "Set-DnsClientServerAddress -InterfaceIndex $i -ServerAddresses @({})",
            p.servers
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(",")
        ),
        None => "Set-DnsClientServerAddress -InterfaceIndex $i -ResetServerAddresses".to_string(),
    };
    format!(
        "# GMID Tool: DNS ayarı (yalnız internete çıkan bağdaştırıcılar)\n\
         $ErrorActionPreference = 'Stop'\n\
         try {{\n\
         \x20 $ifs = Get-NetIPConfiguration | Where-Object {{ $_.IPv4DefaultGateway -ne $null -and $_.NetAdapter.Status -eq 'Up' }} | ForEach-Object {{ $_.InterfaceIndex }}\n\
         \x20 if (-not $ifs) {{ exit 3 }}\n\
         \x20 foreach ($i in $ifs) {{ {set} }}\n\
         \x20 Clear-DnsClientCache\n\
         \x20 exit 0\n\
         }} catch {{ exit 2 }}\n"
    )
}

/// DNS'i değiştirir (Windows izin penceresi açılır).
pub fn set(id: &str) -> Result<(), String> {
    let preset = match id {
        "auto" => None,
        _ => Some(
            PRESETS
                .iter()
                .find(|p| p.id == id)
                .ok_or("Bilinmeyen DNS")?,
        ),
    };
    match crate::util::run_elevated_ps("gmid-dns", &apply_script(preset))? {
        0 => Ok(()),
        3 => Err("İnternete bağlı bir ağ bağdaştırıcısı bulunamadı.".into()),
        c => Err(format!("DNS değiştirilemedi (kod {c}).")),
    }
}

/// Şu an kullanılan DNS'ler.
#[cfg(windows)]
pub fn status() -> Status {
    let (adapters, servers) = win::current();
    let preset = match_preset(&servers).map(String::from);
    Status {
        servers,
        adapters,
        preset,
        presets: PRESETS,
    }
}

#[cfg(not(windows))]
pub fn status() -> Status {
    Status {
        presets: PRESETS,
        ..Default::default()
    }
}

#[cfg(windows)]
mod win {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    use windows::Win32::Networking::WinSock::{
        AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6, SOCKET_ADDRESS,
    };

    fn ip(a: &SOCKET_ADDRESS) -> Option<String> {
        if a.lpSockaddr.is_null() {
            return None;
        }
        unsafe {
            match (*a.lpSockaddr).sa_family {
                AF_INET => {
                    let s = &*(a.lpSockaddr as *const SOCKADDR_IN);
                    let b = s.sin_addr.S_un.S_addr.to_ne_bytes();
                    Some(std::net::Ipv4Addr::new(b[0], b[1], b[2], b[3]).to_string())
                }
                AF_INET6 => {
                    let s = &*(a.lpSockaddr as *const SOCKADDR_IN6);
                    let b = s.sin6_addr.u.Byte;
                    let v6 = std::net::Ipv6Addr::from(b);
                    // Bağdaştırıcıya özel (fec0::) eski varsayılanlar gösterilmez.
                    (!v6.to_string().starts_with("fec0:")).then(|| v6.to_string())
                }
                _ => None,
            }
        }
    }

    /// İnternete çıkan (çalışan ve ağ geçidi olan) bağdaştırıcıların adları ve DNS'leri.
    pub fn current() -> (Vec<String>, Vec<String>) {
        let mut size: u32 = 32 * 1024;
        let mut buf: Vec<u8> = vec![0; size as usize];
        let mut adapters = Vec::new();
        let mut servers: Vec<String> = Vec::new();
        unsafe {
            let mut rc = GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                GAA_FLAG_INCLUDE_GATEWAYS,
                None,
                Some(buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut size,
            );
            if rc == 111 {
                // ERROR_BUFFER_OVERFLOW: istenen boyutla yeniden.
                buf = vec![0; size as usize];
                rc = GetAdaptersAddresses(
                    AF_UNSPEC.0 as u32,
                    GAA_FLAG_INCLUDE_GATEWAYS,
                    None,
                    Some(buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                    &mut size,
                );
            }
            if rc != 0 {
                return (adapters, servers);
            }
            let mut p = buf.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
            while !p.is_null() {
                let a = &*p;
                if a.OperStatus == IfOperStatusUp && !a.FirstGatewayAddress.is_null() {
                    adapters.push(a.FriendlyName.to_string().unwrap_or_default());
                    let mut d = a.FirstDnsServerAddress;
                    while !d.is_null() {
                        if let Some(s) = ip(&(*d).Address) {
                            if !servers.contains(&s) {
                                servers.push(s);
                            }
                        }
                        d = (*d).Next;
                    }
                }
                p = a.Next;
            }
        }
        (adapters, servers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_recognised() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            match_preset(&s(&["1.1.1.1", "1.0.0.1"])),
            Some("cloudflare")
        );
        assert_eq!(
            match_preset(&s(&["2606:4700:4700::1111", "8.8.8.8"])),
            Some("google")
        );
        assert_eq!(match_preset(&s(&["1.1.1.3"])), Some("cloudflare-family"));
        assert_eq!(match_preset(&s(&["192.168.1.1"])), None);
        assert_eq!(match_preset(&[]), None);
    }

    #[test]
    fn query_packet_shape() {
        let q = query(0x1234, "discord.com");
        assert_eq!(&q[..2], &[0x12, 0x34]);
        assert_eq!(q[12], 7);
        assert_eq!(&q[13..20], b"discord");
        assert_eq!(&q[q.len() - 5..], &[0, 0, 1, 0, 1]);
    }

    #[test]
    fn scripts_only_touch_connected_adapters() {
        let s = apply_script(Some(&PRESETS[0]));
        assert!(s.contains("IPv4DefaultGateway -ne $null"));
        assert!(s.contains("'1.1.1.1','1.0.0.1'"));
        assert!(s.contains("Clear-DnsClientCache"));
        assert!(apply_script(None).contains("-ResetServerAddresses"));
    }
}
