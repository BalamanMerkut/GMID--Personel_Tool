# Ek C — Oyun İçi Panel, Mikrofon Kilidi, Otomatik Bilgi ve Galeri Araştırması

> Tarih: 2026-10-10. **(doğrulanmalı)** etiketli maddeler kesin değil; kodlama sırasında test edilecek.
> Bazı birincil kaynaklara araştırma ortamından erişilemedi; bilgiler arama sonuçlarındaki alıntılara dayanıyor.
> Ana doküman: [TASARIM.md](TASARIM.md) §3.3, §3.8, §3.11, §3.12

---

## 1. Oyun içi panel (oyuna kod sokmadan)

### 1a. Ayrı, "her zaman üstte" bir pencere hangi oyun modunda görünür?

| Oyun modu | Görünür mü? | Not |
|---|---|---|
| **Pencereli / Çerçevesiz (borderless)** | **Evet** | Oyun Windows'un masaüstü çizicisi (DWM) üzerinden ekrana geldiği için üstteki pencere normal görünür. |
| **Modern "tam ekran" (DX11/DX12, Windows'un tam ekran iyileştirmeleri açık)** | **Pratikte evet** | Windows bu oyunları aslında ekranı kaplayan optimize bir pencere olarak çalıştırır; Microsoft bunun faydaları arasında "Alt+Tab, çoklu monitör ve overlay'ler"i sayıyor. Panel açıkken gecikme çok az artabilir **(doğrulanmalı)**. |
| **Gerçek "özel tam ekran"** (DX9 ve eski oyunlar, iyileştirmeler kapalı, bazı Vulkan/OpenGL oyunları) | **Hayır, güvenilir değil** | Pencere görünmez; odak alırsa oyun tam ekrandan çıkabilir / küçülebilir. |

Windows 11'deki "Pencereli oyunlar için iyileştirmeler" (Ayarlar → Sistem → Ekran → Grafikler) DX10/11 oyunlarının pencereli modunu da modern yönteme taşıyor; DX12 zaten bu yöntemi kullanıyor.

**Sonuç:** Kullanıcıya "oyunu Çerçevesiz / Pencereli tam ekranda çalıştır" önerisi gösterilecek — Discord da aynısını yapıyor.

Kaynaklar: https://devblogs.microsoft.com/directx/demystifying-full-screen-optimizations/ · https://devblogs.microsoft.com/directx/dxgi-flip-model/

### 1b. Diğer overlay'ler nasıl çalışıyor?

- **Discord (yeni overlay, Mart 2025):** Artık oyunun içine kod sokmuyor, parçacık tabanlı ve anti-cheat'i tetiklememesi bekleniyor. **Yalnızca pencereli/çerçevesiz modda çalışıyor**, gerçek tam ekranda çalışmıyor. Eski (kod sokan) overlay "legacy" seçeneği olarak duruyor ve kaldırılabileceği söyleniyor. → **Bizim planımızla aynı yaklaşım.**
  Kaynaklar: https://discord.com/blog/player-release-q12025 · https://support.discord.com/hc/en-us/articles/25289844838551
- **MSI Afterburner / RTSS:** Oyuna kod sokuyor (hook). Bazı anti-cheat'ler engelliyor (ör. Vanguard). **Örnek alınmayacak.**
- **NVIDIA App (Alt+Z):** Ayrı süreç ve sürücü düzeyinde yakalama; oyuna kod sokup sokmadığına dair resmî bilgi bulunamadı **(doğrulanmalı)**.
- **Xbox Game Bar:** İşletim sistemi düzeyinde, ayrı süreç.

### 1c. Xbox Game Bar parçacığı (widget)

- SDK hâlâ güncel (NuGet `Microsoft.Gaming.XboxGameBar`, Mart 2026'da yeni sürüm adayı).
- Teknoloji: UWP (C# veya C++/WinRT). Microsoft Store olmadan, geliştirici modunda kişisel kullanım için kurulabiliyor.
- Masaüstü uygulamasıyla iletişim: en kolayı ikisini aynı pakette toplamak; haberleşme "named pipe" ile.
- Gerçek özel tam ekranda görünüp görünmediği resmî olarak garanti değil **(doğrulanmalı)**.
- Efor: 2–3 hafta + paketleme işi. **Öneri: v1'de yapılmayacak**; çerçevesiz modda kendi panelimiz yeterli.

### 1d. Odak çalmayan ekran göstergesi (OSD)

- Windows'un ses göstergesi gibi: "odak almayan + tıklamaları geçiren + her zaman üstte" pencere. Çerçevesiz ve modern tam ekran oyunlarda görünür, oyun klavye/fare kontrolünü kaybetmez. Gerçek özel tam ekranda görünmez (kısayolun işi yine yapılır).
- **Tauri'ye özel not:** Pencereyi Tauri'nin kendi "göster" komutuyla açmak odağı çalabiliyor; bu yüzden gösterge Windows'un doğrudan "odaklamadan göster" çağrısıyla açılacak. Tıklama geçirme özelliğinde bilinen küçük hatalar var (tauri #11461, #9250) — test edilecek.

---

## 2. Mikrofon seviyesinin kendiliğinden düşmesi

**Bilinen sebepler**
- **Discord:** Ayarlar → Ses ve Görüntü → (Gelişmiş) → **Otomatik Kazanç Kontrolü**. Türkçe arayüzdeki tam adı **(doğrulanmalı)**.
- **Chrome/Edge ve tarayıcıdaki Discord, Meet vb.:** WebRTC'nin "giriş ses seviyesini ayarlamasına izin ver" davranışı Windows mikrofon seviyesini değiştirebiliyor.
- **Teams / Zoom / Skype:** "Mikrofon ayarlarını otomatik yap" seçenekleri (menü adları sürüme göre değişiyor).
- **Windows:** Mikrofon → Özellikler → Gelişmiş → "Uygulamaların bu cihazın özel denetimini almasına izin ver" — kapatmak kısmen yardımcı olur, tek başına yetmeyebilir.
- Üretici yazılımları (Realtek, Nahimic, SteelSeries Sonar, kulaklık yazılımları) **(yaygın raporlar)**.

**Teknik çözüm (doğrulandı):**
- Windows'un resmî ses arayüzü (`IAudioEndpointVolume`) seviye veya sessiz durumu değiştiğinde **kendisi haber veriyor** (olay tabanlı, sürekli kontrol yok).
- Haber gelince seviye seçilen değere geri çekilir; kendi yaptığımız değişikliği tanıyıp sonsuz döngüye girmemek için işaretleme yapılır.
- **Yönetici izni gerekmez.**
- Varsayılan mikrofon değişirse yeni cihaz için kilit otomatik yeniden kurulur.
- Sessize alma ve seviye artır/azalt kısayolları da aynı arayüzle basitçe yapılır.

Referans araçlar: MicLock (github.com/froxel/MicLock), MicVolumeLocker, Static Mic Volume, VolumeLocker.
Kaynak: https://learn.microsoft.com/windows/win32/api/endpointvolume/

---

## 3. Otomatik oyun ve film bilgisi

| Kaynak | Anahtar | Türkçe | Tür / etiket | Limit / koşul |
|---|---|---|---|---|
| **Steam mağazası** (`appdetails`, `l=turkish`) | Gerekmez | **Evet** (türler Türkçe; açıklama geliştirici çevirdiyse) | Türler + **kategoriler** (Co-op, Online Co-op, Tek oyunculu…). **Kullanıcı etiketleri yok.** | Resmî değil; ~200 istek / 5 dk **(doğrulanmalı)**. Önbellek şart. |
| **Steam kullanıcı etiketleri** (Roguelike vb.) | Gerekmez | Evet | Mağaza servisleri üzerinden Türkçe etiket adlarıyla alınabiliyor **(doğrulanmalı, belgelenmemiş)**; yedek: mağaza sayfasından okuma | — |
| **SteamSpy** | Gerekmez | Hayır | Etiketler | 2026'da kısmen bozuk → **güvenilmez yedek** |
| **IGDB** (Twitch hesabı ile) | Gerekir | Hayır | Türler, temalar, **anahtar kelimeler** (roguelike muhtemelen burada **(doğrulanmalı)**), oyun modları, geliştirici/yayıncı; Steam kimliği eşlemesi | 4 istek/sn; kişisel kullanım ücretsiz |
| **RAWG** | Ücretsiz anahtar | Hayır | Topluluk etiketleri (roguelike dahil **(doğrulanmalı)**) | ~20.000 istek/ay **(doğrulanmalı)**; kullanılan yerde RAWG'ye link zorunlu |
| **TMDB** (film/dizi) | Ücretsiz anahtar | **Evet** (`tr-TR`: başlık, özet, tür) | Türler; yönetmen, oyuncular, dizilerde sezon/bölüm | Ticari olmayan kullanım ücretsiz; "Hakkında"da atıf + logo zorunlu |
| **OMDb** (IMDb puanı, isteğe bağlı) | Ücretsiz anahtar | Hayır | — | 1.000 istek/gün |

**Birleşik strateji**
1. **Steam oyunu:** Steam mağazasından Türkçe türler, geliştirici, yayıncı, yıl, açıklama, kapak, kategoriler (Co-op…) + Steam kullanıcı etiketleri.
2. **Steam dışı oyun (Epic/EA/elle):** IGDB'de isimle arama → oyunun Steam karşılığı bulunursa 1. adıma geçilir (Türkçe veri için); yoksa IGDB verisi, yedek olarak RAWG.
3. **Etiket birleştirme:** Kaynaklardan gelen her şey sabit bir Türkçe sözlüğe eşlenir:
   `roguelike, rogue-lite, Roguelite → Roguelike` · `soulslike, Souls-like → Soulslike` ·
   `Co-op, Online Co-op, LAN Co-op, Co-operative → Co-op` · `Survival → Hayatta kalma` · `Open World → Açık dünya` · `Horror → Korku` · `Metroidvania` …
   Eşlenemeyen etiketler saklanır ama filtrede gösterilmez (sözlük zamanla büyür).
4. **Ağ nezaketi:** Her şey yerel veritabanında önbelleğe alınır; istekler sıraya konup yavaşlatılır, "çok istek" uyarısında beklenir.
5. **Atıf:** "Hakkında" ekranında TMDB, IGDB, RAWG atıfları.

---

## 4. Ekran görüntüsü ve klip klasörleri

| Araç | Varsayılan konum | Değiştirilebilir mi? |
|---|---|---|
| **NVIDIA App / GeForce Experience** (Instant Replay, kayıt, ekran görüntüsü) | `%USERPROFILE%\Videos\<Oyun adı>\` (masaüstü kayıtları "Desktop"); sürüme göre `Videos\NVIDIA\<Oyun>` olabilir **(doğrulanmalı)** | **Evet** — Alt+Z → Ayarlar → Dosyalar ve disk alanı. Kullanıcı bir kez GMID klasörünü seçebilir. |
| **Xbox Game Bar** | `%USERPROFILE%\Videos\Captures` (alt klasör yok) | Evet |
| **Steam** | `<Steam>\userdata\<hesap>\760\remote\<appid>\screenshots\` | "Sıkıştırılmamış kopya" seçeneğiyle ek klasör seçilebilir |
| **AMD Adrenalin (ReLive)** | `%USERPROFILE%\Videos\Radeon ReLive` | Evet |
| **OBS (Replay Buffer dahil)** | `%USERPROFILE%\Videos` | Evet |
| **Medal.tv** | `C:\Medal\` (oyuna göre alt klasörler) | Evet; ama dosyaları elle taşımak Medal kütüphanesini bozar |

**Dosya taşıma kuralları**
- Yeni dosya klasör izleme ile algılanır; dosya **yazılması bitene kadar** (kilit kalkana ve boyut birkaç saniye sabit kalana kadar) beklenir.
- Taşıma güvenli yapılır: aynı diskte doğrudan taşı; farklı diskte kopyala → doğrula → sil.
- **Dikkat:** NVIDIA ve Medal kendi galerilerini tutuyor; dosyayı taşımak **o programların kendi galerisinde** klibin görünmemesine yol açar (GMID galerisinde görünür). Steam ekran görüntüleri Steam'in yükleme özelliği için yerinde kalmalı → **Steam için her zaman kopyala**.
