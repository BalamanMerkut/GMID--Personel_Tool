# GMID — Kişisel Oyuncu Araç Kutusu
## Tasarım Dokümanı · v0.1 (taslak)

> **Durum:** Fikir / karar aşaması. Kod yazılmadı.
> Bu doküman birlikte güncellenecek. Her bölümün sonunda **❓ Karar bekleyen** maddeler var;
> sen cevapladıkça ✅ olarak işaretleyip kesinleşen kararları **Bölüm 11**'e taşıyacağız.

---

## İçindekiler
1. [Vizyon ve temel ilkeler](#1-vizyon-ve-temel-ilkeler)
2. [Uygulama yapısı (kategoriler ve sekmeler)](#2-uygulama-yapısı-kategoriler-ve-sekmeler)
3. [Modüller](#3-modüller)
4. [Önerdiğim ek özellikler](#4-önerdiğim-ek-özellikler)
5. [Teknik mimari](#5-teknik-mimari)
6. [Veri klasörü düzeni (derli toplu yapı)](#6-veri-klasörü-düzeni-derli-toplu-yapı)
7. [Veritabanı taslağı](#7-veritabanı-taslağı)
8. [Gelecek: çevrim içi sürüm](#8-gelecek-çevrim-içi-sürüm)
9. [Yol haritası](#9-yol-haritası)
10. [Riskler ve yasal notlar](#10-riskler-ve-yasal-notlar)
11. [Kesinleşen kararlar ve açık sorular](#11-kesinleşen-kararlar-ve-açık-sorular)

Ekler: [Oyun ve film puanlama araştırması](ARASTIRMA-PUANLAMA.md) · [Teknik fizibilite araştırması](ARASTIRMA-TEKNIK.md)

---

## 1. Vizyon ve temel ilkeler

**Tek cümleyle:** Oyuncunun bilgisayarında açık duran, küçük ama her işe yarayan; oyunlarını,
filmlerini, notlarını, indirdiği videoları ve Windows'taki küçük can sıkıcı işleri tek bir şık
pencerede toplayan kişisel araç.

**Öncelik sırasına göre üç temel ilke** (her tasarım kararında bunlara bakılacak):

| # | İlke | Pratikte ne demek? |
|---|------|--------------------|
| 1 | **Optimizasyon** | Arka planda çalışırken RAM/CPU'yu neredeyse hiç kullanmamalı (hedef: boşta **< %1 CPU**, tepside **< 100 MB RAM**). Oyun açıkken hiçbir animasyon/ağır iş çalışmamalı. Hızlı açılış (hedef **< 1 sn**). |
| 2 | **Anlaşılabilirlik** | Her ekran tek bir işi yapar. En fazla 2 tıkla her şeye ulaşılır. Türkçe arayüz, net ikonlar, boş ekranlarda "ne yapmalıyım" ipucu. |
| 3 | **Derli toplu** | Uygulamanın ürettiği **her şey tek bir ana klasörde** durur (videolar, notlar, arka planlar, veritabanı, yedekler). Hiçbir şey "İndirilenler"e ya da masaüstüne dağılmaz. Klasörü kopyalayan, her şeyini kopyalamış olur (format sonrası için de önemli). |

**Kapsam:** Şimdilik **tek kullanıcı, yerel (offline-first), Windows 10/11**. Mimari, ileride
çevrim içi özelliklerin (sohbet vb.) yeniden yazmaya gerek kalmadan eklenebileceği şekilde kurulacak.

---

## 2. Uygulama yapısı (kategoriler ve sekmeler)

Sol tarafta daraltılabilir bir menü, sağda içerik. Kategoriler:

```
🏠 Ana Sayfa            → Özet panosu (bugün oynanan süre, yaklaşan hatırlatıcılar,
                           son notlar, devam eden indirmeler)

🎮 Oyun
   ├─ Kütüphanem        → Steam / Epic / EA / elle eklenen oyunlar + oynama süreleri
   ├─ Oturum Geçmişi    → Hangi gün hangi oyunu ne kadar oynadım (grafik)
   └─ Oyun Modu*        → Tek tıkla "oyuna hazırlan" profili (bkz. Bölüm 4)

🍿 Eğlence
   ├─ Oyun Değerlendirme
   ├─ Film & Dizi Değerlendirme
   └─ Videolar          → İndirilen videolar (IDM benzeri indiricinin kütüphanesi)

🧰 Araçlar
   ├─ Notlar
   ├─ Hatırlatıcılar
   ├─ Sayaç             → Kronometre + geri sayım (+ Pomodoro*)
   ├─ Ses Cihazları     → Kulaklık / hoparlör tek tık geçiş
   └─ Format Sonrası Kurulum

⚙️ Ayarlar             → Görünüm (arka plan), başlangıçta açılma, kısayollar,
                           veri klasörü, yedekleme, eklenti bağlantısı
```
`*` = benim önerim, onayına bağlı.

**Hızlı erişim (pencereyi açmadan):**
- **Sistem tepsisi ikonu** (saatin yanı): sağ tık → ses cihazı değiştir, hızlı not, hatırlatıcı ekle, sayaç başlat.
- **Genel kısayol tuşları** (ör. `Ctrl+Alt+N` hızlı not, `Ctrl+Alt+A` ses cihazı değiştir) — ayarlardan değiştirilebilir.

❓ **Karar bekleyen**
- Oyun değerlendirme "Eğlence" altında mı kalsın, yoksa oyunun kütüphane kartının içinde bir sekme mi olsun? (Önerim: veri aynı oyun kaydında tutulsun, **iki yerden de** ulaşılsın.)
- "Dizi" de değerlendirilsin mi, sadece film mi?

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

**MVP**
- YouTube, Twitter/X, Instagram (ve yt-dlp'nin desteklediği yüzlerce site, ücretsiz bonus).
- Çözünürlük seçimi, sadece ses seçeneği, indirme kuyruğu, duraklat/iptal.
- Videolar sekmesi: küçük resim, başlık, kaynak site, süre, boyut, tarih; arama ve filtre; tek tıkla oynat / klasörde göster / sil.
- Dosyalar otomatik olarak siteye göre alt klasörlere: `Videolar/YouTube/`, `Videolar/Twitter/`…

**Sonra**
- Oynatma listesi / kanal toplu indirme, altyazı indirme, video kırpma (başlangıç–bitiş).
- Panodan otomatik link yakalama ("Panoda YouTube linki var, indireyim mi?").

**Teknik not:** Eklenti tek başına indirme yapmaz; sadece "kapı zili" görevi görür. Sayfa adresini
Chrome'un resmî **Native Messaging** kanalıyla (port açmadan) masaüstü uygulamasına iletir. Asıl işi
uygulamanın içindeki **yt-dlp + ffmpeg + deno** üçlüsü yapar (yüksek çözünürlüklerde görüntü ve ses
ayrı gelir, ffmpeg birleştirir; YouTube 2025 sonundan beri deno'yu da gerektiriyor). YouTube sık
değiştiği için yt-dlp kendini otomatik günceller. Instagram/X gibi giriş isteyen sitelerde eklenti
**yalnızca o sitenin** çerezlerini geçici olarak uygulamaya verir, iş bitince silinir.
Ayrıntılar: [ARASTIRMA-TEKNIK.md](ARASTIRMA-TEKNIK.md) §1.

> ⚠️ **Önemli:** Chrome Web Store, YouTube'dan indirme yapan eklentileri kabul etmiyor. Bu yüzden
> eklenti mağazadan değil, **"Geliştirici modu → Paketlenmemiş öğe yükle"** ile kurulacak.
> Kişisel kullanım için sorun değil. Opera, Chrome eklentilerini doğrudan çalıştırabildiği için
> **aynı eklenti Opera'da da çalışacak** (ekstra iş yok denecek kadar az).

❓ **Karar bekleyen**
- Varsayılan kalite ne olsun? (Önerim: "Her seferinde sor" + "Hep en iyisini indir" seçeneği.)
- Videonun üstüne düğme mi eklensin, yoksa sadece eklenti ikonu yeterli mi?

---

### 3.2 Oyun Kütüphanesi ve Süre Takibi (Discord tarzı algılama)

**Amaç:** Hangi platformdan olursa olsun oynadığın oyunları otomatik yakalayıp kaç saat
oynadığını kaydetmek.

**İki katmanlı algılama:**

| Katman | Ne yapar? | Kaynak |
|--------|-----------|--------|
| **1. Platform içe aktarma** | Steam / Epic / EA'da kurulu (ve Steam'de geçmişte oynanmış) oyunları ve **platformun kendi tuttuğu toplam süreyi** içe aktarır. | Steam yerel dosyaları + (isteğe bağlı) Steam Web API, Epic/EA kurulum kayıtları |
| **2. Canlı algılama (Discord gibi)** | Bilgisayarda çalışan süreçleri birkaç saniyede bir hafifçe kontrol eder; bilinen bir oyun açılınca **oturum başlatır**, kapanınca bitirir. Platformdan bağımsız: crack'siz bağımsız oyunlar, emülatörler, Battle.net, Riot vb. hepsi. | Discord'un herkese açık "algılanabilir oyunlar" listesi + Steam/Epic kurulum klasörleri + elle eklenen .exe'ler |

**MVP**
- İlk açılışta "Oyunlarını tara" sihirbazı → bulunan oyunlar listelenir, istemediklerini çıkarırsın.
- Her oyun için: kapak resmi, toplam süre (platform + uygulamanın ölçtüğü), son oynama, oturum sayısı.
- **Oturum geçmişi**: günlük/haftalık grafik ("Bu hafta 14 saat oynadın, %60'ı Elden Ring").
- Tanınmayan bir oyun için: "Şu an çalışan uygulamalardan oyun ekle" düğmesi.
- AFK algılama (isteğe bağlı): 15 dk klavye/fare hareketi yoksa süreyi saymayı durdur.

**Sonra**
- Kütüphaneden oyunu doğrudan başlatma (Playnite gibi birleşik başlatıcı).
- Discord Rich Presence entegrasyonu, oyun bazlı istatistik kartları, yıllık "Wrapped" özeti.

**Teknik not:** Süreç kontrolü 5 sn aralıkla ve sadece süreç adı karşılaştırması yapılarak
yapılacak — CPU etkisi ölçülemeyecek kadar az. Oyunların içine **hiçbir şey enjekte edilmeyecek**
(anti-cheat güvenliği). Ayrıntılar: [ARASTIRMA-TEKNIK.md](ARASTIRMA-TEKNIK.md) §2.

❓ **Karar bekleyen**
- Steam Web API anahtarı almak ister misin? (Ücretsiz, 1 dakika sürer; geçmişte oynayıp sildiğin oyunların sürelerini de getirir.)
- AFK süresi sayılsın mı?

---

### 3.3 Oyun Değerlendirme

**Amaç:** Oynadığın oyunları tutarlı bir ölçütle puanlayıp kişisel bir veritabanı oluşturmak.

**Puanlama yaklaşımı (araştırma özeti — tam hali [ARASTIRMA-PUANLAMA.md](ARASTIRMA-PUANLAMA.md)):**

- Profesyonel siteler (IGN, GameSpot, OpenCritic…) bugün çoğunlukla **tek bütünsel puan** veriyor;
  eski IGN gibi kategorili sistemlerde bile genel puan "ortalama değildir" diye ayrıca verilirdi.
- Bu yüzden önerim **ikisini birleştirmek**: kategorilerden **hesaplanan puan** + istersen elle
  girdiğin **nihai puan**.

**Varsayılan kriterler ve puana etkisi (%):**

| Kriter | Etkisi | Neye bakılır (kısaca) |
|---|---|---|
| Oynanış / Mekanikler | **%25** | Oyun döngüsü eğlenceli mi, derinlik, zorluk dengesi |
| Hikâye & Dünya | **%15** | Senaryo, karakterler, lore |
| Kişisel Keyif / Etki | **%15** | Aklımda kaldı mı, içine çekti mi |
| Sanat Yönetimi & Görsellik | **%10** | Stil (teknikten önemli) + grafik kalitesi |
| Ses & Müzik | **%8** | Müzik, efektler, seslendirme |
| İçerik & Tekrar Oynanabilirlik | **%8** | Doluluk, yan içerik, tekrar oynama sebebi |
| Kontroller & Arayüz | **%7** | Tepkisellik, menü/HUD netliği |
| Performans & Teknik | **%7** | FPS, hata/çökme, yükleme |
| Fiyat / Değer | **%5** | Paraya ve zamana değdi mi |

- **Tür şablonları:** Hikâye odaklı RPG, Rekabetçi Multiplayer, Bulmaca/Indie, Yarış/Spor/Simülasyon,
  Korku/Anlatı — her biri ağırlıkları otomatik değiştirir (ör. multiplayer'da Oynanış %35, Hikâye yok).
- **"Yok" (N/A) ≠ 0 puan:** Hikâyesi olmayan oyunda Hikâye "Yok" işaretlenir, ağırlığı diğerlerine
  oransal dağılır. Hikâyesi kötü olan oyun ise düşük puan alır.
- **1–10 ölçek çapaları** (5 = vasat, 7 = iyi, 8 = çok iyi, 9 = mükemmel, 10 = başyapıt) her kaydırıcıda ipucu olarak görünür; puanların zamanla 8–9'a yığılmasını önler.
- Her değerlendirme, puanlandığı andaki ağırlıkları da saklar → şablonu sonradan değiştirsen eski puanlar bozulmaz.

**Değerlendirme ekranında olacaklar**
- Kriter başına **kaydırıcı (1–10)**; her kriterin yanında küçük "neye bakmalıyım?" ipucu.
- **Hesaplanan puan** canlı güncellenir; istersen **"Benim son puanım"** ile elle ezebilirsin (ikisi de saklanır).
- Tür ön ayarı seçimi (ağırlıklar otomatik değişir) + kriteri "**Yok/Uygulanamaz**" işaretleme (ağırlık diğerlerine dağıtılır).
- Durum: `Oynanıyor · Bitirdim · %100 · Bıraktım · Beklemede · İstek listesi`
- Platform, bitirme tarihi, oynama süresi (kütüphaneden otomatik), etiketler, "Tekrar oynar mıyım?", kısa yorum, spoiler'lı uzun not.
- Liste görünümü: puana / türe / yıla göre sırala, filtrele; "En iyi 10" kartı.

❓ **Karar bekleyen**
- Ölçek 1–10 mu (önerim), 1–100 mü, yoksa 5 yıldız mı?
- Ağırlıkları sen değiştirebilmeli misin? (Önerim: evet, ayarlardan; varsayılanlar araştırmadaki gibi.)

---

### 3.4 Film & Dizi Değerlendirme

Oyun değerlendirmesiyle **aynı ekran yapısı**, farklı kriterler.

| Kriter | Etkisi |
|---|---|
| Senaryo & Hikâye | **%22** |
| Duygusal Etki / Kişisel Keyif | **%20** |
| Oyunculuk | **%15** |
| Yönetmenlik | **%15** |
| Görüntü & Sanat Yönetimi | **%10** |
| Kurgu & Tempo | **%10** |
| Müzik & Ses | **%8** |

Şablonlar: Aksiyon/Gişe (görüntü ve kurgu ağırlıklı), Belgesel (oyunculuk yerine "Bilgi/Konu"),
Animasyon (oyunculuk → seslendirme), Dizi (+ sezonlar arası tutarlılık).
Ayrıca **Hızlı mod** (sadece tek puan + durum) ve **Detaylı mod** (tüm kriterler) olacak — her film
için 7 kaydırıcıyla uğraşmak zorunda kalmazsın.

**Ek alanlar:** İzleme tarihi, nerede izlendi (sinema/Netflix/…), kiminle izlendi, "tekrar izlerim",
izleme listesi. **Sonra:** TMDB'den afiş/özet/oyuncu bilgisini otomatik çekme (ücretsiz API).

---

### 3.5 Notlar

**Amaç:** "Yeni Metin Belgesi.txt" kaosunu bitirmek. Hızlı açılan, güzel görünen, kaybolmayan notlar.

**MVP**
- **Hızlı not penceresi:** Kısayol tuşuyla (`Ctrl+Alt+N`) her yerden açılan küçük, kenarlıksız, yarı saydam pencere. Yaz → kapat → otomatik kaydedilir. "Kaydet" düğmesi yok, kaybolma riski yok.
- **Notlar sekmesi:** Sol tarafta not listesi (arama + sabitlenmiş notlar üstte), sağda düzenleyici.
- **Biçimlendirme:** Kalın, italik, başlık, madde işareti, **yapılacaklar kutucuğu ☐**, kod bloğu — Markdown kısayollarıyla ama görsel olarak (ham işaretler görünmez).
- **Görünüm:** Not başına renk/arka plan, birkaç şık küçük yazı tipi seçeneği (ör. Inter, JetBrains Mono, Caveat el yazısı), yazı boyutu.
- **Masaüstüne yapıştır (sticky note):** Bir notu "her zaman üstte" küçük bir yapışkan not olarak masaüstünde açık bırakabilme.
- Klasör/etiket ile düzenleme, çöp kutusu (30 gün).

**Sonra:** Nota resim yapıştırma, not içinde oyun/film bağlantısı, dışa aktarma (.md / .txt).

**Teknik not:** Notlar hem veritabanında hem de `Notlar/` klasöründe **düz .md dosyası** olarak
saklanabilir → uygulama olmasa bile notlarını Not Defteri'yle açabilirsin.

❓ **Karar bekleyen** — Notlar .md dosyası olarak da dursun mu? (Önerim: evet.)

---

### 3.6 Hatırlatıcılar

**MVP**
- Başlık + tarih/saat + (isteğe bağlı) not.
- Tekrar: tek sefer / her gün / hafta içi / haftalık / aylık / özel.
- Hızlı giriş: "**30 dk sonra**", "**yarın 09:00**" gibi hazır düğmeler.
- **Windows bildirimi** (sağ alt köşe) — "Ertele 10 dk" ve "Tamam" düğmeleriyle; isteğe bağlı ses.
- Uygulama penceresi kapalıyken de çalışır (tepside bekler). Bilgisayar kapalıyken kaçırılan hatırlatıcılar açılışta gösterilir.

**Sonra:** Oyun içindeyken bildirimi ertele (oyun bitince göster), "Epic ücretsiz oyun yenilendi" gibi otomatik hatırlatıcılar.

---

### 3.7 Sayaç

**MVP**
- **Kronometre** (tur kaydı ile) ve **geri sayım** (bitince bildirim + ses).
- Birden fazla geri sayım aynı anda (ör. "Fırın 20 dk", "Mola 5 dk").
- Tepsiden tek tıkla başlatma.

**Sonra (önerim):** Pomodoro modu; "oyun süre sınırı" (ör. "bugün 2 saat oynayacağım" → dolunca uyarı).

---

### 3.8 Ses Cihazları (kulaklık ↔ hoparlör)

**Amaç:** Windows ayarlarına girmeden, **tek tıkla** ses çıkışını değiştirmek.

**MVP**
- Tüm çıkış cihazlarının listesi (ikonu, adı, varsayılan olanı işaretli); tıkla → varsayılan olur.
- Cihazlara takma ad ve ikon verme ("Kulaklık 🎧", "Hoparlör 🔊"); kullanmadıklarını gizleme.
- **Tepsi menüsünden** ve **kısayol tuşuyla** (`Ctrl+Alt+A` → sıradaki cihaza geç) değiştirme; ekranda küçük bir "🎧 Kulaklık" bilgisi belirir.
- Her cihaz için ses seviyesi kaydırıcısı.
- Mikrofon (giriş) cihazları için de aynısı.

**Sonra:** Uygulama bazlı çıkış (ör. Discord kulaklıktan, oyun hoparlörden), oyun açılınca otomatik kulaklığa geç.

**Teknik not:** Windows bunun için resmi bir düğme sunmuyor, ama SoundSwitch gibi açık kaynak
programların yıllardır kullandığı yöntemle (IPolicyConfig COM arayüzü) güvenilir şekilde yapılabiliyor.
Ayrıntılar: [ARASTIRMA-TEKNIK.md](ARASTIRMA-TEKNIK.md) §3.

---

### 3.9 Format Sonrası Kurulum

**Amaç:** Format attıktan sonra Chrome, Steam, Discord, Claude, WinRAR… gibi programları tek tek
indirmek yerine listeden seçip **tek tıkla hepsini kurmak**.

**MVP**
- **Hazır katalog** (kategorili, ikonlu): Tarayıcılar, Oyun platformları, İletişim, Araçlar, Medya, Geliştirici…
- Kendi listeni oluştur: katalogdan ekle/çıkar, sürükle-bırak sıralama, birden fazla liste ("Oyun PC'si", "Minimal").
- "**Kur**" düğmesi → sırayla, sessiz kurulum, her biri için ilerleme ve ✅/❌ durumu.
- Katalogda olmayan bir program için arama (winget deposunun tamamında arama).
- **Önemli:** Format sonrası uygulamanın kendisi de kurulu olmayacak. Bu yüzden listeyi bir
  **tek başına çalışan kurulum dosyasına** (`KurulumListem.bat` / `.ps1`) dışa aktarma özelliği olacak.
  Bunu USB'ye / buluta koyarsın, formattan sonra çift tıklarsın, her şey kurulur — ilk kurulan da GMID olur.

**Sonra:** Kurulumdan sonra Windows ayarları paketi (dosya uzantılarını göster, karanlık mod, gereksiz başlangıç programlarını kapat vb.), sürücü kurulum hatırlatıcısı.

**Teknik not:** Windows 10/11'de yerleşik gelen **winget** (Microsoft'un resmi paket yöneticisi)
kullanılacak. Ayrıntılar ve paket kimlikleri: [ARASTIRMA-TEKNIK.md](ARASTIRMA-TEKNIK.md) §5.

---

### 3.10 Görünüm ve Arka Plan

**MVP**
- Arka plan: **düz renk / degrade / resim (JPG, PNG, WebP) / hareketli (GIF, ayrıca WebM-MP4)**.
- Bulanıklık ve karartma kaydırıcıları (yazıların okunabilir kalması için), "cam" efektli paneller.
- Vurgu rengi seçimi; açık/koyu tema.
- Arka planlar `Arkaplanlar/` klasörüne kopyalanır (orijinal dosya silinse de bozulmaz).
- Notlar ve hızlı not penceresi için ayrı arka plan seçimi.

**Optimizasyon kuralları (ilke #1 gereği):**
- Pencere küçültülünce / arkada kalınca / **bir oyun algılanınca hareketli arka plan otomatik durur**.
- GIF yerine video (WebM/MP4) önerilir: aynı görüntü için genelde 5–10 kat daha küçük, daha az işlemci.
  Kullanıcı GIF seçerse, istersen tek tıkla videoya dönüştürme (ffmpeg zaten var).

---

## 4. Önerdiğim ek özellikler

Oyuncu araçlarını (Playnite, Discord, SoundSwitch, Backloggd, HowLongToBeat, IsThereAnyDeal,
NVIDIA/Xbox Game Bar…) inceleyip senin uygulamana uyanları seçtim. Teknik ayrıntılar:
[ARASTIRMA-TEKNIK.md](ARASTIRMA-TEKNIK.md) §8.

**Önerim:** ⭐ = kesinlikle eklensin · 👍 = eklenirse güzel olur · 🤔 = sonra düşünürüz · ❌ = eklemeyelim

| # | Özellik | Ne işe yarar? | Zorluk | Önerim |
|---|---------|---------------|--------|--------|
| 1 | **Oyun Modu** | Tek tık (veya oyun açılınca otomatik): güç planını "Yüksek performans"a al, kulaklığa geç, bildirimleri sustur, seçtiğin arka plan uygulamalarını (ör. tarayıcı) kapat, GMID animasyonlarını durdur. Oyun kapanınca her şeyi geri al. | Orta | ⭐ |
| 2 | **Mikrofonu sessize al kısayolu** | Her yerden çalışan tek tuş (ör. `Pause`), ekranın köşesinde küçük 🎙️/🔇 göstergesi. Discord/oyun fark etmeksizin mikrofonu Windows seviyesinde kapatır. | Kolay | ⭐ |
| 3 | **Otomatik kapak ve oyun bilgisi** | Kütüphanedeki oyunlara SteamGridDB'den şık kapak/logo, türü ve çıkış yılını otomatik getirir. Kütüphane "Excel tablosu" gibi değil, vitrin gibi görünür. | Kolay | ⭐ |
| 4 | **Ekran görüntüsü ve klip düzenleyici** | Steam / NVIDIA / Xbox Game Bar'ın her yere dağıttığı ekran görüntüsü ve klipleri `GMID\Galeri\<Oyun adı>\` altına toplar, galeri olarak gösterir. ("Derli toplu" ilkesine birebir uyuyor.) | Kolay | ⭐ |
| 5 | **Birleşik oyun başlatıcı** | Steam/Epic/EA oyunlarını GMID kütüphanesinden doğrudan başlat; hangi platformda olduğunu düşünmene gerek kalmaz. | Orta | 👍 |
| 6 | **Ücretsiz oyun bildirimi** | Epic'in haftalık ücretsiz oyunu ve Steam'deki geçici ücretsiz oyunlar çıkınca Windows bildirimi. | Kolay | 👍 |
| 7 | **İstek listesi + indirim takibi** | İstediğin oyunu listeye ekle, hedef fiyat gir; o fiyatın altına düşünce bildirim (IsThereAnyDeal). | Kolay–Orta | 👍 |
| 8 | **Oyun süresi sınırı / mola hatırlatıcı** | "Bugün en fazla 3 saat" veya "her 2 saatte bir mola ver" → oyun algılandığında sayar, dolunca bildirim. Sayaç modülüyle birlikte. | Kolay | 👍 |
| 9 | **Komut paleti (`Ctrl+K`)** | Her şeyi tek arama kutusundan yap: "kulaklık", "not: market listesi", "hatırlat 20dk çay", "elden ring"… Anlaşılabilirlik ilkesine çok uygun. | Orta | 👍 |
| 10 | **Oyuna bağlı notlar** | Bir oyunun kartında ona ait notlar: build'ler, şifreler, nerede kaldım, yapılacak görevler. | Kolay | 👍 |
| 11 | **Yıllık özet ("GMID Wrapped")** | Yıl sonunda: en çok oynanan oyunlar, toplam saat, en yüksek puanlar, izlenen filmler — paylaşılabilir bir görsel kart. | Kolay | 👍 |
| 12 | **Discord Rich Presence** | Discord profilinde "GMID'de: Elden Ring değerlendiriyor" gibi durum. Online sürüme geçince tanıtım için de iyi. | Kolay | 🤔 |
| 13 | **FPS / donanım izleme penceresi** | CPU/GPU sıcaklık, kullanım, FPS. Sıcaklık sensörleri için gereken sürücü bazı anti-cheat'lerle (FACEIT vb.) sorun çıkarabiliyor; MSI Afterburner zaten bu işi iyi yapıyor. | Orta–Zor | 🤔 |
| 14 | **Sürücü güncelleme hatırlatıcısı** | "Ekran kartı sürücünü 60 gündür güncellemedin." | Orta | 🤔 |
| 15 | Pano geçmişi / ekran görüntüsü aracı | Windows'ta zaten var (`Win+V`, `Win+Shift+S`). | Kolay | ❌ |
| 16 | Makro, otomatik tıklama, nişangâh (crosshair), oyun içi overlay | Online oyunlarda **ban riski** (Vanguard, EasyAntiCheat, BattlEye). Uygulamanın güvenilirliğini tehlikeye atar. | — | ❌ |

> **Güvenlik kuralı (tüm uygulama için):** GMID hiçbir oyunun belleğini okumaz, oyuna kod enjekte
> etmez, oyunun üstüne çizim yapmaz. Sadece "hangi program çalışıyor" bilgisini okur — Discord'un
> yaptığı kadar. Böylece anti-cheat sistemleriyle hiçbir sorun yaşanmaz.

---

## 5. Teknik mimari

### 5.1 Teknoloji seçimi (önerim)

| Katman | Seçim | Neden? |
|--------|-------|--------|
| Uygulama çatısı | **Tauri 2** | Electron'a göre ~10 kat küçük kurulum ve çok daha az RAM; Windows'un derin özelliklerine (ses, süreçler, bildirimler) doğrudan erişim. |
| Arka uç (çekirdek) | **Rust** | Hızlı, az bellek, çökmeye dayanıklı. Oyun algılama, ses değiştirme, indirme yönetimi burada. |
| Arayüz | **Svelte 5 + TypeScript** | React'ten daha hafif ve hızlı; animasyonlu arka plan, cam efektleri, sürükle-bırak kolay. |
| Veritabanı | **SQLite** (tek dosya) | Kurulum gerektirmez, hızlı, yedeklemesi = dosya kopyalamak. |
| Video | **yt-dlp + ffmpeg + deno** | Sektör standardı, ücretsiz, açık kaynak. Uygulamayla birlikte gelir, kendini günceller. |
| Uygulama kurulumu | **winget** | Windows 10/11'de yerleşik, Microsoft'un resmî paket yöneticisi. |
| Tarayıcı eklentisi | **Manifest V3** (Chrome + Opera aynı kod) | Chrome'un güncel eklenti standardı. |

Alternatifler (Electron, C# WPF, Python) karşılaştırması: [ARASTIRMA-TEKNIK.md](ARASTIRMA-TEKNIK.md) §7.
Eğer C# biliyorsan ve Rust öğrenmek istemiyorsan **WPF** de iyi bir seçenek; kodu ben yazacağım için
bu bir engel değil ama bakımını kendin de yapmak istersen önemli (bkz. açık sorular).

### 5.2 Genel yapı

```
┌──────────── Chrome / Opera ────────────┐
│  GMID Eklentisi (videoyu fark eder,     │
│  "İndir" düğmesi + kalite menüsü)       │
└───────────────────┬─────────────────────┘
                    │ Native Messaging (port açmadan, güvenli)
┌───────────────────▼─────────────────────────────────────────────┐
│ GMID Masaüstü Uygulaması (Tauri 2)                                │
│                                                                   │
│  ┌─ Arayüz (sadece pencere açıkken yaşar) ────────────────────┐  │
│  │  Menü · Sekmeler · Arka plan · Hızlı not penceresi          │  │
│  └──────────────────────────┬──────────────────────────────────┘  │
│                  komutlar ⇅ olaylar                               │
│  ┌─ Çekirdek (Rust, her zaman çalışır, tepside) ───────────────┐  │
│  │  Oyun Algılayıcı   → süreç listesi (5 sn), tam ekran tespiti │  │
│  │  Zamanlayıcı       → hatırlatıcılar, sayaçlar, bildirimler   │  │
│  │  İndirme Yöneticisi→ yt-dlp / ffmpeg / deno kuyruğu          │  │
│  │  Ses Servisi       → cihaz listesi, varsayılan değiştirme    │  │
│  │  Kurulum Servisi   → winget                                  │  │
│  │  Depo Katmanı      → SQLite + dosya klasörleri               │  │
│  │  Tepsi ikonu · Genel kısayollar · Windows bildirimleri       │  │
│  └──────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────┘
```

### 5.3 Optimizasyon stratejisi

- **Pencere kapatılınca arayüz tamamen kapanır**, sadece küçük Rust çekirdeği tepside kalır
  (tahmini **15–30 MB RAM**). Pencereyi açınca arayüz yeniden yüklenir (< 1 sn).
- **Oyun açıkken "sessiz mod":** Animasyonlar durur, arka plan görevleri (kapak indirme, güncelleme kontrolü) oyun bitene kadar ertelenir.
- **Tembel yükleme:** Her sekme ancak açıldığında yüklenir; kullanmadığın modül hiç kaynak harcamaz.
- **Modül aç/kapa:** Ayarlardan istemediğin modülü (ör. Video İndirici) tamamen kapatabilirsin.
- **Ölçülebilir hedefler** (her sürümde kontrol edilecek):

  | Durum | RAM | CPU |
  |-------|-----|-----|
  | Sadece tepside | < 30 MB | ~%0 |
  | Pencere açık, sabit arka plan | < 150 MB | < %1 |
  | Pencere açık, video arka plan | < 200 MB | < %3 (GPU ile çözülür) |
  | Oyun oynanırken | < 30 MB | ~%0 |

### 5.4 Arayüz ilkeleri

- Koyu tema varsayılan, açık tema seçeneği. Yuvarlatılmış köşeler, "cam" (yarı saydam bulanık) paneller → arka plan resmi görünür ama yazılar okunur.
- Her ekranda tek bir ana eylem düğmesi (ör. Notlar'da "+ Yeni not").
- Boş ekranlar boş kalmaz: "Henüz notun yok — `Ctrl+Alt+N` ile her yerden hızlı not alabilirsin" gibi yol gösterici mesajlar.
- Klavye dostu: tüm ana işlemlerin kısayolu var, `Ctrl+K` ile komut paleti (öneri #9).
- Silinen her şey önce çöp kutusuna gider; "Geri al" bildirimi.

---

## 6. Veri klasörü düzeni (derli toplu yapı)

Varsayılan konum `Belgeler\GMID\` (ayarlardan başka diske, ör. `D:\GMID\` taşınabilir).
Format atarken **bu tek klasörü yedeklemen yeterli.**

```
GMID\
├─ gmid.db                ← Tek SQLite veritabanı (oyunlar, puanlar, notlar, hatırlatıcılar, ayarlar)
├─ Videolar\
│  ├─ YouTube\
│  ├─ Twitter\
│  ├─ Instagram\
│  └─ Diger\
├─ Notlar\                ← Notların .md kopyaları (uygulama olmadan da okunabilir)
├─ Arkaplanlar\
├─ Kapaklar\              ← Oyun/film kapak resimleri önbelleği (silinirse yeniden indirilir)
├─ Galeri\               ← (öneri #4) Oyun ekran görüntüleri ve klipler, oyun adına göre
├─ Kurulum\               ← Format sonrası kurulum listeleri + dışa aktarılan .bat/.ps1
├─ Yedekler\              ← Veritabanının otomatik günlük yedekleri (son 7 gün)
└─ Araclar\               ← yt-dlp.exe, ffmpeg.exe, deno.exe (uygulama kendisi günceller)
```

---

## 7. Veritabanı taslağı

İlk sürüm için kabaca tablolar (kodlama aşamasında kesinleşecek):

| Tablo | Temel alanlar |
|-------|---------------|
| `games` | id, ad, platform (steam/epic/ea/manual), platform_id, exe_adlari, kapak, tür, eklenme |
| `play_sessions` | id, game_id, başlangıç, bitiş, süre_sn, kaynak (canlı/içe aktarma) |
| `game_reviews` | game_id, durum, ön_ayar, kriter_puanları (JSON), hesaplanan_puan, son_puan, tekrar_oynar, yorum, bitirme_tarihi |
| `movies` / `movie_reviews` | id, ad, yıl, tür, afiş · kriter_puanları, puanlar, izleme_tarihi, yorum |
| `rating_presets` | ad, tür (oyun/film), kriter→ağırlık (JSON) |
| `downloads` | id, url, site, başlık, kalite, dosya_yolu, boyut, durum, tarih |
| `notes` | id, başlık, içerik, renk, yazı_tipi, sabit, klasör, etiketler, güncellenme |
| `reminders` | id, başlık, zaman, tekrar_kuralı, ertelenen, tamamlandı |
| `audio_devices` | windows_id, takma_ad, ikon, gizli, sıra |
| `install_lists` / `install_items` | liste adı · winget_id, ad, kategori, sıra |
| `settings` | anahtar, değer |

---

## 8. Gelecek: çevrim içi sürüm

Şu an **hiçbir çevrim içi özellik yok**, ama mimari buna hazır olacak:

- Uygulamanın içi baştan **"istemci ↔ servis"** mantığıyla bölünecek; çevrim içi geldiğinde sadece yeni bir "sunucu" servisi eklenecek.
- VDS'e kurulacak küçük bir sunucu (WebSocket) ile: **arkadaş listesi, "şu an ne oynuyor" durumu, sohbet, puan/inceleme paylaşımı, ortak oyun listeleri**.
- Hesap sistemi gerektiği için en başta değil, uygulama beğenilip Discord'da yayılırsa eklenecek.
- Yerel veriler her zaman yerelde kalır; neyin paylaşılacağını kullanıcı seçer.

---

## 9. Yol haritası

Mantık: **önce her gün kullanacağın, kolay ve hızlı biten modüller** → uygulamayı erkenden
kullanmaya başlarsın, eksikleri yaşarken görürsün. Zor modüller (video indirici) sonraya.

| Faz | İçerik | Sonunda elinde ne olacak? |
|-----|--------|---------------------------|
| **0 — Tasarım** *(şu an)* | Bu doküman, kararlar, arayüz taslakları (wireframe) | Onaylanmış plan |
| **1 — İskelet** | Tauri projesi, sol menü, Ayarlar, veri klasörü, SQLite, tepsi ikonu, Windows ile başlatma, arka plan (resim/GIF/video), otomatik yedek | Boş ama şık, açılıp kapanan, tepside duran uygulama |
| **2 — Günlük araçlar** | Ses Cihazları (+ mikrofon sessize alma), Notlar (+ hızlı not penceresi, yapışkan not), Hatırlatıcılar, Sayaç | Her gün kullandığın 4 araç |
| **3 — Oyun** | Oyun algılama, Steam/Epic/EA içe aktarma, kütüphane, oturum geçmişi, kapaklar, oyun değerlendirme | Discord gibi oyun yakalayan, süre tutan kütüphane |
| **4 — Video İndirici** | Masaüstü indirme yöneticisi, Videolar sekmesi, Chrome/Opera eklentisi | IDM benzeri indirici |
| **5 — Eğlence ve kurulum** | Film & dizi değerlendirme, format sonrası kurulum + dışa aktarılan betik | Tüm istenen modüller tamam (**v1.0**) |
| **6 — Öneriler** | Onayladığın ek özellikler (Oyun Modu, galeri, ücretsiz oyunlar, indirim takibi…) | v1.x |
| **7 — Çevrim içi** *(beğenirsen)* | Sunucu (VDS), hesaplar, arkadaşlar, sohbet | v2.0 |

Her faz sonunda: çalışan bir sürüm (`.exe` kurulum dosyası), kısa değişiklik notu, performans ölçümü.

---

## 10. Riskler ve yasal notlar

| Risk | Etkisi | Önlem |
|------|--------|-------|
| YouTube/Instagram/X yapılarını değiştirir, indirme bozulur | Video indirici geçici çalışmaz | yt-dlp otomatik güncellenir; sorun olursa "Araçları güncelle" düğmesi |
| Eklenti Chrome Web Store'a konamaz | Mağazadan kurulamaz | Geliştirici modunda paketlenmemiş kurulum (kişisel kullanım için sorun yok) |
| YouTube kullanım şartları indirmeyi yasaklıyor | Kişisel kullanımda düşük risk; **herkese dağıtımda yüksek risk** | Online sürüme geçilirse video indirici ayrı/isteğe bağlı bir eklenti olarak tutulur |
| Ses cihazı değiştirme belgelenmemiş Windows arayüzüne dayanıyor | Bir Windows güncellemesi bozabilir (düşük olasılık) | SoundSwitch gibi projeleri takip; bozulursa Windows ses ayarlarını açan yedek düğme |
| Discord oyun listesi / Epic / EA verileri belgelenmemiş | Bazı oyunlar algılanmayabilir | Elle oyun ekleme + "çalışan programlardan ekle" düğmesi her zaman var |
| Anti-cheat sistemleri | Ban riski | Oyunlara enjeksiyon, bellek okuma, overlay **yok** (Bölüm 4 güvenlik kuralı) |
| Veri kaybı | Notlar, puanlar kaybolur | Günlük otomatik yedek (son 7 gün) + tek klasör yapısı + notların .md kopyası |
| Çerezlerin (Instagram/X) uygulamaya geçmesi | Hassas veri | Sadece ilgili site, sadece indirme sırasında, iş bitince silinir; diske kalıcı yazılmaz |

---

## 11. Kesinleşen kararlar ve açık sorular

### ✅ Kesinleşenler
- Platform: Windows 10/11, tek kullanıcı, yerel.
- İlkeler: optimizasyon > anlaşılabilirlik > derli toplu.
- Tüm uygulama verisi tek bir ana klasörde.
- Video indirici bir tarayıcı eklentisi + masaüstü uygulaması ikilisi olacak (Chrome öncelikli, Opera aynı eklentiyle).
- Kodlamaya bu doküman onaylandıktan sonra geçilecek.

### ❓ Açık sorular (cevaplarını bekliyorum)
**Genel**
1. **Programlama deneyimin var mı?** (Hangi dil?) Kodu ben yazacağım ama ileride kendin de değiştirmek istersen teknoloji seçimini etkiler (Tauri/Rust önerim, alternatif C#/WPF).
2. **Uygulamanın adı "GMID" mi kalsın?** Açılımı nedir / logo, renk tercihin var mı?
3. **Görsel tarz:** Sevdiğin bir uygulamanın görünümü var mı? (Discord, Steam, Spotify, Notion…) Koyu "gamer" tarzı mı, sade/minimal mi?
4. **Arayüz dili:** Sadece Türkçe mi, ileride İngilizce de olsun mu? (Online sürüm düşünülüyorsa baştan çok dilli altyapı kurmak ucuz.)
5. **Veri klasörü:** `Belgeler\GMID` mi, yoksa ayrı bir disk (ör. `D:\GMID`) mi? (Format sonrası kaybolmaması için D: daha mantıklı olabilir.)

**Modüller**
6. Oyun değerlendirme "Eğlence" altında mı, oyun kartının içinde mi? (Önerim: ikisinden de ulaşılsın.) — §2
7. Film yanında **dizi** de değerlendirilsin mi? — §3.4
8. Puan ölçeği **1–10** mu (önerim), 100'lük mü, 5 yıldız mı? Ağırlıkları değiştirebilmek ister misin? — §3.3
9. Puanlama kriterleri ve yüzdeleri sana mantıklı geldi mi? Eklemek/çıkarmak istediğin kriter var mı? — §3.3, [Ek A](ARASTIRMA-PUANLAMA.md)
10. **Steam Web API anahtarı** almak ister misin? AFK süresi oyun süresine sayılsın mı? — §3.2
11. Video indirmede varsayılan kalite "her seferinde sor" mu, "hep en iyisi" mi? Videonun üstüne düğme eklensin mi? — §3.1
12. Notlar ayrıca **.md dosyası** olarak da dursun mu? — §3.5
13. Format sonrası kurulum listene kesin girecek programlar neler? (Chrome, Steam, Discord, Claude, WinRAR… başka?) — §3.9

**Öneriler**
14. Bölüm 4'teki önerilerden hangilerini istiyorsun? (⭐ olanlar benim önerim.)
15. Aklına gelen başka özellik var mı? 🙂

---

### Değişiklik geçmişi
| Sürüm | Tarih | Değişiklik |
|-------|-------|------------|
| v0.1 | 2026-10-10 | İlk taslak: istekler + araştırma sonuçları + öneriler |
