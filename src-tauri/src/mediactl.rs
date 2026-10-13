//! Şimdi çalan: Windows'un medya oturumu (Spotify, tarayıcıda YouTube, Apple Music, VLC… sistemin medya tuşlarına
//! bağlanan her program). Oyun İçi Panel'den oyundan çıkmadan önceki / oynat-duraklat / sonraki.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NowPlaying {
    /// Programın adı ("Spotify", "Chrome"…).
    pub app: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    /// Kapak (data: URL, JPEG/PNG); yoksa `None`.
    pub thumb: Option<String>,
    /// Ham uygulama kimliği (ses karıştırıcısındaki programla eşlemek için).
    pub source: String,
    /// Karıştırma açık mı; program desteklemiyorsa `None`.
    pub shuffle: Option<bool>,
    /// "none" | "track" | "list"; desteklemiyorsa `None`.
    pub repeat: Option<String>,
    /// Saniye; program bildirmiyorsa `None`.
    pub position: Option<f64>,
    pub duration: Option<f64>,
}

/// Tekrar düğmesi: kapalı → liste → parça → kapalı.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn next_repeat(current: Option<&str>) -> &'static str {
    match current {
        Some("list") => "track",
        Some("track") => "none",
        _ => "list",
    }
}

/// "Spotify.exe", "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify", "Chrome" → okunur ad.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn app_name(aumid: &str) -> String {
    let lower = aumid.to_lowercase();
    for (key, name) in [
        ("spotify", "Spotify"),
        ("chrome", "Chrome"),
        ("msedge", "Edge"),
        ("firefox", "Firefox"),
        ("opera", "Opera"),
        ("brave", "Brave"),
        ("vlc", "VLC"),
        ("applemusic", "Apple Music"),
        ("zunemusic", "Medya Oynatıcı"),
        ("deezer", "Deezer"),
        ("youtube", "YouTube Music"),
        ("gmid", "GMID Tool"),
    ] {
        if lower.contains(key) {
            return name.into();
        }
    }
    let base = aumid
        .rsplit(['\\', '!'])
        .next()
        .unwrap_or(aumid)
        .trim_end_matches(".exe");
    base.to_string()
}

#[cfg(windows)]
mod win {
    use super::{app_name, NowPlaying};
    use base64::Engine as _;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
    };
    use windows::Media::MediaPlaybackAutoRepeatMode as Repeat;
    use windows::Storage::Streams::DataReader;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    fn session() -> Option<Session> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        Manager::RequestAsync()
            .ok()?
            .join()
            .ok()?
            .GetCurrentSession()
            .ok()
    }

    /// Kapak: en çok 512 KB.
    fn thumb(
        props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties,
    ) -> Option<String> {
        let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
        let size = stream.Size().ok()?;
        if size == 0 || size > 512 * 1024 {
            return None;
        }
        let kind = stream
            .ContentType()
            .map(|s| s.to_string())
            .unwrap_or_default();
        let reader = DataReader::CreateDataReader(&stream).ok()?;
        let op: windows_future::IAsyncOperation<u32> =
            windows_core::Interface::cast(&reader.LoadAsync(size as u32).ok()?).ok()?;
        let got = op.join().ok()? as usize;
        let mut buf = vec![0u8; got];
        reader.ReadBytes(&mut buf).ok()?;
        let mime = if kind.starts_with("image/") {
            kind
        } else {
            "image/jpeg".into()
        };
        Some(format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&buf)
        ))
    }

    /// Windows zamanı (1601'den beri 100 ns) şimdi.
    fn now_ticks() -> i64 {
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64 / 100)
            .unwrap_or(0);
        unix + 116_444_736_000_000_000
    }

    pub fn now() -> Option<NowPlaying> {
        let s = session()?;
        let props = s.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
        let info = s.GetPlaybackInfo().ok();
        let playing = info
            .as_ref()
            .and_then(|p| p.PlaybackStatus().ok())
            .is_some_and(|st| st == Status::Playing);
        let source = s
            .SourceAppUserModelId()
            .map(|h| h.to_string())
            .unwrap_or_default();
        let title = props.Title().map(|h| h.to_string()).unwrap_or_default();
        if title.trim().is_empty() {
            return None;
        }
        let controls = info.as_ref().and_then(|p| p.Controls().ok());
        let shuffle = controls
            .as_ref()
            .filter(|c| c.IsShuffleEnabled().unwrap_or(false))
            .and_then(|_| info.as_ref()?.IsShuffleActive().ok()?.Value().ok());
        let repeat = controls
            .as_ref()
            .filter(|c| c.IsRepeatEnabled().unwrap_or(false))
            .and_then(|_| info.as_ref()?.AutoRepeatMode().ok()?.Value().ok())
            .map(|m| {
                match m {
                    Repeat::Track => "track",
                    Repeat::List => "list",
                    _ => "none",
                }
                .to_string()
            });
        let (mut position, mut duration) = (None, None);
        if let Ok(t) = s.GetTimelineProperties() {
            let start = t.StartTime().map(|x| x.Duration).unwrap_or(0);
            let end = t.EndTime().map(|x| x.Duration).unwrap_or(0);
            if end > start {
                let mut pos = t.Position().map(|x| x.Duration).unwrap_or(0) - start;
                // Konum son güncellemedeki değer: çalıyorsa aradan geçen süre eklenir.
                if playing {
                    if let Ok(at) = t.LastUpdatedTime() {
                        pos += (now_ticks() - at.UniversalTime).clamp(0, end - start);
                    }
                }
                duration = Some((end - start) as f64 / 1e7);
                position = Some(pos.clamp(0, end - start) as f64 / 1e7);
            }
        }
        Some(NowPlaying {
            app: app_name(&source),
            title,
            artist: props.Artist().map(|h| h.to_string()).unwrap_or_default(),
            playing,
            thumb: thumb(&props),
            source,
            shuffle,
            repeat,
            position,
            duration,
        })
    }

    /// "toggle" | "next" | "prev" | "shuffle" | "repeat" | "seek:<saniye>"
    pub fn control(action: &str) -> Result<(), String> {
        let s = session().ok_or("Çalan bir şey yok")?;
        let err = |e: windows::core::Error| e.message().to_string();
        let op = if let Some(secs) = action.strip_prefix("seek:") {
            let secs: f64 = secs.parse().map_err(|_| "Geçersiz konum")?;
            let start = s
                .GetTimelineProperties()
                .and_then(|t| t.StartTime())
                .map(|x| x.Duration)
                .unwrap_or(0);
            s.TryChangePlaybackPositionAsync(start + (secs.max(0.0) * 1e7) as i64)
        } else if action == "shuffle" {
            let on = s
                .GetPlaybackInfo()
                .and_then(|p| p.IsShuffleActive())
                .and_then(|v| v.Value())
                .unwrap_or(false);
            s.TryChangeShuffleActiveAsync(!on)
        } else if action == "repeat" {
            let cur = s
                .GetPlaybackInfo()
                .and_then(|p| p.AutoRepeatMode())
                .and_then(|v| v.Value())
                .ok()
                .map(|m| match m {
                    Repeat::Track => "track",
                    Repeat::List => "list",
                    _ => "none",
                });
            let next = match super::next_repeat(cur) {
                "track" => Repeat::Track,
                "list" => Repeat::List,
                _ => Repeat::None,
            };
            s.TryChangeAutoRepeatModeAsync(next)
        } else {
            match action {
                "next" => s.TrySkipNextAsync(),
                "prev" => s.TrySkipPreviousAsync(),
                _ => s.TryTogglePlayPauseAsync(),
            }
        }
        .map_err(err)?;
        if !op.join().map_err(err)? {
            return Err("Bu program bunu desteklemiyor.".into());
        }
        Ok(())
    }
}

#[cfg(windows)]
pub use win::{control, now};

#[cfg(not(windows))]
pub fn now() -> Option<NowPlaying> {
    None
}

#[cfg(not(windows))]
pub fn control(_action: &str) -> Result<(), String> {
    Err("Yalnızca Windows'ta".into())
}

#[cfg(test)]
mod tests {
    #[test]
    fn repeat_cycles() {
        assert_eq!(super::next_repeat(None), "list");
        assert_eq!(super::next_repeat(Some("none")), "list");
        assert_eq!(super::next_repeat(Some("list")), "track");
        assert_eq!(super::next_repeat(Some("track")), "none");
    }

    #[test]
    fn app_names() {
        assert_eq!(super::app_name("Spotify.exe"), "Spotify");
        assert_eq!(
            super::app_name("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"),
            "Spotify"
        );
        assert_eq!(super::app_name("Chrome"), "Chrome");
        assert_eq!(super::app_name("308046B0AF4A39CB"), "308046B0AF4A39CB");
        assert_eq!(super::app_name(r"C:\Apps\foobar2000.exe"), "foobar2000");
    }
}
