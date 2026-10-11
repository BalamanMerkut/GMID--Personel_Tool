# GMID Tool — Game Mansion In Discord
## Tasarım Dokümanı · v0.5 (taslak)

> **Durum:** Faz 1 (İskelet) ve Faz 2 (Günlük araçlar) kodlandı. Sonraki fazlar bu dokümana göre ilerliyor.
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

### 3.1 Video İndirici + Chrome/Opera Eklentisi (IDM benzeri)

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

**Sonra:** Windows ayarları paketi (dosya uzantılarını göster, karanlık mod, gereksiz başlangıç programlarını kapat vb.).

**Teknik not:** Windows 10/11'de yerleşik **winget** (Microsoft'un resmî paket yöneticisi). Paket kimlikleri: [Ek B](ARASTIRMA-TEKNIK.md) §5.

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
| 8 | **Discord Rich Presence** | Discord profilinde "GMID Tool'da: Hades II puanlıyor" gibi durum. Malikânenin tanıtımı için de iyi. | Kolay | 👍 |
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
├─ Kapaklar\                ← Oyun/film kapak önbelleği (silinirse yeniden indirilir)
├─ Kurulum\                 ← Format sonrası kurulum listeleri + dışa aktarılan .bat
├─ Yedekler\                ← Veritabanının otomatik günlük yedekleri (son 7 gün)
└─ Araclar\                 ← yt-dlp, ffmpeg, deno (uygulama kendisi günceller)
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
| `downloads` | id, url, site, başlık, kalite, dosya_yolu, boyut, durum, tarih |
| `notes` | id, başlık, içerik, renk, yazı_tipi, sabit, klasör, etiketler, bağlı_oyun, güncellenme |
| `reminders` | id, başlık, zaman, tekrar_kuralı, ertelenen, tamamlandı |
| `audio_devices` | windows_id, takma_ad, ikon, gizli, sıra, kilit_seviyesi |
| `install_lists` / `install_items` | liste adı · winget_id, ad, kategori, sıra |
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
| **3 — Oyun** | Oyun algılama, Steam/Epic/EA içe aktarma, kütüphane, otomatik bilgi ve kapak, Oyun Arşivi (puan, yorum, keşfet), Kliplerim | Discord gibi oyun yakalayan, puanlayıp filtreleyebildiğin arşiv |
| **4 — Oyun içi deneyim** | Oyun İçi Panel, Oyun Modu | Oyundan çıkmadan kontrol |
| **5 — Video İndirici** | İndirme yöneticisi, Videolar sekmesi, Chrome/Opera eklentisi | IDM benzeri indirici |
| **6 — Film & Dizi + Kurulum** | Film & Dizi Arşivi, format sonrası kurulum + dışa aktarılan .bat | Tüm istenen modüller tamam (**v1.0**) |
| **7 — Öneriler** | Onaylanan ek özellikler (ücretsiz oyunlar, Wrapped, Rich Presence…) | v1.x |
| **8 — Çevrim içi** *(tarih belirsiz)* | Sunucu (VDS), ortak arşiv, arkadaşlar, sohbet | v2.0 |

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
| v0.7 | 2026-10-11 | Faz 2 kodlandı: Ses & Mikrofon (Windows Core Audio, olay tabanlı mikrofon kilidi), değiştirilebilir genel kısayollar, odak çalmayan ekran göstergesi, Notlar (TipTap, .md kopyası, çöp kutusu), hızlı not ve yapışkan not pencereleri, hatırlatıcılar (tekrar, erteleme, sağ alt uyarı kartı), kronometre ve geri sayımlar |
| v0.6 | 2026-10-11 | Faz 1 (İskelet) kodlandı: Tauri 2 + Svelte 5 projesi, açılış animasyonu, Malikâne teması, menü, Kontrol Odası, veri klasörü, SQLite, günlük yedek, tepsi, Windows ile başlatma; GitHub Actions ile otomatik Windows kurulum dosyası |
| v0.5 | 2026-10-11 | Görsel taslak eklendi; Keşfet sağ filtre paneline taşındı (alışveriş sitesi düzeni); Video İndirici uygulamada görünür hale geldi (kendi sekmesi, menü rozeti, üst çubuk göstergesi, Giriş Salonu kutusu, bağlantı yapıştırma, eklenti önizlemesi) |
| v0.4 | 2026-10-10 | Görsel kimlik (Malikâne teması, logolardan ölçülen renk paleti, Merkut Studios imzası); açılış animasyonu; malikâne odası isimli menü; klipler taşınmıyor, uygulamada gösteriliyor (Galeri → Kliplerim); Videolarım; GMID Tool botlardan bağımsız, Butler önerileri ve Card Idler çıkarıldı; kendiliğinden sığan arayüz kuralları (§5.5); açık sorular varsayılanlarla sadeleştirildi |
| v0.3 | 2026-10-10 | Puanlama 5 kısa soruya çevrildi (oyun ve film); kapsam: önce yerel, sunucu tarihi belirsiz; Galeri taşıma kararı; tam ekran oyunlar için sesli geri bildirim; "gerçek oynama" ve "Steam süresi" ayrımı; GMID evreninin mevcut parçaları (Butler, Music Bot, Card Idler) incelendi; yeni öneriler: paylaşım dosyası, Butler'ın ağzından bildirimler, Steam Kart Toplayıcı |
| v0.2 | 2026-10-10 | İsim ve hikâye (Game Mansion In Discord); denge kuralı; sade puanlama + otomatik oyun bilgisi + Keşfet ekranı; dizi eklendi; Ses & Mikrofon (seviye kilidi, kısayollar, ekran göstergesi); yeni modüller: Oyun İçi Panel, Galeri; Oyun Modu onaylandı; teknoloji seçimi kesinleşti; Butler entegrasyonu fikri; yol haritası güncellendi |
