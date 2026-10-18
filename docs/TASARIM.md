# GMID Tool — Game Mansion In Discord
## Tasarım Dokümanı · v0.14 (taslak)

> **Durum:** Faz 1 (İskelet), Faz 2 (Günlük araçlar), Faz 3a (Sunucular) ve Faz 5 (Video İndirici) kodlandı. Sonraki fazlar bu dokümana göre ilerliyor.
> Bu doküman birlikte güncelleniyor. Kesinleşen kararlar ✅, cevap bekleyenler ❓ ile işaretli;
> hepsinin özeti **Bölüm 11**'de.

---

## İçindekiler
1. [Vizyon ve temel ilkeler](#1-vizyon-ve-temel-ilkeler)
2. [Uygulama yapısı (kategoriler ve sekmeler)](#2-uygulama-yapısı-kategoriler-ve-sekmeler)
3. [Modüller](#3-modüller)
4. [Ek özellikler (öneriler)](#4-ek-özellikler-öneriler)
5. [Teknik mimari](#5-teknik-mimari)
6. [Veri klasörü düzeni (derli toplu yapı)](#6-veri-klasörü-düzeni-derli-toplu-yapı)
7. [Veritabanı taslağı](#7-veritabanı-taslağı)
8. [Gelecek: çevrim içi sürüm ve GMID evreni](#8-gelecek-çevrim-içi-sürüm-ve-gmid-evreni)
9. [Yol haritası](#9-yol-haritası)
10. [Riskler ve yasal notlar](#10-riskler-ve-yasal-notlar)
11. [Kesinleşen kararlar ve açık sorular](#11-kesinleşen-kararlar-ve-açık-sorular)

Ekler: [A — Oyun ve film puanlama araştırması](ARASTIRMA-PUANLAMA.md) ·
[B — Teknik fizibilite araştırması](ARASTIRMA-TEKNIK.md) ·
[C — Oyun içi panel, otomatik bilgi ve galeri araştırması](ARASTIRMA-OVERLAY-METADATA.md)

**Görsel taslak:** [taslak/gmid-tool-taslak.html](taslak/gmid-tool-taslak.html) — tıklanabilir prototip (açılış animasyonu, Giriş Salonu, Oyun Arşivi ve puanlama kartı, Kliplerim, Notlar, Ses & Mikrofon, Oyun İçi Panel). Tarayıcıda açılır.

---

## 1. Vizyon ve temel ilkeler

### 1.1 İsim ve hikâye

**GMID = Game Mansion In Discord.** Discord'da kurulmuş bir oyun malikânesi; Butler'ı, yapay zekâ
botu, müzik botu olan kendi oyun evrenimiz. **GMID Tool** da bu malikânenin **alet çantası**:
malikâne sakinlerinin bilgisayarında duran; oyunlarını, film-dizilerini, notlarını, kliplerini,
indirdiği videoları ve Windows'taki küçük can sıkıcı işleri tek bir şık pencerede toplayan uygulama.

Uygulama **Merkut Studios** imzasını taşır: açılışta Merkut Studios logosu belirir (§3.13), "Hakkında"
ekranında *GMID Tool · Merkut Studios* yazar.

### 1.2 Temel ilkeler

| İlke | Pratikte ne demek? |
|------|--------------------|
| **Optimizasyon** | Tepside beklerken ve oyun oynarken neredeyse sıfır kaynak (hedef: **< 30 MB RAM, ~%0 CPU**). Hızlı açılış (< 1 sn). |
| **Güzel görünüm** | Arka plan resmi/GIF, cam efektli paneller, oyun kapakları, yumuşak geçişler. Açtığında "vay" dedirtmeli. |
| **Anlaşılabilirlik** | Her ekran tek bir işi yapar. En fazla 2 tıkla her şeye ulaşılır. Türkçe arayüz, net ikonlar, boş ekranlarda "ne yapmalıyım" ipucu. Elle veri girmek en aza iner (bilgiler internetten otomatik gelir). |
| **Derli toplu** | GMID'in **kendi ürettiği her şey** (indirilen videolar, notlar, arka planlar, veritabanı, yedekler) tek bir klasörde durur: `Belgeler\GMID Tool`. Başka programların dosyaları (ör. NVIDIA klipleri) **yerinde kalır**, ama uygulamanın içinde **tek yerde, görsel olarak** gösterilir. Dosya gezgininde klasör aramak yerine her şeyi uygulamadan görür, çift tıklayıp açarsın. |

### 1.3 Denge kuralı: güzellik ↔ optimizasyon ✅

İkisinden de vazgeçmiyoruz. Kural şu:

> **Güzellik, sadece sen bakarken maliyet üretir.**
> Pencere açıkken uygulama tüm şıklığıyla çalışır; pencere kapalıyken, küçültülmüşken veya
> bir oyun açıkken görsel maliyet **sıfıra** iner.

Bunu sağlayan kurallar:
- **Ucuz güzellik serbest:** Renkler, yazı tipleri, gölgeler, köşeler, kapak resimleri, sabit arka plan — neredeyse hiçbir şeye mal olmaz, cömertçe kullanılır.
- **Pahalı güzellik akıllı:** Hareketli arka plan (video/GIF) ve bulanık cam efektleri ekran kartıyla çizilir, sadece görünürken çalışır; pencere arkada kalınca veya oyun başlayınca otomatik durur.
- Animasyonlar yalnızca ekran kartının "bedava" yaptığı türden (kayma, solma) olur; uygulamayı kasan ağır efektler kullanılmaz.
- **Görünüm kalitesi ayarı:** `Otomatik (önerilen)` · `Yüksek` · `Hafif`. Otomatik mod, bilgisayar zorlanırsa (ör. dizüstü pilde) efektleri kendisi azaltır.

### 1.4 Kapsam ✅

Başlangıçta **yerel (internetsiz çalışan), Windows 10/11**, sadece senin bilgisayarında ve uzun süre
sadece sen kullanacaksın. Arkadaşların da ileride puan girebilsin istiyorsun; ne zaman VDS'e geçileceği
belli değil. Bu yüzden:
- **Veri modeli baştan çok kişili** kuruluyor (her puanın, notun bir "yazarı" var).
- Sunucu gelene kadar **zorunlu hiçbir çevrim içi özellik yok.**
- Ortak arşiv geldiğinde (§8) hiçbir şeyi yeniden yazmak gerekmeyecek; istenirse sunucusuz bir ara çözüm de var (öneri #9).

### 1.5 Görsel kimlik: Oyun Malikânesi ✅

Üç logo tek bir temada buluşuyor: **gece, altın ve neon** — modern bir oyun malikânesi.

| Logo | Kullanıldığı yer | Dosya |
|------|------------------|-------|
| **Merkut Studios** (turkuaz kartal) | Açılış animasyonu, Hakkında ekranı | `assets/marka/merkut-studios.jpg` |
| **GMID** (altın şato) | Uygulama ikonu, görev çubuğu, sistem tepsisi, sol menünün üstü | `assets/marka/gmid-logo.webp` |
| **GMID AI Butler** (neon uşak) | Sadece renk ilhamı (neon camgöbeği ve mor) | `assets/marka/gmid-ai-butler.webp` |

**Renk paleti** (logolardan ölçüldü):

| Rol | Renk | Kaynak | Nerede? |
|-----|------|--------|---------|
| Gece (zemin) | `#121417` | — | Pencere zemini |
| Kömür (paneller) | `#2F3235` | GMID logosu zemini | Kartlar, menü, paneller |
| **Malikâne Altını** (ana vurgu) | `#E3C270` · parlama `#F0D878` | GMID logosu | Seçili menü, ana düğmeler, puan yıldızları, ince altın çizgiler |
| **Merkut Turkuazı** (ikincil vurgu) | `#35A0A2` | Merkut Studios | Aktif durumlar, ilerleme çubukları, bağlantılar |
| Neon Camgöbeği | `#60D8D8` | Butler | Ekran göstergesi, bildirim, "canlı" durumlar (ör. oyun oynanıyor) |
| Lacivert | `#0B203E` | Merkut Studios | Açılış ekranı geçişi, koyu vurgu alanları |
| Butler Moru | `#7848A8` | Butler | Çok az: etiketler, özel rozetler |

**Tarz:**
- GMID logosundaki gibi **ince çizgi (line-art) ikonlar**, seçiliyken hafif **altın parlama**.
- Koyu zemin + cam paneller; arka plan resmi/GIF'i bunların arkasında görünür (§3.10).
- Varsayılan arka plan: koyu zeminde çok hafif altın bir şato silueti (sabit resim, maliyeti yok).
- Yazı tipi: başlıklarda logodaki gibi yuvarlak hatlı kalın bir yazı tipi (ör. *Montserrat*), metinde okunaklı *Inter*.

**Menü isimleri:** Kategoriler malikânenin odaları; içlerindeki sekmeler **düz ve anlaşılır** isimde kalır
(tema havası verir ama "bu ne işe yarıyordu?" dedirtmez). Bkz. §2.

---

## 2. Uygulama yapısı (kategoriler ve sekmeler)

Sol tarafta daraltılabilir bir menü (en üstte altın GMID logosu), sağda içerik. Kategoriler malikânenin
odaları, sekmeler düz isimli:

```
🏰 Giriş Salonu          → Ana sayfa: bugün/bu hafta oynanan süre, son klipler, yaklaşan
                            hatırlatıcılar, son notlar, devam eden indirmeler

🎮 Oyun Salonu
   ├─ Sunucular          → GMID topluluğunun oyun sunucuları: açık mı, kaç kişi, harita; tek tıkla Bağlan
   ├─ Kütüphanem         → Steam / Epic / EA / elle eklenen oyunlar, kapaklar, oynama süreleri
   ├─ Oturum Geçmişi     → Hangi gün hangi oyunu ne kadar oynadım (grafik)
   ├─ Kliplerim          → Ekran görüntüleri ve klipler (NVIDIA Instant Replay vb.), oyuna göre
   └─ Oyun Modu          → Tek tıkla "oyuna hazırlan" profili

🍿 Eğlence Salonu
   ├─ Oyun Arşivi        → Puanladığımız oyunlar, yorumlar, notlar + tür/etiket filtreleme
   ├─ Film & Dizi Arşivi → Aynısı film ve diziler için
   └─ Video İndirici     → Bağlantı yapıştır + indirme kuyruğu + "Videolarım" kütüphanesi (IDM benzeri)

🧰 Çalışma Odası
   ├─ Notlar
   ├─ Hatırlatıcılar
   ├─ Sayaç              → Kronometre + geri sayım
   ├─ Ses & Mikrofon     → Kulaklık/hoparlör geçişi, mikrofon seviyesi kilidi, kısayollar
   └─ Format Sonrası Kurulum

⚙️ Kontrol Odası        → Ayarlar: görünüm (arka plan, kalite, açılış animasyonu), kısayol
                            tuşları, başlangıçta açılma, veri klasörü, yedekleme, eklenti bağlantısı
```

**Pencereyi açmadan erişim:**
- **Oyun İçi Panel** (Steam'deki `Shift+Tab` gibi): oyundan çıkmadan açılan yarı saydam panel — bkz. §3.11.
- **Sistem tepsisi ikonu** (saatin yanı): sağ tık → ses cihazı, mikrofon, hızlı not, hatırlatıcı, sayaç.
- **Genel kısayol tuşları** — tam liste §3.8'de, hepsi ayarlardan değiştirilebilir.

✅ Oyun arşivine hem **Eğlence Salonu → Oyun Arşivi**'nden hem de **oyunun kütüphane kartından** ulaşılır (veri tek yerde).
✅ Film'in yanında **dizi** de var.

---

## 3. Modüller

Her modül için: **Amaç → MVP (ilk sürümde olacaklar) → Sonra (sonraki sürümler) → Teknik not.**

### 3.1 Video İndirici + Chrome/Opera Eklentisi (IDM benzeri) ✅ *(Faz 5 — kodlandı)*

> **Kodlanan hâli (özet):**
> - **Araçlar ilk kullanımda iner:** yt-dlp (~18 MB), FFmpeg (~89 MB, sadece ffmpeg/ffprobe ve kütüphaneleri) ve
>   Deno (~43 MB) resmî GitHub sürümlerinden `Araclar\` klasörüne. Kurulum dosyası küçük kalır. yt-dlp günde en fazla
>   bir kez, boştayken kendini günceller; "Şimdi güncelle" ve "Araçları yeniden indir" düğmeleri var. İnternet
>   bağlantıları Windows'un sertifika deposunu ve vekil sunucu ayarını kullanır (antivirüs HTTPS taraması olan
>   bilgisayarlarda da çalışsın diye).
> - **Kalite kartları** sitenin gerçek biçim listesinden hesaplanır: her çözünürlük için önce her yerde açılan
>   H.264/MP4, sonra VP9, sonra AV1; doğrudan indirme parçalı yayından önce. Tahmini boyut, inecek dosyaların
>   toplamıdır. Önerilen: 1080p ya da altındaki en yüksek. Ayrıca "Sadece ses (MP3)".
> - **Kuyruk:** aynı anda 1–4 indirme (varsayılan 2), duraklat / devam ettir (kaldığı yerden), iptal (yarım dosyalar
>   silinir), hata olursa anlaşılır Türkçe mesaj ve "Tekrar dene". Uygulama kapanırken inenler açılışta "duraklatıldı".
> - **Dosya adları:** `Başlık [kimlik] 1080p.mp4`, ses için `Başlık [kimlik] (ses).mp3` — aynı videonun farklı
>   kaliteleri birbirinin üzerine yazmaz. Küçük resimler `Kapaklar\Videolar\`.
> - **Eklenti köprüsü:** Native Messaging yerine **sadece bu bilgisayardan erişilen küçük bir yerel sunucu**
>   (127.0.0.1, port 47913–47917). Sebep: Native Messaging ayrı bir yardımcı program ve her tarayıcı için ayrı kayıt
>   defteri girdisi istiyor; Opera'da çalışıp çalışmadığı belirsizdi. Güvenlik: eklenti bir kez uygulamada gösterilen
>   **6 haneli kodla** eşleşir (10 dk geçerli, 5 deneme) ve gizli bir anahtar alır; web sitelerinden gelen istekler
>   reddedilir.
> - **Eklenti:** araç çubuğu penceresi (kalite kartları + inen işlerin ilerlemesi), YouTube/X/Instagram/TikTok video
>   sayfalarında sol altta "GMID ile indir" düğmesi, sağ tık menüsünde "GMID Tool ile indir" (önerilen kaliteyle).
>   Instagram/X/Facebook'ta o sitenin çerezleri indirme süresince geçici dosyayla verilir, iş bitince silinir.
>   Eklenti dosyaları uygulamanın içinde gelir; "Hazırla" ile `Eklenti\` klasörüne yazılır.
> - **Çalma listeleri (v0.11):** liste bağlantısı → videolar seçilir, kalite seçilir, toplu iner; `Videolar\<Site>\<Liste adı>\`.
> - **Her sitede yakalama (v0.11):** sayfa tanınmazsa eklenti oynatıcının yüklediği mp4/HLS/DASH kaynağını ağ trafiğinden
>   yakalar; uygulama sayfa adresi (Referer), tarayıcı kimliği ve sayfa başlığıyla indirir. Video üstü düğme, sağ tık
>   kalite menüsü (istenen kalite yoksa en yakını).
> - **Oynatıcı ve yazılar (v0.11):** uygulama içi oynatıcı (MP4/WebM; MKV dışarıda açılır), sağda senkron yazılar,
>   o anki satır/kelime işaretli. Kaynak: sitenin altyazısı (YouTube'da önce videonun kendi dilindeki kelime zamanlı
>   otomatik altyazı) ya da yerel Whisper (whisper.cpp; model `Araclar\whisper\models`). `.srt`/`.lrc` dosyası yazılır.
> - **v0.12 (geri bildirim turu):**
>   - *Kendiliğinden yeniden deneme:* geçici hatalar (403, 5xx, zaman aşımı, bağlantı kopması) 3 kez, artan aralıkla
>     (5/10/20 sn) yeniden denenir; durum "Tekrar deneniyor (n/3)". Hata mesajları sebebe göre (internet ≠ site reddi).
>   - *Gizli yayın listeleri:* eklenti sayfanın kendi dünyasında (`sniff.js`, MAIN world) oynatıcının indirdiği
>     yanıtların içeriğine bakar; `#EXTM3U` ile başlayan her yanıt (adı `.txt`/`.png` olsa da) HLS olarak yakalanır.
>     Gömülü oynatıcıda (iframe) Referer olarak iframe'in adresi kullanılır. X/Instagram akışında videonun kendi
>     gönderi bağlantısı önce denenir.
>   - *Şarkı sözü veritabanı:* "Otomatik" kaynakta önce LRCLIB (lrclib.net; satır zamanlı sözler; yalnızca şarkı adı,
>     sanatçı ve süre gönderilir, süre ±6 sn tutmalı), sonra site altyazısı, sonra Whisper. Ayardan kapatılabilir.
>   - *Whisper uydurmaları:* dil kaydın ortasındaki 30 sn'den bulunur (girişteki müzik yanıltmasın), başlık ipucu
>     (`--prompt`) verilir, konuşma dışı simgeler bastırılır; "Altyazı M.K.", "Thanks for watching" gibi eğitim
>     verisi imzaları ve uzun sessizliğe yayılmış, aynen tekrarlanan kısa satırlar atılır.
>   - *Yeni oynatıcı:* özel kontroller (oynat/duraklat, önceki/sonraki, karıştır, tekrar: kapalı/liste/parça,
>     ilerleme, hız menüsü, ses kaydırıcısı + sessiz, tam ekran), kapağa/videoya tıklayınca durur, **Sahne** ve
>     **Liste** görünümü (solda parçalar, sağda sözler), Spotify benzeri alt çubuk, Windows medya tuşları.
> - **v0.13:**
>   - *Medya Oynatıcı ayrı ekran* (Eğlence Salonu): kütüphane (Müzik / Videolar / site / çalma listesi süzgeçleri,
>     "Hepsini çal"); Video İndirici yalnızca indirme işleri ve son indirilenler. Oynatıcı uygulama genelinde
>     yaşar: kapatınca (Esc / ⌄) küçülür, başka ekranlarda altta çalma çubuğu, video köşede küçük oynar.
>   - *Tam ekran düzeltildi:* filmde görüntü bütün ekranı kaplar, kontroller altta süzülür, fare durunca gizlenir.
>   - *Dublaj ve altyazı:* çözümlemede ses dilleri (biçimlerin `language` alanı) ve sitenin altyazıları listelenir;
>     indirirken seçilir (ayardaki tercih: Türkçe dublaj, Türkçe altyazı). Seçilen dil `ba[language=…]` ile,
>     altyazılar video bittikten sonra ayrı bir `--skip-download --write-subs` çağrısıyla `<ad>.<dil>.vtt` olarak
>     iner (altyazı hatası videoyu bozmaz). Eklenti oynatıcının ayrıca yüklediği WEBVTT/SRT dosyalarını da
>     içeriğinden tanır ve videoyla birlikte gönderir. Oynatıcıda **CC** menüsü; tercih edilen dil kendiliğinden açılır.
>   - *MP3 sabit bit hızı (320K CBR):* değişken hızlı MP3'te ileri sarınca sözler kayıyordu.
> - **v0.14 · Sesle yazma (Wispr Flow benzeri, tamamen yerel):**
>   - *Tuş:* `WH_KEYBOARD_LL` kancası yalnızca izler (tuşları engellemez, programların gönderdiği tuşları saymaz).
>     Ctrl (sol/sağ) + **sol** Alt, başka tuş olmadan 250 ms basılı → kayıt; AltGr'nin sahte sol Ctrl'u
>     (scanCode 0x21D) ve sağ Alt sayılmaz. Araya tuş girerse iptal. Alt bırakılınca menü açılmasın diye 0xE8
>     "maske" tuşu gönderilir.
>   - *Kayıt:* WASAPI paylaşımlı mod, `AUTOCONVERTPCM` ile doğrudan 16 kHz tek kanal 16 bit (dönüştürme Windows'ta).
>   - *Yazıya dökme:* `whisper-server` (model bir kez yüklenir; `/inference`, multipart), istekte ipucu: özel
>     sözcükler + noktalamalı örnek cümle. NVIDIA (CUDA 12) sürümü `Araclar\whisper-cuda\`; açılamazsa işlemci.
>     Şarkı sözü çıkaran `whisper-cli` de CUDA sürümünü önce dener.
>   - *Toparlama:* `llama-server` (llama.cpp CUDA 12 + cudart) + Gemma 3 4B Q4_K_M; OpenAI uyumlu
>     `/v1/chat/completions`, sıcaklık 0.2. Çıktı denetlenir (boş, %30'dan kısa ya da çok uzunsa ham yazı).
>   - *Yapıştırma:* Ctrl/Alt/Shift/Win bırakılınca pano + `SendInput` Ctrl+V; 0,7 sn sonra eski pano yazısı geri
>     konur. Yönetici olarak açılmış pencerelere yapıştırılamaz → son 20 dikte ayarlarda kopyalanabilir.
>   - Sunucular 15 dk kullanılmazsa kapanır; tuşa basınca konuşma sürerken yeniden yüklenir.
> - **v0.15 (geri bildirim):** Medya Oynatıcı **Müzik** (şarkı listesi) ve **Videolar** (kartlar, site süzgeci)
>   olarak ikiye ayrıldı. Tam ekranda fare pencerenin her yerinde izlenir, kontrollerin üstündeyken gizlenmez.
>   Kenar çubuğundaki gösterge duraklatınca durur. Eski şarkılar: LRCLIB'de yalnızca düz söz varsa, yazı oradan
>   alınır; zamanlama sitenin otomatik altyazısıyla ya da Whisper'ın duyduğu kelimelerle **hizalanarak**
>   (Needleman–Wunsch, Türkçe harf sadeleştirme) bulunur ("lyrics-timed"). Sesle yazma: mikrofon 90 ms'de açılır
>   (ilk hece kaçmaz), 220 ms'de başlangıç sesi; bırakınca bitiş sesi; gösterge penceresi önceden hazırlanır ve
>   gizleme sayacı pencere göründükten sonra başlar ("Yazıya dökülüyor…" takılı kalmaz).
> - **v0.16:** Kendi çalma listeleri (göç 8: `playlists`, `playlist_items`; şarkı silinince listeden düşer):
>   Müzik'te "+ Yeni liste", şarkı seçme penceresi (işaretleme sırası liste sırası), şarkı satırında "listeye ekle".
>   Tam ekranda kontroller görüntünün üstüne binmez, altta ayrı şerit; fare durunca kalkar, alt kenarda görünmez
>   bir "uyandırma bölgesi" var. Mini videoda sağ üst kapat, sağ alt büyüt. Sesle yazmada yazılı OSD yerine
>   ekranın alt ortasında yazısız gösterge (`dikte` penceresi): dinlerken mikrofon seviyesiyle oynayan dalga ve
>   süre, yazıya dökerken üç nokta; zamanlayıcıyla değil durumla kapanır.
> - **v0.17 — Faz 3b-1 (Kütüphanem + Oturum Geçmişi):** göç 9: `games` (kaynak + kaynak kimliği tekil; kurulum
>   klasörü, exe listesi, başlatma adresi, Steam appid, kapak, bilgiler, platform süresi) ve `play_sessions`.
>   "Oyunları tara": Steam (`libraryfolders.vdf`, `appmanifest_*.acf`, `localconfig.vdf` süreleri) ve Epic
>   (`Manifests/*.item`) okunur, istediklerini seçip içe aktarırsın. Elle ekleme: çalışan pencereli
>   uygulamalardan ya da .exe seçerek. Canlı algılama 5 sn'de bir süreç listesine bakar (kurulum klasörü ya da exe
>   adı eşleşmesi; hiçbir şey enjekte edilmez); 30 sn'den kısa oturumlar silinir, açık oturum dakikada bir kaydedilir,
>   uygulama çökerse açılışta kapatılır. Bilgi ve kapak: Steam mağazası (Türkçe açıklama, tür, geliştirici, yıl;
>   `library_600x900` kapak `Kapaklar/Oyunlar/`). Kütüphanem: kapak ızgarası, kaynak süzgeci, arama, sıralama,
>   "Şu an oynuyorsun" şeridi, ayrıntı penceresinde Oyna / Klasör / **Gerçek oynama** ve **Steam'deki süre** ayrı.
>   Oturum Geçmişi: 7/30 gün, günlük yığılmış çubuk grafik (ilk 5 oyun renkli, gerisi "Diğer"), gün gün oturumlar.
> - **v0.18 — Faz 3b-2 (Oyun Arşivi):** göç 10: `reviews` (yazar, konu, soru puanları JSON — `null` = "Yok",
>   elle düzeltme, genel puan, tek cümle, notlar, durum, tavsiye, bitirme tarihi; yazar+konu tekil), `games.tags`
>   (Türkçe etiket listesi) ve `games.in_library` (puanlanmış ama kurulu olmayan oyunlar yalnızca arşivde).
>   Etiketler Steam mağaza sayfasındaki kullanıcı etiketlerinden (`InitAppTagModal`) ve kategorilerden tek bir Türkçe
>   listeye çevrilir ("Action Roguelike", "Rogue-lite" → **Roguelike**; bilinmeyenler alınmaz). Puanlama kartı:
>   5 kaydırıcı (7'den başlar), Hikâye ve Performans'ta **Yok**, genel puan otomatik (35/20/20/15/10, "Yok"un payı
>   oransal dağılır) + yarımşar ±1 elle; durum (Oynuyorum · Bitirdim · %100 · Bıraktım · İstiyorum), tavsiye, notlarda
>   `||spoiler||`. Keşfet: sağda filtre kutusu (tür ve etiket — ilk 8 + "Tümünü göster", etiketler VE ile; puan 7+/8+/9+;
>   durum, tavsiye, mağaza VEYA ile), her seçeneğin yanında seçilirse kalacak oyun sayısı, sıfırsa soluk; seçilenler
>   ✕'li etiketler, "Temizle"; dar pencerede sağdan çekmece. Sıralama: puan, son oynama, süre, çıkış yılı, son puanlanan.
>   **Bana bir oyun seç**: filtreye uyanlardan (bırakılan ve "uzak dur" denenler hariç) rastgele, kısa bir zar animasyonu.
>   Arşive ekleme: kütüphaneden ya da Steam'de adıyla aratarak; Steam'de yoksa yalnızca adıyla. Son 30 günde oynanıp
>   puanlanmamış oyunlar arşivin üstünde "puanını bekliyor" şeridinde. Kütüphanem kartlarında puan rozeti, oyun
>   penceresinde "Puanla"; puanlanmış oyun kütüphaneden kaldırılırsa arşivde kalır.
> - **v0.19 — Faz 3b-3 (Kliplerim):** göç 11: `captures` (yol tekil, tür, kaynak, oyun, klasör/dosya adındaki oyun
>   ipucu, elle ad, boyut, süre, çözünürlük, küçük resim, favori, elle seçilen oyun). İzleme 15 sn'de bir:
>   Videolar kökünde OBS adları, `Captures` (Xbox), `Radeon ReLive` (AMD), `NVIDIA\<Oyun>` ve NVIDIA adlandırmasına
>   uyan (`Oyun 2024.10.11 - 21.33.12.02.DVR.mp4`) oyun klasörleri; Steam `userdata\*\760\remote\<appid>\screenshots`;
>   `C:\Medal`; kullanıcının eklediği klasörler. Kökteki rastgele filmler alınmaz. Dosya boyutu bir tur sabit kalınca
>   eklenir. Oyun: Steam appid → çekildiği an açık oyun (oturum ±) → klasör/dosya adı; kütüphaneye sonradan eklenen oyunla
>   10 dakikada bir yeniden eşleşir; elle seçilen korunur. Küçük resim ve süre ffmpeg ile (`Kapaklar\Klipler`).
>   Yeni klipte gösterge ("Klip kaydedildi — Valorant (0:30)"). Ekran: tür süzgeci, oyun seçimi, oyuna göre gruplar,
>   üzerine gelince sessiz önizleme, tıklayınca büyük görünüm (←/→), çift tıkla varsayılan programda açma; favori,
>   ad, oyun değiştirme, klasörde göster, sil (Geri Dönüşüm Kutusu). **Discord'a sığdır:** süreye göre bit hızı ve
>   çözünürlük (≤ 10 MB, gerekirse yeniden), kopya `Discord'a Hazır` (3 gün sonra silinir) ve panoya **dosya** olarak
>   (Discord'da Ctrl+V); zaten küçükse kendisi. Kontrol Odası → Oyunlar: klasörler, gösterge, eski klip temizliği
>   (favori olmayan, 30/60/90 günden eski; varsayılan kapalı).

**Amaç:** O an açık sekmedeki videoyu (YouTube, Twitter/X, Instagram…) yakalamak, mevcut
çözünürlükleri listelemek, seçileni **uygulamanın kendi Videolar klasörüne** indirmek.

**Nasıl çalışacak (akış):**
1. Tarayıcıda video olan bir sayfadasın → eklenti ikonunda bir rozet çıkar / videonun üstünde küçük "İndir" düğmesi belirir.
2. Tıklayınca eklenti sayfanın adresini masaüstü uygulamasına yollar.
3. Uygulama videonun kaynağını çözer ve seçenekleri döndürür: `2160p · 1440p · 1080p · 720p · 480p · Sadece ses (MP3/M4A)` + tahmini boyutlar.
4. Seçtiğin kalite indirilir, ilerleme hem eklentide hem uygulamada görünür.
5. Bitince Windows bildirimi: "İndirme tamamlandı — Aç / Klasörde göster".

**Uygulamada nerede?** ✅ Video indirici uygulamanın **en görünür** parçalarından biri:
- Eğlence Salonu'nda kendi sekmesi: **Video İndirici** (menüde devam eden indirme sayısını gösteren rozet).
- Üst çubukta her ekrandan görünen indirme göstergesi: "2 indiriliyor · %47" → tıklayınca Video İndirici açılır.
- Giriş Salonu'nda "Devam eden indirmeler" kutusu.

**Video İndirici ekranı (yukarıdan aşağıya):**
1. **Bağlantı yapıştır** kutusu → "Kaliteleri getir" → kalite kartları (2160p · 1440p · 1080p *önerilen* · 720p · 480p · sadece ses MP3), her birinde tahmini boyut → **İndir**. Tarayıcı açık olmasa da çalışır.
2. **Eklenti durumu:** "Chrome eklentisi bağlı · Opera eklentisi bağlı".
3. **İndirmeler** kuyruğu: küçük resim, başlık, site, kalite, ilerleme çubuğu, hız, kalan süre, duraklat / iptal.
4. **Videolarım:** indirilenler; siteye göre sekmeler (Tümü · YouTube · X · Instagram), çift tıkla aç.

**Tarayıcı eklentisi (Chrome / Opera):** Video olan sayfada eklenti simgesinde rozet çıkar; tıklayınca küçük bir
pencerede kaliteler listelenir → **İndir**. Videonun üstünde ayrıca küçük bir "GMID ile indir" düğmesi belirir.
İndirme başlayınca GMID Tool'un kuyruğunda görünür. (Görsel taslakta "Eklenti nasıl görünüyor?" ile görülebilir.)

**MVP**
- YouTube, Twitter/X, Instagram (ve yt-dlp'nin desteklediği yüzlerce site, ücretsiz bonus).
- Çözünürlük seçimi, sadece ses seçeneği, indirme kuyruğu, duraklat/iptal.
- **Videolarım** sekmesi: küçük resimli kartlar (başlık, kaynak site, süre, boyut, tarih); arama ve filtre; üzerine gelince kısa önizleme; **çift tık → video açılır**; sağ tık → klasörde göster / sil.
- Dosyalar arka planda `Belgeler\GMID Tool\Videolar\` altına, siteye göre alt klasörlere iner (`YouTube\`, `Twitter\`…). Klasöre girmen hiç gerekmez; hepsi uygulamada görünür.

**Sonra**
- Oynatma listesi / kanal toplu indirme, altyazı indirme, video kırpma (başlangıç–bitiş).
- Panodan otomatik link yakalama ("Panoda YouTube linki var, indireyim mi?").

**Teknik not:** Eklenti tek başına indirme yapmaz; sadece "kapı zili" görevi görür. Sayfa adresini
Chrome'un resmî **Native Messaging** kanalıyla (port açmadan) masaüstü uygulamasına iletir. Asıl işi
uygulamanın içindeki **yt-dlp + ffmpeg + deno** üçlüsü yapar (yüksek çözünürlüklerde görüntü ve ses
ayrı gelir, ffmpeg birleştirir; YouTube 2025 sonundan beri deno'yu da gerektiriyor). YouTube sık
değiştiği için yt-dlp kendini otomatik günceller. Instagram/X gibi giriş isteyen sitelerde eklenti
**yalnızca o sitenin** çerezlerini geçici olarak uygulamaya verir, iş bitince silinir.
Ayrıntılar: [Ek B](ARASTIRMA-TEKNIK.md) §1.

> ⚠️ **Önemli:** Chrome Web Store, YouTube'dan indirme yapan eklentileri kabul etmiyor. Bu yüzden
> eklenti mağazadan değil, **"Geliştirici modu → Paketlenmemiş öğe yükle"** ile kurulacak (kurulumu
> adım adım ben anlatacağım). Opera, Chrome eklentilerini doğrudan çalıştırabildiği için
> **aynı eklenti Opera'da da çalışacak**.

❓ Varsayılan kalite: "her seferinde sor" mu, "hep en iyisi" mi? Videonun üstüne düğme eklensin mi? (soru 4)

---

### 3.2 Oyun Kütüphanesi ve Süre Takibi (Discord tarzı algılama)

**Amaç:** Hangi platformdan olursa olsun oynadığın oyunları otomatik yakalayıp kaç saat
oynadığını kaydetmek.

**İki katmanlı algılama:**

| Katman | Ne yapar? | Kaynak |
|--------|-----------|--------|
| **1. Platform içe aktarma** | Steam / Epic / EA'da kurulu (ve Steam'de geçmişte oynanmış) oyunları ve **platformun kendi tuttuğu toplam süreyi** içe aktarır. | Steam yerel dosyaları + (isteğe bağlı) Steam Web API, Epic/EA kurulum kayıtları |
| **2. Canlı algılama (Discord gibi)** | Çalışan programları birkaç saniyede bir hafifçe kontrol eder; bilinen bir oyun açılınca **oturum başlatır**, kapanınca bitirir. Platformdan bağımsız: Battle.net, Riot, emülatörler, bağımsız oyunlar… | Discord'un herkese açık "algılanabilir oyunlar" listesi + Steam/Epic kurulum klasörleri + elle eklenen .exe'ler |

**MVP**
- İlk açılışta "Oyunlarını tara" sihirbazı → bulunan oyunlar listelenir, istemediklerini çıkarırsın.
- Her oyun için: **otomatik kapak** ve bilgiler (§3.3), toplam süre (platform + uygulamanın ölçtüğü), son oynama, oturum sayısı.
- **Oturum geçmişi**: günlük/haftalık grafik ("Bu hafta 14 saat oynadın, %60'ı Elden Ring").
- Tanınmayan bir oyun için: "Şu an çalışan uygulamalardan oyun ekle" düğmesi.
- AFK algılama (isteğe bağlı): 15 dk klavye/fare hareketi yoksa süreyi saymayı durdur.

- ✅ **"Kütüphaneye ekleyeyim mi?" (v0.21):** Kütüphanede olmayan bir program açılınca sorulur: Discord'un herkese açık
  bilinen oyunlar listesinde varsa 20 sn sonra, listede yoksa 1 dakikadan uzun tam ekran çalışırsa (tarayıcı, video
  oynatıcı, Windows programları hariç). Sağ altta sessiz kart: **Ekle · Şimdi değil · Bir daha sorma**; cevaplanmayanlar
  Kütüphanem'in üstünde "Algılanan oyunlar" olarak durur (ad düzeltilip eklenebilir). Ayarlardan kapatılabilir.

**Sonra:** Kütüphaneden oyunu doğrudan başlatma (Playnite gibi), yıllık "GMID Wrapped" özeti.

**İki ayrı süre:** Steam'in tuttuğu süre, kart toplama / süre artırma programlarıyla şişebilir.
Bu yüzden her oyunda iki sayı görünür: **"Gerçek oynama"** (GMID'in ölçtüğü, oyun gerçekten açıkken) ve
**"Steam'deki süre"**. İstatistikler ve Wrapped gerçek oynamayı kullanır.

**Teknik not:** Süreç kontrolü 5 sn aralıkla, sadece program adı karşılaştırmasıyla yapılır — işlemciye
etkisi ölçülemeyecek kadar az. Oyunların içine **hiçbir şey enjekte edilmez** (anti-cheat güvenliği).
Ayrıntılar: [Ek B](ARASTIRMA-TEKNIK.md) §2.

❓ Steam Web API anahtarı / AFK süresi (soru 3)

---

### 3.3 Oyun Arşivi — Puanla, Yaz, Keşfet

**Amaç:** Arkadaşlar arasında, samimi bir şekilde oyunları puanlayıp yorum ve not yazdığımız; sonra
**"canım roguelike bir şey oynamak istiyor"** dendiğinde tür seçip filtreleyerek eski puanlarımızı ve
notlarımızı görebildiğimiz arşiv.

#### a) Puanlama kartı: 5 kısa soru ✅

Sadece "1–10 kaç?" diye sormuyor, ama ayrıntıya da boğmuyor. **5 kısa soru**, hepsi tek kaydırıcı,
toplam ~30 saniye. Genel puan cevaplardan **otomatik hesaplanır**; içine sinmezse elle oynayabilirsin.

```
┌───────────────────────────────────────────────────────────────────┐
│ [KAPAK]  Hades II                                                  │
│          Roguelike · Aksiyon · İzometrik        2025 · Supergiant  │
│                                                                   │
│  🎮 Oynaması ne kadar zevkli?              ●━━━━━━━━━○  9         │
│  📖 Hikâye ve atmosfer nasıl?              ●━━━━━━━○━━  8   [Yok] │
│  🎨 Görsellik ve ses nasıl?                ●━━━━━━━━○━  9         │
│  ⚙️ Performans / optimizasyon nasıl?       ●━━━━━━━━○━  9         │
│  ❤️ Seni ne kadar sardı, aklında kaldı mı? ●━━━━━━━━━○ 10         │
│                                                                   │
│  ⭐ Genel puan: 9.0 / 10  (otomatik — istersen ±1 değiştir)         │
│                                                                   │
│  Tek cümle   "Bir tur daha derken sabah oldu."                     │
│  Durum       [Oynuyorum] [Bitirdim] [%100] [Bıraktım] [İstiyorum]   │
│  Tavsiye     👍 Kesin oyna   🤷 Duruma göre   👎 Uzak dur           │
│  Notlar      (uzun yazı, ipuçları, build'ler; spoiler gizlenebilir) │
│                                                      [Kaydet]     │
└───────────────────────────────────────────────────────────────────┘
```

**Sorular ve genel puana etkisi** (araştırmadaki 9 kriterin sadeleştirilmiş hali — [Ek A](ARASTIRMA-PUANLAMA.md)):

| Soru | İçinde neler var? | Etkisi |
|------|-------------------|--------|
| 🎮 Oynanış | Eğlence, kontroller, zorluk dengesi, içerik | **%35** |
| 📖 Hikâye & Atmosfer | Senaryo, karakterler, dünya, atmosfer | **%20** |
| ❤️ Keyif & Akılda kalıcılık | Seni ne kadar sardı, tekrar oynar mısın | **%20** |
| 🎨 Görsellik & Ses | Sanat tarzı, grafik, müzik, seslendirme | **%15** |
| ⚙️ Performans | FPS, hata/çökme, optimizasyon | **%10** |

- **[Yok] düğmesi:** Hikâyesi olmayan bir multiplayer oyunda "Hikâye" sorusu **Yok** işaretlenir; etkisi diğer sorulara oransal dağılır (puanı düşürmez). Kötü hikâye ≠ hikâye yok.
- **Puan rehberi:** Her kaydırıcının yanında `5 = vasat · 7 = iyi · 8 = çok iyi · 9 = mükemmel · 10 = başyapıt` → puanlar zamanla hep 8–9'a kaymaz.
- **Varsayılan konum:** Kaydırıcılar 7'den başlar; sadece farklı düşündüğün soruyu oynatırsın.
- Oynama süresi, platform ve bitirme tarihi **kütüphaneden otomatik** dolar.
- Yüzdeler ayarlardan değiştirilebilir (ama değiştirmek zorunda değilsin).

#### b) Bilgiler internetten otomatik gelir ✅

Oyun eklendiğinde (kütüphaneden ya da isimle aratarak) **hiçbir şeyi elle yazmazsın**. Otomatik gelenler:

| Bilgi | Örnek |
|-------|-------|
| Kapak, banner, logo | Yüksek kaliteli kapak resmi |
| Tür(ler) | Aksiyon, RPG, Strateji… |
| **Etiketler** | **Roguelike**, Soulslike, Metroidvania, Co-op, Açık dünya, Hayatta kalma, Hikâye odaklı… |
| Geliştirici / Yayıncı | Supergiant Games / Supergiant Games |
| Çıkış yılı, platformlar | 2025 · PC, PS5, Switch |
| Kısa açıklama | Mümkünse Türkçe |
| Oyun modları | Tek oyunculu, çok oyunculu, co-op |

- **Kaynaklar:** Steam'deki oyunlar için Steam mağazasının kendi bilgileri (türler **Türkçe** gelir, Co-op gibi
  kategoriler ve Roguelike gibi kullanıcı etiketleri dahil); Steam dışındaki oyunlar için büyük oyun
  veritabanları (önce IGDB, yedek RAWG); kapaklar için SteamGridDB. Ayrıntı: [Ek C](ARASTIRMA-OVERLAY-METADATA.md) §3.
- **Etiketler tek bir Türkçe listeye çevrilir:** Farklı kaynaklar "Rogue-like", "Roguelite", "Action Roguelike"
  gibi farklı yazsa da hepsi filtrede tek bir **Roguelike** kutusunda toplanır.
- Yanlış eşleşme olursa: "Bu oyun değil" → doğru oyunu seç. Bilgiler istenirse elle düzeltilebilir.

#### c) Keşfet ekranı (filtrele, bul, öner) ✅

**Yerleşim ✅:** Alışveriş sitelerindeki gibi **sağda filtre kutusu**, solda oyun kartları.
- Filtre grupları: **Tür ve etiket** (en çok kullanılan 8'i görünür, "Tümünü göster" ile hepsi), **Puan** (7+, 8+, 9+), **Durum**, **Tavsiye**, **Mağaza**.
- Her seçeneğin yanında, seçersen **kaç oyun kalacağı** yazar; sonuç çıkmayacak seçenekler soluk görünür.
- Seçilen filtreler kartların üstünde ✕'li etiketler olarak durur; tek tıkla kaldırılır. "Temizle" hepsini sıfırlar.
- Kartların üstünde: sonuç sayısı, sıralama ve **"Bana bir oyun seç"** düğmesi.
- Dar pencerede filtre kutusu gizlenir; **"Filtrele"** düğmesiyle sağdan açılan bir çekmeceye dönüşür ("5 oyunu göster" düğmesiyle kapanır).

Senaryo: *Arkadaşın "roguelike bir şey oynamak istiyorum" dedi.*

1. Oyun Arşivi → filtre çubuğundan **Roguelike** etiketine tıkla.
2. İstersen ekle: `Puan ≥ 7` · `Co-op` · `Kısa oyun` · `Bitirdiklerimiz` · `Kim puanladı`.
3. Sonuç: kapaklı kartlar; her kartta **verdiğimiz puanlar, tek cümlelik yorumlar, tavsiye simgesi**. Karta tıklayınca notların tamamı.
4. Kararsız mısın? **🎲 "Bana bir oyun seç"** düğmesi filtreye uyan oyunlardan birini rastgele önerir.

- Sıralama: puana, son oynamaya, oynama süresine, çıkış yılına göre.
- Arkadaşlar da puan girdiğinde (ortak arşiv, §8) kartta **"Ben 9 · Ali 7 · Ortalama 8"** gibi görünür.

---

### 3.4 Film & Dizi Arşivi

Oyun arşiviyle **aynı kart yapısı ve aynı Keşfet ekranı**; sorular filme göre:

| Soru | İçinde neler var? | Etkisi |
|------|-------------------|--------|
| 📖 Hikâye & Senaryo | Konu, diyaloglar, sonun tatmin ediciliği | **%25** |
| 🎭 Oyunculuk | İnandırıcılık, karakterler | **%20** |
| 🎬 Yönetmenlik & Görsellik | Görüntü, müzik, atmosfer | **%20** |
| ⏱️ Tempo | Sıktı mı, süresini hak etti mi | **%15** |
| ❤️ Keyif & Etki | Sende ne bıraktı, tekrar izler misin | **%20** |

- **Otomatik bilgi (TMDB):** Türkçe ad, afiş, özet, tür(ler), yönetmen, başrol oyuncuları, yıl, süre; dizilerde sezon/bölüm sayısı.
- **Kart alanları:** 5 soru + otomatik genel puan, tek cümle, durum (`İzlenecek · İzliyorum · İzledim · Yarıda bıraktım`), tavsiye, notlar, izleme tarihi, nerede izlendi (sinema/Netflix/…), kiminle izlendi.
- **Diziler için:** "Nerede kaldım?" (ör. **S2 · B5**) ve isteğe bağlı sezon bazında puan.
- Filtre örnekleri: "Bilim kurgu + puan ≥ 8", "Christopher Nolan filmleri", "Kısa diziler (1–2 sezon)".

---

### 3.5 Notlar

**Amaç:** "Yeni Metin Belgesi.txt" kaosunu bitirmek. Hızlı açılan, güzel görünen, kaybolmayan notlar.

**MVP**
- **Hızlı not penceresi:** Kısayolla (`Ctrl+Alt+N`) her yerden açılan küçük, kenarlıksız, yarı saydam pencere. Yaz → kapat → otomatik kaydedilir. "Kaydet" düğmesi yok, kaybolma riski yok.
- **Notlar sekmesi:** Solda not listesi (arama + sabitlenmiş notlar üstte), sağda düzenleyici.
- **Biçimlendirme:** Kalın, italik, başlık, madde işareti, **yapılacaklar kutucuğu ☐**, kod bloğu — ham işaretler görünmeden.
- **Görünüm:** Not başına renk/arka plan, birkaç şık küçük yazı tipi (ör. Inter, JetBrains Mono, Caveat el yazısı), yazı boyutu.
- **Yapışkan not:** Bir notu masaüstünde "her zaman üstte" küçük not olarak açık bırakabilme.
- Klasör/etiket, çöp kutusu (30 gün).
- **Oyuna bağlı notlar:** Bir oyunun kartından o oyuna ait not (build'ler, kodlar, nerede kaldım). Oyun İçi Panel'den de yazılabilir.

**Sonra:** Nota resim yapıştırma, dışa aktarma (.md / .txt).

**Teknik not:** Notlar veritabanının yanında `Notlar\` klasöründe **düz .md dosyası** olarak da
saklanabilir → uygulama olmasa bile Not Defteri'yle açılır.

❓ Notlar .md dosyası olarak da dursun mu? (soru 5)

---

### 3.6 Hatırlatıcılar

**MVP**
- Başlık + tarih/saat + (isteğe bağlı) not.
- Tekrar: tek sefer / her gün / hafta içi / haftalık / aylık / özel.
- Hızlı giriş: "**30 dk sonra**", "**yarın 09:00**" gibi hazır düğmeler.
- **Windows bildirimi** (sağ alt köşe) — "Ertele 10 dk" ve "Tamam" düğmeleriyle; isteğe bağlı ses.
- Pencere kapalıyken de çalışır (tepside bekler). Bilgisayar kapalıyken kaçırılanlar açılışta gösterilir.
- **Oyun sırasında:** Bildirim oyunun üstünde küçük bir gösterge olarak çıkar (§3.11), oyunu alta atmaz.

**Sonra:** "Epic ücretsiz oyun yenilendi" gibi otomatik hatırlatıcılar.

---

### 3.7 Sayaç

**MVP**
- **Kronometre** (tur kaydı ile) ve **geri sayım** (bitince bildirim + ses).
- Birden fazla geri sayım aynı anda (ör. "Fırın 20 dk", "Mola 5 dk").
- Tepsiden ve Oyun İçi Panel'den tek tıkla başlatma.

**Sonra:** Pomodoro modu; "oyun süre sınırı / mola hatırlatıcı" (öneri #5).

---

### 3.8 Ses & Mikrofon

**Amaç:** Windows ayarlarına girmeden, oyundan çıkmadan sesi ve mikrofonu yönetmek.

**MVP**
- **Çıkış cihazları:** Liste (ikon, ad, varsayılan işaretli); tıkla → varsayılan olur. Takma ad ("Kulaklık 🎧", "Hoparlör 🔊"), kullanılmayanları gizleme, cihaz başına ses seviyesi.
- **Mikrofonlar:** Aynı liste + seviye kaydırıcısı + sessize alma.
- **🔒 Mikrofon seviyesi kilidi:** "Mikrofonum kendi kendine kısılıyor, sürekli %100'e çekiyorum" sorununun çözümü.
  Kilidi açınca GMID, bir program mikrofon seviyesini değiştirdiği **anda** fark edip seçtiğin seviyeye (ör. %100) geri çeker.
  Sürekli kontrol etmez; Windows "seviye değişti" diye haber verince devreye girer → **işlemciye maliyeti sıfıra yakın**.
- **Neden kısılıyor? rehberi:** Ses & Mikrofon ekranında kısa bir kontrol listesi: Discord'un otomatik kazanç kontrolü,
  Chrome/Teams/Zoom'un "mikrofonu otomatik ayarla" seçenekleri, Windows'un "uygulamaların özel denetimine izin ver" ayarı.
  Asıl sebebi kapatmak da mümkün; kilit her durumda güvence.
- **Ekran göstergesi (OSD):** Her kısayolda ekranın köşesinde küçük, şık bir bilgi belirir (🎧 Kulaklık · 🔇 Mikrofon kapalı · 🎙️ %100 🔒).
  Oyundan odağı **çalmaz**, oyunu alta atmaz. Göstergenin görünmediği eski tip tam ekran oyunlarda
  **kısa sesli geri bildirim** verilir (§3.11).

**Kısayol tuşları** (varsayılanlar, hepsi ayarlardan değiştirilebilir):

| İşlem | Varsayılan kısayol |
|-------|--------------------|
| Ses çıkışını değiştir (kulaklık ↔ hoparlör) | `Ctrl+Alt+A` |
| Mikrofonu sessize al / aç | `Ctrl+Alt+M` |
| Mikrofon seviyesi kilidini aç / kapat | `Ctrl+Alt+L` |
| Mikrofon seviyesi +5 / −5 | `Ctrl+Alt+PageUp` / `Ctrl+Alt+PageDown` |
| Mikrofonu hemen %100'e çek | `Ctrl+Alt+Home` |
| Oyun İçi Panel'i aç / kapat | `Ctrl+Shift+G` |
| Hızlı not | `Ctrl+Alt+N` |

(Not: `Ctrl+Alt+Ok tuşları` bazı Intel ekran kartı sürücülerinde ekranı döndürdüğü için bilerek kullanılmadı.)

**Sonra:** Uygulama bazlı çıkış (ör. Discord kulaklıktan, oyun hoparlörden), oyun açılınca otomatik kulaklığa geçiş (Oyun Modu ile).

**Teknik not:** Varsayılan cihazı değiştirmek için Windows'un resmî bir düğmesi yok; SoundSwitch gibi
açık kaynak programların yıllardır kullandığı yöntemle yapılıyor. Mikrofon kilidi ve sessize alma ise
Windows'un resmî ses arayüzüyle yapılıyor. Ayrıntılar: [Ek B](ARASTIRMA-TEKNIK.md) §3, [Ek C](ARASTIRMA-OVERLAY-METADATA.md) §2.

❓ Mikrofon kilidi için hedef seviye %100 mü? (soru 2)

---

### 3.9 Format Sonrası Kurulum

**Amaç:** Format attıktan sonra Chrome, Steam, Discord, Claude, WinRAR… gibi programları tek tek
indirmek yerine listeden seçip **tek tıkla hepsini kurmak**.

**MVP**
- **Hazır katalog** (kategorili, ikonlu): Tarayıcılar, Oyun platformları, İletişim, Araçlar, Medya…
- Kendi listeni oluştur: katalogdan ekle/çıkar, sürükle-bırak sıralama, birden fazla liste ("Oyun PC'si", "Minimal").
- "**Kur**" düğmesi → sırayla, sessiz kurulum, her biri için ilerleme ve ✅/❌ durumu.
- Katalogda olmayan bir program için arama (Microsoft'un paket deposunun tamamında).
- **Format sonrası uygulamanın kendisi de kurulu olmayacak.** Bu yüzden liste, **tek başına çalışan bir kurulum dosyasına**
  (`KurulumListem.bat`) dışa aktarılır. USB'ye / buluta koyarsın, formattan sonra çift tıklarsın, her şey kurulur — GMID Tool dahil.
- ✅ **GMID'in kendisi de yedeklenir (istek, 2026-10-11):** Dışa aktarma, kurulum listesinin yanına GMID'in tüm verisini de koyar:
  **ayarlar** (görünüm, kısayollar, sesle yazma, klip klasörleri…), veritabanı (notlar, hatırlatıcılar, oyun kütüphanesi,
  oturum geçmişi, puanlar, çalma listeleri, sunucular), notların .md kopyaları, arka planlar ve kapaklar
  (`GMID-Yedek.zip`). `.bat` önce GMID Tool'u kurar, sonra yedeği yerine açar: uygulama formattan önceki haliyle,
  geçmişiyle birlikte açılır. Büyük ve yeniden indirilebilen şeyler (Whisper/dil modelleri, yt-dlp/ffmpeg, indirilen
  videolar) isteğe bağlı; klipler zaten kendi programlarının klasöründe durur.

**Sonra:** Windows ayarları paketi (dosya uzantılarını göster, karanlık mod, gereksiz başlangıç programlarını kapat vb.).

**Teknik not:** Windows 10/11'de yerleşik **winget** (Microsoft'un resmî paket yöneticisi). Paket kimlikleri: [Ek B](ARASTIRMA-TEKNIK.md) §5.

✅ **Kodlandı (Faz 6b, v0.25):**
- **Katalog** (~60 program, 9 kategori): kimlikler Microsoft'un `winget-pkgs` deposunda tek tek doğrulandı (2026-10).
  WhatsApp Microsoft Store'dan (`msstore`); winget'te olmayan **NVIDIA App** ve **AMD Adrenalin** için resmî indirme
  sayfası açılır. Katalogda olmayanlar için **Tüm depoda ara** (`winget search`). Liste tek; sıra yukarı/aşağı ile.
  Çoklu liste ("Oyun PC'si", "Minimal") şimdilik yok — tek liste yeterli göründü.
- **Kurulu olanlar** `winget export` ile bulunur (Kurulu rozeti; "Şimdi kur" yalnız eksikleri kurar).
- **Şimdi kur:** sırayla, sessiz (`--silent --disable-interactivity`), her biri için indirme yüzdesi ve ✅/❌; "zaten
  kurulu" ve "yeniden başlatma gerekebilir" winget çıkış kodlarından ayrılır. **Durdur** o anki kurulumu yarıda
  kesmez (program yarım kalmasın), sıradakilere geçmez. Kurulamayanlar tek tıkla yeniden denenir.
- **Format Kiti** (seçilen klasörde, ör. USB): `Kur.bat` + `kurulum.ps1` + `GMID-Tool-Kurulum.exe` (İndirilenler'de
  bulunursa kendiliğinden) + `GMID-Yedek.zip` + `BENİOKU.txt`. Betik: winget hazır değilse kaydeder ya da Store'daki
  "Uygulama Yükleyicisi" sayfasını açar; programları kurar; GMID kapalıyken yedeği `Belgeler\GMID Tool`'a açar (orada
  eski veri varsa "GMID Tool (önceki …)" olarak kenara alır); GMID'i sessiz kurup açar; indirme sayfalarını açar; sonunda
  özet. PowerShell 5.1 Türkçe harfleri bozmasın diye UTF-8 BOM'lu.
- **Yedek:** veritabanı (tutarlı kopya, `VACUUM INTO`), Notlar, Arkaplanlar, Kapaklar, Sunucular, Eklenti; isteğe bağlı
  Videolar ve Araclar (modeller). Zip içindeki `gmid-yedek.json` eski veri klasörü ve kullanıcı klasörünü tutar; ilk
  açılışta veritabanındaki bütün yollar (düz ve JSON içindeki) yeni bilgisayara çevrilir — kullanıcı adı değişse de
  videolar, kapaklar, arka plan bozulmaz.
- **Kontrol Odası → Veri → Yedekten geri yükle:** zip doğrulanır (bildirge + SQLite başlığı; yalnız bilinen klasörler
  açılır, zip dışına yazamaz), onaylanınca uygulama yeniden başlar; veritabanı kapalıyken mevcut olan
  `Yedekler\geri-yukleme-oncesi-….db` olarak saklanır, yedek yerine konur.

❓ Listene kesin girecek programlar (soru 6)

---

### 3.10 Görünüm ve Arka Plan

**MVP**
- Arka plan: **düz renk / degrade / resim (JPG, PNG, WebP) / hareketli (GIF, WebM, MP4)**.
- Bulanıklık ve karartma kaydırıcıları (yazılar hep okunur kalsın diye), cam efektli paneller.
- Varsayılan tema **Malikâne** (§1.5: gece zemin, altın ve turkuaz vurgular); vurgu rengi değiştirilebilir. Açık tema seçeneği de olacak.
- Arka planlar `Arkaplanlar\` klasörüne kopyalanır (orijinal silinse de bozulmaz).
- Notlar ve hızlı not penceresi için ayrı arka plan seçimi.
- **Görünüm kalitesi:** Otomatik / Yüksek / Hafif (§1.3).

**Optimizasyon (denge kuralı gereği):**
- Pencere küçültülünce / arkada kalınca / **bir oyun algılanınca hareketli arka plan otomatik durur**.
- GIF seçersen, GMID onu arka planda aynı görünen ama 5–10 kat daha hafif bir videoya çevirir (görüntü aynı, yük daha az).
  *(Bu dönüştürme ffmpeg gerektirdiği için Faz 5'te Video İndirici ile birlikte gelecek. Faz 1'de GIF doğrudan gösterilir;
  pencere arkadayken ilk karesinde durdurulur.)*

---

### 3.11 Oyun İçi Panel (Steam'deki Shift+Tab gibi) ✅

**Amaç:** Oyunu alta atmadan, Alt+Tab yapmadan ses, mikrofon, not, sayaç gibi şeyleri kontrol etmek.

**Üç katman** (en hafiften en kapsamlıya):

| Katman | Ne yapar? | Nerede çalışır? |
|--------|-----------|-----------------|
| **1. Kısayol + ekran göstergesi** | Kısayola basarsın, iş yapılır; köşede küçük gösterge çıkar (§3.8). Odak oyunda kalır. | Pencereli ve **çerçevesiz tam ekran** (borderless) oyunlarda; Windows'un tam ekran iyileştirmeleri sayesinde modern tam ekran oyunların çoğunda da. Gösterge görünmese bile **kısayolun kendisi her durumda çalışır**. |
| **2. GMID Oyun Paneli** | `Ctrl+Shift+G` → oyunun üstünde yarı saydam panel açılır. Aynı tuş / `Esc` ile kapanır, oyuna geri dönersin. | Pencereli / çerçevesiz tam ekran oyunlarda ve modern tam ekran oyunların çoğunda. Eski tip "özel tam ekran" oyunlarda panel açılınca oyun alta düşebilir → orada katman 1 kullanılır. |
| **3. Xbox Game Bar parçacığı** *(v1'de yok, belki ileride)* | `Win+G` ile açılan Xbox Game Bar'ın içine GMID parçacığı. Microsoft'un resmî yolu, ama ayrı bir teknolojiyle (UWP) yazılması gerekiyor (~2–3 hafta ek iş). | Game Bar'ın açıldığı yerlerde; gerçek özel tam ekranda çalışıp çalışmadığı kesin değil. |

**Panelde neler var?**
- 🎧 Ses çıkışı seçimi, ses seviyesi
- 🎙️ Mikrofon: seviye, 🔒 kilit, sessize alma
- ⏱️ Bu oturumda oynanan süre, sayaç / mola zamanlayıcısı
- 📝 Bu oyuna not (oyuna bağlı notlar)
- 📸 Son kaydedilen klip / ekran görüntüsü
- 🔔 Yaklaşan hatırlatıcılar

**Güvenlik:** Steam'in ve Discord'un eski overlay'i gibi oyunun **içine** kod sokmuyoruz; panel, oyunun
üstünde duran **ayrı bir Windows penceresi**. Bu yüzden anti-cheat (Vanguard, EasyAntiCheat, BattlEye)
açısından risk yok. Discord da Mart 2025'te overlay'ini tam olarak bu yönteme geçirdi (ve o da yalnızca
çerçevesiz/pencereli modda çalışıyor) — yani doğru yoldayız. Ayrıntılar: [Ek C](ARASTIRMA-OVERLAY-METADATA.md) §1.

**Senin kullanımın ✅:** Çoğunlukla **çerçevesiz**, bazı oyunlarda **tam ekran** oynuyorsun. Buna göre:
- **Çerçevesiz oyunlar:** Panel ve gösterge sorunsuz çalışır.
- **Tam ekran oyunlar:** Modern oyunların çoğu (DX11/DX12) Windows'ta aslında "optimize pencere" olarak
  çalıştığı için panel büyük ihtimalle yine görünür. Görünmediği eski tip oyunlarda:
  - Kısayollar **yine çalışır** (mikrofon sessize alma, seviye, cihaz değiştirme).
  - Gösterge görünmediği için **sesli geri bildirim** devreye girer: mikrofon kapanınca/açılınca Discord'daki
    gibi farklı kısa sesler, cihaz değişince kısa bir "tık". (Ayarlardan kapatılabilir.)
  - GMID bu oyunları hatırlar; o oyunda paneli açmaya çalışıp oyunu alta atmaz.
- İleride tam ekran oyunlarda panel çok lazım olursa **Xbox Game Bar parçacığı** (katman 3) eklenebilir.

---

### 3.12 Kliplerim — Ekran Görüntüleri ve Klipler ✅

**Amaç:** NVIDIA Instant Replay, Xbox Game Bar, Steam vb.'nin her yere dağıttığı klip ve ekran
görüntülerini **uygulamanın içinde tek yerde, oyuna göre düzenli** göstermek.

✅ **Karar: Dosyalar taşınmaz, yerinde kalır.** GMID onları sadece **bulur, oyuna göre gruplar ve
gösterir**. Dosya gezgininde klasör aramak yerine her şeyi uygulamadan görürsün, çift tıklayıp açarsın.
Böylece:
- NVIDIA'nın, Steam'in, Medal'ın kendi galerileri de **bozulmadan** çalışmaya devam eder.
- Taşıma/kopyalama olmadığı için disk iki kat dolmaz, dosya kaybolma riski de yok.

**Nasıl çalışır?**
1. GMID, kayıt programlarının klasörlerini sessizce izler (NVIDIA App / Instant Replay, Xbox Game Bar, Steam, AMD, OBS, Medal…). İlk açılışta mevcut klipleri de tarar.
2. Yeni bir klip kaydedilince (ör. NVIDIA'da `Alt+F10`), dosyanın yazılması bitince listeye ekler.
3. **Hangi oyunda çekildiğini bilir** — çünkü o an hangi oyunun açık olduğunu zaten takip ediyor (§3.2).
   Böylece "Desktop" ya da garip isimli klasörlere düşen klipler bile uygulamada doğru oyunun altında görünür.
4. Küçük bir gösterge: "🎬 Klip kaydedildi — Valorant (0:30)".

**Kliplerim ekranı (MVP)**
- Oyuna göre gruplanmış kapaklı bölümler, içinde küçük resimler; tarih sırası.
- Üzerine gelince sessiz önizleme oynar; **çift tık → klip açılır.**
- Favori ⭐, yeniden adlandır, klasörde göster, sil (Windows Geri Dönüşüm Kutusu'na gider, geri alınabilir).
- **"Discord'a sığdır"** düğmesi: Klibin Discord'un ücretsiz yükleme sınırına (10 MB) sığan bir kopyasını
  üretir ve panoya koyar → Discord'a direkt yapıştırırsın. (Kopya `Belgeler\GMID Tool\Discord'a Hazır\`
  altında durur, birkaç gün sonra kendiliğinden silinir.)
- Disk koruması: "Favori olmayan, X günden eski klipleri geri dönüşüm kutusuna gönder" seçeneği (varsayılan kapalı).

**Sonra:** Klip kırpma (başlangıç–bitiş), birden fazla klibi birleştirme, GIF'e çevirme.

Ayrıntılar (klasör konumları): [Ek C](ARASTIRMA-OVERLAY-METADATA.md) §4.

---

### 3.13 Açılış Animasyonu ✅

Uygulama açılırken karanlıktan yumuşak bir geçişle **Merkut Studios** logosu belirir, sonra kararır ve
GMID Tool açılır:

```
 0.0 sn  ■ Tam siyah ekran
 0.3 sn  ◐ Merkut Studios logosu karanlıktan yavaşça belirir (hafif büyüyerek, %96 → %100)
 1.2 sn  ● Logo tam görünür; turkuaz çizgilerde çok hafif bir parıltı geçer
 2.0 sn  ◑ Logo yavaşça kararır
 2.5 sn  ▶ GMID Tool karanlıktan açılır (menü ve içerik yumuşakça belirir)
```

- Toplam ~2,5 saniye. **Tıklayınca veya `Esc`/`Boşluk`'a basınca atlanır.**
- Sadece ekran kartının "bedava" yaptığı geçişler kullanılır (solma, büyüme) → takılma olmaz.
- **Ne zaman oynar?** Uygulama **kapalıyken** açıldığında. Tepsiden pencereyi açarken **oynamaz** (anında açılır).
  Windows açılışında otomatik başlarken de oynamaz; uygulama sessizce tepsiye yerleşir.
- Ayar: `Açılış animasyonu: Açık / Kapalı` (Kontrol Odası → Görünüm).

### 3.14 Sunucular ✅ *(Faz 3a — kodlandı)*

GMID topluluğunun eski oyunlar için açtığı sunucular (Call of Duty 2, Minecraft…) tek ekranda:

```
┌──────────────────────────────────────────────┐
│ [CoD2]  GMID · Call of Duty 2        ● Açık  │
│         Call of Duty 2  GMID                 │
│ Her akşam 21:00'de TDM.                      │
│ 👥 14 / 24  ████████░░░░░░  🔒               │
│ Toujane · Takım ölüm maçı · 1.3 · 41 ms      │
│ cod2.gmid.gg:28960                    [⧉]    │
│ [▶ Bağlan] [Nasıl bağlanırım?] [👥 14]       │
└──────────────────────────────────────────────┘
```

- **Kutucuklar:** logo (yoksa oyunun kısa adıyla renkli rozet; Minecraft'ta sunucunun kendi simgesi),
  açık/kapalı, oyuncu sayısı ve doluluk çubuğu, harita, oyun modu (CoD için Türkçe: "Takım ölüm maçı",
  "Arama & İmha"…), sürüm, gecikme (ms), şifreli mi. "Oyuncular" ile içerideki oyuncuların adları ve skorları.
- **Canlı:** ekran açıkken her 30 saniyede bir sorulur; tek bir kaçırılan yanıt "Kapalı" saydırmaz
  (bir önceki bilgi soluk gösterilir). Giriş Salonu'nda da küçük bir özet kutusu var.
- **Bağlan:**
  | Oyun | Ne olur? |
  |---|---|
  | CoD 1 / UO / 2 / 4 | Oyun `+connect adres` ile açılır, doğrudan sunucuya girer. İlk seferde oyunun `.exe`'si Steam klasörlerinde aranır; bulunamazsa bir kez seçtirilir. |
  | CS 1.6, CS2, diğer Steam oyunları | `steam://connect/…` — Steam oyunu açıp sunucuya sokar. |
  | Minecraft Bedrock | Sunucu Minecraft'taki Sunucular listesine eklenir. |
  | SA-MP, MTA | Kendi istemcileri açılır. |
  | Minecraft Java, Valheim | Doğrudan bağlanma yolu yok → adres panoya kopyalanır, "Nasıl bağlanırım?" adım adım anlatır. |
- **Topluluk listesi:** GMID sunucuları bu projenin GitHub'daki [`topluluk/sunucular.json`](../topluluk/sunucular.json)
  dosyasından gelir. Dosya düzenlenince herkesin uygulamasında birkaç dakika içinde güncellenir (nasıl düzenleneceği:
  [`topluluk/README.md`](../topluluk/README.md)). İnternet yoksa son liste gösterilmeye devam eder. Adres Sunucular
  ekranının altından değiştirilebilir ya da kapatılabilir.
- **Kendi sunucuların:** "Sunucu ekle" ile (adres `1.2.3.4:28961` gibi portla da yazılabilir, isteğe bağlı logo,
  açıklama ve bağlanma notu). Bunlar sadece senin bilgisayarında durur.
- **Desteklenen sorgular** (hepsi uygulamanın kendi kodu, ek program yok): Quake 3 `getstatus` (Call of Duty ailesi;
  Türkçe isimler için Windows-1254), Minecraft Java durum sorgusu, Minecraft Bedrock (RakNet), Valve A2S
  (CS 1.6, CS2, TF2, GMod, Valheim…), SA-MP. MTA ve "Diğer" sadece adres ve bağlanma bilgisi gösterir.
- **Güvenlik:** listeden gelen adresler sadece harf, rakam, nokta ve tire içerebilir; oyun komut satırına ya da
  bağlantıya başka bir şey sızamaz. Logolar en fazla 2 MB.

---

## 4. Ek özellikler (öneriler)

**Durum:** ✅ = onaylandı · ⭐ = önerim · 👍 = eklenirse güzel olur · 🤔 = sonra düşünürüz · ❌ = eklemeyelim

**Onaylananlar:** Oyun Modu, mikrofon kısayolları (+ seviye kilidi), Oyun İçi Panel, otomatik kapak ve bilgi, Kliplerim.

| # | Özellik | Ne işe yarar? | Zorluk | Durum |
|---|---------|---------------|--------|-------|
| 1 | **Oyun Modu** | Tek tık (veya oyun açılınca otomatik): güç planını "Yüksek performans"a al, kulaklığa geç, mikrofon kilidini aç, bildirimleri sustur, seçtiğin uygulamaları (ör. tarayıcı) kapat, GMID animasyonlarını durdur. Oyun kapanınca her şeyi geri al. | Orta | ✅ |
| 2 | **Birleşik oyun başlatıcı** | Steam/Epic/EA oyunlarını GMID kütüphanesinden doğrudan başlat. | Orta | 👍 |
| 3 | **Ücretsiz oyun bildirimi** | Epic'in haftalık ücretsiz oyunu ve Steam'deki geçici ücretsiz oyunlar çıkınca bildirim. | Kolay | ⭐ |
| 4 | **İstek listesi + indirim takibi** | Oyun Arşivi'nde "İstiyorum" durumundaki oyun hedef fiyatın altına düşünce bildirim. | Kolay–Orta | 👍 |
| 5 | **Oyun süresi sınırı / mola hatırlatıcı** | "Bugün en fazla 3 saat" veya "her 2 saatte bir mola" → oyun algılanınca sayar, dolunca gösterge. | Kolay | 👍 |
| 6 | **Komut paleti (`Ctrl+K`)** | Her şeyi tek arama kutusundan yap: "kulaklık", "not: market", "hatırlat 20dk çay", "roguelike"… | Orta | 👍 |
| 7 | **Yıllık özet ("GMID Wrapped")** | En çok oynanan oyunlar, toplam saat, en yüksek puanlar, izlenen filmler — Discord'da paylaşılabilir görsel kart. | Kolay | ⭐ |
| 8 | **Discord Rich Presence** | Discord profilinde "GMID Tool'da: Hades II puanlıyor" gibi durum. Malikânenin tanıtımı için de iyi. | Kolay | ✅ v0.45 (Kontrol Odası → Oyunlar) |
| 9 | **Arşiv paylaşım dosyası** | Sunucu olmadan arkadaş puanları: puan ve yorumlarını tek bir dosya olarak dışa aktar, Discord'dan arkadaşına at; o içe aktarınca kartlarda "Ben 9 · Ali 7" görünür. VDS gelene kadar köprü. | Kolay | ⭐ |
| 10 | **FPS / donanım izleme** | CPU/GPU sıcaklık, FPS. Sıcaklık sürücüsü bazı anti-cheat'lerle sorun çıkarabiliyor; MSI Afterburner zaten iyi yapıyor. | Orta–Zor | 🤔 |
| 11 | **Sürücü güncelleme hatırlatıcısı** | "Ekran kartı sürücünü 60 gündür güncellemedin." | Orta | 🤔 |
| 12 | Pano geçmişi / ekran görüntüsü aracı | Windows'ta zaten var (`Win+V`, `Win+Shift+S`). | — | ❌ |
| 13 | Makro, otomatik tıklama, nişangâh (crosshair), oyunun **içine** çizim yapan overlay | Online oyunlarda **ban riski**. | — | ❌ |

> **Güvenlik kuralı (tüm uygulama için):** GMID Tool hiçbir oyunun belleğini okumaz, oyuna kod enjekte
> etmez, oyunun içine çizim yapmaz. Sadece "hangi program çalışıyor" bilgisini okur (Discord kadar) ve
> kendi ayrı pencerelerini gösterir. Böylece anti-cheat sistemleriyle sorun yaşanmaz.

---

## 5. Teknik mimari

### 5.1 Teknoloji seçimi ✅

Programlama tarafı tamamen bende olduğu için seçimi **optimizasyon + güzel görünüm** dengesine göre yaptım:

| Katman | Seçim | Neden? |
|--------|-------|--------|
| Uygulama çatısı | **Tauri 2** | Electron'a göre ~10 kat küçük kurulum ve çok daha az RAM; Windows'un derin özelliklerine (ses, süreçler, bildirimler) doğrudan erişim. |
| Çekirdek | **Rust** | Hızlı, az bellek, çökmeye dayanıklı. Oyun algılama, ses/mikrofon, indirme, galeri izleme burada. |
| Arayüz | **Svelte 5 + TypeScript** | Çok hafif ve hızlı; cam efektleri, hareketli arka plan, yumuşak geçişler kolay. |
| Veritabanı | **SQLite** (tek dosya) | Kurulum gerektirmez, hızlı, yedeklemesi = dosya kopyalamak. |
| Video | **yt-dlp + ffmpeg + deno** | Sektör standardı, ücretsiz, açık kaynak. Uygulamayla birlikte gelir, kendini günceller. |
| Oyun / film bilgileri | **Steam mağazası, IGDB/RAWG, SteamGridDB, TMDB** | Ücretsiz; bilgiler bir kez çekilip yerelde saklanır (internetsiz de görünür). |
| Program kurulumu | **winget** | Windows'ta yerleşik, Microsoft'un resmî paket yöneticisi. |
| Tarayıcı eklentisi | **Manifest V3** (Chrome + Opera aynı kod) | Chrome'un güncel eklenti standardı. |

Kurulum: Tek bir `GMID-Tool-Kurulum.exe`; güncellemeler uygulama içinden tek tıkla.

### 5.2 Genel yapı

```
┌──────────── Chrome / Opera ────────────┐
│  GMID Eklentisi (videoyu fark eder,     │
│  "İndir" düğmesi + kalite menüsü)       │
└───────────────────┬─────────────────────┘
                    │ Native Messaging (port açmadan, güvenli)
┌───────────────────▼───────────────────────────────────────────────┐
│ GMID Tool (Tauri 2)                                                 │
│                                                                     │
│  ┌─ Pencereler (sadece açıkken yaşar) ───────────────────────────┐  │
│  │  Ana pencere · Oyun İçi Panel · Ekran göstergesi · Hızlı not   │  │
│  └──────────────────────────┬─────────────────────────────────────┘  │
│                  komutlar ⇅ olaylar                                 │
│  ┌─ Çekirdek (Rust, her zaman çalışır, tepside) ─────────────────┐  │
│  │  Oyun Algılayıcı    → çalışan programlar (5 sn), tam ekran tespiti│
│  │  Ses Servisi        → cihazlar, mikrofon kilidi (olay tabanlı)  │  │
│  │  Klip İzleyici      → kayıt klasörlerini izler, klipleri listeler│  │
│  │  Bilgi Servisi      → oyun/film bilgisi ve kapak çeker, önbellek│  │
│  │  Zamanlayıcı        → hatırlatıcılar, sayaçlar, bildirimler     │  │
│  │  İndirme Yöneticisi → yt-dlp / ffmpeg / deno kuyruğu            │  │
│  │  Kurulum Servisi    → winget                                    │  │
│  │  Depo Katmanı       → SQLite + dosya klasörleri                 │  │
│  │  Tepsi ikonu · Genel kısayollar · Windows bildirimleri          │  │
│  └─────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### 5.3 Optimizasyon stratejisi

- **Pencere kapatılınca arayüz tamamen kapanır**, sadece küçük Rust çekirdeği tepside kalır. Pencereyi açınca arayüz yeniden yüklenir (< 1 sn).
- **Oyun açıkken "sessiz mod":** Animasyonlar durur; kapak indirme, güncelleme kontrolü gibi işler oyun bitene kadar ertelenir.
- **Olay tabanlı çalışma:** Mikrofon kilidi, galeri izleyici gibi servisler sürekli kontrol etmez; Windows "bir şey değişti" deyince uyanır.
- **Tembel yükleme:** Her sekme ancak açıldığında yüklenir. Kullanmadığın modül ayarlardan tamamen kapatılabilir.
- **Ölçülebilir hedefler** (her sürümde ölçülecek):

  | Durum | RAM | CPU |
  |-------|-----|-----|
  | Sadece tepside | < 30 MB | ~%0 |
  | Oyun oynanırken (panel kapalı) | < 30 MB | ~%0 |
  | Pencere açık, sabit arka plan | < 150 MB | < %1 |
  | Pencere açık, hareketli arka plan | < 200 MB | < %3 (ekran kartıyla çözülür) |

### 5.4 Arayüz ilkeleri

- **Malikâne kimliği:** Gece zemin, altın ve turkuaz vurgular, GMID logosu (§1.5). Yuvarlatılmış köşeler, cam paneller → arka plan görünür ama yazılar okunur.
- **Kapak odaklı:** Oyun ve film listeleri tablo değil, kapaklı kart vitrini.
- Her ekranda tek bir ana eylem düğmesi (ör. Notlar'da "+ Yeni not").
- Boş ekranlar boş kalmaz: "Henüz notun yok — `Ctrl+Alt+N` ile her yerden hızlı not alabilirsin" gibi yol gösterici mesajlar.
- Klavye dostu: ana işlemlerin kısayolu var.
- Silinen her şey önce çöp kutusuna gider; "Geri al" bildirimi.

### 5.5 Kendiliğinden sığan arayüz ✅

Pencereyi küçültüp büyüttüğünde, tam ekran yaptığında ya da farklı bir monitöre taşıdığında **her şey
kendiliğinden yerine oturur**. Hiçbir içerik pencerenin dışına taşmaz, arkasında kalmaz, ezilmez veya
uzamaz. Bunu sağlayan kurallar:

**Resimler, kapaklar, videolar**
- Hepsi **oranını koruyarak** sığar: kapaklar ve küçük resimler kutularını düzgünce doldurur, hiçbir zaman yamuk/uzamış görünmez.
- Video oynatıcı ve klip önizlemeleri pencere boyutuna göre büyür/küçülür; görüntü kesilmez (gerekirse kenarlarda ince siyah bant).
- Arka plan resmi/GIF'i her pencere boyutunda ekranı kaplar; resmin **hangi kısmının görüneceğini** (ör. karakterin yüzü) ayarlardan seçebilirsin, pencere daralınca o kısım ortada kalır.
- Ekranın netliğine göre doğru boyutta resim yüklenir (4K'da keskin, küçük pencerede gereksiz büyük resim yüklenmez → hız).

**Yerleşim**
- **Kart ızgaraları akışkan:** Oyun/film/video kartlarının sütun sayısı pencere genişliğine göre otomatik değişir (geniş pencerede 6–7, dar pencerede 2–3). Kartlar boşluk bırakmadan yeniden dizilir.
- **Sol menü:** Geniş pencerede isimleriyle, dar pencerede **sadece ikonlara** daralır, çok dar pencerede açılır-kapanır menüye dönüşür. İçerik alanı hiçbir zaman menünün altında kalmaz.
- **Uzun yazılar** satır kaydırır; tek satıra sığması gereken başlıklar "…" ile kısalır, üzerine gelince tamamı görünür.
- **Pencereler ve kutular** (puanlama kartı, ayarlar, onay pencereleri) her zaman pencerenin içinde ortalanır; sığmazsa kendi içinde kaydırılır — düğmeler asla görünmez olmaz.
- **Tablolar** dar pencerede karta dönüşür (yan yana sütunlar alt alta).
- En küçük pencere boyutu sınırlı (ör. 900×600); daha küçüğe indirmek "yarım ekran" görünümü bozmasın diye engellenir.

**Ekran ve monitör**
- Windows ekran ölçeklendirmesi (%100–%200), 1080p, 1440p, 4K ve geniş (ultrawide) ekranlarda aynı düzgünlükte görünür.
- Pencerenin boyutu ve konumu hatırlanır. Monitör çıkarılırsa ya da çözünürlük değişirse pencere **ekranın dışında açılmaz**, görünen alana geri getirilir.
- Açılış animasyonu, Oyun İçi Panel, ekran göstergesi, hızlı not ve yapışkan notlar da bulundukları ekrana göre ölçeklenir ve ekran kenarından taşmaz.

**Test kuralı:** Her faz sonunda her ekran şu boyutlarda kontrol edilir: küçük pencere (900×600), Full HD
tam ekran, 4K (%150 ölçek) ve ultrawide. Taşan, kesilen, üst üste binen tek bir öğe bile kalmaz.

---

## 6. Veri klasörü düzeni (derli toplu yapı)

Konum ✅: `Belgeler\GMID Tool\` (istersen ayarlardan başka diske taşınabilir).
Bu klasör arka planda düzenli durur ama **içine girmen gerekmez** — her şey uygulamada görsel olarak görünür.
Format atarken **bu tek klasörü yedeklemen yeterli.**

```
GMID Tool\
├─ gmid.db                  ← Tek SQLite veritabanı (oyunlar, puanlar, notlar, klip listesi, ayarlar)
├─ Videolar\                ← İndirilen videolar (uygulamada "Videolarım")
│  ├─ YouTube\  ├─ Twitter\  ├─ Instagram\  └─ Diger\
├─ Discord'a Hazır\         ← "Discord'a sığdır" kopyaları (birkaç gün sonra kendiliğinden silinir)
├─ Notlar\                  ← Notların .md kopyaları (uygulama olmadan da okunabilir)
├─ Arkaplanlar\
├─ Sunucular\              ← Sunucu logoları (topluluk listesinden indirilen ve elle seçilen)
├─ Eklenti\                ← Chrome/Opera eklentisi (tarayıcıda "Paketlenmemiş öğe yükle" ile bu klasör seçilir)
├─ Kapaklar\                ← Oyun/film kapak önbelleği ve video küçük resimleri (Videolar\)
├─ Kurulum\                 ← (ayrılmış) Format Kiti istenen yere, ör. USB'ye çıkarılır
├─ Yedekler\                ← Veritabanının otomatik günlük yedekleri (son 7 gün)
├─ Araclar\                 ← yt-dlp, ffmpeg, deno (ilk kullanımda iner, uygulama kendisi günceller)
└─ gmid.log                 ← Sorun olursa ne olduğunu gösteren küçük günlük dosyası
```

Klipler ve ekran görüntüleri bu klasörde **değildir**; kaydedildikleri yerde (NVIDIA, Steam vb. klasörleri)
kalır, uygulama sadece listesini tutar (§3.12).

---

## 7. Veritabanı taslağı

İlk sürüm için kabaca tablolar (kodlama aşamasında kesinleşecek):

| Tablo | Temel alanlar |
|-------|---------------|
| `people` | id, ad, avatar — şimdilik sadece "ben"; ortak arşivde arkadaşlar |
| `games` | id, ad, kaynak (steam/igdb/rawg/manual), kaynak_id, exe_adları, kapak, geliştirici, yayıncı, çıkış_yılı, açıklama, oyun_modları |
| `tags` / `game_tags` | Türkçe etiket listesi (Roguelike, Co-op…) · oyun↔etiket |
| `play_sessions` | id, game_id, başlangıç, bitiş, süre_sn, kaynak (canlı/içe aktarma) |
| `reviews` | id, yazar (people), konu (oyun/film/dizi + id), soru_puanları (JSON: soru→puan veya "yok"), genel_puan, elle_düzeltme, tek_cümle, notlar, durum, tavsiye, tarih |
| `titles` | id, tmdb_id, tür (film/dizi), ad, orijinal_ad, yıl, afiş, özet, yönetmen, oyuncular, sezon_sayısı |
| `captures` | id, game_id, dosya_yolu (dosyanın kendi yerinde), tür (klip/ekran görüntüsü), kaynak (nvidia/xbox/steam…), süre, boyut, favori, küçük_resim, tarih |
| `downloads` | id, url, video_kimliği, başlık, yükleyen, site, küçük_resim, süre, kalite, biçim_seçimi, parça_sayısı, sadece_ses, durum, ilerleme, boyut, dosya_yolu, hata, kaynak (uygulama/eklenti), tarihler *(Faz 5 ✅)* |
| `notes` | id, başlık, içerik, renk, yazı_tipi, sabit, klasör, etiketler, bağlı_oyun, güncellenme |
| `reminders` | id, başlık, zaman, tekrar_kuralı, ertelenen, tamamlandı |
| `audio_devices` | windows_id, takma_ad, ikon, gizli, sıra, kilit_seviyesi |
| `settings: install.list` | Format Sonrası Kurulum listesi (JSON: kimlik, ad, kaynak winget/msstore/web, kategori, sıra) *(Faz 6b ✅; ayrı tablo gerekmedi)* |
| `servers` | id, ad, oyun, adres, port, sorgu_portu, logo, logo_adresi, açıklama, bağlanma_notu, topluluk_mu, sıra *(Faz 3a ✅)* |
| `settings` | anahtar, değer |

Tüm kayıtlar ileride senkronizasyona hazır olacak şekilde benzersiz kimlik + oluşturma/güncelleme zamanı taşır.

---

## 8. Gelecek: çevrim içi sürüm ve GMID evreni

### 8.1 Ortak arşiv ve sunucu *(tarih belirsiz — VDS alınınca)*

- **Ortak arşiv:** Malikâne sakinlerinin oyun/film puanları ve yorumları tek havuzda; kartlarda "Ben 9 · Ali 7 · Ortalama 8".
  Veri modeli bunun için şimdiden hazır (§7 `people`, `reviews.yazar`).
- **VDS'teki küçük sunucu** (WebSocket): arkadaş listesi, "şu an ne oynuyor" durumu, sohbet, ortak oyun listeleri.
- Yerel veriler her zaman yerelde kalır; neyin paylaşılacağını herkes kendisi seçer.
- Sunucu gelene kadar arkadaş puanları için **paylaşım dosyası** (öneri #9) kullanılabilir.

### 8.1.1 Sohbet penceresi *(Faz 3c — barındırma kararı bekleniyor)*

Steam'deki arkadaş listesi + sohbet penceresi gibi: kim çevrim içi, kim hangi oyunda (Faz 3b'deki oyun algılamasından),
özel mesaj ve herkesin konuştuğu bir "Malikâne" odası. Ayrı, küçük bir pencere; tepsiden ve kısayolla açılır,
oyundayken gelen mesaj sağ altta odak çalmayan küçük bir kartla görünür.

- **Giriş:** Discord hesabıyla (şifre yok, herkes zaten Discord'da). Sadece GMID Discord sunucusunun üyeleri girebilir.
- **Gerekli olan:** mesajları ileten, sürekli açık küçük bir sunucu (WebSocket). Seçenekler ve maliyetleri sohbette
  konuşulup buraya yazılacak.

### 8.2 GMID evreninin diğer parçaları (bilgi için)

GMID Tool **tek başına çalışan** bir uygulama; Discord botlarına hiçbir şekilde ihtiyaç duymaz.
Botlarla tek ortak noktası **tema ve logolar**. İleride sunucu kurulursa bağlantı fikirleri isteğe bağlıdır.

| Proje | Teknoloji | GMID Tool ile ilişkisi |
|-------|-----------|------------------------|
| [GMID Butler](https://github.com/BalamanMerkut/GMID-Butler) | Python · discord.py · Gemini 2.5 Flash · SQLite · Render'da | Yok. Logosu tema renklerine ilham verdi. *(İleride istenirse: sunucu kurulduktan sonra Discord'da arşivden oyun önerebilir.)* |
| [GMID Music Bot](https://github.com/BalamanMerkut/GMID-Discord-MusicBot) | Python · discord.py · yt-dlp · OpenAI | Yok. |
| [Steam Card Idler](https://github.com/BalamanMerkut/Steam-Card-Idler) | C# · .NET 8 · WPF · SteamKit2 | Şimdilik gerek yok. |

**Botlar için bilmekte fayda var** (GMID Tool'dan bağımsız):
- Müzik botu da GMID Tool'un video indiricisi gibi **yt-dlp** kullanıyor. yt-dlp, 2025 sonundan beri YouTube için ek olarak
  **deno** istiyor. Müzik botu bir gün YouTube'dan çalmayı bırakırsa sebebi büyük ihtimalle bu: bot sunucusuna deno kurulup yt-dlp güncellenmeli.
- VDS'e geçildiğinde botlar da aynı VDS'e taşınabilir; Render'daki "uyanık tutma" hilesine gerek kalmaz.

---

## 9. Yol haritası

Mantık: **önce her gün kullanacağın, kolay ve hızlı biten modüller** → uygulamayı erkenden
kullanmaya başlarsın, eksikleri yaşarken görürsün. Zor modüller (video indirici) sonraya.

| Faz | İçerik | Sonunda elinde ne olacak? |
|-----|--------|---------------------------|
| **0 — Tasarım** ✅ | Bu doküman, kararlar, ekran taslakları | Onaylanmış plan |
| **1 — İskelet** ✅ *kodlandı* | Tauri projesi, Malikâne teması ve logolar, **açılış animasyonu**, sol menü, Kontrol Odası (ayarlar), veri klasörü, SQLite, tepsi ikonu, Windows ile başlatma, arka plan (resim/GIF/video), otomatik yedek | Boş ama şık, tepside duran uygulama |
| **2 — Günlük araçlar** ✅ *kodlandı* | Ses & Mikrofon (kilit, kısayollar, ekran göstergesi), Notlar (+ hızlı not, yapışkan not), Hatırlatıcılar, Sayaç | Her gün kullandığın araçlar |
| **3a — Sunucular** ✅ *kodlandı* | Topluluk ve kendi sunucuların, canlı oyuncu sayısı/harita, tek tıkla Bağlan, GitHub'dan güncellenen topluluk listesi | Discord'daki sunucuları uygulamadan görüp girmek |
| **3b — Oyun** ✅ *kodlandı* | Oyun algılama, Steam/Epic/EA içe aktarma, kütüphane, otomatik bilgi ve kapak, Oyun Arşivi (puan, yorum, keşfet), Kliplerim | Discord gibi oyun yakalayan, puanlayıp filtreleyebildiğin arşiv |
| **3c — Sohbet** *(barındırma kararından sonra)* | Discord ile giriş, arkadaş listesi + kim ne oynuyor, özel mesaj ve ortak oda | Steam sohbeti gibi pencere |
| **4 — Oyun içi deneyim** ✅ *kodlandı* | Oyun İçi Panel, Oyun Modu | Oyundan çıkmadan kontrol |
| **5 — Video İndirici** ✅ *kodlandı* | İndirme yöneticisi, Videolarım, Chrome/Opera eklentisi, araçların kendiliğinden kurulup güncellenmesi | IDM benzeri indirici |
| **6 — Film & Dizi + Kurulum** ✅ *kodlandı* | Film & Dizi Arşivi, format sonrası kurulum + dışa aktarılan .bat | Tüm istenen modüller tamam (**v1.0**) |
| **7 — Öneriler** 🔨 *büyük kısmı kodlandı* | ✅ Uygulama içinden güncelleme · ✅ Fırsatlar (ücretsiz oyunlar + istek listesi indirimleri) · ✅ GMID Wrapped · ✅ Her şeyde tek arama (Ctrl+K) · ✅ Klip kırpma + daha net Discord'a sığdırma · ✅ Panelde FPS/CPU/GPU · ⏸ Otomatik haftalık tam yedek *(şimdilik bekliyor)* | v1.x |
| **8 — Çevrim içi** *(tarih belirsiz)* | Sunucu (VDS), ortak arşiv, arkadaşlar, sohbet, **oyun gecesi planlayıcı**, **GMID'e paylaş** (puan kartı, klip, Wrapped ve "bu hafta ne oynadım" özetinin tek tıkla GMID Discord sunucusuna gitmesi; kullanıcı kararı: VDS ile birlikte) | v2.0 |

**Yapılacaklar listesi (2026-10-12, kullanıcıyla konuşulan):**
- ⏸ **Otomatik haftalık tam yedek:** Format Kiti'ndeki `GMID-Yedek.zip`, her hafta seçilen bir klasöre (OneDrive/Drive)
  kendiliğinden; son N tanesi saklanır. Altyapı hazır (`restore::archive`); yalnız zamanlayıcı ve ayar gerekiyor.
- ⏸ **GMID'e paylaş (VDS gerekince):** Discord sunucusuna gönderim bir bot/sunucu üzerinden. Not: yalnız bir kanala
  göndermek için Discord webhook'u VDS'siz de çalışır; istenirse ara adım olarak yapılabilir.
- ⏸ **Discord uygulaması / bot (VDS alınınca; 2026-10-13 kullanıcı kararı):** GMID'in Discord Developer Portal'daki
  uygulaması bot ile birlikte açılacak; paylaşma özellikleri (GMID'e paylaş, oyun gecesi) o bota bağlanacak. **Discord'da
  durum** (Rich Presence) kodu hazır (`presence.rs`, VDS gerektirmez) ama ayarı gizlendi: bot uygulamasının kimliği
  uygulamaya gömülünce kendiliğinden açılır, kullanıcıdan kimlik istenmez.
- ⏸ **Oyun gecesi planlayıcı (VDS alınınca; 2026-10-13 kullanıcı kararı):** "Cuma 21:00 CS2" gibi etkinlik oluştur;
  GMID Discord'una duyuru (bot), katılacaklar "Varım/Yokum" der, saati gelince katılanlara GMID hatırlatıcısı ve
  oyun içi panelde geri sayım; kaç kişi olduğuna göre Çark & Çekiliş'teki dengeli takımlar tek tıkla. Ortak takvim
  ve katılım listesi sunucuda tutulacağı için VDS ile birlikte.
- ⏸ **Disk temizliği (2026-10-13: kullanıcı "şimdilik kalsın" dedi):** yapılırsa kurallar: yalnız Windows'un geçici
  klasörleri (`%TEMP%`, `Windows\Temp`, Windows Update indirme önbelleği, küçük resim önbelleği, hata dökümleri) ya da
  Windows'un Disk Temizleme altyapısı; yalnız 7 günden eski ve kullanılmayan dosyalar; önce liste ve boyut, onaysız
  silme yok; İndirilenler, oyun klasörleri, klipler ve belgelere asla dokunulmaz.
- 💡 PlayStation Store ve Microsoft Store **indirimleri** için anahtarsız resmî kaynak yok; şimdilik yalnız ücretsizleri
  geliyor. Güvenilir bir kaynak bulunursa Fırsatlar'a eklenir.

**Sıra değişikliği (2026-10-11):** Önce uygulamanın tek kullanıcı için yerelde sağlam çalışması ve Video İndirici
istendi; Faz 3b (oyun algılama, kütüphane, arşiv, klipler), Faz 4 ve sohbet (3c) sonraya kaldı.

Her faz sonunda: çalışan bir sürüm (`.exe` kurulum dosyası), kısa değişiklik notu, performans ölçümü.
*(Sunucu beklemeden arkadaş puanları istenirse paylaşım dosyası — öneri #9 — Faz 7'de yapılabilir.)*

---

## 10. Riskler ve yasal notlar

| Risk | Etkisi | Önlem |
|------|--------|-------|
| YouTube/Instagram/X yapılarını değiştirir, indirme bozulur | Video indirici geçici çalışmaz | yt-dlp otomatik güncellenir; "Araçları güncelle" düğmesi |
| Eklenti Chrome Web Store'a konamaz | Mağazadan kurulamaz | Geliştirici modunda paketlenmemiş kurulum (kişisel kullanım için sorun yok) |
| YouTube kullanım şartları indirmeyi yasaklıyor | Kişisel kullanımda düşük; **herkese dağıtımda yüksek risk** | Çevrim içi sürümde video indirici ayrı/isteğe bağlı tutulur |
| Ses cihazı değiştirme belgelenmemiş Windows arayüzüne dayanıyor | Bir Windows güncellemesi bozabilir (düşük olasılık) | SoundSwitch gibi projeleri takip; yedek olarak Windows ses ayarlarını açan düğme |
| Discord oyun listesi / Epic / EA verileri belgelenmemiş | Bazı oyunlar algılanmayabilir | Elle ekleme + "çalışan programlardan ekle" düğmesi |
| Eski tip "özel tam ekran" oyunlar | Oyun İçi Panel orada açılamayabilir | Kısayollar her durumda çalışır; çerçevesiz pencere önerisi; ileride Xbox Game Bar parçacığı |
| Oyun bilgi kaynaklarının (IGDB/RAWG/Steam) kullanım sınırları | Çok hızlı toplu eklemede yavaşlama | Bilgiler bir kez çekilip yerelde saklanır; istekler sıraya alınır |
| Klipler diski doldurur | Disk alanı | İsteğe bağlı otomatik temizlik (geri dönüşüm kutusuna); klipler zaten kayıt programının seçtiği diskte durur |
| Anti-cheat sistemleri | Ban riski | Oyunlara enjeksiyon, bellek okuma, oyun içine çizim **yok** (§4 güvenlik kuralı) |
| Veri kaybı | Notlar, puanlar kaybolur | Günlük otomatik yedek (son 7 gün) + tek klasör + notların .md kopyası |
| Çerezlerin (Instagram/X) uygulamaya geçmesi | Hassas veri | Sadece ilgili site, sadece indirme sırasında, iş bitince silinir |

---

## 11. Kesinleşen kararlar ve açık sorular

### ✅ Kesinleşenler
- **İsim:** GMID Tool — *Game Mansion In Discord*'un alet çantası; imza: **Merkut Studios**.
- **Görsel kimlik:** Oyun Malikânesi teması — gece zemin, Malikâne Altını + Merkut Turkuazı, ince çizgi ikonlar; GMID logosu uygulama ikonu (§1.5).
- **Açılış animasyonu:** Karanlıktan Merkut Studios logosu belirir, kararır, uygulama açılır; atlanabilir (§3.13).
- **Menü:** Kategoriler malikâne odaları (Giriş Salonu, Oyun Salonu, Eğlence Salonu, Çalışma Odası, Kontrol Odası), sekmeler düz isimli (§2).
- **Kapsam:** Windows 10/11, yerel; başta sadece sen. Veri modeli baştan çok kişili; sunucu (VDS) tarihi belirsiz (§1.4, §8).
- **GMID Tool botlardan bağımsız**; Butler vb. ile zorunlu bir bağlantı yok. Steam Card Idler şimdilik yok.
- İlkeler: optimizasyon, güzel görünüm, anlaşılabilirlik, derli toplu — **denge kuralı** (§1.3).
- Teknoloji: Tauri 2 + Rust + Svelte + SQLite (kodlama tamamen benden).
- **Kendiliğinden sığan arayüz:** Pencere boyutu değişince resimler, videolar, kartlar ve menü otomatik uyum sağlar; hiçbir içerik taşmaz veya gizli kalmaz (§5.5).
- **Veri klasörü:** `Belgeler\GMID Tool`; ama her şey uygulamanın içinde görsel olarak gösterilir, klasöre girmek gerekmez.
- **Klipler taşınmaz:** yerinde kalır, uygulamada oyuna göre gruplanıp gösterilir; çift tıkla açılır (§3.12).
- **Puanlama: 5 kısa soru** → otomatik genel puan + tek cümle + notlar + durum + tavsiye (§3.3, §3.4). ✅ Onaylandı.
- **Oyun bilgileri (tür, etiket, geliştirici, yayıncı, yıl, kapak) internetten otomatik**; tür/etiket ile filtrelenebilir Keşfet ekranı.
- Film'in yanında **dizi** de var; bilgiler TMDB'den otomatik.
- Onaylanan ekler: **Oyun Modu, mikrofon kısayolları + seviye kilidi, Oyun İçi Panel, otomatik kapak, Kliplerim**.
- **Oyun İçi Panel:** Çerçevesiz oyunlar ana hedef; tam ekranda kısayollar + sesli geri bildirim.
- Video indirici: tarayıcı eklentisi + masaüstü ikilisi (Chrome öncelikli, Opera aynı eklentiyle); videolar uygulamada "Videolarım"da.
- Kodlamaya bu doküman onaylandıktan sonra geçilecek.

### ❓ Açık sorular
**Küçük ayarlar** (cevap vermezsen parantez içindeki varsayılanla ilerlerim)
1. Arayüz sadece Türkçe mi? *(Varsayılan: Türkçe; altyapı ileride İngilizce eklenebilecek şekilde.)*
2. Mikrofon kilidi hedef seviyesi? *(Varsayılan: %100.)*
3. Steam Web API anahtarı alalım mı? AFK süresi oyun süresine sayılsın mı? *(Varsayılan: anahtar isteğe bağlı; AFK 15 dk sonra sayılmaz.)*
4. Video indirmede varsayılan kalite? *(Varsayılan: her seferinde sor + "hep en iyisi" seçeneği; videonun üstünde küçük İndir düğmesi.)*
5. Notlar ayrıca .md dosyası olarak da dursun mu? *(Varsayılan: evet.)*
6. Format sonrası kurulum listene kesin girecek programlar? *(Varsayılan: Chrome, Steam, Discord, Claude, WinRAR, Epic Games, NVIDIA App, VLC, 7-Zip.)*

**Öneriler**
7. §4'teki öneriler (#2–#11) arasından istediklerin hangileri? *(Varsayılan: ⭐ olanlar — ücretsiz oyun bildirimi, GMID Wrapped, paylaşım dosyası.)*
8. Aklına gelen başka özellik var mı? 🙂

---

### Değişiklik geçmişi
| Sürüm | Tarih | Değişiklik |
|-------|-------|------------|
| v0.1 | 2026-10-10 | İlk taslak: istekler + araştırma sonuçları + öneriler |
| v0.51 | 2026-10-13 | **Program Kaldırıcı** (yeni; yan menüde Programlar → Format Kurulumu · Kaldırıcı). Kurulu programlar kaldırma kayıtlarından (HKLM, HKLM WOW6432Node, HKCU; sistem bileşenleri ve güncellemeler hariç; Windows kayıt API'siyle). **Kaldır**: programın kendi kaldırıcısı açılır (MSI'de `/I` → `/X`; ShellExecute, yönetici iznini kaldırıcı kendisi ister), kapanınca ya da kaldırma kaydı silinince **artık taraması**; program hâlâ kurulu görünüyorsa tarama yapılmaz (kurulu programın dosyasına dokunulmaz). **Katı kurallar** (`uninstall/rules.rs`, sınamalı): ad birebir tutmalı (sürüm/bit/dil ekleri atılmış ad, yayıncısız ad, kurulum klasörü adı; harf/noktalama farkı yok sayılır, benzerlik/içerme yetmez); yalnız Program Files ×2, ProgramData, AppData Roaming/Local, Local\Programs, Başlat menüsü (kullanıcı/ortak) birinci kat ve `Yayıncı\Program`; masaüstünde yalnız `.lnk`; Windows/Microsoft/Common Files/WindowsApps/Packages/System32… ve kök klasörlerin kendisi asla; hâlâ kurulu bir programın klasörü/exe'si adayın içindeyse ya da aday onun içindeyse, SharedDLLs'te geçen bir dosya varsa, çalışan bir program oradan açılmışsa, aday ya da içinde junction/symlink varsa → "dokunulmadı" (nedeniyle gösterilir); genel adlar (update, common, google, steam… ) ve 4 harften kısa adlar eşleşmez; exe adıyla eşleşen "zayıf" adaylar seçilmeden gelir; yayıncı klasörü/anahtarı yalnız içinde başka hiçbir şey kalmayacaksa. Kayıt defterinde yalnız `Software\<Ad>`, `Software\<Yayıncı>\<Ad>` (HKCU, HKLM, WOW6432Node), programın artık kalan kaldırma kaydı ve verisi programın klasörünü gösteren Run değerleri; Microsoft/Classes/Policies/servisler/COM hiç taranmaz. **Silme geri alınabilir:** dosyalar Geri Dönüşüm Kutusu'na (VisualBasic FileSystem, SendToRecycleBin), kayıtlar önce `reg export` (yedeklenemezse silinmez) sonra `reg delete`; yedekler `Yedekler\Kaldırıcı <tarih> <program>`; yalnız son taramada bulunan ve seçilen öğeler; HKLM ya da kullanıcı klasörü dışı öğe varsa tek UAC izni. Karaoke: kayıt penceresindeki önizleme aynı dosya adında önbellekten eski karışımı çalıyordu (ayarlar ve yankı işe yaramıyor görünüyordu) → `?v=` ile her yenilemede taze; yankı varsayılan kapalı ve çok daha hafif; sese hafif kapı (oda sesi/nefes); müzik −21 LUFS |
| v0.52 | 2026-10-13 | **Kaldırıcı simgeleri:** her programın simgesi (`uninstall/icon.rs`): sırayla `DisplayIcon` (ortam değişkenleri açılır, `,dizin` / `,-kaynak` desteklenir), MSI'de `Installer\Products\<sıkıştırılmış kod>\ProductIcon`, kurulum klasöründe adı programa en çok benzeyen exe (kaldırıcı/güncelleyici değil), son çare kaldırıcının exe'si; `SHDefExtractIconW` 64 px → RGBA (alfasız eski simgelerde maske) → PNG data URL, bellekte önbellek; liste önce gelir, simgeler arkadan dolar; simgesi olmayan programda baş harf. **Dışarı sürükleme:** Medya'da video kartları ve şarkılar, Kliplerim'de klipler basılı tutulup 8 px kaydırılınca Windows'un kendi sürüklemesi başlar (`dragout.rs`: kabuk öğe dizisinden IDataObject + `SHDoDragDrop`, yalnız kopyalama; Masaüstü, klasör, Discord, tarayıcı kabul eder); kısa tıklama eskisi gibi oynatır, sürüklemeden sonraki tıklama yutulur; görünüm değişmedi |
| v0.53 | 2026-10-13 | **Kaldırıcı: süzgeç ve ayrıntılar** (`uninstall/info.rs`, sınamalı). Her program için **tür**: Oyun (Steam/Epic/Riot kaldırma bağlantısı, `Steam App` kaydı ya da steamapps/Epic Games/Riot Games/… klasörü), Çalışma ortamı (Visual C++, .NET, DirectX, WebView2, Java, PhysX, Vulkan…), Sürücü & donanım (driver/chipset/BIOS/… adı ya da NVIDIA/AMD/Intel/Realtek/ASUS/MSI/Logitech/Razer/… yayıncısı), Microsoft, Uygulama; türün **uyarısı** (çalışma ortamı ve sürücü için kaldırmadan önce sarı uyarı, onay penceresinde de). **Gerçek klasör** (InstallLocation, yoksa ana exe ya da kaldırıcının klasörü; Windows\\Installer ve Package Cache sayılmaz), disk harfi, **yükleyici** (MSI, Steam, Epic, Riot, Inno Setup, Squirrel, kurulum paketi, kendi kaldırıcısı), **kapsam** (tüm kullanıcılar / yalnız bu kullanıcı), **mimari** (64/32-bit). Arayüz: tür düğmeleri (sayılarıyla), disk seçimi, aramada klasör yolu da; satıra tıklayınca ayrıntılar (konum + Klasörü aç, tür + uyarı, kurulum, boyut/tarih, kaldırma komutu, kayıt anahtarı); listede gizlenen sistem bileşeni/güncelleme sayısı ve nedeni. **Güvenlik:** gizlenen kayıtların (SystemComponent, güncelleme, alt bileşen) klasörleri de artık taramasında "başka programa ait" sayılır |
| v0.54 | 2026-10-13 | **Windows Defender yanlış alarmı (Trojan:Win32/Bearfoos.A!ml / B!ml)** için şeffaflık: (1) **Güncelleyici** artık kurulumu `%TEMP%`'e indirip görünmez `cmd` + `ping` beklemesiyle sessiz (`/S`) çalıştırmıyor; `%LOCALAPPDATA%\com.merkutstudios.gmidtool\Güncellemeler`'e indirir, SHA-256 doğrular, kurulumu ShellExecute ile **görünür ilerleme penceresiyle** `/P /R /UPDATE` açar (Tauri NSIS: soru yok, kısayollar yeniden oluşturulmaz, bitince GMID'i kendisi açar), eski kurulum dosyaları açılışta silinir. (2) **PowerShell**: hiçbir yerde `-EncodedCommand`, `-ExecutionPolicy Bypass`, `-WindowStyle Hidden` yok; betikler `…\gmidtool\Betikler\<ad>.ps1` olarak okunabilir dosya (UTF-8 BOM, CRLF) yazılıp `-ExecutionPolicy RemoteSigned -File` ile çalışır (sistem bilgisi, FPS izni, kaldırıcı temizliği; yönetici izni gerekenlerde UAC penceresi dosya yolunu gösterir); Format kiti `.bat`'ı da RemoteSigned. Kalıcı çözüm kod imzalama (README) |
| v0.55 | 2026-10-13 | **Medya'daki videolar için kırpma / Discord'a hazırla:** video kartında makas yerine klip simgeli "Kırp / Discord'a hazırla" düğmesi Kliplerim'in penceresini (ClipTrim) açar. Arka uçta kimlik `dl:<indirme>` ise klip yerine indirme kaydı kullanılır (`clips::lookup`: yol, başlık, süre, kaliteden yükseklik); seçenekler ffmpeg ile okunan gerçek boyuta göre (dikey videolar doğru). "Sınırsız" (yalnız kırp) kopya videonun yanına yazılır ve **Medya'ya yeni video** olarak eklenir (`downloader::add_copy`: başlık + "(kırpılmış)", aynı kanal/site/kapak, listeye girmez); boyutlu seçenekler eskisi gibi "Discord'a Hazır" klasörüne ve panoya |
| v0.56 | 2026-10-13 | **Oyun paneli fare:** bakış kipinde (fare oyunda kilitli, "Fareyi serbest bırak" düğmesine tıklanamıyordu) panel açıkken farenin **orta tuşu** (ayardan sol tık ya da kapalı) izlenir (kanca yok, `GetAsyncKeyState` 25 ms; panel kapanınca/odak alınca biter), basılınca panel odağı alır (`panel-grabbed`). Odak alırken `AttachThreadInput` ile gerçekten öne geçilir ve oyunun imleç kilidi (`ClipCursor`) kaldırılır. Oyun Modu → Oyun İçi Panel: "Panel açılınca fare" (Kendiliğinden / Hemen panele geçsin / Oyunda kalsın) ve geçiş tuşu. **Eklenti 1.5.1:** kalite menüsü dışarı tıklayınca ve Esc ile kapanır. **İndirme gecikmesi:** menüde gösterilen çözümleme önbelleğe sayfanın adresiyle konuyordu; "İndir" sitenin asıl adresini (`webpage_url`) gönderdiği için video ikinci kez yt-dlp ile çözümleniyordu. Artık iki adresle de önbelleğe alınır (sınamalı) |
| v0.57 | 2026-10-13 | **Futbol** (yeni; Eğlence Salonu → Futbol, `football.rs`, sınamalı). Kaynak ESPN'in sitesinin kullandığı açık, anahtarsız JSON servisi: maçlar `site/v2/sports/soccer/<lig>/scoreboard?dates=YYYYMMDD` (gün gün; aralık her sporda çalışmıyor), puan durumu `apis/v2/…/standings` (site/v2 boş döner). Ligler: Süper Lig, Şampiyonlar/Avrupa/Konferans Ligi, Premier Lig, La Liga, Serie A, Bundesliga, Ligue 1, Eredivisie, Liga Portugal (yalnız bu kodlar istenir). **Maçlar:** lig düğmeleri (Tüm ligler), gün şeridi (−3…+4; yerel gün için bir önceki gün de istenip yerel tarihe göre süzülür), canlı süzgeci; satırda saat ya da canlı dakika (nabızlı kırmızı nokta) / İY / MS / Ertelendi, logolar, skor, altında gol atanlar (P: penaltı, K.K.: kendi kalesine) ve kırmızı kartlar dakikalarıyla; canlı maç varken 30 sn'de bir, bugünse 5 dk'da bir kendiliğinden yenilenir. **Puan durumu:** # O G B M A:Y AV P, sıranın anlamına göre renkli kenar (Şampiyonlar Ligi, Avrupa, küme düşme…) ve açıklaması; gruplu ligler. Önbellek: bugün/canlı 30 sn, diğer günler ve puan durumu 10 dk; istekler aynı anda, bir kısmı başarısız olursa gelenler gösterilir. ESPN'e bu ortamdan erişilemediği için yanıt biçimi belgelere göre okunur, eksik alanlar hata sayılmaz |
| v0.58 | 2026-10-13 | Oyun paneli: kullanıcı isteğiyle "Oyun önde, fare oyunda · Fareyi serbest bırak" kutusu ve "Oyunda kalsın" seçeneği kaldırıldı (düğme sorun çıkarıyordu); kayıtlı `panel.mode = glance` kendiliğinden gibi davranır. Orta tuşla panele geçiş ve "Hemen panele geçsin" kaldı |
| v0.59 | 2026-10-13 | **Futbol 2:** solda gruplu lig menüsü (Türkiye · Avrupa kupaları · Avrupa ligleri; dar ekranda üstte), lig başlığı ve sekmeler **Maçlar · Puan durumu · Krallık**. **Takım sayfası** (maç satırındaki, puan tablosundaki ve krallıktaki takıma tıklayınca): logo, sıra ("Ligde N. sırada"), stat; **Kadro** mevkiye göre (Kaleciler, Defans, Orta saha, Forvet; forma no, yaş, uyruk Türkçe), **Maçlar**: sıradaki maçlar (başka kupadaysa etiketi) ve son maçlar G/B/M rozetleriyle, son 5 maç formu. Uçlar: `teams/<id>`, `/roster` (düz ya da mevkiye gruplu liste), `/schedule` ve `/schedule?fixture=true` (skor nesne olarak gelir), aynı anda; gelmeyen bölüm boş kalır. **Krallık:** gol ve asist ilk 10 (`sports.core.api.espn.com/…/leagues/<lig>/leaders`; oyuncu ve takım `$ref` bağlantıları yalnız bu sunucuya, https ile, aynı anda; 1 saat önbellek). **Gol dakikası** gelmiyordu: `clock.displayClock` yoksa `clock.value` (saniye) dakikaya çevrilir. Sınamalı (8 sınama) |
| v0.60 | 2026-10-14 | **Futbol 3:** maçlar **hafta hafta** (Pzt–Paz; ‹ › ile önceki/sonraki hafta, "Bu hafta"), günlere göre başlıklı (bugün altın), her günde lig lig; maçlı günler için süzgeç düğmeleri (gün · maç sayısı) ve "Tüm hafta". Arka uç `football_week`: lig başına tek aralık isteği (`dates=YYYYMMDD-YYYYMMDD`), site aralığı kabul etmezse gün gün (en çok 10 gün); yerel hafta için bir gün öncesi de istenir. **Krallık 400 hatası:** krallık servisi sırayla `leaders?lang=en&region=us`, `seasons/<yıl>/types/1/leaders`, `seasons/<yıl>/leaders` ile denenir (sezon yılı Temmuz'a göre); hiçbiri olmazsa gol krallığı sezonun maçlarındaki gollerden sayılır (kendi kalesine sayılmaz; bu yolda asist yok). Sınamalı |
| v0.61 | 2026-10-14 | Futbol maç satırı ortalandı: soldaki saat/durum sütunu kaldırıldı; ev sahibi · skor · deplasman simetrik, durum (MS, İY, canlı dakika nabızlı) skorun altında, başlamamış maçta skor kutusunda saat; olaylar aynı ızgarada |
| v0.62 | 2026-10-14 | Futbol favori takım: takım sayfasında ☆ Favorilere ekle, kenar menüde Favorilerim; arka planda favorilerin bugünkü maçları izlenir (oynanırken 45 sn, değilse 10 dk): maç başladı / GOL / maç bitti bildirimi (oyun öndeyse OSD, değilse Windows bildirimi; ilk görüşte bildirim yok), `football.notify` ile kapatılır. Oyun İçi Panel üstünde "Maç" şeridi (canlı skor + dakika), `panel.football` ile Futbol ekranından ya da paneldeki × ile gizlenir |
| v0.63 | 2026-10-14 | Geri bildirim turu: yedek ayarı (`backup.keep` 1/3/5/7, varsayılan 3; `backup.every` her gün/haftada bir/kapalı; yedek yalnız gmid.db, medya dosyaları girmez); sunucuya şifre sütunu (göç 17) ve `connect ip:port; password x` / `steam://connect/…` yapıştırma, Bağlan → Oyna; aynı anda indirme 2×2; klip penceresi düğmeleri (çöp kutusu, Oynatıcıda aç, Klasörü aç); dublaj/altyazı metinleri (sitenin ses parçaları, Whisper değil); sesle yazma düzeltmeleri kendiliğinden öğrenir (yalnız yanlış duyulmuşa benzeyenler: yakın yazılış, kısaltma/ad, sayı) + 10 sn Geri al kartı, `dictation.autoLearn`; Görünüm baştan (arka plan kartları, canlı önizleme, Arkaplanlar kitaplığı: tek tıkla dön/sil, kalite kartları); Fırsatlar kırmızı rozet (görülmemiş ücretsiz/istek listesi indirimi, `deals.viewed`); oynatıcı sol menüyü örtmez; Film & Dizi tür sekmeleri; çark sekme değişince sıfırlanır; turnuva ağacı iki taraftan ortaya, şampiyon ortada, Turnuvayı temizle; kronometre devam edince 00:00 kayması düzeltildi; yeni kliplerde kapak 20 sn sonra, başarısızsa 2 dk sonra yeniden + kareden yedek |
| v0.64 | 2026-10-14 | Film & Dizi: ayrıntı penceresinde iki tıkla "Arşivden kaldır"; İngilizce tür adları Türkçeleşir ve aynı anlamlılar birleşir (Thriller = Gerilim, Crime = Suç, Black comedy = Kara komedi). Sesle yazma: düzeltme artık gönderilmeden de görülür (yazı değişip 6 sn dokunulmazsa); yazı alanı okunamazsa ve öğrenilmeyen düzeltmeler gmid.log'a yazılır |
| v0.65 | 2026-10-14 | Favorinin Avrupa kupası maçları da izlenir (lig + 3 UEFA kupası; kupa sayfasından eklendiyse hepsi). Notlara resim: Ctrl+V ile ekran görüntüsü/kopyalanan resim, araç çubuğunda Resim ekle; dosyalar `Notlar\\Resimler`e kopyalanır (bayt imzasıyla PNG/JPG/GIF/WebP, en fazla 25 MB), notta `![](Resimler/ad.png)`; hiçbir notta geçmeyen 1 günden eski resimler açılışta silinir |
| v0.66 | 2026-10-14 | Sesle yazma göstergesi gecikmesi: Ctrl+Alt basılır basılmaz gösterge görünmez ("armed") olarak açılır, 220 ms dolunca yalnız görünür olur (gizli pencerenin ve sayfasının uyanması beklenmez); Whisper modelinin yüklenmesi 350 ms sonra başlar, göstergenin ilk çizimiyle çakışmaz |
| v0.67 | 2026-10-14 | Oyun İçi Panel notu zengin düzenleyici (resim yapıştırma). Tek tuşla ekran görüntüsü (`screen.shot`, varsayılan Print Screen; Windows tutuyorsa Ctrl+Print Screen): ön plandaki pencerenin ekranı DXGI masaüstü çoğaltmasıyla (olmazsa GDI) yakalanır, hiçbir pencere açılmaz ve odak değişmez; panoya PNG + DIB, `Videolar\\GMID\\<oyun>\\` altına PNG (Kliplerim'de "GMID" kaynağı), sağ üstte kısa gösterge |
| v0.68 | 2026-10-15 | Ekran görüntüsü kısayolu varsayılanı Ctrl+Alt+P (Print Screen Windows'la çakışıyordu; Kısayollar'dan değiştirilir). Mikrofon açılınca yükselen, kapanınca alçalan kısa ve kısık iki nota (kısayol, tepsi, uygulama); `audio.micSound` ile kapatılır |
| v0.69 | 2026-10-15 | Bağlantı Testi'nde DNS seçici: Cloudflare, Google, Quad9, AdGuard, Cloudflare Aile; yanıt süreleri UDP sorgusuyla aynı anda ölçülür (en hızlısı işaretlenir); seçilen DNS internete çıkan bağdaştırıcılara yönetici izniyle uygulanır (Set-DnsClientServerAddress + önbellek temizliği), Otomatiğe dön; oyun açıkken değiştirilmez |
| v0.70 | 2026-10-15 | Sesle yazma sözlüğü bozulmasın: yalnız tek sözcük düzeltmeleri öğrenilir (cümle sonunda boşluğu kaybolmuş "bedava.QR" gibi birleşik sözcükler ayrılır, boşluklu ya da yalnız eki değişen "telefon→telefonlar" düzeltmeleri öğrenilmez); eski bozuk kurallar bir kez temizlenir; kendiliğinden öğrenince sağ üstte "Sözlüğe eklendi" bildirimi (aç/kapa: Öğrenince haber ver), tek tık Geri al 20 sn; sözlükte Hepsini sil |
| v0.71 | 2026-10-16 | Ekran görüntüsü siyah çıkmasın: DXGI çoğaltmasının ilk (yalnız imleç) kareleri atlanır, ekranın çizildiği kare beklenir; kare yine siyahsa klasik kopyaya (GDI) geçilir. Futbol takım sayfası: lig + Avrupa kupaları + ülke kupası maçları birlikte (kupa adı satırda), Son maçlar en üstte (en yeni başta, 20 maç), Sıradaki maçlar altta; kupa sayfasından açılınca takımın asıl ligi bulunur. Görünüm: "Videolarımdan" ile indirilen MP4/WebM video arka plan yapılır (kopyalama arayüzü dondurmaz) |
| v0.72 | 2026-10-18 | Ses Paneli (Soundpad gibi), Ses & Mikrofon'un ikinci sekmesi: numpad (ya da F / Ctrl+Alt'lı) tuşlara ses ya da şarkı atanır, tuşa basınca mikrofondan çalar. Windows başka programın mikrofonuna ses koydurmadığından ücretsiz VB-CABLE sanal kablosu kullanılır (algılanır, yoksa kurulum adımları ve indirme sayfası): GMID gerçek mikrofonu + çalan sesleri WASAPI ile 48 kHz karıştırıp "CABLE Input"a yazar, Discord'da mikrofon "CABLE Output" seçilir; sesler kulaklıktan da duyulur (aç/kapa, seviye). Varsayılan mikrofon kabloya kaymışsa gerçek mikrofon kendiliğinden seçilir (geri besleme yok). Ses dosyaları yerinde kalır, FFmpeg ile arka planda çözülür (çözme sürerken çalmaya başlar, 300 MB önbellek); tuşlar yalnız panel açıkken kayıtlı, Num 0 hepsini durdurur, oyunda sağ üstte çalan sesin adı. Medyamdan ekleme, ses başına seviye, üst üste çalma, Discord'da kesilme için ipuçları |
| v0.50 | 2026-10-13 | Geri bildirim: internetten bulunan müzikle karaoke iyi, ama **kayıtta ses çok kısık, müzik çok yüksek** (mikrofon kaydı ham seviyede, müzik masterlanmış tam seviyede karıştırılıyordu). Artık karıştırmadan önce iki kaydın ses yüksekliği ölçülür (ffmpeg `ebur128`, EBU R128, LUFS; müzikte yalnız şarkının kayıt aralığı): ses −16, müzik −20 LUFS'a getirilir (ses en çok +40 dB yükseltilir, müzik en çok −20 dB kısılır), sese 80 Hz alt kesme ve hafif sıkıştırma (3:1, −20 dB), sonunda sınırlayıcı; sonuç penceresinde Sesin / Müzik kaydırıcıları bu dengeli noktanın üzerine çarpan. Sınama: −52 LUFS'luk (çok kısık) ses, karışımda müziğin +1,5 dB üstünde. Kayıt boyunca mikrofondan neredeyse hiç ses gelmediyse uyarı (doğru mikrofon seçili mi). **Hafif bozulma:** kaydederken taşan örnekler tek tek kırpılıyordu (YouTube sesleri 0 dBFS'e yakın; 48→44,1 kHz dönüşümünde taşabiliyor) → artık taşma varsa bütün parça birlikte kısılır |
| v0.49 | 2026-10-13 | Geri bildirim: karaoke çalışıyor, ama ses girince yapay zekâyla ayrılan müzik boğuklaşıyor (maske ayırmanın doğası; erişilebilir daha iyi model yok). Kullanıcının önerisi (müziği notalardan yeniden üretmek, ör. Spotify Basic Pitch → MIDI) davul, tını ve miksi kaybeder, "MIDI kapağı" gibi yapmacık çıkar; onun yerine **İnternetten temiz müzik bul**: şarkının resmî enstrümantal / karaoke sürümü YouTube'da aranır (yt-dlp `ytsearch8`, sorgu = başlıktan "(Official Video)" gibi ekler atılmış ad + instrumental / karaoke / beat), süre farkı gösterilir ("süre tutuyor" ≤4 sn), seçilen indirilir ve şarkıya **hizalanır**: iki kaydın ilk 2 dakikası 11 kHz tek kanala indirgenip FFT ile çapraz ilinti, ±20 sn içinde en iyi kayma (sınama: 1,37 sn geç ve 1 sn erken başlayan sürümler örnek düzeyinde bulundu), kaydırılıp şarkı uzunluğuna getirilir, `muzik-internet-<zaman>.m4a` (her indirmede yeni ad: arayüz önbellekten eskisini çalmasın) + `muzik-internet.json` (başlık, adres, kayma, benzerlik). Karaoke önce bunu, yoksa ayrılanı çalar; kayıt karışımı da aynı müzikle. Kurulum ekranı iki seçenekli; hazır ekranda müzik kaynağı satırı (Değiştir, Yapay zekâyla ayrılana dön, İşlemciyle yeniden ayır); vokal dosyası yoksa "Orijinal ses" kaydırıcısı gizlenir. Not: Spleeter (Deezer, 2019) Demucs v4'ten (kullandığımız HTDemucs) belirgin şekilde zayıf |
| v0.48 | 2026-10-13 | Geri bildirim: **karaoke müziği yerine cızırtı/patlama.** İşlemci yolu doğru (yeni sınama: flite ile konuşan ses + davul/bas/akor müziği karıştırılıp ayrıldı → vokal 21,1 dB, müzik 21,3 dB); bozuk sonuç ekran kartı (DirectML) yolundan. Artık: ekran kartı varsa şarkının ortasından tek parçalık kesit hem ekran kartıyla hem işlemciyle ayrılır, ikisi ≥20 dB tutmazsa (ya da ekran kartı hata verirse) bütün şarkı işlemciyle ayrılır (günlüğe uyum yazılır); sonuç kaydedilmeden önce denetlenir (sayı olmayan örnek ya da karışımdan 1,5 kat güçlü "vokal" → kaydetmez, uyarır); kaydederken örnekler [-1, 1]'e kırpılır (patlama olmaz). Sonuç sürümü (`surum.txt` = 2): önceki bozuk karaoke dosyaları kendiliğinden yeniden hazırlanır. Karaoke ekranında "Müzik bozuk mu? İşlemciyle yeniden hazırla"; vokal–müzik eşitleme en çok saniyede bir (sık atlatma cızırtı yapar) |
| v0.47 | 2026-10-13 | Geri bildirim: **karaoke müziği hiç çalmıyordu** — arayüz yalnız izin verilen klasörlerdeki dosyaları açabiliyor (`asset_protocol_scope`), `Karaoke` klasörü listede yoktu → eklendi; dosya açılamazsa artık ekranda yazar. **Kura** sekmesinde "Kaç kazanan" kutusu düğmenin altına giriyordu (listedeki sıra numarası için eklenen `.num` sınıfı aynı adlı etiketi 18 px'e daraltıyordu; `.ino` oldu). **Kronometre** çalışırken de iki haneli salise (00:21,63); son iki hane kaymaz, eşit genişlikli rakamlar (genişlik sabit, ölçüldü) |
| v0.46 | 2026-10-13 | Geri bildirim turu. **Karaoke** (oynatıcıda düğme): vokal ayırma Rust'ta — HTDemucs (ONNX, npm `demucs@1.0.0` paketindeki ağırlıklar; Meta'nın ağırlıkları kişisel kullanım lisanslı, uygulamayla dağıtılmaz, kullanıcının bilgisayarına iner) + ONNX Runtime 1.24.4 DirectML (PyPI `onnxruntime-directml` tekerleğinden `onnxruntime.dll`, `DirectML.dll`; `ort` 2.0.0-rc.10 `load-dynamic`), ikisi de SHA-256 ile doğrulanır; DirectML olmazsa işlemci. `_spec`/`_ispec` PyTorch ile aynı (yansıtmalı dolgu torch'taki gibi; 4096/1024 Hann, normalized), 7,8 sn'lik %25 üst üste parçalar, üçgen ağırlık; müzik = karışım − vokal; ffmpeg ile çözme (44,1 kHz stereo f32) ve AAC kayıt (`Karaoke\<id>\muzik.m4a`, `vokal.m4a`). Sınamalar: spec→ispec birebir (kenar çerçeveleri hariç), gerçek modelle dört kaynağın toplamı karışımı 28 dB tutuyor, uçtan uca çöz→ayır→kaydet→sesle karıştır, npm paketinden model çıkarma. Ekran: sözler ortada kelime kelime dolar (önceki, şimdiki büyük, sonraki iki satır; girişte 3-2-1), Orijinal ses ve Müzik kaydırıcıları, iki dosya eşzamanlı (80 ms'den fazla kayarsa vokal yetişir); **Kaydet**: `getUserMedia` (kulaklıkta yankı engelleme kapalı) + MediaRecorder (opus), seviye göstergesi; bitince ffmpeg ile karışım (`atrim` ile gecikme, ses seviyesi, isteğe bağlı `aecho`, `alimiter`) → `Karaoke\Kayıtlar\<şarkı> - <tarih>.m4a`; sonuç penceresinde dinle, ses/gecikme/yankı ile yeniden karıştır, klasörde göster, sil. **Sözler kelime kelime:** kelime zamanı olmayan satırlarda (LRCLIB) satır süresi kelime uzunluğuna göre paylaştırılır (saniyede ~9 harf üst sınırı, araya müzik girince dolum erken biter); her karede `--p` ile soldan sağa yumuşak kenarlı altın dolum. **Whisper %0'da bekliyordu:** dil tespiti NVIDIA sürümü kuruluyken bile işlemcideki Whisper'la yapılıyordu (büyük modelle dakikalar) → kurulu en hızlı sürümle; ekranda "Şarkının dili bulunuyor" / "Whisper hazırlanıyor (model yükleniyor)" ve yüzde gelene kadar kayan şerit. **Oyun İçi Panel → Şimdi çalan:** ilerleme çubuğu (konuma git), karıştır ve tekrar (kapalı → liste → parça; program desteklemiyorsa soluk), çalan programın ses seviyesi (karıştırıcıdaki süreçle eşleşir); GMID'in kendi oynatıcısı Windows'a karıştır/tekrar bildirmediği için oynatıcı ile panel pencereler arası olayla konuşur (`gmid-player` durum, `gmid-player-cmd` komut). Karıştırıcıda GMID'in WebView2 süreci "Msedgewebview2" yerine "GMID Tool". **Medya:** kartın/satırın herhangi bir yerine tıklamak oynatır (içindeki düğmeler kendi işini yapar). **Yan menü sadeleşti (20 → 12 giriş):** Oyunlarım (Kütüphane · Oturumlar · Puanlarım · Wrapped), Sunucular (Sunucular · Bağlantı Testi), Kliplerim, Fırsatlar, Oyun Modu; Medya (Oynatıcı · İndirici), Film & Dizi, Çark & Çekiliş; Ajanda (Notlar · Hatırlatıcılar · Sayaç), Ses & Mikrofon, Format Kurulumu; sekmeler başlığın yanında, girişe tekrar tıklayınca son açık sekme; adresler aynı. **Çark & Çekiliş listesi sade:** tek giriş kutusu (çok satır/virgüllü yapıştırma hepsini ekler; ayrı "Toplu ekle" kalktı), başlıkta karıştır/kaydet/temizle simgeleri, Hızlı doldur üç kutucuk, kayıtlı listeler "Listelerim", satırlarda sıra no ve × yalnız üzerine gelince. **Hatırlatıcı (panel):** düğmeler aynı yükseklikte, sustur kare. **Discord'da durum** ayarı gizlendi: Discord uygulaması bot ile birlikte VDS'e (yol haritası) |
| v0.45 | 2026-10-13 | Öneri listesinden seçilenler + **Medya Oynatıcı yeni tasarım**. **Oyun İçi Panel → Şimdi çalan:** Windows'un medya oturumu (`GlobalSystemMediaTransportControlsSessionManager`; Spotify, tarayıcı, VLC, Apple Music…): program adı, şarkı, sanatçı, kapak (≤512 KB, data URL), önceki / oynat-duraklat / sonraki; panel açıkken 2 sn'de bir. **Discord'da durum** (Rich Presence, VDS gerekmez): Discord'un yerel bağlantısı (`\\.\pipe\discord-ipc-N`; çerçeve: işlem kodu + uzunluk + JSON; el sıkışma → READY → `SET_ACTIVITY`); oyundayken "X oynuyor", "Game Mansion In Discord", geçen süre, büyük resim `gmid`, **GMID'e katıl** düğmesi (topluluk listesindeki davet); 15 sn'de bir, değişmediyse gönderilmez; Discord kapalıysa bağlanmaya çalışmaz. Discord, durumu bir uygulama kimliği adına gösterdiği için Developer Portal'da bir kez "GMID Tool" uygulaması açılıp kimliği Kontrol Odası → Oyunlar → Discord'da durum'a yapıştırılır (açık/kapalı anahtarı orada). **Medya Oynatıcı:** Müzikte solda liste şeridi (Kitaplık: Tüm şarkılar, Favoriler `media.favs`, Son eklenenler (en yeni 30), En çok dinlenenler (`media.plays`, oynatıcıda her şarkı açılışında +1); Listelerim (+ Yeni liste); İndirilen listeler; boyut ve Klasör), dar pencerede yatay çipler. Üstte **Şimdi çalan** kartı: kapağın bulanık hali arka plan, 152 px kapak (tıkla → oynatıcı), "Şimdi çalıyor" ses dalgası, ad, sanatçı · liste, **o an söylenen söz satırı** (oynatıcıdaki zamanlı sözlerden, satır değişince kayarak gelir), büyük oynat/duraklat (oynatıcı öne gelmeden), Karıştırarak çal, Oynatıcıyı aç; çalan yokken seçili listenin kartı (4 kapaklı mozaik, şarkı sayısı/süre, Hepsini çal, Karıştır). Satırlar sade: "LOL" liste sütunu kalktı; kalp (favori), ekle/klasör/sil yalnız üzerine gelince (menü açıkken ya da silme onayı beklerken görünür kalır); kapağın üstünde oynat; çalan satır altın + ses dalgası (duraklatılınca durgun). Sıralama: Eklenme / liste sırası, Ad, Sanatçı, Süre, En çok dinlenen (`media.sort`); **Liste / Kapak** görünümü (`media.layout`; kapakta kare kartlar, üzerine gelince oynat ve kalp). Şarkıya tıklamak artık oynatıcıyı tam ekran açmaz, altta çalar (videolar açar). **Videolar:** kartlar büyüdü (en az 290 px), süre ve kalite kapağın üstünde, üzerine gelince kalkar ve 56 px oynat; Eklenme / Ad / Süre sıralaması. Yapılmadı (istenmedi): mola hatırlatıcısı, mikrofon testi. FPS sorunu kullanıcının yeniden başlatma denemesinden sonra |
| v0.44 | 2026-10-13 | Geri bildirim: **Sayaç** kronometresi çalışırken saniyenin onda biri (salise her karede değişip titreşiyor, "son rakam parlıyor" görünüyordu); durunca salise. **Oyun İçi Panel**: çalan hatırlatıcı/sayaç çubuğu kırmızı ile GMID sarısı arasında 0,9 sn'de yumuşak gidip gelir (çerçeve, ışıltı, zil). **Discord'a hazırla** penceresi yukarıdan sabit, video alanı sabit yükseklikte (46vh); zaman çizelgesi/seçenekler/hazırlananlar altta kendi içinde kayar. **Sistem özellikleri** veritabanında saklanır (`sysinfo.cache`): her açılışta PowerShell çalışmaz, "Yenile" yeniden okur; açık kalma süresi `GetTickCount64` ile hep güncel; ekranda "bilgiler N gün önce okundu". **Yazıcılar** (Win32_Printer; PDF/XPS/OneNote/Faks gibi sanallar hariç, varsayılan önce) ve **kameralar** (PnP Camera/Image). Sürücü denetimi kurulu sürümü saklanan bilgiden değil kayıt defterinden okur (`…\Class\{4d36e968…}\00NN` DriverDesc/DriverVersion) → sürücü güncellenince yanlış uyarı vermez. **Çark & Çekiliş**: sekmeler koyu ray üzerinde, seçili altın; her sekme ve Kura alt sekmesinde "i" → üzerine gelince açıklama balonu. **Turnuva**: ad (ör. "LoL 1v1 Turnuvası"), kürsü (şampiyon, finalist, yarı finalistler), **şampiyon kartı** (1200×675 PNG: ad, kupa, 1-2-3, oyuncu sayısı, final, tarih) panoya → Discord'da Ctrl+V; yazı olarak da kopyalanır |
| v0.43 | 2026-10-13 | Geri bildirim + öneri listesinden seçilenler. **Klip kapakları** bazı arkadaşlarda yoktu: kapak ffmpeg ile çıkarılıyor, ffmpeg ise yalnız Video İndirici'nin araçlarıyla iniyordu → kapağı eksik klip varsa yalnız ffmpeg arka planda kendiliğinden iner (`Tools::install_only`), Kliplerim'de durum notu; kare sürenin %20–60'ı arasından rastgele (JPEG ≥6 KB değilse, yani neredeyse tek renkse başka yer); başarısızlar 6 saat sonra yeniden denenir (eskiden bir kez denenip bırakılıyordu), son 2 dk'da değişen dosya sonraki tura kalır, silinen kapak dosyası yeniden üretilir; açılamayan kapakta simge. **Bağlantı Testi** (yeni ekran, Oyun Salonu): Windows ICMP (`IcmpSendEcho`, yönetici gerekmez) ile modem (`GetAdaptersAddresses` ağ geçidi; Wi-Fi/kablo türü), Cloudflare, Google, Discord, GMID sunucuları (ping'e kapalıysa oyunun UDP sorgusu) ve Steam SDR yönlendirme noktalarından en yakın üçüne (GetSDRConfig; hepsine 2 ping, en hızlılar saklanır) aynı anda 12 ping; ortalama/en iyi/en kötü/titreme/kayıp, ping çubukları; teşhis kuralları: modemde kayıp ≥%3, titreme >15 ms ya da gecikme >25 ms (Wi-Fi) / >10 ms (kablo) → "ev ağı"; internette kayıp ≥%3, titreme >25 ya da >90 ms → "sağlayıcı"; yalnız bir sunucu kayıplı/yavaşsa → "sunucu tarafı". **Oyun İçi Panel'de Ping kartı**: tek tıkla 6 ping'lik kısa test. **Ekran kartı sürücüsü:** NVIDIA'da kurulu sürüm (32.0.15.6614 → 566.14), ürün listesinden (lookupValueSearch) seri/ürün numarası, sürücü servisinden en yeni WHQL DCH sürümü; açılıştan 3 dk sonra ve 12 saatte bir, her yeni sürüm için bir bildirim (ayar: Bildirimler); Sistem özelliklerinde "Sürücü güncel" ya da "Yeni sürücü · İndir"; AMD/Intel için kurulu sürüm ve resmî sayfa. **Turnuva** (Çark & Çekiliş): eleme ağacı (2'nin kuvveti değilse ilk turda bay; bay-bay maçı olmaz), kazanana tıkla (yeniden tıkla: geri al, sonraki turlar temizlenir), 🎲 tek maçı ya da kalanların hepsini rastgele oynat, şampiyon kutlaması, Discord'a tur tur sonuç. **Dengeli takımlar:** Kura → Takımlara böl → "Seviyeye göre dengele": listede 1–5 yıldız; en yüksekten başlayıp en zayıf takıma ekleme (kişi sayısı en çok bir fark) + güç farkını azaltan takaslar; eşit seviyeler rastgele. Yol haritası: **oyun gecesi planlayıcı** VDS ile birlikte (§ Yapılacaklar) |
| v0.42 | 2026-10-13 | Geri bildirim (Çark & Çekiliş → Zar & yazı-tura): üç araç önce seçim kartı; birine tıklayınca liste paneli gizlenir, araç sahnenin ortasında büyür, sol üstte Geri (Esc), Boşluk/Enter atar. **Yazı-tura** baştan: 3B para (iki yüz + 9 katlı kenar, `preserve-3d`, perspektif), Web Animations ile atış: 1,6 sn'de yükselip düşer (yavaşlayan çıkış, hızlanan iniş, küçük sekme), X ekseninde 5–7 tur döner ve seçilen yüzde durur; yerdeki gölge küçülüp büyür; atışta ve yere değişte metal "ting" sesi; oturumluk Yazı/Tura sayacı. **Zar**: noktalı fildişi yüzler (3×3 ızgara), yuvarlanırken zıplayıp döner, dururken yerine oturur; 1–6 zar seçimi düğmelerle. **Sayı seç**: büyük rakam |
| v0.41 | 2026-10-13 | Geri bildirim: **Çark & Çekiliş** (Eğlence Salonu, yeni ekran): liste (tek tek, toplu yapıştırma: satır ya da virgül, baştaki "1." "-" atılır; en çok 100; İzlenecek filmler / İstediğim oyunlar / Kurulu oyunlar ile doldurma; adlı listeler `fun.lists`), **Çark** (ilk çıkan; çıkan sonraki çevirişte çıkarılabilir), **Eleme** (son kalan; elenen dilim 450 ms'de daralır, renkler listedeki yere bağlı kalır; Otomatik), **Kura** (N kazanan / sıraya diz / takımlara böl: sayılar en çok bir farklı), **Zar & yazı-tura** (1–6 zar, yazı-tura, aralıktan sayı). `crypto.getRandomValues` + red örnekleme (modülo yanlılığı yok), Fisher–Yates; çark önce hedef dilimi seçer, sonra o dilimin %15–85 arasında duracak son açıyı hesaplar (`landing`; 40 dilime kadar her hedef için sınandı); dörtlü yavaşlama, dilim geçişinde WebAudio tık ve ibre sekmesi; kutlama (konfeti, hareket azaltmada yok), Büyük göster, son 15 sonuç, Discord'a kopyala. **Sayaç akıcı:** rakamlar kayarak değişir (`RollingClock`: değişen rakam alttan/üstten gelir, salise canlandırılmaz), güncelleme 100 ms aralık yerine her karede. **Zamanlı kapatma:** son dakika uyarısında × ile gizle (plan `hidden`; süre işler, zamanı gelince uygulanır; OSD'de kalan süre); oyun içi panel, kapatma uyarısı için açıldığında hemen kapanıyordu (yalnız hatırlatıcı/sayaca bakılıyordu), düzeltildi. **Eklenti 1.5.0 · TikTok:** akışta adres çubuğu akışın adresi; videonun kimliği oynatıcı kutusundan (`xgwrapper-…-KİMLİK`) ya da karttaki `/video/` bağlantısından, kullanıcı adı profil bağlantısından → `tiktok.com/@kullanıcı/video/KİMLİK`. TikTok'ta oynatıcının dosyası (imzalı, çereze bağlı) denenmez; eklenti penceresinden de ekranda en çok görünen/oynayan videonun bağlantısı alınır |
| v0.40 | 2026-10-13 | Geri bildirim: **sesle yazma ilk kullanımda yavaş.** Nedenler: sunucular ilk dikteyle başlıyordu (model diske/ekran kartı belleğine yükleniyor); ekran kartının ilk işi CUDA hazırlığı (yeni kartlarda sürücü çekirdekleri derler, önbelleğe alır); ilk açılışta büyük CUDA dosyaları virüs taramasından geçer; NVIDIA sürümü 90 sn'de hazır olmazsa oturum boyunca işlemciye düşülüyordu (dikte başına 5–15 sn). Düzeltme: GMID açıldıktan 20 sn sonra (oyun öndeyse değil) modeller arka planda yüklenir ("Açılışta hazırla", varsayılan açık); sunucu hazır olunca 1 sn'lik sessiz sesle ısındırılır (ekran kartında), toparlayıcı tek sözcüklük istekle; NVIDIA sürümüne 240 sn verilir ve yalnızca program çökerse işlemciye geçilir (yavaş açılış sonraki diktede yeniden ekran kartını dener). Model yüklenirken gösterge "Hazırlanıyor" der (5 dk'ya kadar). Günlüğe açılış ve ısınma süreleri yazılır |
| v0.39 | 2026-10-13 | Geri bildirim: **Sistem özellikleri yeniden düzenlendi** ("biraz karışık"). Arka uç artık yapılı veri döndürür (Specs: os/cpu/gpu/ram/board/disks/volumes/monitors/input/audio/net; Windows PowerShell 5.1'in `{"value":[…],"Count":n}` dizi biçimi ve tek öğe/null durumları ayrıştırmada düzeltilir). Toparlama: işlemci adı sadeleşir ("AMD Ryzen 7 7800X3D"), uzak/sanal/temel ekran bağdaştırıcıları atılır, ekran kartları VRAM'e göre sıralanır, EDID ve JEDEC üretici kodları markaya çevrilir (GSM→LG, 04CD→G.Skill), aynı fare/klavye/ses aygıtı bir kez, adı olan aygıt varken "HID-compliant mouse" gibi genel adlar gizlenir; monitör boyutu inç olarak (EDID fiziksel boyuttan). Görünüm: üstte bilgisayar adı, Windows sürümü, açık kalma süresi; dört kutu (İşlemci, Ekran kartı, Bellek, Depolama; dar pencerede 2×2); altında sütunlara akan kartlar: sürücü doluluk çubukları (%90 üstü kırmızı) + disk türü (NVMe · SSD), ekranlar (inç, yıl, ana ekran çözünürlüğü/Hz), çevre birimleri, bellek modülleri, anakart/BIOS, ağ. "Discord'a kopyala" aynı veriden düzenli bir özet yazar |
| v0.38 | 2026-10-13 | Geri bildirim: **Hatırlatıcı/sayaç açılır pencere** (ortada; Ertele 5/10 dk/1 saat, Kapat; Enter/Esc) ve **sakin melodi** (bellekte üretilen kısık müzik kutusu WAV'ı, döngü, en geç 90 sn; ses düzeyi ayarı ve "Dinle"). Öndeki pencere ekranı kaplıyorsa (oyun) pencere yerine **Oyun İçi Panel** açılır, uyarı en üstte; uyarılar bitince panel kapanır, oyuna dönülür. Ayarlar: açılır pencere, oyunda panel, melodi + ses. **Zamanlı kapatma** (Kontrol Odası → Genel, onaylı): Kapat / Yeniden başlat / Uyut, 10 dk–2 saat ya da elle; son 60 sn'de ortada (oyundaysa panelde) geri sayım, İptal / Şimdi; `shutdown /s|/r /f /t 0`, uyku SetSuspendState; GMID kapanırsa plan düşer. **Sistem özellikleri** (Kontrol Odası): Windows, işlemci, ekran kartı (gerçek VRAM kayıt defterinden), RAM modülleri, anakart/BIOS, diskler (SSD/NVMe), monitörler (EDID adı), fare/klavye (USB ürün adı), ses, ağ; tek PowerShell (EncodedCommand, çıktı base64), önbellek, "Kopyala" (Discord'a). **Panelde ek ölçümler** (PDH): işlemci saat hızı, ACPI sıcaklığı (varsa, yaklaşık; sahte sabit değerler ayıklanır), VRAM, disk etkinliği, ağ indirme/yükleme. Gerçek çekirdek sıcaklığı sürücü (WinRing0 vb.) gerektirdiği için okunmuyor (anti-cheat/Defender riski) |
| v0.37 | 2026-10-13 | Geri bildirim: **FPS görevi** bazı bilgisayarlarda arka plan oturumunda (S4U) hiç başlamadı. Yeni görev "GMID FPS 2": kullanıcının oturumunda (Interactive, en yüksek yetki), `conhost --headless` içinde (pencere açmaz), PresentMon hata çıktısı `%TEMP%\gmid-fps.err`'e; 12 sn'de çıktı yoksa nedeni (hata satırı ya da görevin son sonuç kodu) panelde ve günlükte. Eski görev yeniden kurulumda silinir. **Kırp ve hazırla:** başlangıç/bitiş tutamağı sürüklenirken video o kareye gider, tutamakların üstünde zaman yazar |
| v0.36 | 2026-10-12 | Geri bildirim: yan menüdeki Discord düğmesi kenardan taşıyordu (`width: 100%` + yan boşluk): logo solda, "GMID'e katıl". **Sesle yazma sözlüğü:** "şunu duyunca bunu yaz" kuralları (sözcük sınırında, büyük/küçük harf ve noktalı/noktasız i fark etmez, kesme işaretli ek korunur; toparlamadan önce ve sonra uygulanır; doğru yazılışlar Whisper ipucuna eklenir). Öğrenme: yapıştırılan alan UI Automation ile ~2 dk izlenir (değer ya da belge metni; şifre alanı atlanır); gönderilince/silinince son hal sözcük düzeyinde LCS ile karşılaştırılır, en çok 3'er sözcüklük değişiklikler (ve kısaltma büyük harf düzeltmeleri) sağ altta "Bunu böyle mi yazayım?" kartı olur; "Hayır" denenler bir daha sorulmaz. Kontrol Odası'nda kural listesi, elle ekleme, "Son dikteler"de düzelt-öğren. **Discord'a sığdır tek tık:** yeniden anında; varsayılan çözünürlüğü korumak (60 sn 1080p60 → 10 MB: 1080p30 SSIM 0,921 > 900p30 0,918 aynı boyutta); bit piksel başına AV1 eşiğinin yarısının altına düşerse küçültülür. SVT-AV1 `tune=0` (görsel kalite; ölçümde biraz daha iyi). ⚙ seçenekleri açar; "Orijinal çözünürlük" seçeneği eklendi |
| v0.35 | 2026-10-12 | Geri bildirim: **FPS oturum kapatmadan:** "FPS ölçümünü aç" tek UAC ile "GMID FPS" zamanlanmış görevini kaydeder (kullanıcı, S4U, en yüksek yetki, pencere açmaz, 20 dk sınır) — kullanıcı her an başlatabildiği için oturum yenilemesi gerekmez; çıktı `%TEMP%\gmid-fps.csv`'den okunur. Grup üyeliği (Performance Log Users) yedek yol olarak kalır (Add-LocalGroupMember + `net localgroup`, doğrulamalı). PresentMon tüm programları izler (dwm hariç); öndeki pencerenin süreci kare göndermiyorsa (başlatıcı) en çok kare gönderen alınır, oyun duraklamışsa başka programınki gösterilmez. Panelde FPS yoksa nedeni ve "FPS ölçümünü aç" düğmesi görünür. **Kenarlıksız tam ekran** (Borderless Gaming gibi, enjeksiyonsuz): listedeki oyun pencereli açılınca çerçevesi kaldırılır ve ekranı kaplar; panel özel tam ekranda açılamazsa (oyun küçülür) oyun listeye kendiliğinden eklenir, Windows bildirimiyle "Pencereli moda al" denir. Küçülme bir saniye boyunca izlenir. **Discord'a hazırla seçenekleri:** Net / Akıcı (60 fps) / Uyumlu (H.264) 10 MB, Nitro Basic 50 MB, Nitro 500 MB, Olduğu gibi; her birinde çözünürlük, kare hızı, beklenen MB, netlik; uzun klipte "Son 30 sn" önerisi; hazırlanan kopya pencerede izlenir, birden çok seçenek yan yana denenebilir (dosya adı seçeneği söyler) |
| v0.34 | 2026-10-12 | **Uygulama başına ses** (Windows ses karıştırıcısı: `IAudioSessionManager2`, varsayılan çıkış, süreç başına gruplu; Oyun İçi Panel'de "Uygulamalar" ve Ses & Mikrofon'da). **Sorun bildir** (Hakkında): açıklama + sürüm/derleme + Windows sürümü + son günlükler tek zip (`Raporlar\`), kullanıcı klasörü yolu `%USERPROFILE%` olarak gizlenir, kısa özet panoya, Discord açılır. **Sunucu bildirimi**: kartın zil düğmesi — açılınca ve/veya en az N kişi olunca (yalnızca izlenen sunucular 2 dakikada bir sorgulanır; ilk gözlem sessiz, 45 dk bekleme, kalabalık bildirimi sayı düşünce yeniden kurulur). **İlk açılış rehberi** (5 adım: hoş geldin + Discord, kısayollar, oyun içi panel + FPS izni, klip klasörleri, Windows ile başlat; bir kez çıkar, Hakkında'dan yeniden açılır) |
| v0.33 | 2026-10-12 | Geri bildirim: **Oyun İçi Panel Shift+Tab gibi**: pencere açılışta gizli hazırlanır (oyunda ilk oluşturulurken odak kapıp oyunu küçültmesin), odak almadan yerleştirilir; kenarlıksız/pencereli oyunda **etkileşim kipi** (odak alır, fare serbest, oyun arkada görünür), özel tam ekranda (`SHQueryUserNotificationState`) **bakış kipi** + ipucu; oyun odak gidince küçülürse geri getirilir, bakış kipine geçilir ve o oyun hatırlanır. Gerçek oyun içi katman (Steam/Discord gibi oyuna DLL enjekte etmek) anti-cheat yasağı riski yüzünden yapılmadı. **FPS RTSS olmadan:** Intel PresentMon 2.4.1 (SHA-256 sabit, ~1 MB, `Araclar\PresentMon`), ETW ile, oyuna dokunmaz; Windows izni için bir kez UAC ile "Performance Log Users" (S-1-5-32-559) grubuna ekleme + oturumu yenileme (Oyun Modu → FPS ölçümünü aç). **GMID Discord'a katıl** düğmesi (yan menü, Giriş Salonu, Hakkında); davet bağlantısı `topluluk/sunucular.json` → `discord` alanından gelir (uygulama yeniden derlenmeden değiştirilebilir) |
| v0.32 | 2026-10-12 | **Oyun İçi Panel'de performans şeridi:** FPS (RivaTuner/RTSS açıksa paylaşılan belleğinden, oyuna enjeksiyon yok), işlemci yükü, ekran kartı yükü ve sıcaklığı (NVIDIA: `nvidia-smi`; diğerleri: Windows GPU sayacı, sıcaklıksız), RAM; yalnız panel açıkken saniyede bir. Yol haritası §9: Faz 7 durumu ve yapılacaklar listesi (haftalık otomatik yedek bekliyor, GMID'e paylaş VDS ile) |
| v0.31 | 2026-10-12 | **Her şeyde tek arama** (her yerden Ctrl+K ya da üst çubuktaki "Ara"): ekranlar, Kontrol Odası sekmeleri, notlar (başlık ve içerik, eşleşen yerin çevresi gösterilir), oyunlar (kütüphane ve arşiv), film/diziler (yönetmen dahil), videolar/müzikler (açınca çalar), klipler, hatırlatıcılar. Türkçe harfler katlanır ("isik" ~ "ışık"); başta eşleşen önce. Sonuca tıklayınca o kayıt kendi ekranında açılır |
| v0.30 | 2026-10-12 | **GMID Wrapped** (Oyun Salonu): ay ya da yıl özeti tek görselde (1080×1350 PNG) — toplam saat, oturum/gün/oyun sayısı, en çok oynanan 5 oyun (kapaklı, süre çubukları), rozetler (Sadık, Gece Kuşu, Maratoncu, Hafta Sonu Savaşçısı, Kâşif, Sinefil, Eleştirmen, Klip Avcısı; en fazla 3), en uzun oturum, en sevilen gün ve saat, puanlanan oyunlar, bitirilen film/diziler, klip ve video sayısı. "PNG olarak kaydet" (`Ozetler\`) ve "Panoya kopyala" (Discord'da Ctrl+V) |
| v0.29 | 2026-10-12 | **Fırsatlar** (Oyun Salonu): her platformda şu an ücretsiz oyunlar (GamerPower: Steam, Epic, GOG, PlayStation, Xbox, Ubisoft, EA, Prime Gaming, itch, Battle.net; Epic'in kendi listesi kesin tarihli, sıradaki ücretsiz dahil), istek listesindeki ("İstiyorum") oyunların indirimleri (Steam Türkiye fiyatı + CheapShark: GOG, Humble, EA, Ubisoft, Epic, Fanatical…). 3 saatte bir bakılır; yeni ücretsiz oyun ve yeni/daha büyük indirim Windows bildirimiyle (mağaza seçimi, en az indirim %). PlayStation Store ve Microsoft Store indirimleri için anahtarsız resmî kaynak yok: yalnız ücretsizleri (GamerPower) gelir |
| v0.28 | 2026-10-12 | **Uygulama içinden güncelleme:** her derleme GitHub'da `build-N` sürümü olarak yayınlanır (kurulum + SHA-256); uygulama derleme numarasını bilir, açılıştan 30 sn sonra ve 6 saatte bir bakar, Windows bildirimi + yan menüde "Güncelleme var"; Kontrol Odası → Hakkında'dan tek tıkla indir, doğrula, kapan, sessiz kur, yeniden aç |
| v0.27 | 2026-10-12 | **Klip kırpma** ve daha net "Discord'a sığdır": başlangıç/bitiş seçimi, hedef boyut (10 MB ücretsiz · 50 MB Nitro Basic · 500 MB Nitro · sınırsız = yalnız kırp, kopya klibin yanına), kodlama **Net (AV1)** ya da **Uyumlu (H.264, iki geçiş)**; çözünürlük ve kare hızı piksel başına bite göre seçilir (önce netlik), hazırlamadan önce beklenen sonuç gösterilir. Ölçüm (SSIM, aynı boyut): eski 480p tek geçiş 0,897 · H.264 iki geçiş 0,901 · AV1 1080p 0,920 |
| v0.26 | 2026-10-12 | Geri bildirim: film/dizi afişleri birden çok anahtarsız kaynaktan (İngilizce ve Türkçe Vikipedi, Wikidata/Commons afişi, filmde iTunes, dizide TVmaze; ad ve yıl tutmalı), gelmeyenler yeniden denenir (göç 16), **Afiş seç** ile elle; **Oyun İçi Panel odak almadan açılır** — oyun önde kalır, tam ekran oyunlar küçülüp masaüstüne (alt-tab) düşmez; klavye odağı yalnız nota yazarken alınır, kapanınca oyun (küçülmüşse büyütülerek) öne getirilir |
| v0.25 | 2026-10-11 | Faz 6b kodlandı: **Format Sonrası Kurulum** (doğrulanmış winget kataloğu, depoda arama, kurulu olanları bulma, sırayla sessiz kurulum ve ilerleme) ve **Format Kiti** (Kur.bat + kurulum.ps1 + GMID kurulumu + GMID-Yedek.zip); Kontrol Odası'ndan yedekten geri yükleme, yolların yeni bilgisayara çevrilmesi (§3.9). Kritik düzeltme: "Discord'a Hazır" temizliği yanlış klasöre bakıp veri klasörünün kökündeki eski dosyaları silebilirdi |
| v0.24 | 2026-10-11 | Faz 6a kodlandı: **Film & Dizi Arşivi** — bilgiler anahtar gerektirmeyen kaynaklardan (Wikipedia arama + Wikidata: Türkçe ad, yıl, süre, sezon/bölüm, yönetmen/yaratıcı, oyuncular, tür; Türkçe Vikipedi özeti; afiş), 5 film sorusu (hikâye %25, oyunculuk %20, yönetmenlik %20, tempo %15, keyif %20), durumlar (İzliyorum · İzledim · Yarıda bıraktım · İzlenecek — puansız), dizide "nerede kaldın" (S·B), nerede/kiminle izlendi; Keşfet süzgeçleri (film/dizi, tür, yönetmen, puan, durum, tavsiye), "Bana bir film seç"; göç 15 (`titles`, `reviews.extra`). TMDB yerine Wikipedia/Wikidata: kullanıcıdan anahtar istenmesin diye |
| v0.23 | 2026-10-11 | Faz 4 kodlandı: **Oyun Modu** (güç planı yüksek performans, seçilen ses çıkışı, mikrofon kilidi, sessiz, hareketli arka plan durur, seçilen uygulamalar kapanır; elle, Ctrl+Alt+O ya da oyun açılınca kendiliğinden; kapanınca ve uygulama kapanırken geri alınır) ve **Oyun İçi Panel** (Ctrl+Shift+G, oyunun bulunduğu ekranın üstünde yarı saydam: ses, mikrofon, Oyun Modu, sayaç, oyuna not, son klip ve Discord'a sığdır, hatırlatıcılar; Esc ile kapanır, odak oyuna döner) |
| v0.22 | 2026-10-11 | Geri bildirim: video küçük resimleri kaybolmuyor (aynı videonun iki indirmesi ortak resmi silmiyordu → kontrol; eksik/bozuk resimler açılışta ve kartta onarılır: siteden ya da videodan kare, müzikte gömülü kapak), Kliplerim kartında sil (ilk tık "Sil?"), sesle yazma göstergesi inceltildi (34 px), Steam dışı oyunlara Microsoft Store'un dikey posteri (Minecraft; yoksa Wikipedia; göç 14), oyun tam ekranken görünmeyen "Kütüphaneye ekleyeyim mi?" kartı oyun kapanınca yeniden gösterilir |
| v0.21 | 2026-10-11 | Geri bildirim: tam ekran video ekrana tam sığıyor (alt kısmı kesilmiyor, 4:3 yanlarda bantla), fare dururken hiçbir düğme yok, kontroller görüntünün üstüne binmiyor; tarayıcının sağ tık menüsü kapatıldı; Minecraft (Java: pencere başlığıyla, Bedrock) ve Xbox uygulaması oyunları taramada; Steam dışı oyunlara Wikipedia'dan kapak; kütüphanede olmayan oyun açılınca "Kütüphaneye ekleyeyim mi?" (Discord'un bilinen oyunlar listesi + tam ekran ipucu; göç 13) |
| v0.20 | 2026-10-11 | Geri bildirim: sesle yazma göstergesi takılı kalmıyor (pencere Windows'a doğrudan gizletiliyor + gösterge kendi güvenlik ağı), Riot oyunları (League of Legends, VALORANT) taramada, elle kapak seçme, puan kartında önce durum ("İstiyorum"da puan yok; göç 12), format yedeğine GMID ayarları ve geçmişi (§3.9) |
| v0.19 | 2026-10-11 | Faz 3b-3 kodlandı (Faz 3b tamam): Kliplerim — NVIDIA/Xbox/AMD/OBS/Steam/Medal klasörleri, oyuna göre gruplama, önizleme, Discord'a sığdır (10 MB, panoya dosya), Geri Dönüşüm Kutusu'na silme, eski klip temizliği |
| v0.18 | 2026-10-11 | Faz 3b-2 kodlandı: Oyun Arşivi (5 soruluk puan kartı, Yok seçeneği, otomatik genel puan), Türkçe etiketler, Keşfet filtre kutusu, Bana bir oyun seç, arşive Steam'den arayarak ekleme |
| v0.17 | 2026-10-11 | Faz 3b-1 kodlandı: Kütüphanem (Steam/Epic tarama, elle ekleme, otomatik bilgi ve kapak, oyunu başlatma), canlı oyun algılama ve Oturum Geçmişi (grafik) |
| v0.16 | 2026-10-11 | Kendi çalma listeleri, tam ekran kontrol şeridi, mini videoda kapat/büyüt, sesle yazmada alt ortada yazısız gösterge |
| v0.15 | 2026-10-11 | Medya: Müzik ve Videolar sekmeleri, eski şarkılarda düz sözlerin hizalanması, sesle yazmada geri bildirim sesi ve hızlı açılış |
| v0.14 | 2026-10-11 | Sesle yazma (Ctrl+Alt basılı tut): klavye kancası (AltGr ayrımı), WASAPI kayıt, whisper-server (NVIDIA CUDA ya da işlemci), yerel toparlama (Gemma 3 4B / llama.cpp), pano ile yapıştırma; Kontrol Odası → Sesle yazma |
| v0.13 | 2026-10-11 | Medya Oynatıcı ayrı ekran ve arka planda çalma (mini oynatıcı), film tam ekranı düzeltildi, dublaj/altyazı seçimi (indirme, eklenti, oynatıcıda CC), MP3 sabit bit hızı; sesle yazma (Wispr Flow benzeri) araştırıldı |
| v0.12 | 2026-10-11 | Geri bildirim turu: indirmelerde kendiliğinden yeniden deneme ve doğru hata mesajları, eklentide gizli HLS yakalama + iframe Referer + X/Instagram akışı, şarkı sözü veritabanı (LRCLIB), Whisper uydurma süzgeci ve dil tespiti, oynatıcı yeniden yazıldı (özel kontroller, liste görünümü, alt çalma çubuğu) |
| v0.11 | 2026-10-11 | Video İndirici genişledi: YouTube çalma listeleri (seçerek toplu indirme, liste klasörü), eklenti her sitede video yakalama (HLS/DASH/mp4 + Referer), video üstü düğme ve sağ tık kalite menüsü, uygulama içi oynatıcı ve senkron yazılar (site altyazısı ya da yerel Whisper; .srt/.lrc), parçalı hızlı indirme |
| v0.10 | 2026-10-11 | Kontrol Odası sekmeli ayarlar merkezine dönüştü (arama, kısayollar burada, gruplu); yeni kısayollar: pencereyi aç, panodaki bağlantıyı indir; ekran göstergesi ayarı; Video İndirici'de Türkçe Windows dosya adı hatası giderildi |
| v0.9 | 2026-10-11 | Faz 5 kodlandı: Video İndirici (§3.1 özet kutusu), yerel eklenti köprüsü ve eşleştirme, Chrome/Opera eklentisi; kararlılık: Windows'ta pencere açarken kilitlenme giderildi, hatırlatıcılara Windows bildirimi, `gmid.log`; sıra değişikliği (3b/4/3c sonraya) |
| v0.8 | 2026-10-11 | Faz 3a kodlandı: Sunucular (§3.14) — CoD/Minecraft/Steam/SA-MP sunucu sorguları, canlı oyuncu sayısı, Bağlan, GitHub'daki topluluk listesi; sohbet penceresi Faz 3c olarak plana eklendi (§8.1.1) |
| v0.7 | 2026-10-11 | Faz 2 kodlandı: Ses & Mikrofon (Windows Core Audio, olay tabanlı mikrofon kilidi), değiştirilebilir genel kısayollar, odak çalmayan ekran göstergesi, Notlar (TipTap, .md kopyası, çöp kutusu), hızlı not ve yapışkan not pencereleri, hatırlatıcılar (tekrar, erteleme, sağ alt uyarı kartı), kronometre ve geri sayımlar |
| v0.6 | 2026-10-11 | Faz 1 (İskelet) kodlandı: Tauri 2 + Svelte 5 projesi, açılış animasyonu, Malikâne teması, menü, Kontrol Odası, veri klasörü, SQLite, günlük yedek, tepsi, Windows ile başlatma; GitHub Actions ile otomatik Windows kurulum dosyası |
| v0.5 | 2026-10-11 | Görsel taslak eklendi; Keşfet sağ filtre paneline taşındı (alışveriş sitesi düzeni); Video İndirici uygulamada görünür hale geldi (kendi sekmesi, menü rozeti, üst çubuk göstergesi, Giriş Salonu kutusu, bağlantı yapıştırma, eklenti önizlemesi) |
| v0.4 | 2026-10-10 | Görsel kimlik (Malikâne teması, logolardan ölçülen renk paleti, Merkut Studios imzası); açılış animasyonu; malikâne odası isimli menü; klipler taşınmıyor, uygulamada gösteriliyor (Galeri → Kliplerim); Videolarım; GMID Tool botlardan bağımsız, Butler önerileri ve Card Idler çıkarıldı; kendiliğinden sığan arayüz kuralları (§5.5); açık sorular varsayılanlarla sadeleştirildi |
| v0.3 | 2026-10-10 | Puanlama 5 kısa soruya çevrildi (oyun ve film); kapsam: önce yerel, sunucu tarihi belirsiz; Galeri taşıma kararı; tam ekran oyunlar için sesli geri bildirim; "gerçek oynama" ve "Steam süresi" ayrımı; GMID evreninin mevcut parçaları (Butler, Music Bot, Card Idler) incelendi; yeni öneriler: paylaşım dosyası, Butler'ın ağzından bildirimler, Steam Kart Toplayıcı |
| v0.2 | 2026-10-10 | İsim ve hikâye (Game Mansion In Discord); denge kuralı; sade puanlama + otomatik oyun bilgisi + Keşfet ekranı; dizi eklendi; Ses & Mikrofon (seviye kilidi, kısayollar, ekran göstergesi); yeni modüller: Oyun İçi Panel, Galeri; Oyun Modu onaylandı; teknoloji seçimi kesinleşti; Butler entegrasyonu fikri; yol haritası güncellendi |
