# Ek B — Teknik Fizibilite Araştırması

> Tarih: 2026-10-10. **(doğrulanmalı)** etiketli maddeler kesin değil; kodlamadan önce test edilecek.
> Zorluk: Kolay / Orta / Zor. Ana doküman: [TASARIM.md](TASARIM.md)

---

## 1. Video indirici eklentisi (IDM benzeri) — **Orta–Zor**

**Önerilen mimari**
- **Eklenti (Manifest V3):** Aktif sekmenin adresini alır, sayfaya veya araç çubuğuna "İndir" düğmesi ekler. Videonun kendisini eklenti **indirmez**: YouTube ve X videoyu parça parça (MSE/DASH, blob) oynattığı için tarayıcı içinden indirmek pratik değil.
- **Eklenti ↔ masaüstü uygulaması iletişimi:**
  - **(a) Native Messaging (önerilen):** `chrome.runtime.connectNative`. Uygulama kurulurken `HKCU\Software\Google\Chrome\NativeMessagingHosts\<host>` kaydını ve bir manifest JSON'u oluşturur; `allowed_origins` alanına eklentinin kimliği yazılır. Port açılmaz, kimliği tarayıcı doğrular.
  - **(b) Yerel HTTP/WebSocket (yedek):** Sadece `127.0.0.1`, rastgele token + Origin kontrolü.
- **Masaüstü uygulaması:** `yt-dlp -J <url>` ile format listesini JSON olarak alır → kullanıcı çözünürlük seçer → `yt-dlp -f "bv*[height<=1080]+ba/b" -P <Videolar klasörü> <url>`; ilerleme `--newline` / `--progress-template` ile okunur.

**Bilinen tuzaklar**
- **ffmpeg zorunlu:** YouTube'da 720p üstü görüntü ve ses ayrı gelir (DASH); birleştirmek için ffmpeg gerekir. Uygulamayla birlikte gelir.
- **JavaScript çalışma ortamı (2025 sonu değişikliği):** yt-dlp 2025.11.12'den beri YouTube için harici bir JS çalışma ortamı (varsayılan/önerilen: **Deno**) ve `yt-dlp-ejs` bileşeni istiyor; yoksa format listesi kısıtlı geliyor. Sonuç: **yt-dlp + ffmpeg + deno** üçlüsü birlikte paketlenecek. PO token gereksinimleri de zaman zaman değişiyor; yt-dlp wiki'si takip edilmeli.
- **Güncelleme:** YouTube sık değişiyor; uygulama açılışta/haftalık `yt-dlp -U` çalıştıracak.
- **Instagram ve X için çerez gerekebilir.** Windows'ta `--cookies-from-browser chrome`, Chrome 127+ ile gelen App-Bound Encryption yüzünden genelde çalışmıyor. Çözüm: eklenti `chrome.cookies` izniyle **yalnızca ilgili sitenin** çerezlerini okur, Netscape formatına çevirip uygulamaya yollar; uygulama `--cookies <geçici dosya>` ile kullanır ve **iş bitince dosyayı siler**.
- **MV3 kısıtları:** Service worker ~30 sn boşta kalınca kapanır, uzaktan kod çalıştırılamaz. Seçilen mimariyi etkilemiyor.

**Politika ve hukuk**
- **Chrome Web Store** YouTube indiricilerini pratikte reddediyor → eklenti **paketlenmemiş** olarak ("Geliştirici modu → Paketlenmemiş öğe yükle") kurulacak. Chrome açılışta "geliştirici modu eklentileri" uyarısı gösterebilir; ev bilgisayarında sorun değil.
- **YouTube Kullanım Şartları** indirmeyi yasaklıyor. Kişisel kullanımda risk düşük; uygulama herkese dağıtılırsa ciddi risk (bkz. TASARIM.md §10).
- **Opera:** Chromium tabanlı; aynı eklenti `opera://extensions` → geliştirici modunda olduğu gibi yüklenir. Opera'nın Chrome'un Native Messaging kayıt anahtarını okuduğu biliniyor **(doğrulanmalı)**; sorun çıkarsa (b) yöntemi kullanılır.

Kaynaklar: https://developer.chrome.com/docs/webstore/program-policies/policies · https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging · https://github.com/yt-dlp/yt-dlp/issues/15012 · https://github.com/yt-dlp/yt-dlp/issues/10927

---

## 2. Oyun süresi takibi ve oyun algılama — **Orta**

**Genel algılama (Discord'un yöntemi)**
- Discord çalışan süreçlerin exe adlarını kendi listesiyle eşleştirir. Liste `https://discord.com/api/v9/applications/detectable` adresinden JSON olarak alınabiliyor (`executables[].name`, `os`), giriş gerekmez. Resmî olarak belgelenmemiş, değişebilir **(doğrulanmalı)**.
- Liste günde bir kez indirilip SQLite'ta önbelleğe alınır; kullanıcı elle de oyun ekleyebilir.
- **Süreç taraması:** 5–10 sn'de bir `CreateToolhelp32Snapshot` (Rust: `sysinfo` crate'i). WMI daha ağır, tercih edilmez.
- **Ön plan / tam ekran tespiti:** `GetForegroundWindow` + `GetWindowThreadProcessId`; `SHQueryUserNotificationState` → `QUNS_RUNNING_D3D_FULL_SCREEN` / `QUNS_BUSY` ise kullanıcı tam ekran oyunda. Bu sinyal Oyun Modu ve Rahatsız Etmeyin için de kullanılır.
- **Kayıt:** Oturum başlangıç/bitişi `play_sessions` tablosuna; istenirse yalnızca ön planda geçen süre ayrı tutulur.

**Platform verileri**

| Kaynak | Yöntem | Not |
|---|---|---|
| Steam — kurulu oyunlar | `steamapps/libraryfolders.vdf` + `appmanifest_<appid>.acf` | VDF ayrıştırıcı gerekir (Rust: `keyvalues-parser`) |
| Steam — oynama süresi (yerel) | `Steam/userdata/<hesap>/config/localconfig.vdf` → `.../apps/<appid>/Playtime` (dakika), `LastPlayed` | API anahtarı gerekmez; Steam açıkken güncel olmayabilir |
| Steam Web API | `IPlayerService/GetOwnedGames?key=…&steamid=…&include_appinfo=1` → `playtime_forever` | Ücretsiz anahtar; profilde "Oyun detayları" herkese açık olmalı |
| Epic | `C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests\*.item` (JSON: `DisplayName`, `InstallLocation`, `LaunchExecutable`, `AppName`) | Yerelde süre tutulmuyor → süre **bizim takibimizden** gelir |
| EA app | Kayıt defteri oyun anahtarları, oyun klasöründe `__Installer\installerdata.xml`, `C:\ProgramData\EA Desktop` | Belgelenmemiş **(doğrulanmalı)**; süre bizim takibimizden |

**Referans proje: Playnite** (MIT, C#/WPF). Mağaza başına "kütüphane eklentisi" mimarisi; VDF/manifest ayrıştırma kodu iyi bir referans.
https://github.com/JosefNemec/Playnite

---

## 3. Tek tıkla ses çıkış cihazı değiştirme — **Kolay–Orta**

- Windows'ta varsayılan cihazı değiştirmenin **belgelenmiş API'si yok**. Tüm araçlar belgelenmemiş `IPolicyConfig::SetDefaultEndpoint(deviceId, role)` COM arayüzünü kullanıyor; üç rol için ayrı çağrılır: `eConsole`, `eMultimedia`, `eCommunications`. Windows 7–11'de çalışıyor; bir güncelleme bozabilir (düşük olasılık, yıllardır sabit).
- **Cihaz listesi** belgelenmiş `IMMDeviceEnumerator` ile (durum, ad, kimlik).
- **Referanslar:** SoundSwitch (açık kaynak C#), NirSoft `nircmd setdefaultsounddevice`, PowerShell `AudioDeviceCmdlets`. Rust'ta `windows` crate ile ~50 satırlık IPolicyConfig tanımı yeterli.
- **Kısayol:** Win32 `RegisterHotKey` / `tauri-plugin-global-shortcut`.
- **Tuzak:** Bazı oyunlar cihaz değişimini anlık algılamaz, yeniden başlatılmaları gerekebilir. Uygulama başına cihaz atama (EarTrumpet'in yöntemi, `IAudioPolicyConfigFactory`) daha riskli → "Sonra" listesinde.

Kaynaklar: https://github.com/Belphemur/SoundSwitch · https://github.com/frgnca/AudioDeviceCmdlets · https://www.nirsoft.net/utils/nircmd.html

---

## 4. Bildirimler, sistem tepsisi, Windows ile başlatma — **Kolay**

- **Tauri 2:** `tauri-plugin-notification` (Windows bildirimi), yerleşik `TrayIcon`, `tauri-plugin-autostart`. Hatırlatıcılar Rust tarafında zamanlayıcı ile SQLite'tan okunur.
- **Tuzak (AUMID):** Bildirimin doğru uygulama adı/ikonuyla görünmesi için uygulamanın kurulu ve Başlat menüsünde kısayolunun olması gerekir; geliştirme sırasında "PowerShell" adıyla görünebilir.
- Uygulama tamamen kapalıyken hatırlatma için WinRT `ScheduledToastNotification` kullanılabilir; basit çözüm: uygulama tepside çalışır.
- Windows'ta "Rahatsız Etmeyin" açıkken bildirimler bastırılır.

---

## 5. Format sonrası otomatik kurulum — **Kolay**

- **winget:** `winget install --id <ID> -e --silent --accept-package-agreements --accept-source-agreements`. Toplu: `winget export -o apps.json` / `winget import -i apps.json`.
- **Tuzak:** Yeni kurulmuş Windows'ta "Uygulama Yükleyicisi" (winget) ilk dakikalarda hazır olmayabilir. Betik önce `winget --version` kontrol eder; yoksa Microsoft Store'daki App Installer sayfasını açar.
- **Paket kimlikleri** (kurmadan önce `winget search` ile doğrulanacak):

  | Uygulama | winget ID |
  |---|---|
  | Chrome | `Google.Chrome` |
  | Steam | `Valve.Steam` |
  | Discord | `Discord.Discord` |
  | Claude (masaüstü) | `Anthropic.Claude` **(doğrulanmalı)** |
  | WinRAR | `RARLab.WinRAR` |
  | 7-Zip | `7zip.7zip` |
  | VLC | `VideoLAN.VLC` |
  | Epic Games Launcher | `EpicGames.EpicGamesLauncher` |
  | EA app | `ElectronicArts.EADesktop` |
  | OBS Studio | `OBSProject.OBSStudio` |
  | Spotify | `Spotify.Spotify` (yönetici olarak kurulmayı reddeder; normal kullanıcıyla kurulmalı) |
  | NVIDIA App | **(doğrulanmalı)** |

- **Tasarım:** Uygulama listeyi JSON olarak tutar; "Dışa aktar" düğmesi bundan **bağımsız çalışan** bir `kurulum.ps1` (+ çift tıklanabilir `.bat` sarmalayıcı) ve winget import JSON'u üretir. Format sonrası GMID kurulu değilken de çalışır; GMID'in kendisi de listeye eklenebilir.
- Alternatifler: Chocolatey (ayrıca kurulum gerektirir), Ninite (kapalı, sınırlı liste). winget Windows'ta yerleşik olduğu için tercih edildi.

Kaynaklar: https://learn.microsoft.com/windows/package-manager/winget/ · https://wingetly.io/apps/anthropic/claude

---

## 6. Özelleştirilebilir arka plan (resim / GIF / video) — **Kolay**

- **GIF yerine WebM (VP9/AV1) veya MP4 (H.264):** GIF 5–20 kat daha büyük ve işlemciyle çözülür; video ekran kartıyla donanımsal çözülür. GIF yüklenirse ffmpeg ile otomatik WebM'e dönüştürme önerilir.
- Arayüzde `<video autoplay muted loop>` + `object-fit: cover` + bulanıklık/karartma katmanı. Sabit resimlerde WebP/AVIF.
- **Duraklatma:** Pencere küçültülünce, tepsiye gizlenince veya oyun algılanınca video durdurulur. Bu tarayıcı olayına bırakılmaz (WebView2'de güvenilir değil); masaüstü tarafı pencere olaylarını ve oyun algılama sinyalini dinleyip arayüze açıkça "dur" komutu gönderir. Ayrıca "Animasyonları kapat" seçeneği ve çözünürlük/fps sınırı (ör. 1080p/30fps).

---

## 7. Framework seçimi (birinci ölçüt: optimizasyon)

| | **Tauri 2** | Electron | WPF / WinUI 3 (.NET) | PySide6 (Python) |
|---|---|---|---|---|
| Kurulum boyutu | ~5–15 MB (yardımcı araçlar hariç) | 80–150 MB | 1–70 MB | 50–150 MB |
| Boşta RAM | ~40–100 MB (WebView2 dahil) | 150–300 MB | 30–80 MB | 60–120 MB |
| Arka uç dili | Rust (Win32/COM'a tam erişim) | Node (Win32 için native modül gerekir) | C# (Win32/COM için en kolay) | Python (yavaş, paketleme zahmetli) |
| Arayüz ve hareketli arka plan | Web — kolay | Web — kolay | XAML — video arka plan zahmetli | Orta |
| İleride sohbet / web ile kod paylaşımı | Çok iyi | Çok iyi | Orta | Zayıf |

**Öneri: Tauri 2** — arayüz web teknolojisi (Svelte veya React), arka uç Rust, veritabanı SQLite.
- Electron'dan disk ve RAM'de belirgin şekilde hafif.
- Rust tarafı süreç taraması, IPolicyConfig, tam ekran tespiti ve Native Messaging sunucusunu verimli taşır; yt-dlp/ffmpeg/deno "sidecar" olarak paketlenir.
- Web arayüz hareketli arka planı ve ileride sohbet arayüzünü kolaylaştırır.
- **WPF ne zaman?** C# biliniyor ve Rust bilinmiyorsa: daha hızlı yazılır, RAM en düşük olur, ama arayüz esnekliği azalır.

**İleride çevrim içi sohbet (yeniden tasarım olmadan):**
- Baştan bir "servis katmanı" tanımlanır (ör. `ChatService`): ilk sürümde boş/yerel, sonra ağ implementasyonu takılır.
- Sunucu (VDS) seçenekleri: (a) kendi sunucumuz — Rust (axum) veya Node + WebSocket + PostgreSQL, JWT ile giriş; (b) Matrix (Synapse/Conduit) — hazır protokol, uçtan uca şifreleme; (c) Revolt — kendi sunucuna kurulabilen Discord benzeri.
- Kayıtlar baştan `id` (ULID), `created_at`, `sync_state` alanlarıyla tasarlanır → senkronizasyon kolaylaşır.

Kaynaklar: https://v2.tauri.app/ · https://v2.tauri.app/plugin/notification/ · https://v2.tauri.app/develop/sidecar/

---

## 8. Oyuncuya yönelik ek özellikler (öncelik sırasıyla)

1. **Oyun Modu profili (Orta, çok değerli):** Tek tık veya oyun algılanınca otomatik: güç planı (`powercfg /setactive`), ses cihazı, seçili arka plan uygulamalarını kapatma (yalnızca kullanıcının listesi, sistem süreçleri asla), bildirimleri susturma, arka plan animasyonunu durdurma. Oyun kapanınca her şey geri alınır. Anti-cheat riski yok.
2. **Birleşik oyun kütüphanesi ve başlatıcı (Orta):** `steam://rungameid/<id>`, `com.epicgames.launcher://apps/<AppName>?action=launch&silent=true`. Playnite referans.
3. **Kapak ve bilgi çekme (Kolay):** SteamGridDB (ücretsiz anahtar; kapak, logo, banner), IGDB (Twitch hesabıyla), RAWG (ücretsiz, atıf zorunlu).
4. **Ücretsiz oyun takibi (Kolay):** Epic `freeGamesPromotions` (resmî olmayan, değişebilir), GamerPower API. Yeni ücretsiz oyunda bildirim.
5. **İndirim / fiyat takibi (Kolay–Orta):** IsThereAnyDeal API (ücretsiz anahtar). İstek listesindeki oyun hedef fiyatın altına düşünce bildirim. TR bölge desteği **(doğrulanmalı)**.
6. **Oynanacaklar listesi + HowLongToBeat (Orta):** HLTB'nin resmî API'si yok; topluluk kütüphaneleri siteyi kazıyor ve sık bozuluyor → isteğe bağlı, önbellekli.
7. **Donanım / FPS izleme (Orta–Zor):** Sensörler için LibreHardwareMonitor; ancak sürücüsü bazı anti-cheat'lerle (ör. FACEIT) sorun çıkarabiliyor. Sadece CPU/GPU yükü için sürücüsüz yöntemler (Windows performans sayaçları, NVIDIA NVML) daha güvenli. FPS için Intel **PresentMon** (ETW tabanlı, oyuna enjeksiyon yok). **Oyunun içine çizilen overlay yapılmamalı** (ban riski); ayrı, şeffaf, her zaman üstte pencere kullanılır.
8. **Ekran görüntüsü / klip düzenleyici (Kolay):** Steam, NVIDIA, Xbox Game Bar klasörlerini izleyip oyun adı/tarihe göre düzenleme, galeri, kopya tespiti.
9. **Discord Rich Presence (Kolay):** "GMID'de: X oynuyor / video indiriyor".
10. **Oyun süresi istatistik paneli (Kolay):** Haftalık/aylık grafikler, en çok oynananlar.
11. **Sürücü güncelleme hatırlatıcısı (Orta):** Kurulu GPU sürücü sürümü okunabilir; en son sürümü öğrenmenin resmî yolu yok → "X gündür güncellemedin" hatırlatması daha gerçekçi.
12. **Pano geçmişi / hızlı ekran görüntüsü (Kolay):** Windows'ta zaten Win+V ve Win+Shift+S var; katma değeri düşük.

**Kaçınılacaklar**
- **Makro / otomatik tıklama:** Çoğu online oyunun kurallarını ihlal eder; Vanguard, EAC, BattlEye tespit edebilir.
- **Nişangâh (crosshair) overlay:** Rekabetçi oyunlarda yasak sayılabilir; eklenirse uyarılı ve varsayılan kapalı.
- **Oyun belleğini okuyan ya da oyuna kod enjekte eden hiçbir özellik.**

---

## Özet efor tablosu

| # | Özellik | Zorluk | Ana risk |
|---|---|---|---|
| 1 | Video indirici + eklenti | Orta–Zor | YouTube değişiklikleri, kullanım şartları, mağazada yayınlanamaz |
| 2 | Oyun süresi + algılama | Orta | Discord/Epic/EA verileri belgelenmemiş |
| 3 | Ses cihazı değiştirme | Kolay–Orta | IPolicyConfig belgelenmemiş |
| 4 | Bildirim, tepsi, otomatik başlatma | Kolay | Kurulum/AUMID gereksinimi |
| 5 | winget otomatik kurulum | Kolay | Paket kimliği doğrulama, winget'in hazır olması |
| 6 | Hareketli arka plan | Kolay | Duraklatma mantığı |
| 7 | Tauri 2 + SQLite altyapısı | Orta | Rust öğrenme eğrisi |
| 8 | Ek özellikler | Kolay → Zor | Anti-cheat (overlay, sürücü, makro) |

**Ek kaynaklar:** https://docs.isthereanydeal.com/ · https://www.steamgriddb.com/api/v2 · https://api-docs.igdb.com/ · https://github.com/LibreHardwareMonitor/LibreHardwareMonitor · https://github.com/GameTechDev/PresentMon · https://developer.valvesoftware.com/wiki/Steam_Web_API
