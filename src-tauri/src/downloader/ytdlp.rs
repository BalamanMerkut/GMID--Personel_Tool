//! yt-dlp ile konuşma: videoyu çözümleme (`-J`), kalite seçenekleri, indirme komutu ve
//! ilerleme satırlarının okunması.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Kalite kartlarından biri.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QualityOption {
    /// "1080", "720", "mp3", "best"
    pub id: String,
    pub label: String,
    /// "60 fps · MP4"
    pub detail: String,
    /// Tahmini boyut (bayt).
    pub size: Option<u64>,
    pub audio_only: bool,
    pub recommended: bool,
    /// yt-dlp'ye giden biçim seçimi (arayüze gitmez).
    #[serde(skip_serializing)]
    pub format: String,
    /// Görüntü ve ses ayrı ayrı mı iniyor (ilerleme hesabı için).
    #[serde(skip_serializing)]
    pub parts: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    pub url: String,
    pub video_id: String,
    pub title: String,
    pub uploader: String,
    pub site: String,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub options: Vec<QualityOption>,
    /// Birden çok ses dili (dublaj) varsa hepsi; tek dilse boş.
    #[serde(default)]
    pub audio_langs: Vec<Track>,
    /// Sitenin elle hazırlanmış altyazıları (otomatik olanlar hariç).
    #[serde(default)]
    pub subtitles: Vec<Track>,
}

/// Ses dili ya da altyazı: `code` yt-dlp'nin kullandığı ad ("tr", "tur", "en-US"…).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Track {
    pub code: String,
    pub label: String,
}

/// Kullanıcının seçtiği ses dili ve altyazılar.
#[derive(Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Langs {
    /// `None`: sitenin varsayılan sesi.
    pub audio: Option<String>,
    #[serde(default)]
    pub subs: Vec<String>,
}

#[derive(Deserialize, Default)]
struct RawFormat {
    format_id: Option<String>,
    ext: Option<String>,
    vcodec: Option<String>,
    acodec: Option<String>,
    height: Option<u32>,
    fps: Option<f64>,
    tbr: Option<f64>,
    abr: Option<f64>,
    filesize: Option<u64>,
    filesize_approx: Option<u64>,
    protocol: Option<String>,
    has_drm: Option<serde_json::Value>,
    format_note: Option<String>,
    language: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawThumb {
    url: Option<String>,
    width: Option<u32>,
}

#[derive(Deserialize)]
struct RawEntry {
    id: Option<String>,
    url: Option<String>,
    title: Option<String>,
    duration: Option<f64>,
    channel: Option<String>,
    uploader: Option<String>,
    #[serde(default)]
    thumbnails: Vec<RawThumb>,
}

#[derive(Deserialize)]
struct RawInfo {
    #[serde(rename = "_type")]
    kind: Option<String>,
    #[serde(default)]
    entries: Vec<RawEntry>,
    playlist_count: Option<u32>,
    id: Option<String>,
    title: Option<String>,
    uploader: Option<String>,
    channel: Option<String>,
    extractor_key: Option<String>,
    webpage_url: Option<String>,
    duration: Option<f64>,
    thumbnail: Option<String>,
    is_live: Option<bool>,
    #[serde(default)]
    formats: Vec<RawFormat>,
    // Biçim listesi olmayan siteler (tek dosya)
    url: Option<String>,
    ext: Option<String>,
    height: Option<u32>,
    filesize: Option<u64>,
    filesize_approx: Option<u64>,
    #[serde(default)]
    subtitles: BTreeMap<String, serde_json::Value>,
}

/// "tur", "tr-TR", "TR" → "tr" (karşılaştırma için).
pub fn lang_base(code: &str) -> String {
    let base = code
        .split(['-', '_'])
        .next()
        .unwrap_or(code)
        .to_ascii_lowercase();
    let two = match base.as_str() {
        "tur" => "tr",
        "eng" => "en",
        "ger" | "deu" => "de",
        "fre" | "fra" => "fr",
        "spa" => "es",
        "ita" => "it",
        "rus" => "ru",
        "ara" => "ar",
        "jpn" => "ja",
        "kor" => "ko",
        "por" => "pt",
        "chi" | "zho" => "zh",
        "dut" | "nld" => "nl",
        "pol" => "pl",
        "swe" => "sv",
        "gre" | "ell" => "el",
        "hin" => "hi",
        "per" | "fas" => "fa",
        "aze" => "az",
        "ukr" => "uk",
        _ => return base,
    };
    two.to_string()
}

/// Dil kodunun Türkçe adı ("tr" → "Türkçe"); bilinmiyorsa kodun kendisi.
pub fn lang_label(code: &str) -> String {
    let name = match lang_base(code).as_str() {
        "tr" => "Türkçe",
        "en" => "İngilizce",
        "de" => "Almanca",
        "fr" => "Fransızca",
        "es" => "İspanyolca",
        "it" => "İtalyanca",
        "ru" => "Rusça",
        "ar" => "Arapça",
        "ja" => "Japonca",
        "ko" => "Korece",
        "pt" => "Portekizce",
        "zh" => "Çince",
        "nl" => "Felemenkçe",
        "pl" => "Lehçe",
        "sv" => "İsveççe",
        "el" => "Yunanca",
        "hi" => "Hintçe",
        "fa" => "Farsça",
        "az" => "Azerice",
        "uk" => "Ukraynaca",
        "und" | "" => return "Bilinmeyen dil".into(),
        _ => return code.to_string(),
    };
    // "en-US" gibi bölgeli kodlarda bölge de görünsün.
    match code.split_once(['-', '_']) {
        Some((_, region)) if !region.is_empty() => format!("{name} ({region})"),
        _ => name.to_string(),
    }
}

/// yt-dlp biçim süzgecine güvenle yazılabilecek kod mu?
pub fn valid_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 24
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

impl RawFormat {
    fn has_video(&self) -> bool {
        match self.vcodec.as_deref() {
            Some("none") => false,
            Some(_) => true,
            // Bilinmiyorsa: yüksekliği varsa ya da sesi de bilinmiyorsa (doğrudan dosya bağlantısı)
            // görüntü var say.
            None => self.height.is_some() || self.acodec.is_none(),
        }
    }
    fn has_audio(&self) -> bool {
        // "none" açıkça sessiz demek; bilinmiyorsa (X gibi) sesli kabul edilir.
        self.acodec.as_deref() != Some("none")
    }
    fn usable(&self) -> bool {
        let drm = matches!(&self.has_drm, Some(serde_json::Value::Bool(true)));
        let proto = self.protocol.as_deref().unwrap_or("");
        let note = self.format_note.as_deref().unwrap_or("");
        self.format_id.is_some()
            && !drm
            && proto != "mhtml"
            && !note.contains("storyboard")
            && (self.has_video() || self.acodec.as_deref().is_some_and(|a| a != "none"))
    }
    fn size(&self, duration: Option<f64>) -> Option<u64> {
        self.filesize.or(self.filesize_approx).or_else(|| {
            let kbps = self.tbr.or(self.abr)?;
            Some((kbps * 1000.0 / 8.0 * duration?) as u64)
        })
    }
    fn codec(&self) -> &str {
        self.vcodec.as_deref().unwrap_or("")
    }
    /// Görüntü tercihi: önce her yerde açılan H.264/MP4, sonra VP9, sonra AV1; doğrudan
    /// indirme parçalı yayından (m3u8) önce gelir.
    fn video_rank(&self) -> (u8, u8, u64) {
        let c = self.codec();
        let codec = if c.starts_with("avc") || c.starts_with("h264") {
            3
        } else if c.starts_with("vp9") || c.starts_with("vp09") {
            2
        } else if c.starts_with("av01") {
            1
        } else {
            0
        };
        let proto = match self.protocol.as_deref() {
            Some(p) if p.starts_with("http") => 1,
            _ => 0,
        };
        (proto, codec, (self.tbr.unwrap_or(0.0) * 10.0) as u64)
    }
    fn audio_rank(&self) -> (u8, u64) {
        let m4a = (self.ext.as_deref() == Some("m4a")) as u8;
        (m4a, (self.abr.or(self.tbr).unwrap_or(0.0) * 10.0) as u64)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    pub duration: Option<f64>,
    pub uploader: String,
    pub thumbnail: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistProbe {
    pub url: String,
    pub title: String,
    pub uploader: String,
    pub site: String,
    pub entries: Vec<PlaylistEntry>,
}

/// Bağlantı tek bir video mu, bir çalma listesi mi?
#[derive(Serialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Parsed {
    Video(Probe),
    Playlist(PlaylistProbe),
}

/// Video ayrıca çözümlenmeden kullanılan kalite: "1080" → en fazla 1080p, "mp3" → sadece ses.
/// Önce MP4+M4A, sonra WebM+Opus eşleşmesi aranır (uygulama içinde oynatılabilsin diye).
pub fn generic_option(id: &str) -> QualityOption {
    if id == "mp3" {
        return QualityOption {
            id: "mp3".into(),
            label: "Sadece ses".into(),
            detail: "MP3".into(),
            size: None,
            audio_only: true,
            recommended: false,
            format: "ba/b".into(),
            parts: 1,
        };
    }
    let h: u32 = id.parse().unwrap_or(1080).clamp(144, 4320);
    QualityOption {
        id: h.to_string(),
        label: height_label(h),
        detail: String::new(),
        size: None,
        audio_only: false,
        recommended: h == 1080,
        format: format!(
            "bv*[height<={h}][ext=mp4]+ba[ext=m4a]/bv*[height<={h}][ext=webm]+ba[ext=webm]/bv*[height<={h}]+ba/b[height<={h}]/bv*+ba/b"
        ),
        parts: 2,
    }
}

/// İstenen kaliteyi seçer; tam yoksa istenenin altındaki en yükseği, o da yoksa en düşüğü.
/// "auto" (ya da bilinmeyen) önerileni seçer. Eklentinin sağ tık menüsündeki sabit kaliteler için.
pub fn choose(options: &[QualityOption], id: &str) -> Option<QualityOption> {
    if let Some(o) = options.iter().find(|o| o.id == id) {
        return Some(o.clone());
    }
    if id == "mp3" {
        return options.iter().find(|o| o.audio_only).cloned();
    }
    let videos = options.iter().filter(|o| !o.audio_only);
    if let Ok(want) = id.parse::<u32>() {
        let height = |o: &&QualityOption| o.id.parse::<u32>().ok();
        let below = videos
            .clone()
            .filter(|o| height(o).is_some_and(|h| h <= want))
            .max_by_key(|o| height(o));
        if let Some(o) = below.or_else(|| videos.clone().min_by_key(|o| height(o))) {
            return Some(o.clone());
        }
    }
    options
        .iter()
        .find(|o| o.recommended)
        .or(options.first())
        .cloned()
}

/// `yt-dlp -J` çıktısını ayrıştırır: tek video ya da (düz) çalma listesi.
pub fn parse_any(json: &[u8], fallback_url: &str) -> Result<Parsed, String> {
    let info: RawInfo =
        serde_json::from_slice(json).map_err(|_| "Video bilgisi okunamadı".to_string())?;
    if info.kind.as_deref() == Some("playlist") {
        return parse_playlist(info, fallback_url).map(Parsed::Playlist);
    }
    parse_info(info, fallback_url).map(Parsed::Video)
}

fn parse_playlist(info: RawInfo, fallback_url: &str) -> Result<PlaylistProbe, String> {
    let entries: Vec<PlaylistEntry> = info
        .entries
        .into_iter()
        .filter_map(|e| {
            let id = e.id?;
            let url = e
                .url
                .filter(|u| u.starts_with("http"))
                .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}"));
            // Silinmiş / gizli videolar listede başlıksız durur.
            let title = e
                .title
                .filter(|t| !t.starts_with('[') || !t.ends_with(" video]"))?;
            let thumbnail = e
                .thumbnails
                .iter()
                .filter(|t| t.url.is_some())
                .min_by_key(|t| t.width.unwrap_or(0).abs_diff(480))
                .and_then(|t| t.url.clone());
            Some(PlaylistEntry {
                id,
                url,
                title,
                duration: e.duration,
                uploader: e.channel.or(e.uploader).unwrap_or_default(),
                thumbnail,
            })
        })
        .collect();
    if entries.is_empty() {
        return Err("Çalma listesi boş ya da videoları gizli.".into());
    }
    let _ = info.playlist_count;
    Ok(PlaylistProbe {
        url: info.webpage_url.unwrap_or_else(|| fallback_url.to_string()),
        title: info.title.unwrap_or_else(|| "Çalma listesi".into()),
        uploader: info.uploader.or(info.channel).unwrap_or_default(),
        site: info.extractor_key.unwrap_or_else(|| "Youtube".into()),
        entries,
    })
}

/// Tek video için (testlerde ve eski çağrılarda) kısa yol.
#[cfg(test)]
pub fn parse_probe(json: &[u8], fallback_url: &str) -> Result<Probe, String> {
    match parse_any(json, fallback_url)? {
        Parsed::Video(p) => Ok(p),
        Parsed::Playlist(_) => Err("Bu bir çalma listesi.".into()),
    }
}

fn parse_info(info: RawInfo, fallback_url: &str) -> Result<Probe, String> {
    if info.is_live == Some(true) {
        return Err("Canlı yayın indirilemez. Yayın bitince tekrar dene.".into());
    }
    let duration = info.duration;
    let mut formats = info.formats;
    if formats.is_empty() && info.url.is_some() {
        formats.push(RawFormat {
            format_id: Some("0".into()),
            ext: info.ext.clone(),
            height: info.height,
            filesize: info.filesize,
            filesize_approx: info.filesize_approx,
            ..Default::default()
        });
    }
    let usable: Vec<&RawFormat> = formats.iter().filter(|f| f.usable()).collect();
    let audio = usable
        .iter()
        .filter(|f| !f.has_video())
        .max_by_key(|f| f.audio_rank())
        .copied();
    // WebM (VP9) görüntüyle WebM (Opus) ses: dosya WebM olur ve uygulama içinde oynatılabilir
    // (VP9 + M4A ancak MKV'ye sığar, MKV de uygulamada oynamaz).
    let webm_audio = usable
        .iter()
        .filter(|f| !f.has_video() && f.ext.as_deref() == Some("webm"))
        .max_by_key(|f| (f.abr.or(f.tbr).unwrap_or(0.0) * 10.0) as u64)
        .copied();

    let mut heights: Vec<u32> = usable
        .iter()
        .filter(|f| f.has_video())
        .filter_map(|f| f.height)
        .filter(|h| *h >= 144)
        .collect();
    heights.sort_unstable_by(|a, b| b.cmp(a));
    heights.dedup();
    // Çok seçenek varsa en düşükleri gösterme.
    if heights.iter().filter(|h| **h >= 360).count() >= 3 {
        heights.retain(|h| *h >= 360);
    }

    let mut options = Vec::new();
    for h in &heights {
        let Some(v) = usable
            .iter()
            .filter(|f| f.has_video() && f.height == Some(*h))
            .max_by_key(|f| f.video_rank())
        else {
            continue;
        };
        let vid = v.format_id.as_deref().unwrap_or_default();
        let generic = format!("bv*[height<={h}]+ba/b[height<={h}]/bv*+ba/b");
        let pair = if v.ext.as_deref() == Some("webm") {
            webm_audio.or(audio)
        } else {
            audio
        };
        let (format, size, parts) = match pair {
            Some(a) if !v.has_audio() || v.acodec.is_none() && v.vcodec.is_some() => {
                let aid = a.format_id.as_deref().unwrap_or_default();
                let size = v.size(duration).zip(a.size(duration)).map(|(x, y)| x + y);
                (format!("{vid}+{aid}/{generic}"), size, 2)
            }
            _ => (format!("{vid}/{generic}"), v.size(duration), 1),
        };
        let fps = v
            .fps
            .filter(|f| *f > 31.0)
            .map(|f| format!("{} fps", f.round()));
        let container = match (v.ext.as_deref(), pair.and_then(|a| a.ext.as_deref())) {
            (Some("webm"), Some("webm")) | (Some("webm"), None) => "WebM",
            (Some("mp4"), _) => "MP4",
            _ if v.codec().starts_with("avc") => "MP4",
            _ => "MKV",
        };
        let detail = [fps, Some(container.to_string())]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
        options.push(QualityOption {
            id: h.to_string(),
            label: height_label(*h),
            detail,
            size,
            audio_only: false,
            recommended: false,
            format,
            parts,
        });
    }
    // Yüksekliği bilinmeyen (doğrudan dosya gibi) videolar: tek seçenek.
    if options.is_empty() {
        if let Some(v) = usable
            .iter()
            .filter(|f| f.has_video())
            .max_by_key(|f| f.video_rank())
        {
            let ext = v.ext.as_deref().unwrap_or("").to_uppercase();
            options.push(QualityOption {
                id: "best".into(),
                label: "En iyi kalite".into(),
                detail: ext,
                size: v.size(duration),
                audio_only: false,
                recommended: true,
                format: format!("{}/bv*+ba/b", v.format_id.as_deref().unwrap_or("b")),
                parts: 1,
            });
        }
    }
    // Önerilen: 1080p ya da altındaki en yüksek.
    if let Some(o) = options
        .iter_mut()
        .find(|o| o.id.parse::<u32>().is_ok_and(|h| h <= 1080))
    {
        o.recommended = true;
    }
    if let Some(a) = audio.or_else(|| usable.iter().find(|f| f.has_audio()).copied()) {
        options.push(QualityOption {
            id: "mp3".into(),
            label: "Sadece ses".into(),
            detail: "MP3".into(),
            size: a.size(duration),
            audio_only: true,
            recommended: false,
            format: format!("{}/ba/b", a.format_id.as_deref().unwrap_or("ba")),
            parts: 1,
        });
    }
    if options.is_empty() {
        return Err("Bu sayfada indirilebilir video bulunamadı.".into());
    }

    // Dublajlar: ayrı ses parçaları farklı dillerdeyse seçilebilsin.
    let mut audio_langs: Vec<Track> = Vec::new();
    for f in usable.iter().filter(|f| !f.has_video()) {
        if let Some(code) = f.language.as_deref().filter(|c| valid_code(c)) {
            if !audio_langs.iter().any(|t| t.code == code) {
                audio_langs.push(Track {
                    code: code.to_string(),
                    label: lang_label(code),
                });
            }
        }
    }
    if audio_langs.len() < 2 {
        audio_langs.clear();
    }
    let subtitles: Vec<Track> = info
        .subtitles
        .iter()
        .filter(|(code, _)| {
            valid_code(code) && !matches!(code.as_str(), "live_chat" | "danmaku" | "rechat")
        })
        .map(|(code, v)| Track {
            code: code.clone(),
            label: v
                .get(0)
                .and_then(|x| x.get("name"))
                .and_then(|n| n.as_str())
                .filter(|n| !n.trim().is_empty())
                .map(|n| n.trim().to_string())
                .unwrap_or_else(|| lang_label(code)),
        })
        .collect();

    Ok(Probe {
        url: info.webpage_url.unwrap_or_else(|| fallback_url.to_string()),
        video_id: info.id.unwrap_or_default(),
        title: info.title.unwrap_or_else(|| "Video".into()),
        uploader: info.uploader.or(info.channel).unwrap_or_default(),
        site: info.extractor_key.unwrap_or_else(|| "Generic".into()),
        duration,
        thumbnail: info.thumbnail,
        options,
        audio_langs,
        subtitles,
    })
}

fn height_label(h: u32) -> String {
    match h {
        2160.. => format!("{h}p · 4K"),
        1440..=2159 => format!("{h}p · 2K"),
        _ => format!("{h}p"),
    }
}

/// Sitenin dosyaların ineceği alt klasörü.
pub fn site_folder(site: &str) -> &'static str {
    match site.to_ascii_lowercase().as_str() {
        s if s.starts_with("youtube") => "YouTube",
        s if s.starts_with("twitter") => "Twitter",
        s if s.starts_with("instagram") => "Instagram",
        s if s.starts_with("tiktok") => "TikTok",
        _ => "Diger",
    }
}

/// Seçilen dublaj varsa önce o dildeki ses denenir; yoksa normal seçim.
fn format_for(option: &QualityOption, audio_lang: Option<&str>) -> String {
    let Some(l) = audio_lang.filter(|l| valid_code(l)) else {
        return option.format.clone();
    };
    if option.audio_only {
        return format!("ba[language={l}]/{}", option.format);
    }
    let h = option
        .id
        .parse::<u32>()
        .map(|h| format!("[height<={h}]"))
        .unwrap_or_default();
    format!(
        "bv*{h}+ba[language={l}]/b{h}[language={l}]/{}",
        option.format
    )
}

/// Yalnızca altyazıları indiren komutun argümanları; dosyalar videonun yanına
/// `<video adı>.<dil>.vtt` olarak yazılır.
pub fn subtitle_args(url: &str, video_file: &Path, langs: &[String], ctx: &Context) -> Vec<String> {
    let langs: Vec<&str> = langs
        .iter()
        .map(String::as_str)
        .filter(|l| valid_code(l))
        .collect();
    let stem = video_file.with_extension("");
    let mut a: Vec<String> = vec![
        "--skip-download".into(),
        "--no-playlist".into(),
        "--no-warnings".into(),
        "--write-subs".into(),
        "--sub-langs".into(),
        langs.join(","),
        "--sub-format".into(),
        "vtt/srt/best".into(),
        "--convert-subs".into(),
        "vtt".into(),
        "-o".into(),
        format!("{}.%(ext)s", stem.to_string_lossy().replace('%', "%%")),
    ];
    a.extend(ctx.args());
    a.push("--".into());
    a.push(url.into());
    a
}

/// İndirme komutunun argümanları.
/// Eklentinin yakaladığı videolar için: sayfa adresi, tarayıcı kimliği ve sayfa başlığı.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub referer: Option<String>,
    pub user_agent: Option<String>,
    /// Dosya adı için sayfa başlığı (site videoyu tanımıyorsa).
    pub title: Option<String>,
    /// Seçilen ses dili (dublaj); yt-dlp'nin dil kodu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_lang: Option<String>,
    /// İndirilecek altyazılar (sitenin dil kodları).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub_langs: Vec<String>,
    /// Eklentinin sayfada yakaladığı altyazı dosyaları (oynatıcı ayrı yüklüyorsa).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_subs: Vec<ExtraSub>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ExtraSub {
    pub url: String,
    #[serde(default)]
    pub lang: String,
}

impl Context {
    /// yt-dlp argümanları (çözümleme ve indirme için ortak).
    pub fn args(&self) -> Vec<String> {
        let mut a = Vec::new();
        if let Some(r) = self.referer.as_deref().filter(|r| r.starts_with("http")) {
            a.push("--referer".into());
            a.push(r.to_string());
            if let Some(origin) = origin_of(r) {
                a.push("--add-header".into());
                a.push(format!("Origin:{origin}"));
            }
        }
        if let Some(ua) = self.user_agent.as_deref().filter(|u| !u.is_empty()) {
            a.push("--user-agent".into());
            a.push(ua.chars().filter(|c| !c.is_control()).take(400).collect());
        }
        a
    }
}

fn origin_of(url: &str) -> Option<String> {
    let rest = url.split_once("://")?;
    let host = rest.1.split(['/', '?', '#']).next()?;
    Some(format!("{}://{host}", rest.0))
}

/// Dosya/klasör adında Windows'un kabul etmediği karakterleri temizler.
pub fn safe_name(name: &str, max: usize) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => ' ',
            c => c,
        })
        .take(max)
        .collect();
    let trimmed = cleaned.trim().trim_end_matches(['.', ' ']).to_string();
    if trimmed.is_empty() {
        "Video".into()
    } else {
        trimmed
    }
}

pub fn download_args(
    url: &str,
    option: &QualityOption,
    out_dir: &Path,
    thumbs_dir: &Path,
    cookies: Option<&Path>,
    ctx: &Context,
) -> Vec<String> {
    // Sayfa başlığı verildiyse (tanınmayan site) dosya adı o olur; yoksa sitenin başlığı.
    let name = ctx
        .title
        .as_deref()
        .map(|t| safe_name(t, 150).replace('%', "%%"))
        .unwrap_or_else(|| "%(title).150B".into());
    let mut a: Vec<String> = vec![
        "--no-playlist".into(),
        "--newline".into(),
        "--progress".into(),
        "--no-mtime".into(),
        "--windows-filenames".into(),
        // Parçalı yayınlarda (HLS/DASH) 4 parça birden: IDM'deki gibi daha hızlı.
        "--concurrent-fragments".into(),
        "4".into(),
        // Kopan bağlantılarda ve geçici engellerde yt-dlp kendisi de birkaç kez dener (artan bekleme).
        "--retries".into(),
        "10".into(),
        "--fragment-retries".into(),
        "10".into(),
        "--retry-sleep".into(),
        "exp=1:15".into(),
        "--retry-sleep".into(),
        "fragment:exp=1:15".into(),
        "-f".into(),
        format_for(option, ctx.audio_lang.as_deref()),
        "-o".into(),
        out_dir
            .join(if option.audio_only {
                // Ses ayrı adla: aynı videonun MP4'ü varsa yt-dlp onu ses için kullanıp silmesin.
                format!("{name} [%(id)s] (ses).%(ext)s")
            } else {
                format!("{name} [%(id)s]%(height& {{}}p|)s.%(ext)s")
            })
            .to_string_lossy()
            .into_owned(),
        "-o".into(),
        format!(
            "thumbnail:{}",
            thumbs_dir.join("%(id)s.%(ext)s").to_string_lossy()
        ),
        "--write-thumbnail".into(),
        "--convert-thumbnails".into(),
        "jpg".into(),
        "--progress-template".into(),
        "download:GMIDP %(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(info.format_id)s".into(),
        "--progress-template".into(),
        "postprocess:GMIDPP %(progress.postprocessor)s|%(progress.status)s".into(),
        "--print".into(),
        "after_move:GMIDFILE %(filepath)s".into(),
    ];
    if option.audio_only {
        a.extend(
            // Sabit bit hızı (CBR): değişken hızlı MP3'te ileri sarınca oynatıcı yeri tam bulamıyor
            // ve şarkı sözleri kayıyordu.
            ["-x", "--audio-format", "mp3", "--audio-quality", "320K"]
                .iter()
                .map(|s| s.to_string()),
        );
    } else {
        // MP4 ya da WebM: ikisi de uygulama içinde oynar; uymazsa MKV.
        a.extend(["--merge-output-format".into(), "mp4/webm/mkv".into()]);
    }
    if let Some(c) = cookies {
        a.push("--cookies".into());
        a.push(c.to_string_lossy().into_owned());
    }
    a.extend(ctx.args());
    a.push("--".into());
    a.push(url.into());
    a
}

#[derive(Debug, PartialEq)]
pub enum Line {
    Progress {
        downloaded: u64,
        total: Option<u64>,
        speed: Option<f64>,
        eta: Option<u64>,
        format_id: String,
    },
    /// Birleştirme / MP3'e çevirme
    Processing(String),
    File(String),
    Other,
}

pub fn parse_line(line: &str) -> Line {
    let num = |s: &str| {
        s.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v >= 0.0)
    };
    if let Some(rest) = line.strip_prefix("GMIDP ") {
        let f: Vec<&str> = rest.split('|').collect();
        if f.len() >= 6 {
            return Line::Progress {
                downloaded: num(f[0]).unwrap_or(0.0) as u64,
                total: num(f[1]).or_else(|| num(f[2])).map(|v| v as u64),
                speed: num(f[3]),
                eta: num(f[4]).map(|v| v as u64),
                format_id: f[5].trim().to_string(),
            };
        }
    }
    if let Some(rest) = line.strip_prefix("GMIDPP ") {
        let pp = rest.split('|').next().unwrap_or("").trim();
        return Line::Processing(pp.to_string());
    }
    if let Some(rest) = line.strip_prefix("GMIDFILE ") {
        return Line::File(rest.trim().to_string());
    }
    if line.starts_with("[Merger]") {
        return Line::Processing("Merger".into());
    }
    if line.starts_with("[ExtractAudio]") {
        return Line::Processing("ExtractAudio".into());
    }
    Line::Other
}

/// yt-dlp'nin hata satırlarını anlaşılır Türkçeye çevirir.
pub fn friendly_error(stderr: &str) -> String {
    let last = stderr
        .lines()
        .rev()
        .find(|l| l.starts_with("ERROR:"))
        .or_else(|| stderr.lines().rev().find(|l| !l.trim().is_empty()))
        .unwrap_or("")
        .trim();
    let l = last.to_ascii_lowercase();
    let msg = if l.contains("sign in to confirm") || l.contains("not a bot") {
        "YouTube bot doğrulaması istedi. Biraz sonra tekrar dene ya da tarayıcı eklentisinden indir."
    } else if l.contains("private video") || l.contains("private") && l.contains("video") {
        "Bu video gizli."
    } else if l.contains("login") || l.contains("cookies") || l.contains("authentication") {
        "Bu site giriş istiyor. Tarayıcı eklentisinden indirirsen giriş bilgin kullanılır."
    } else if l.contains("unsupported url") {
        "Bu adres desteklenmiyor."
    } else if l.contains("video unavailable") || l.contains("not available") {
        "Video bulunamadı ya da kaldırılmış."
    } else if l.contains("429") || l.contains("too many requests") {
        "Site çok fazla istek aldığını söylüyor. Birkaç dakika sonra tekrar dene."
    } else if l.contains("no space left") {
        "Diskte yer kalmadı."
    } else if l.contains("403") || l.contains("forbidden") {
        "Site indirmeyi geçici olarak reddetti (403). YouTube bunu sık yapar; tekrar denenince iner."
    } else if l.contains("http error 5")
        || l.contains("bad gateway")
        || l.contains("service unavailable")
    {
        "Site geçici bir hata verdi. Tekrar denenince iner."
    } else if l.contains("getaddrinfo")
        || l.contains("name or service not known")
        || l.contains("no address")
    {
        "İnternet bağlantısında sorun var (site adresi bulunamadı)."
    } else if l.contains("timed out")
        || l.contains("connection reset")
        || l.contains("remote end closed")
    {
        "Bağlantı yarıda kesildi. Tekrar denenince iner."
    } else if l.contains("unable to download webpage") {
        "Siteye ulaşılamadı ya da site erişimi reddetti."
    } else if l.contains("unable to download") {
        "Video verisi indirilemedi. Tekrar denenince iner."
    } else if last.is_empty() {
        "Bilinmeyen bir hata oluştu."
    } else {
        return last
            .trim_start_matches("ERROR:")
            .trim()
            .chars()
            .take(240)
            .collect();
    };
    msg.to_string()
}

/// Tekrar denenince düzelme ihtimali olan hata mı (geçici engel, kopan bağlantı)?
pub fn is_transient(stderr: &str) -> bool {
    let l = stderr.to_ascii_lowercase();
    [
        "403",
        "forbidden",
        "429",
        "too many requests",
        "http error 5",
        "timed out",
        "connection reset",
        "remote end closed",
        "unable to download",
        "incomplete",
        "fragment",
        "temporarily",
    ]
    .iter()
    .any(|k| l.contains(k))
        && !l.contains("private video")
        && !l.contains("video unavailable")
        && !l.contains("unsupported url")
}

#[cfg(test)]
mod tests {
    use super::*;

    const YT: &str = r#"{
      "id": "abc123", "title": "Deneme Videosu", "uploader": "GMID", "extractor_key": "Youtube",
      "webpage_url": "https://www.youtube.com/watch?v=abc123", "duration": 100, "thumbnail": "https://i.ytimg.com/x.jpg",
      "formats": [
        {"format_id": "sb0", "ext": "mhtml", "vcodec": "none", "acodec": "none", "protocol": "mhtml", "format_note": "storyboard"},
        {"format_id": "139", "ext": "m4a", "vcodec": "none", "acodec": "mp4a.40.5", "abr": 48, "filesize": 600000, "protocol": "https"},
        {"format_id": "140", "ext": "m4a", "vcodec": "none", "acodec": "mp4a.40.2", "abr": 129, "filesize": 1600000, "protocol": "https"},
        {"format_id": "251", "ext": "webm", "vcodec": "none", "acodec": "opus", "abr": 140, "filesize": 1700000, "protocol": "https"},
        {"format_id": "18", "ext": "mp4", "vcodec": "avc1.42001E", "acodec": "mp4a.40.2", "height": 360, "tbr": 500, "protocol": "https"},
        {"format_id": "134", "ext": "mp4", "vcodec": "avc1.4d401e", "acodec": "none", "height": 360, "tbr": 300, "filesize": 3000000, "protocol": "https"},
        {"format_id": "136", "ext": "mp4", "vcodec": "avc1.64001f", "acodec": "none", "height": 720, "tbr": 1500, "filesize": 15000000, "protocol": "https"},
        {"format_id": "247", "ext": "webm", "vcodec": "vp9", "acodec": "none", "height": 720, "tbr": 1400, "filesize": 14000000, "protocol": "https"},
        {"format_id": "137", "ext": "mp4", "vcodec": "avc1.640028", "acodec": "none", "height": 1080, "fps": 60, "tbr": 4000, "filesize": 40000000, "protocol": "https"},
        {"format_id": "401", "ext": "mp4", "vcodec": "av01.0.12M.08", "acodec": "none", "height": 2160, "tbr": 18000, "filesize_approx": 180000000, "protocol": "https"},
        {"format_id": "96", "ext": "mp4", "vcodec": "avc1", "acodec": "mp4a", "height": 1080, "protocol": "m3u8_native", "has_drm": true}
      ]
    }"#;

    #[test]
    fn youtube_options() {
        let p = parse_probe(YT.as_bytes(), "x").unwrap();
        let ids: Vec<_> = p.options.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids, ["2160", "1080", "720", "360", "mp3"]);
        let o1080 = &p.options[1];
        assert!(o1080.recommended);
        assert_eq!(
            o1080.format,
            "137+140/bv*[height<=1080]+ba/b[height<=1080]/bv*+ba/b"
        );
        assert_eq!(o1080.size, Some(41_600_000));
        assert_eq!(o1080.parts, 2);
        assert_eq!(o1080.detail, "60 fps · MP4");
        // 720p'de H.264, VP9'dan önce gelir.
        assert!(p.options[2].format.starts_with("136+140"));
        assert_eq!(p.options[0].label, "2160p · 4K");
        assert_eq!(p.options[4].format, "140/ba/b");
        assert_eq!(p.site, "Youtube");
        assert_eq!(site_folder(&p.site), "YouTube");
    }

    #[test]
    fn twitter_like_combined_formats() {
        let json = r#"{"id":"1","title":"t","extractor_key":"Twitter","formats":[
            {"format_id":"hls-audio","ext":"mp4","vcodec":"none","acodec":"mp4a","protocol":"m3u8_native","tbr":128},
            {"format_id":"http-832","ext":"mp4","height":720,"tbr":832,"protocol":"https"},
            {"format_id":"hls-832","ext":"mp4","vcodec":"avc1","acodec":"none","height":720,"tbr":832,"protocol":"m3u8_native"}
        ]}"#;
        let p = parse_probe(json.as_bytes(), "x").unwrap();
        // Doğrudan indirilen birleşik dosya seçilir; ayrı ses eklenmez.
        assert!(
            p.options[0].format.starts_with("http-832/"),
            "{}",
            p.options[0].format
        );
        assert_eq!(p.options[0].parts, 1);
    }

    #[test]
    fn direct_file_links() {
        let json = r#"{"_type":"video","id":"GMID Deneme","title":"GMID Deneme","extractor_key":"Generic",
            "formats":[{"format_id":"mp4","ext":"mp4","vcodec":null,"acodec":null,"protocol":"http"}]}"#;
        let p = parse_probe(json.as_bytes(), "x").unwrap();
        let ids: Vec<_> = p.options.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids, ["best", "mp3"]);
        assert!(p.options[0].recommended);
        assert_eq!(p.options[0].format, "mp4/bv*+ba/b");
    }

    #[test]
    fn playlists_are_parsed() {
        let json = r#"{"_type":"playlist","id":"PL1","title":"Lofi 🎧","channel":"Chill","extractor_key":"YoutubeTab",
            "webpage_url":"https://www.youtube.com/playlist?list=PL1","entries":[
            {"id":"a1","url":"https://www.youtube.com/watch?v=a1","title":"Bir","duration":120,"channel":"X",
             "thumbnails":[{"url":"https://i.ytimg.com/a1/s.jpg","width":168},{"url":"https://i.ytimg.com/a1/m.jpg","width":480}]},
            {"id":"a2","url":null,"title":"[Private video]"},
            {"id":"a3","title":"Üç"}]}"#;
        let Parsed::Playlist(p) = parse_any(json.as_bytes(), "x").unwrap() else {
            panic!()
        };
        assert_eq!(p.title, "Lofi 🎧");
        assert_eq!(p.entries.len(), 2, "gizli video atlanır");
        assert_eq!(
            p.entries[0].thumbnail.as_deref(),
            Some("https://i.ytimg.com/a1/m.jpg")
        );
        assert_eq!(p.entries[1].url, "https://www.youtube.com/watch?v=a3");
        assert_eq!(site_folder(&p.site), "YouTube");
    }

    #[test]
    fn nearest_quality_is_chosen() {
        let p = parse_probe(YT.as_bytes(), "x").unwrap();
        assert_eq!(choose(&p.options, "1440").unwrap().id, "1080");
        assert_eq!(choose(&p.options, "720").unwrap().id, "720");
        assert_eq!(choose(&p.options, "240").unwrap().id, "360");
        assert_eq!(choose(&p.options, "mp3").unwrap().id, "mp3");
        assert_eq!(choose(&p.options, "auto").unwrap().id, "1080");
        assert!(generic_option("720").format.contains("height<=720"));
        assert!(generic_option("mp3").audio_only);
    }

    #[test]
    fn dubs_and_subtitles() {
        let json = r#"{"id":"f1","title":"Film","extractor_key":"Generic","duration":5400,
          "formats":[
            {"format_id":"v1080","ext":"mp4","vcodec":"avc1.640028","acodec":"none","height":1080,"protocol":"m3u8_native"},
            {"format_id":"a-tur","ext":"mp4","vcodec":"none","acodec":"mp4a.40.2","language":"tur","protocol":"m3u8_native"},
            {"format_id":"a-eng","ext":"mp4","vcodec":"none","acodec":"mp4a.40.2","language":"eng","protocol":"m3u8_native"}],
          "subtitles":{"tr":[{"ext":"vtt","name":"Türkçe"}],"en":[{"ext":"vtt"}],"live_chat":[{"ext":"json"}]}}"#;
        let p = parse_probe(json.as_bytes(), "https://x").unwrap();
        assert_eq!(
            p.audio_langs
                .iter()
                .map(|t| t.label.as_str())
                .collect::<Vec<_>>(),
            ["Türkçe", "İngilizce"]
        );
        assert_eq!(
            p.subtitles
                .iter()
                .map(|t| (t.code.as_str(), t.label.as_str()))
                .collect::<Vec<_>>(),
            [("en", "İngilizce"), ("tr", "Türkçe")]
        );
        let o = choose(&p.options, "1080").unwrap();
        assert!(format_for(&o, Some("tur")).starts_with("bv*[height<=1080]+ba[language=tur]/"));
        assert_eq!(format_for(&o, Some("bad]code")), o.format);
        assert_eq!(lang_base("tr-TR"), "tr");
        assert_eq!(lang_label("en-US"), "İngilizce (US)");
        let a = subtitle_args(
            "https://x",
            Path::new("/v/Film [f1] 1080p.mp4"),
            &["tr".into()],
            &Context::default(),
        );
        assert!(a
            .windows(2)
            .any(|w| w[0] == "-o" && w[1] == "/v/Film [f1] 1080p.%(ext)s"));
    }

    #[test]
    fn names_and_context() {
        assert_eq!(safe_name(" Film: Bölüm 1/2? ", 50), "Film_ Bölüm 1_2_");
        assert_eq!(safe_name("...", 10), "Video");
        let ctx = Context {
            referer: Some("https://site.example/izle/1".into()),
            user_agent: Some("UA".into()),
            title: Some("%100 Gerçek".into()),
            ..Default::default()
        };
        let a = download_args(
            "https://cdn.example/v.m3u8",
            &generic_option("720"),
            Path::new("/v"),
            Path::new("/t"),
            None,
            &ctx,
        );
        assert!(a
            .windows(2)
            .any(|w| w[0] == "--referer" && w[1] == "https://site.example/izle/1"));
        assert!(a
            .windows(2)
            .any(|w| w[0] == "--add-header" && w[1] == "Origin:https://site.example"));
        assert!(a.iter().any(|x| x.contains("%%100 Gerçek [%(id)s]")));
        assert_eq!(a.last().unwrap(), "https://cdn.example/v.m3u8");
    }

    #[test]
    fn playlists_and_live_are_rejected() {
        assert!(parse_any(br#"{"_type":"playlist","entries":[]}"#, "x").is_err());
        assert!(parse_probe(br#"{"is_live":true,"formats":[]}"#, "x").is_err());
    }

    #[test]
    fn progress_lines() {
        assert_eq!(
            parse_line("GMIDP 1048576|NA|4194304|524288.5|6|137"),
            Line::Progress {
                downloaded: 1_048_576,
                total: Some(4_194_304),
                speed: Some(524_288.5),
                eta: Some(6),
                format_id: "137".into()
            }
        );
        assert_eq!(
            parse_line("GMIDPP Merger|started"),
            Line::Processing("Merger".into())
        );
        assert_eq!(
            parse_line("GMIDFILE C:\\Videolar\\a [x].mp4"),
            Line::File("C:\\Videolar\\a [x].mp4".into())
        );
        assert_eq!(parse_line("[youtube] abc: Downloading"), Line::Other);
    }

    #[test]
    fn transient_errors() {
        assert!(is_transient(
            "ERROR: unable to download video data: HTTP Error 403: Forbidden"
        ));
        assert!(
            friendly_error("ERROR: unable to download video data: HTTP Error 403: Forbidden")
                .contains("403")
        );
        assert!(!is_transient("ERROR: [youtube] x: Private video"));
        assert!(!is_transient("ERROR: Unsupported URL: https://a"));
    }

    #[test]
    fn errors_are_translated() {
        assert!(friendly_error(
            "WARNING: x\nERROR: [youtube] abc: Sign in to confirm you're not a bot"
        )
        .contains("bot"));
        assert_eq!(
            friendly_error("ERROR: something odd happened"),
            "something odd happened"
        );
    }
}
