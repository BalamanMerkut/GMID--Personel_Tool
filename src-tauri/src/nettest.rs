//! Bağlantı testi: "lag bende mi, sunucuda mı?"
//!
//! Aynı anda birkaç yere ping atılır ve sonuç yan yana konur:
//! - **Ev ağı:** modem (varsayılan ağ geçidi). Wi-Fi sorunları burada görünür.
//! - **İnternet:** Cloudflare ve Google (dünyanın en yakın sunucuları; sağlayıcı sorunu burada görünür).
//! - **GMID sunucuları:** topluluk listesindeki oyun sunucuları (ping'e kapalıysa oyunun kendi sorgusuyla).
//! - **Steam:** CS2 gibi Valve oyunlarının yönlendirme noktalarından en yakın üçü.
//!
//! Ping Windows'un kendi ICMP işleviyle atılır (yönetici izni gerekmez). Ortalama, en iyi/en kötü,
//! titreme (art arda iki ping arasındaki farkın ortalaması) ve kayıp hesaplanır; sonuçtan kısa bir teşhis çıkar.

use crate::db::Db;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, ToSocketAddrs};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const STEAM_SDR: &str = "https://api.steampowered.com/ISteamApps/GetSDRConfig/v1/?appid=730";
const STEAM_KEY: &str = "net.steamPops";

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    pub id: String,
    /// "net" (ev ağı) | "internet" | "gmid" | "game"
    pub group: &'static str,
    pub label: String,
    pub host: String,
    /// Her ping'in süresi (ms); yanıt gelmediyse `None`.
    pub samples: Vec<Option<u32>>,
    pub avg: Option<f64>,
    pub min: Option<u32>,
    pub max: Option<u32>,
    pub jitter: Option<f64>,
    /// Kayıp yüzdesi (0–100).
    pub loss: f64,
    /// "icmp" | "game" (sunucu ping'e kapalı, oyunun sorgusuyla ölçüldü)
    pub via: &'static str,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// "wifi" | "ethernet" | "other" | "" (bilinmiyor)
    pub kind: &'static str,
    pub name: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Verdict {
    /// "good" | "warn" | "bad"
    pub level: &'static str,
    pub title: String,
    pub detail: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub at: i64,
    pub quick: bool,
    pub link: Link,
    pub probes: Vec<Probe>,
    pub verdict: Verdict,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Pop {
    pub id: String,
    pub desc: String,
    pub ip: String,
}

#[derive(Default)]
pub struct NetTest {
    /// Steam yönlendirme noktaları (oturum boyunca bir kez indirilir).
    pops: Mutex<Option<Vec<Pop>>>,
    running: Mutex<()>,
}

struct Target {
    id: String,
    group: &'static str,
    label: String,
    host: String,
    /// Ping'e yanıt vermezse oyunun sorgusuyla ölç.
    game: Option<(crate::servers::games::Proto, u16)>,
}

// ---------------------------------------------------------------- hesap

/// Örneklerden ortalama, en iyi/kötü, titreme ve kayıp.
pub fn stats(p: &mut Probe) {
    let ok: Vec<u32> = p.samples.iter().flatten().copied().collect();
    let n = p.samples.len().max(1) as f64;
    p.loss = ((p.samples.len() - ok.len()) as f64 / n * 100.0).round();
    if ok.is_empty() {
        p.avg = None;
        p.min = None;
        p.max = None;
        p.jitter = None;
        return;
    }
    p.avg = Some(ok.iter().sum::<u32>() as f64 / ok.len() as f64);
    p.min = ok.iter().min().copied();
    p.max = ok.iter().max().copied();
    p.jitter = Some(if ok.len() < 2 {
        0.0
    } else {
        ok.windows(2)
            .map(|w| (w[0] as f64 - w[1] as f64).abs())
            .sum::<f64>()
            / (ok.len() - 1) as f64
    });
}

fn ms(v: f64) -> String {
    if v < 1.0 {
        "<1 ms".into()
    } else {
        format!("{} ms", v.round())
    }
}

/// Sonuçlardan kısa teşhis: sorun ev ağında mı, sağlayıcıda mı, sunucuda mı?
pub fn diagnose(probes: &[Probe], link: &Link) -> Verdict {
    let wifi = link.kind == "wifi";
    let answered = |p: &&Probe| p.avg.is_some();
    let gw = probes.iter().filter(|p| p.group == "net").find(answered);
    let inet = probes
        .iter()
        .filter(|p| p.group == "internet")
        .filter(answered)
        .min_by(|a, b| {
            (a.loss, a.avg.unwrap_or(f64::MAX))
                .partial_cmp(&(b.loss, b.avg.unwrap_or(f64::MAX)))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    let Some(inet) = inet else {
        return if gw.is_some() {
            Verdict {
                level: "bad",
                title: "İnternete çıkılamıyor".into(),
                detail: "Modeme ulaşılıyor ama ötesine değil: modemin internet bağlantısı ya da internet sağlayıcısı."
                    .into(),
            }
        } else {
            Verdict {
                level: "bad",
                title: "Bağlantı yok".into(),
                detail: "Ne modeme ne internete ulaşılabildi. Kabloyu ya da Wi-Fi bağlantısını kontrol et.".into(),
            }
        };
    };
    if let Some(g) = gw {
        let avg = g.avg.unwrap_or(0.0);
        let jitter = g.jitter.unwrap_or(0.0);
        let slow = if wifi { 25.0 } else { 10.0 };
        if g.loss >= 3.0 || jitter > 15.0 || avg > slow {
            let tip = if wifi {
                "Wi-Fi zayıf ya da kalabalık: kabloyla bağlanmak, modeme yaklaşmak ya da 5 GHz ağa geçmek düzeltir."
            } else {
                "Kabloyu kontrol et; modemi yeniden başlatmak çoğu zaman düzeltir."
            };
            return Verdict {
                level: if g.loss >= 10.0 || avg > 60.0 {
                    "bad"
                } else {
                    "warn"
                },
                title: "Sorun ev ağında (modeme kadar olan kısım)".into(),
                detail: format!(
                    "Modem: {} · titreme {} · kayıp %{}. {tip}",
                    ms(avg),
                    ms(jitter),
                    g.loss
                ),
            };
        }
    }
    let (avg, jitter) = (inet.avg.unwrap_or(0.0), inet.jitter.unwrap_or(0.0));
    if inet.loss >= 3.0 || jitter > 25.0 || avg > 90.0 {
        return Verdict {
            level: if inet.loss >= 10.0 || avg > 150.0 {
                "bad"
            } else {
                "warn"
            },
            title: "Sorun internet sağlayıcısında".into(),
            detail: format!(
                "Ev ağın temiz{} ama dışarısı {}: internet {} · titreme {} · kayıp %{}. Modemi yeniden başlat; sürerse sağlayıcıyı ara.",
                gw.and_then(|g| g.avg)
                    .map(|a| format!(" (modem {})", ms(a)))
                    .unwrap_or_default(),
                if inet.loss >= 3.0 { "kayıplı" } else { "yavaş" },
                ms(avg),
                ms(jitter),
                inet.loss
            ),
        };
    }
    let bad: Vec<&str> = probes
        .iter()
        .filter(|p| matches!(p.group, "gmid" | "game"))
        .filter(|p| p.avg.is_some() && (p.loss >= 5.0 || p.avg.unwrap_or(0.0) > 130.0))
        .map(|p| p.label.as_str())
        .collect();
    if !bad.is_empty() {
        return Verdict {
            level: "warn",
            title: "Bağlantın iyi; sorun sunucu tarafında".into(),
            detail: format!(
                "Senin internetin temiz ({}), ama {} yavaş ya da kayıplı. Sorun o sunucuda ya da aradaki yolda.",
                ms(avg),
                bad.join(", ")
            ),
        };
    }
    Verdict {
        level: "good",
        title: "Bağlantın sağlıklı".into(),
        detail: format!(
            "İnternet {} · titreme {} · kayıp %{}.{}",
            ms(avg),
            ms(jitter),
            inet.loss,
            if wifi {
                " Wi-Fi'desin; oyunda kablo daha da kararlı olur."
            } else {
                ""
            }
        ),
    }
}

/// Steam'in yönlendirme noktaları listesi (GetSDRConfig).
pub fn parse_pops(json: &serde_json::Value) -> Vec<Pop> {
    let Some(pops) = json["pops"].as_object() else {
        return Vec::new();
    };
    let mut out: Vec<Pop> = pops
        .iter()
        .filter_map(|(id, p)| {
            let ip = p["relays"].as_array()?.first()?["ipv4"].as_str()?;
            ip.parse::<Ipv4Addr>().ok()?;
            let desc = p["desc"].as_str().unwrap_or(id);
            Some(Pop {
                id: id.clone(),
                desc: city(desc),
                ip: ip.to_string(),
            })
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// "Frankfurt (Germany)" / "Frankfurt - Germany" → "Frankfurt"
fn city(desc: &str) -> String {
    desc.split([',', '('])
        .next()
        .unwrap_or(desc)
        .split(" - ")
        .next()
        .unwrap_or(desc)
        .trim()
        .to_string()
}

// ---------------------------------------------------------------- ölçüm

fn resolve(host: &str) -> Option<Ipv4Addr> {
    if let Ok(ip) = host.parse() {
        return Some(ip);
    }
    (host, 0)
        .to_socket_addrs()
        .ok()?
        .find_map(|a| match a.ip() {
            std::net::IpAddr::V4(v) => Some(v),
            _ => None,
        })
}

/// Bir hedefe `count` ping, aralarında `gap` bekleyerek.
fn measure(ip: Ipv4Addr, count: usize, gap: Duration, timeout_ms: u32) -> Vec<Option<u32>> {
    let pinger = win::Pinger::new();
    (0..count)
        .map(|i| {
            if i > 0 {
                std::thread::sleep(gap);
            }
            pinger.as_ref().and_then(|p| p.ping(ip, timeout_ms))
        })
        .collect()
}

fn run_target(t: &Target, count: usize, gap: Duration) -> Probe {
    let mut p = Probe {
        id: t.id.clone(),
        group: t.group,
        label: t.label.clone(),
        host: t.host.clone(),
        via: "icmp",
        ..Default::default()
    };
    p.samples = match resolve(&t.host) {
        Some(ip) => measure(ip, count, gap, 1000),
        None => vec![None; count],
    };
    // Ping'e kapalı oyun sunucusu: oyunun kendi sorgusuyla (UDP) ölç.
    if p.samples.iter().all(Option::is_none) {
        if let Some((proto, port)) = t.game {
            p.via = "game";
            p.samples = (0..count.min(5))
                .map(|i| {
                    if i > 0 {
                        std::thread::sleep(gap);
                    }
                    crate::servers::query::query(proto, &t.host, port)
                        .ok()
                        .map(|info| info.ping_ms)
                })
                .collect();
        }
    }
    stats(&mut p);
    p
}

impl NetTest {
    fn steam_pops(&self) -> Vec<Pop> {
        let mut guard = self.pops.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(p) = guard.as_ref() {
            return p.clone();
        }
        let agent = crate::net::agent(Duration::from_secs(5), Duration::from_secs(10));
        let pops = agent
            .get(STEAM_SDR)
            .call()
            .ok()
            .filter(|r| r.status() == 200)
            .and_then(|mut r| r.body_mut().read_to_string().ok())
            .and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
            .map(|v| parse_pops(&v))
            .unwrap_or_default();
        if !pops.is_empty() {
            *guard = Some(pops.clone());
        }
        pops
    }

    /// En yakın Steam noktaları: hepsine 2 ping, en hızlı üçü (sonraki hızlı testler için de saklanır).
    fn nearest_pops(&self, db: &Db) -> Vec<Pop> {
        let pops = self.steam_pops();
        if pops.is_empty() {
            return db.setting_json(STEAM_KEY).unwrap_or_default();
        }
        let mut timed: Vec<(f64, Pop)> = std::thread::scope(|s| {
            let handles: Vec<_> = pops
                .iter()
                .map(|p| {
                    s.spawn(move || {
                        let ip: Ipv4Addr = p.ip.parse().ok()?;
                        let got: Vec<u32> = measure(ip, 2, Duration::from_millis(80), 600)
                            .into_iter()
                            .flatten()
                            .collect();
                        let min = *got.iter().min()?;
                        Some((min as f64, p.clone()))
                    })
                })
                .collect();
            handles
                .into_iter()
                .filter_map(|h| h.join().ok().flatten())
                .collect()
        });
        timed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let best: Vec<Pop> = timed.into_iter().take(3).map(|(_, p)| p).collect();
        if !best.is_empty() {
            if let Ok(j) = serde_json::to_string(&best) {
                let _ = db.set_setting(STEAM_KEY, &j);
            }
        }
        best
    }

    /// Testi çalıştırır. `quick`: oyun içi panel için kısa test (6 ping, son bilinen Steam noktası).
    pub fn run(&self, app: &AppHandle, quick: bool) -> Report {
        let _one = self.running.lock().unwrap_or_else(|e| e.into_inner());
        let db = app.state::<Db>();
        let (gateway, link) = win::gateway();
        let mut targets = Vec::new();
        if let Some(gw) = gateway {
            targets.push(Target {
                id: "gateway".into(),
                group: "net",
                label: "Modem".into(),
                host: gw.to_string(),
                game: None,
            });
        }
        for (id, label, host) in [
            ("cloudflare", "Cloudflare", "1.1.1.1"),
            ("google", "Google", "8.8.8.8"),
        ] {
            targets.push(Target {
                id: id.into(),
                group: "internet",
                label: label.into(),
                host: host.into(),
                game: None,
            });
        }
        if !quick {
            targets.push(Target {
                id: "discord".into(),
                group: "internet",
                label: "Discord".into(),
                host: "discord.com".into(),
                game: None,
            });
        }
        let servers = crate::servers::list(&db).unwrap_or_default();
        for s in servers.iter().take(if quick { 3 } else { 12 }) {
            let game = crate::servers::games::find(&s.game)
                .filter(|g| g.proto != crate::servers::games::Proto::None)
                .map(|g| (g.proto, s.query_port.unwrap_or(s.port + g.query_offset)));
            targets.push(Target {
                id: format!("gmid-{}", s.id),
                group: "gmid",
                label: s.name.clone(),
                host: s.host.clone(),
                game,
            });
        }
        let pops = if quick {
            db.setting_json::<Vec<Pop>>(STEAM_KEY)
                .unwrap_or_default()
                .into_iter()
                .take(1)
                .collect()
        } else {
            self.nearest_pops(&db)
        };
        for p in pops {
            targets.push(Target {
                id: format!("steam-{}", p.id),
                group: "game",
                label: format!("Steam · {}", p.desc),
                host: p.ip,
                game: None,
            });
        }
        let (count, gap) = if quick {
            (6, Duration::from_millis(100))
        } else {
            (12, Duration::from_millis(150))
        };
        let probes: Vec<Probe> = std::thread::scope(|s| {
            let handles: Vec<_> = targets
                .iter()
                .map(|t| s.spawn(move || run_target(t, count, gap)))
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        let verdict = diagnose(&probes, &link);
        crate::log!(
            "Bağlantı testi{}: {} — {}",
            if quick { " (hızlı)" } else { "" },
            verdict.title,
            verdict.detail
        );
        Report {
            at: crate::util::now_ms(),
            quick,
            link,
            probes,
            verdict,
        }
    }
}

// ---------------------------------------------------------------- Windows

#[cfg(windows)]
mod win {
    use super::Link;
    use std::net::Ipv4Addr;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::NetworkManagement::IpHelper::{
        GetAdaptersAddresses, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho,
        GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER,
        GAA_FLAG_SKIP_MULTICAST, ICMP_ECHO_REPLY, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::Networking::WinSock::{AF_INET, SOCKADDR_IN};

    pub struct Pinger(HANDLE);

    impl Pinger {
        pub fn new() -> Option<Self> {
            unsafe { IcmpCreateFile().ok().map(Pinger) }
        }

        /// Tek ping; yanıt süresi (ms).
        pub fn ping(&self, ip: Ipv4Addr, timeout_ms: u32) -> Option<u32> {
            let data = *b"GMID-Tool-ping-32-bytes-payload!";
            let mut reply = vec![0u8; std::mem::size_of::<ICMP_ECHO_REPLY>() + data.len() + 8];
            let n = unsafe {
                IcmpSendEcho(
                    self.0,
                    u32::from_ne_bytes(ip.octets()),
                    data.as_ptr().cast(),
                    data.len() as u16,
                    None,
                    reply.as_mut_ptr().cast(),
                    reply.len() as u32,
                    timeout_ms,
                )
            };
            if n == 0 {
                return None;
            }
            let r = unsafe { std::ptr::read_unaligned(reply.as_ptr().cast::<ICMP_ECHO_REPLY>()) };
            // 0: IP_SUCCESS
            (r.Status == 0).then_some(r.RoundTripTime)
        }
    }

    impl Drop for Pinger {
        fn drop(&mut self) {
            unsafe {
                let _ = IcmpCloseHandle(self.0);
            }
        }
    }

    /// Varsayılan ağ geçidi (modem) ve bağlantı türü: açık ve ağ geçidi olan ilk bağdaştırıcı.
    pub fn gateway() -> (Option<Ipv4Addr>, Link) {
        let flags = GAA_FLAG_INCLUDE_GATEWAYS
            | GAA_FLAG_SKIP_ANYCAST
            | GAA_FLAG_SKIP_MULTICAST
            | GAA_FLAG_SKIP_DNS_SERVER;
        let mut size: u32 = 32 * 1024;
        let mut buf: Vec<u64> = Vec::new();
        for _ in 0..3 {
            buf = vec![0u64; (size as usize).div_ceil(8)];
            let r = unsafe {
                GetAdaptersAddresses(
                    AF_INET.0 as u32,
                    flags,
                    None,
                    Some(buf.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>()),
                    &mut size,
                )
            };
            match r {
                0 => break,
                111 => continue, // ERROR_BUFFER_OVERFLOW: `size` büyüdü
                _ => return (None, Link::default()),
            }
        }
        let mut cur = buf.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        while !cur.is_null() {
            let a = unsafe { &*cur };
            cur = a.Next;
            // 1: IfOperStatusUp
            if a.OperStatus.0 != 1 || a.FirstGatewayAddress.is_null() {
                continue;
            }
            let gw = unsafe { &*a.FirstGatewayAddress };
            let sa = gw.Address.lpSockaddr;
            if sa.is_null() || unsafe { (*sa).sa_family } != AF_INET {
                continue;
            }
            let sin = unsafe { &*sa.cast::<SOCKADDR_IN>() };
            let ip = Ipv4Addr::from(unsafe { sin.sin_addr.S_un.S_addr }.to_ne_bytes());
            if ip.is_unspecified() {
                continue;
            }
            let kind = match a.IfType {
                71 => "wifi",
                6 => "ethernet",
                _ => "other",
            };
            let name = unsafe { a.FriendlyName.to_string() }.unwrap_or_default();
            return (Some(ip), Link { kind, name });
        }
        (None, Link::default())
    }
}

#[cfg(not(windows))]
mod win {
    use super::Link;
    use std::net::{Ipv4Addr, SocketAddr, TcpStream};
    use std::time::{Duration, Instant};

    /// Windows dışında (geliştirme): TCP bağlantı süresi yaklaşık ping yerine.
    pub struct Pinger;

    impl Pinger {
        pub fn new() -> Option<Self> {
            Some(Pinger)
        }
        pub fn ping(&self, ip: Ipv4Addr, timeout_ms: u32) -> Option<u32> {
            let t = Instant::now();
            for port in [443, 80, 53] {
                let addr = SocketAddr::from((ip, port));
                if TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms as u64))
                    .is_ok()
                {
                    return Some(t.elapsed().as_millis() as u32);
                }
            }
            None
        }
    }

    pub fn gateway() -> (Option<Ipv4Addr>, Link) {
        (None, Link::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(group: &'static str, label: &str, samples: &[Option<u32>]) -> Probe {
        let mut p = Probe {
            id: label.into(),
            group,
            label: label.into(),
            samples: samples.to_vec(),
            ..Default::default()
        };
        stats(&mut p);
        p
    }

    #[test]
    fn stats_are_computed() {
        let p = probe("internet", "x", &[Some(20), Some(30), None, Some(20)]);
        assert_eq!(p.loss, 25.0);
        assert_eq!(p.min, Some(20));
        assert_eq!(p.max, Some(30));
        assert!((p.avg.unwrap() - 23.333).abs() < 0.01);
        assert!((p.jitter.unwrap() - 10.0).abs() < 0.01);
        let none = probe("internet", "y", &[None, None]);
        assert_eq!((none.loss, none.avg), (100.0, None));
    }

    #[test]
    fn diagnosis_points_at_the_right_place() {
        let wifi = Link {
            kind: "wifi",
            name: "Wi-Fi".into(),
        };
        let wired = Link {
            kind: "ethernet",
            name: "Ethernet".into(),
        };
        let good_net = probe("net", "Modem", &[Some(2), Some(3), Some(2), Some(2)]);
        let good_inet = probe(
            "internet",
            "Cloudflare",
            &[Some(18), Some(19), Some(18), Some(20)],
        );
        // Her şey iyi.
        let v = diagnose(&[good_net.clone(), good_inet.clone()], &wired);
        assert_eq!(v.level, "good", "{v:?}");
        // Wi-Fi: modeme kadar kayıp ve titreme.
        let bad_net = probe(
            "net",
            "Modem",
            &[Some(5), Some(80), None, Some(4), Some(60), Some(3)],
        );
        let v = diagnose(&[bad_net, good_inet.clone()], &wifi);
        assert!(v.title.contains("ev ağında"), "{v:?}");
        assert!(v.detail.contains("Wi-Fi"), "{v:?}");
        // Sağlayıcı: modem temiz, internet kayıplı.
        let bad_inet = probe(
            "internet",
            "Cloudflare",
            &[Some(40), None, Some(45), Some(42), None, Some(41)],
        );
        let v = diagnose(&[good_net.clone(), bad_inet], &wired);
        assert!(v.title.contains("sağlayıcı"), "{v:?}");
        // Sunucu: internet iyi, bir sunucu yavaş.
        let slow = probe("gmid", "GMID CoD2", &[Some(180), Some(190), Some(185)]);
        let v = diagnose(&[good_net.clone(), good_inet.clone(), slow], &wired);
        assert!(v.title.contains("sunucu"), "{v:?}");
        assert!(v.detail.contains("GMID CoD2"));
        // Modem ping'e kapalı ama internet iyi: modem yok sayılır.
        let silent = probe("net", "Modem", &[None, None]);
        assert_eq!(diagnose(&[silent.clone(), good_inet], &wired).level, "good");
        // İnternet yok.
        let dead = probe("internet", "Cloudflare", &[None, None]);
        assert!(diagnose(&[good_net, dead.clone()], &wired)
            .title
            .contains("İnternete"));
        assert_eq!(diagnose(&[silent, dead], &wired).title, "Bağlantı yok");
    }

    #[test]
    fn steam_pops_are_parsed() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"revision":1,"pops":{
              "fra":{"desc":"Frankfurt (Germany)","geo":[8.6,50.1],"relays":[{"ipv4":"155.133.226.75","port_range":[27015,27060]}]},
              "ist":{"desc":"Istanbul - Turkey","relays":[{"ipv4":"155.133.230.2"}]},
              "fsn":{"desc":"Falkenstein","relays":[]},
              "bad":{"desc":"X","relays":[{"ipv4":"yok"}]}
            },"success":true}"#,
        )
        .unwrap();
        let p = parse_pops(&v);
        assert_eq!(p.len(), 2, "{p:?}");
        assert_eq!((p[0].id.as_str(), p[0].desc.as_str()), ("fra", "Frankfurt"));
        assert_eq!(p[1].desc, "Istanbul");
    }
}
