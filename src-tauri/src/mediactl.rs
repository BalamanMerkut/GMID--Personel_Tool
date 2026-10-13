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

    pub fn now() -> Option<NowPlaying> {
        let s = session()?;
        let props = s.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
        let playing = s
            .GetPlaybackInfo()
            .and_then(|p| p.PlaybackStatus())
            .is_ok_and(|st| st == Status::Playing);
        let app = s
            .SourceAppUserModelId()
            .map(|h| app_name(&h.to_string()))
            .unwrap_or_default();
        let title = props.Title().map(|h| h.to_string()).unwrap_or_default();
        if title.trim().is_empty() {
            return None;
        }
        Some(NowPlaying {
            app,
            title,
            artist: props.Artist().map(|h| h.to_string()).unwrap_or_default(),
            playing,
            thumb: thumb(&props),
        })
    }

    pub fn control(action: &str) -> Result<(), String> {
        let s = session().ok_or("Çalan bir şey yok")?;
        let op = match action {
            "next" => s.TrySkipNextAsync(),
            "prev" => s.TrySkipPreviousAsync(),
            _ => s.TryTogglePlayPauseAsync(),
        }
        .map_err(|e| e.message().to_string())?;
        op.join().map_err(|e| e.message().to_string())?;
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
