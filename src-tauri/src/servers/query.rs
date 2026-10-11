//! Oyun sunucularına "kaç kişi var, hangi harita" diye soran küçük istemciler.
//! Her oyun ailesinin kendi dili var; hepsi aynı [`Info`] özetine çevrilir.

use super::games::Proto;
use serde::Serialize;
use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Serialize, Clone, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub name: String,
    pub score: Option<i64>,
    pub ping: Option<u32>,
}

#[derive(Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    /// Sunucunun kendine verdiği ad (Minecraft'ta açıklama yazısı).
    pub name: String,
    pub map: String,
    /// Oyun modu (CoD: `tdm`, `sd`…; Minecraft Bedrock: `Survival`…).
    pub mode: String,
    pub version: String,
    pub players: u32,
    pub max_players: u32,
    pub bots: u32,
    pub password: bool,
    pub player_list: Vec<Player>,
    /// Minecraft sunucu simgesi (`data:image/png;base64,…`).
    pub favicon: Option<String>,
    pub ping_ms: u32,
}

/// Sunucuyu sorgular; ilk denemede yanıt gelmezse bir kez daha dener.
pub fn query(proto: Proto, host: &str, port: u16) -> Result<Info, String> {
    let addr = resolve(host, port)?;
    let once = || match proto {
        Proto::Quake3 => quake3(addr),
        Proto::MinecraftJava => minecraft_java(host, addr),
        Proto::MinecraftBedrock => minecraft_bedrock(addr),
        Proto::Valve => valve(addr),
        Proto::Samp => samp(addr),
        Proto::None => Err(Error::Unsupported),
    };
    match once() {
        Err(Error::Io(_)) => once(),
        r => r,
    }
    .map_err(|e| e.to_string())
}

enum Error {
    Io(io::Error),
    Bad,
    Unsupported,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Error::Io(e) => match e.kind() {
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => "Yanıt vermedi",
                io::ErrorKind::ConnectionRefused => "Bağlantıyı reddetti",
                io::ErrorKind::ConnectionReset => "Bağlantı kesildi",
                _ => "Ulaşılamadı",
            },
            Error::Bad => "Anlaşılmayan yanıt",
            Error::Unsupported => "Bu oyun sorgulanamıyor",
        })
    }
}

type Res<T> = Result<T, Error>;

fn resolve(host: &str, port: u16) -> Result<SocketAddr, String> {
    let addrs: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|_| "Adres bulunamadı".to_string())?
        .collect();
    // Eski oyun sunucularının neredeyse hepsi IPv4.
    addrs
        .iter()
        .find(|a| a.is_ipv4())
        .or(addrs.first())
        .copied()
        .ok_or_else(|| "Adres bulunamadı".into())
}

fn ms(since: Instant) -> u32 {
    since.elapsed().as_millis().min(9999) as u32
}

fn udp(addr: SocketAddr) -> io::Result<UdpSocket> {
    let s = UdpSocket::bind(if addr.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    })?;
    s.set_read_timeout(Some(TIMEOUT))?;
    s.connect(addr)?;
    Ok(s)
}

fn exchange(s: &UdpSocket, req: &[u8]) -> io::Result<Vec<u8>> {
    s.send(req)?;
    let mut buf = vec![0u8; 65_535];
    let n = s.recv(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
}

// ---------------------------------------------------------------- Call of Duty (Quake 3)

fn quake3(addr: SocketAddr) -> Res<Info> {
    let s = udp(addr)?;
    let t = Instant::now();
    let resp = exchange(&s, b"\xFF\xFF\xFF\xFFgetstatus\n")?;
    let ping = ms(t);
    let mut info = parse_quake3(&resp)?;
    info.ping_ms = ping;
    Ok(info)
}

fn parse_quake3(buf: &[u8]) -> Res<Info> {
    let body = buf
        .strip_prefix(b"\xFF\xFF\xFF\xFFstatusResponse")
        .ok_or(Error::Bad)?;
    let text = decode(body);
    let mut lines = text
        .split('\n')
        .map(|l| l.trim_end_matches('\r'))
        .skip_while(|l| l.is_empty());

    let vars: Vec<&str> = lines
        .next()
        .ok_or(Error::Bad)?
        .split('\\')
        .skip(1)
        .collect();
    let get = |key: &str| {
        vars.chunks(2)
            .find(|kv| kv[0].eq_ignore_ascii_case(key))
            .and_then(|kv| kv.get(1))
            .map(|v| v.trim())
            .unwrap_or("")
    };

    let player_list: Vec<Player> = lines
        .filter_map(|line| {
            let mut parts = line.splitn(3, ' ');
            let score = parts.next()?.parse().ok()?;
            let ping = parts.next()?.parse().ok()?;
            let name = strip_q3_colors(parts.next()?.trim().trim_matches('"'));
            Some(Player {
                name,
                score: Some(score),
                ping: Some(ping),
            })
        })
        .collect();

    let version = match get("shortversion") {
        "" => get("version")
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" "),
        v => v.to_string(),
    };
    Ok(Info {
        name: strip_q3_colors(get("sv_hostname")),
        map: get("mapname").to_string(),
        mode: get("g_gametype").to_string(),
        version,
        players: player_list.len() as u32,
        max_players: get("sv_maxclients").parse().unwrap_or(0),
        bots: 0,
        password: get("g_needpass") == "1" || get("pswrd") == "1",
        player_list,
        favicon: None,
        ping_ms: 0,
    })
}

/// `^1Kırmızı ^7Beyaz` → `Kırmızı Beyaz`
fn strip_q3_colors(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '^' {
            chars.next();
        } else if !c.is_control() {
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// Eski oyunlar metni Windows kod sayfasıyla gönderir; Türkçe Windows'ta bu CP1254'tür.
/// Geçerli UTF-8 ise (yeni sunucular) olduğu gibi okunur.
fn decode(b: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(b) {
        return s.to_string();
    }
    const HIGH: [char; 32] = [
        '€', '�', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '�', '�', '�', '�', '‘',
        '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '�', '�', 'Ÿ',
    ];
    b.iter()
        .map(|&c| match c {
            0x80..=0x9F => HIGH[(c - 0x80) as usize],
            0xD0 => 'Ğ',
            0xDD => 'İ',
            0xDE => 'Ş',
            0xF0 => 'ğ',
            0xFD => 'ı',
            0xFE => 'ş',
            _ => c as char,
        })
        .collect()
}

// ---------------------------------------------------------------- Minecraft Java

fn write_varint(out: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        if v & !0x7F == 0 {
            out.push(v as u8);
            return;
        }
        out.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
}

fn read_varint(r: &mut impl Read) -> Res<i32> {
    let mut value = 0u32;
    for i in 0..5 {
        let mut b = [0u8];
        r.read_exact(&mut b)?;
        value |= ((b[0] & 0x7F) as u32) << (7 * i);
        if b[0] & 0x80 == 0 {
            return Ok(value as i32);
        }
    }
    Err(Error::Bad)
}

fn packet(id: i32, data: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(data.len() + 1);
    write_varint(&mut body, id);
    body.extend_from_slice(data);
    let mut out = Vec::with_capacity(body.len() + 3);
    write_varint(&mut out, body.len() as i32);
    out.extend(body);
    out
}

fn minecraft_java(host: &str, addr: SocketAddr) -> Res<Info> {
    let mut s = TcpStream::connect_timeout(&addr, TIMEOUT)?;
    s.set_read_timeout(Some(TIMEOUT))?;
    s.set_write_timeout(Some(TIMEOUT))?;

    let mut hs = Vec::new();
    write_varint(&mut hs, -1); // protokol: "bilmiyorum", sunucu kendi sürümünü söyler
    write_varint(&mut hs, host.len() as i32);
    hs.extend_from_slice(host.as_bytes());
    hs.extend_from_slice(&addr.port().to_be_bytes());
    write_varint(&mut hs, 1); // durum sorgusu
    let mut req = packet(0, &hs);
    req.extend(packet(0, &[]));
    s.write_all(&req)?;

    let len = read_varint(&mut s)?;
    if !(1..=2_000_000).contains(&len) {
        return Err(Error::Bad);
    }
    let mut body = vec![0u8; len as usize];
    s.read_exact(&mut body)?;

    // Gecikme: ayrı bir ping paketiyle ölçülür (durum yanıtı büyük olabilir).
    let t = Instant::now();
    let ping = s
        .write_all(&packet(1, &1i64.to_be_bytes()))
        .and_then(|_| {
            let mut echo = [0u8; 10];
            s.read_exact(&mut echo)
        })
        .map(|_| ms(t))
        .unwrap_or(0);

    let mut info = parse_minecraft_java(&body)?;
    info.ping_ms = ping;
    Ok(info)
}

fn parse_minecraft_java(body: &[u8]) -> Res<Info> {
    let mut r = body;
    if read_varint(&mut r)? != 0 {
        return Err(Error::Bad);
    }
    let len = read_varint(&mut r)? as usize;
    let json: serde_json::Value =
        serde_json::from_slice(r.get(..len).ok_or(Error::Bad)?).map_err(|_| Error::Bad)?;

    let players = &json["players"];
    let player_list = players["sample"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| p["name"].as_str())
                .map(|n| Player {
                    name: strip_section(n),
                    ..Default::default()
                })
                .collect()
        })
        .unwrap_or_default();
    let favicon = json["favicon"]
        .as_str()
        .filter(|f| f.starts_with("data:image/png;base64,"))
        .map(String::from);

    Ok(Info {
        name: strip_section(&flatten_chat(&json["description"]))
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(" · "),
        version: strip_section(json["version"]["name"].as_str().unwrap_or("")),
        players: players["online"].as_u64().unwrap_or(0) as u32,
        max_players: players["max"].as_u64().unwrap_or(0) as u32,
        player_list,
        favicon,
        ..Default::default()
    })
}

/// Minecraft'ın sohbet bileşenini (`{"text":…, "extra":[…]}`) düz metne çevirir.
fn flatten_chat(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(a) => a.iter().map(flatten_chat).collect(),
        serde_json::Value::Object(o) => {
            let mut s = o.get("text").map(flatten_chat).unwrap_or_default();
            if let Some(extra) = o.get("extra") {
                s.push_str(&flatten_chat(extra));
            }
            s
        }
        _ => String::new(),
    }
}

/// `§aYeşil §lkalın` → `Yeşil kalın`
fn strip_section(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

// ---------------------------------------------------------------- Minecraft Bedrock (RakNet)

const RAKNET_MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

fn minecraft_bedrock(addr: SocketAddr) -> Res<Info> {
    let s = udp(addr)?;
    let mut req = vec![0x01];
    req.extend_from_slice(&(crate::util::now_ms()).to_be_bytes());
    req.extend_from_slice(&RAKNET_MAGIC);
    req.extend_from_slice(&0x474d_4944_u64.to_be_bytes()); // "GMID"
    let t = Instant::now();
    let resp = exchange(&s, &req)?;
    let ping = ms(t);
    let mut info = parse_minecraft_bedrock(&resp)?;
    info.ping_ms = ping;
    Ok(info)
}

fn parse_minecraft_bedrock(buf: &[u8]) -> Res<Info> {
    // 0x1C, zaman (8), sunucu kimliği (8), sihirli sayı (16), uzunluk (2), metin
    if buf.first() != Some(&0x1C) || buf.len() < 35 {
        return Err(Error::Bad);
    }
    let len = u16::from_be_bytes([buf[33], buf[34]]) as usize;
    let text = String::from_utf8_lossy(buf.get(35..35 + len).ok_or(Error::Bad)?);
    let f: Vec<&str> = text.split(';').collect();
    if f.len() < 6 {
        return Err(Error::Bad);
    }
    let field = |i: usize| strip_section(f.get(i).copied().unwrap_or("").trim());
    Ok(Info {
        name: field(1),
        version: field(3),
        players: f[4].parse().unwrap_or(0),
        max_players: f[5].parse().unwrap_or(0),
        map: field(7),
        mode: field(8),
        ..Default::default()
    })
}

// ---------------------------------------------------------------- Valve (A2S)

struct Reader<'a> {
    b: &'a [u8],
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Res<&'a [u8]> {
        if self.b.len() < n {
            return Err(Error::Bad);
        }
        let (head, tail) = self.b.split_at(n);
        self.b = tail;
        Ok(head)
    }
    fn u8(&mut self) -> Res<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16le(&mut self) -> Res<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32le(&mut self) -> Res<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn cstr(&mut self) -> Res<String> {
        let end = self.b.iter().position(|&c| c == 0).ok_or(Error::Bad)?;
        let s = String::from_utf8_lossy(&self.b[..end]).into_owned();
        self.b = &self.b[end + 1..];
        Ok(s)
    }
}

/// İstek gönderir; sunucu "önce şu sayıyı geri yolla" derse (0x41) isteği onunla tekrarlar.
fn a2s(s: &UdpSocket, req: &[u8], challenge_suffix: bool) -> Res<Vec<u8>> {
    let mut resp = exchange(s, req)?;
    if resp.len() >= 9 && resp[..5] == [0xFF, 0xFF, 0xFF, 0xFF, 0x41] {
        let mut again = req.to_vec();
        if !challenge_suffix {
            again.truncate(5); // A2S_PLAYER: FF FF FF FF yer tutucusunun yerine
        }
        again.extend_from_slice(&resp[5..9]);
        resp = exchange(s, &again)?;
    }
    if resp.len() < 5 || resp[..4] != [0xFF, 0xFF, 0xFF, 0xFF] {
        // 0xFFFFFFFE: parçalı yanıt (çok büyük sunucular); desteklenmiyor.
        return Err(Error::Bad);
    }
    Ok(resp)
}

fn valve(addr: SocketAddr) -> Res<Info> {
    let s = udp(addr)?;
    let t = Instant::now();
    let resp = a2s(&s, b"\xFF\xFF\xFF\xFFTSource Engine Query\0", true)?;
    let ping = ms(t);
    let mut info = parse_a2s_info(&resp)?;
    info.ping_ms = ping;
    // Oyuncu listesi gelmezse bilgi yine de geçerli.
    if let Ok(list) =
        a2s(&s, b"\xFF\xFF\xFF\xFFU\xFF\xFF\xFF\xFF", false).and_then(|r| parse_a2s_players(&r))
    {
        info.player_list = list;
    }
    Ok(info)
}

fn parse_a2s_info(buf: &[u8]) -> Res<Info> {
    let mut r = Reader { b: &buf[4..] };
    match r.u8()? {
        // Source ve güncel GoldSrc
        0x49 => {
            r.u8()?; // protokol
            let name = r.cstr()?;
            let map = r.cstr()?;
            r.cstr()?; // klasör
            let game = r.cstr()?;
            r.take(2)?; // Steam uygulama kimliği
            let players = r.u8()? as u32;
            let max_players = r.u8()? as u32;
            let bots = r.u8()? as u32;
            r.take(2)?; // sunucu türü, işletim sistemi
            let password = r.u8()? == 1;
            r.u8()?; // VAC
            let version = r.cstr().unwrap_or_default();
            Ok(Info {
                name,
                map,
                mode: game,
                version,
                players,
                max_players,
                bots,
                password,
                ..Default::default()
            })
        }
        // Eski GoldSrc (CS 1.6'nın bazı sunucuları)
        0x6D => {
            r.cstr()?; // adres
            let name = r.cstr()?;
            let map = r.cstr()?;
            r.cstr()?; // klasör
            let game = r.cstr()?;
            let players = r.u8()? as u32;
            let max_players = r.u8()? as u32;
            r.take(3)?; // protokol, sunucu türü, işletim sistemi
            let password = r.u8()? == 1;
            Ok(Info {
                name,
                map,
                mode: game,
                players,
                max_players,
                password,
                ..Default::default()
            })
        }
        _ => Err(Error::Bad),
    }
}

fn parse_a2s_players(buf: &[u8]) -> Res<Vec<Player>> {
    let mut r = Reader { b: &buf[4..] };
    if r.u8()? != 0x44 {
        return Err(Error::Bad);
    }
    let count = r.u8()?;
    let mut list = Vec::with_capacity(count as usize);
    for _ in 0..count {
        r.u8()?; // sıra
        let name = r.cstr()?;
        let score = r.u32le()? as i32;
        r.take(4)?; // bağlı kalma süresi
        if !name.is_empty() {
            list.push(Player {
                name,
                score: Some(score as i64),
                ping: None,
            });
        }
    }
    Ok(list)
}

// ---------------------------------------------------------------- SA-MP

fn samp(addr: SocketAddr) -> Res<Info> {
    let IpAddr::V4(ip) = addr.ip() else {
        return Err(Error::Unsupported);
    };
    let s = udp(addr)?;
    let mut head = b"SAMP".to_vec();
    head.extend_from_slice(&ip.octets());
    head.extend_from_slice(&addr.port().to_le_bytes());

    let t = Instant::now();
    let resp = exchange(&s, &[head.as_slice(), b"i"].concat())?;
    let ping = ms(t);
    let mut info = parse_samp_info(&resp)?;
    info.ping_ms = ping;
    // Oyuncu listesi ('c') sadece 100 kişiye kadar gelir.
    if info.players > 0 && info.players <= 100 {
        if let Ok(list) = exchange(&s, &[head.as_slice(), b"c"].concat())
            .map_err(Error::from)
            .and_then(|r| parse_samp_clients(&r))
        {
            info.player_list = list;
        }
    }
    Ok(info)
}

fn parse_samp_info(buf: &[u8]) -> Res<Info> {
    if buf.len() < 11 || &buf[..4] != b"SAMP" || buf[10] != b'i' {
        return Err(Error::Bad);
    }
    let mut r = Reader { b: &buf[11..] };
    let password = r.u8()? == 1;
    let players = r.u16le()? as u32;
    let max_players = r.u16le()? as u32;
    let mut text = || -> Res<String> {
        let n = r.u32le()? as usize;
        Ok(decode(r.take(n)?))
    };
    let name = text()?;
    let mode = text()?;
    let map = text().unwrap_or_default(); // "language" alanı; sunucular çoğu zaman haritayı yazar
    Ok(Info {
        name,
        map,
        mode,
        players,
        max_players,
        password,
        ..Default::default()
    })
}

fn parse_samp_clients(buf: &[u8]) -> Res<Vec<Player>> {
    if buf.len() < 11 || buf[10] != b'c' {
        return Err(Error::Bad);
    }
    let mut r = Reader { b: &buf[11..] };
    let count = r.u16le()?;
    let mut list = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let n = r.u8()? as usize;
        let name = decode(r.take(n)?);
        let score = r.u32le()? as i32;
        list.push(Player {
            name,
            score: Some(score as i64),
            ping: None,
        });
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quake3_status() {
        let mut buf = b"\xFF\xFF\xFF\xFFstatusResponse\n\\g_gametype\\tdm\\mapname\\mp_toujane\\shortversion\\1.3\\sv_hostname\\^1GMID ^7CoD2\\sv_maxclients\\24\\g_needpass\\0\n".to_vec();
        buf.extend_from_slice(b"12 48 \"^2Merkut\"\n0 999 \"A\xF0a\xE7\"\n");
        let info = parse_quake3(&buf).ok().unwrap();
        assert_eq!(info.name, "GMID CoD2");
        assert_eq!(info.map, "mp_toujane");
        assert_eq!(info.mode, "tdm");
        assert_eq!(info.version, "1.3");
        assert_eq!((info.players, info.max_players), (2, 24));
        assert!(!info.password);
        assert_eq!(info.player_list[0].name, "Merkut");
        assert_eq!(info.player_list[0].score, Some(12));
        assert_eq!(info.player_list[1].name, "Ağaç");
    }

    #[test]
    fn minecraft_java_status() {
        let json = r#"{"version":{"name":"Paper 1.21.1","protocol":767},"players":{"max":20,"online":3,"sample":[{"name":"Merkut","id":"x"}]},"description":{"text":"","extra":[{"text":"§6GMID "},{"text":"Survival"}]},"favicon":"data:image/png;base64,AAAA"}"#.as_bytes();
        let mut body = Vec::new();
        write_varint(&mut body, 0);
        write_varint(&mut body, json.len() as i32);
        body.extend_from_slice(json);
        let info = parse_minecraft_java(&body).ok().unwrap();
        assert_eq!(info.name, "GMID Survival");
        assert_eq!(info.version, "Paper 1.21.1");
        assert_eq!((info.players, info.max_players), (3, 20));
        assert_eq!(info.player_list[0].name, "Merkut");
        assert!(info.favicon.is_some());
    }

    #[test]
    fn varint_round_trip() {
        for v in [0, 1, 127, 128, 25565, 2_097_151, -1] {
            let mut b = Vec::new();
            write_varint(&mut b, v);
            assert_eq!(read_varint(&mut b.as_slice()).ok(), Some(v));
        }
    }

    #[test]
    fn bedrock_pong() {
        let text = b"MCPE;GMID Bedrock;712;1.21.20;4;30;123;Dunya;Survival;1;19132;19133;";
        let mut buf = vec![0x1C];
        buf.extend_from_slice(&[0; 16]);
        buf.extend_from_slice(&RAKNET_MAGIC);
        buf.extend_from_slice(&(text.len() as u16).to_be_bytes());
        buf.extend_from_slice(text);
        let info = parse_minecraft_bedrock(&buf).ok().unwrap();
        assert_eq!(info.name, "GMID Bedrock");
        assert_eq!((info.players, info.max_players), (4, 30));
        assert_eq!(info.mode, "Survival");
    }

    #[test]
    fn a2s_info_and_players() {
        let mut buf = b"\xFF\xFF\xFF\xFFI\x11GMID CS\0de_dust2\0cstrike\0Counter-Strike\0".to_vec();
        buf.extend_from_slice(&[10, 0, 5, 16, 1, b'd', b'w', 1, 1]);
        buf.extend_from_slice(b"1.1.2.7\0");
        let info = parse_a2s_info(&buf).ok().unwrap();
        assert_eq!(info.name, "GMID CS");
        assert_eq!(info.map, "de_dust2");
        assert_eq!((info.players, info.max_players, info.bots), (5, 16, 1));
        assert!(info.password);

        let mut p = b"\xFF\xFF\xFF\xFFD\x02".to_vec();
        p.extend_from_slice(b"\0Merkut\0");
        p.extend_from_slice(&7i32.to_le_bytes());
        p.extend_from_slice(&1.5f32.to_le_bytes());
        p.extend_from_slice(b"\x01\0");
        p.extend_from_slice(&0i32.to_le_bytes());
        p.extend_from_slice(&0f32.to_le_bytes());
        let list = parse_a2s_players(&p).ok().unwrap();
        assert_eq!(list.len(), 1, "isimsiz bağlanan oyuncu atlanır");
        assert_eq!(list[0].score, Some(7));
    }

    #[test]
    fn samp_info() {
        let mut buf = b"SAMP\x01\x02\x03\x04\x61\x1Ei".to_vec();
        buf.push(0);
        buf.extend_from_slice(&12u16.to_le_bytes());
        buf.extend_from_slice(&100u16.to_le_bytes());
        for s in [&b"GMID Roleplay"[..], b"RP", b"San Andreas"] {
            buf.extend_from_slice(&(s.len() as u32).to_le_bytes());
            buf.extend_from_slice(s);
        }
        let info = parse_samp_info(&buf).ok().unwrap();
        assert_eq!(info.name, "GMID Roleplay");
        assert_eq!((info.players, info.max_players), (12, 100));
        assert_eq!(info.mode, "RP");
    }

    #[test]
    fn quake3_over_udp() {
        // Yerel sahte sunucu: ilk isteği yutar (kayıp paket), ikincisine yanıt verir.
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let mut buf = [0u8; 64];
            let _ = server.recv_from(&mut buf);
            let (_, from) = server.recv_from(&mut buf).unwrap();
            let reply = b"\xFF\xFF\xFF\xFFstatusResponse\n\\sv_hostname\\Yerel\\sv_maxclients\\8\n";
            server.send_to(reply, from).unwrap();
        });
        let info = query(Proto::Quake3, "127.0.0.1", port).unwrap();
        assert_eq!(info.name, "Yerel");
        assert_eq!((info.players, info.max_players), (0, 8));
    }

    #[test]
    fn colors_are_stripped() {
        assert_eq!(strip_q3_colors("^1A^7B^"), "AB");
        assert_eq!(strip_section("§l§6Altın"), "Altın");
    }
}
