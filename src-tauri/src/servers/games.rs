//! Desteklenen oyunlar: nasıl sorgulanır, varsayılan port, "Bağlan" nasıl yapılır.

use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Proto {
    /// Quake 3 ailesi `getstatus` (Call of Duty 1 / UO / 2 / 4).
    Quake3,
    MinecraftJava,
    MinecraftBedrock,
    /// Valve A2S (CS 1.6, CS2, TF2, GMod, Valheim…).
    Valve,
    Samp,
    /// Sorgulanamaz; sadece adres ve bağlanma bilgisi gösterilir.
    None,
}

#[derive(Clone, Copy, Debug)]
pub enum Join {
    /// `steam://connect/host:port`
    Steam,
    /// Oyunun .exe'si `+connect host:port` ile başlatılır.
    Exe(&'static str),
    /// Sabit bir adres şablonu: `{host}`, `{port}`, `{name}` yerine konur.
    Uri(&'static str),
    /// Doğrudan bağlanma yolu yok: adres kopyalanır.
    Copy,
}

pub struct Game {
    pub id: &'static str,
    pub name: &'static str,
    /// Kutucukta logo yoksa gösterilen kısa ad.
    pub short: &'static str,
    pub proto: Proto,
    pub default_port: u16,
    /// Sorgu portu = oyun portu + bu değer (Valheim: +1).
    pub query_offset: u16,
    pub join: Join,
    /// Kullanıcıya gösterilen bağlanma tarifi.
    pub how_to: &'static str,
}

pub const GAMES: &[Game] = &[
    Game { id: "cod2", name: "Call of Duty 2", short: "CoD2", proto: Proto::Quake3, default_port: 28960, query_offset: 0, join: Join::Exe("CoD2MP_s.exe"),
        how_to: "Oyunu aç → Multiplayer → Join Game. Ya da konsolu (~) açıp yaz: /connect {host}:{port}" },
    Game { id: "cod4", name: "Call of Duty 4", short: "CoD4", proto: Proto::Quake3, default_port: 28960, query_offset: 0, join: Join::Exe("iw3mp.exe"),
        how_to: "Oyunu aç → Join Game. Ya da konsolu (~) açıp yaz: /connect {host}:{port}" },
    Game { id: "coduo", name: "Call of Duty: United Offensive", short: "UO", proto: Proto::Quake3, default_port: 28960, query_offset: 0, join: Join::Exe("CoDUOMP.exe"),
        how_to: "Oyunu aç → Multiplayer → Join Game. Ya da konsolu (~) açıp yaz: /connect {host}:{port}" },
    Game { id: "cod1", name: "Call of Duty", short: "CoD", proto: Proto::Quake3, default_port: 28960, query_offset: 0, join: Join::Exe("CoDMP.exe"),
        how_to: "Oyunu aç → Multiplayer → Join Game. Ya da konsolu (~) açıp yaz: /connect {host}:{port}" },
    Game { id: "minecraft", name: "Minecraft (Java)", short: "MC", proto: Proto::MinecraftJava, default_port: 25565, query_offset: 0, join: Join::Copy,
        how_to: "Minecraft'ı aç → Çok Oyunculu → Sunucu Ekle. Adres: {host}:{port}" },
    Game { id: "minecraft-bedrock", name: "Minecraft (Bedrock)", short: "MCB", proto: Proto::MinecraftBedrock, default_port: 19132, query_offset: 0,
        join: Join::Uri("minecraft://?addExternalServer={name}|{host}:{port}"),
        how_to: "Bağlan'a basınca sunucu Minecraft'taki Sunucular listesine eklenir. Elle: Oyna → Sunucular → Sunucu Ekle, adres {host}, port {port}" },
    Game { id: "cs16", name: "Counter-Strike 1.6", short: "CS", proto: Proto::Valve, default_port: 27015, query_offset: 0, join: Join::Steam,
        how_to: "Bağlan'a basınca Steam oyunu açıp sunucuya sokar. Elle: konsola (~) connect {host}:{port}" },
    Game { id: "cs2", name: "Counter-Strike 2", short: "CS2", proto: Proto::Valve, default_port: 27015, query_offset: 0, join: Join::Steam,
        how_to: "Bağlan'a basınca Steam oyunu açıp sunucuya sokar. Elle: konsola (~) connect {host}:{port}" },
    Game { id: "source", name: "Diğer Steam oyunları (TF2, GMod…)", short: "SRC", proto: Proto::Valve, default_port: 27015, query_offset: 0, join: Join::Steam,
        how_to: "Bağlan'a basınca Steam oyunu açıp sunucuya sokar. Elle: konsola (~) connect {host}:{port}" },
    Game { id: "valheim", name: "Valheim", short: "VH", proto: Proto::Valve, default_port: 2456, query_offset: 1, join: Join::Copy,
        how_to: "Valheim → Oyuna Başla → karakter → Sunucuya Katıl → IP ekle: {host}:{port}" },
    Game { id: "samp", name: "GTA San Andreas (SA-MP)", short: "SA", proto: Proto::Samp, default_port: 7777, query_offset: 0, join: Join::Uri("samp://{host}:{port}"),
        how_to: "Bağlan'a basınca SA-MP açılır. Elle: SA-MP → Add Server → {host}:{port}" },
    Game { id: "mta", name: "MTA: San Andreas", short: "MTA", proto: Proto::None, default_port: 22003, query_offset: 0, join: Join::Uri("mtasa://{host}:{port}"),
        how_to: "Bağlan'a basınca MTA açılır. Elle: Quick Connect → {host}:{port}" },
    Game { id: "other", name: "Diğer", short: "?", proto: Proto::None, default_port: 0, query_offset: 0, join: Join::Copy,
        how_to: "Adres: {host}:{port}" },
];

pub fn find(id: &str) -> Option<&'static Game> {
    GAMES.iter().find(|g| g.id == id)
}

/// Arayüze giden özet.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    id: &'static str,
    name: &'static str,
    short: &'static str,
    default_port: u16,
    /// "steam" | "exe" | "uri" | "copy"
    join: &'static str,
    exe: Option<&'static str>,
    queryable: bool,
    how_to: &'static str,
}

pub fn catalog() -> Vec<GameInfo> {
    GAMES
        .iter()
        .map(|g| GameInfo {
            id: g.id,
            name: g.name,
            short: g.short,
            default_port: g.default_port,
            join: match g.join {
                Join::Steam => "steam",
                Join::Exe(_) => "exe",
                Join::Uri(_) => "uri",
                Join::Copy => "copy",
            },
            exe: match g.join {
                Join::Exe(e) => Some(e),
                _ => None,
            },
            queryable: g.proto != Proto::None,
            how_to: g.how_to,
        })
        .collect()
}

/// Bağlantı komutlarına giden adresin sadece güvenli karakterler içerdiğini doğrular
/// (topluluk listesinden gelen bir adres komut satırına ya da bağlantıya bir şey sokamasın).
pub fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && !host.starts_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_validation() {
        assert!(valid_host("cod2.gmid.gg"));
        assert!(valid_host("185.12.34.56"));
        assert!(!valid_host("1.2.3.4 +exec evil"));
        assert!(!valid_host("a\"b"));
        assert!(!valid_host("-flag"));
        assert!(!valid_host(""));
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = GAMES.iter().map(|g| g.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), GAMES.len());
    }
}
