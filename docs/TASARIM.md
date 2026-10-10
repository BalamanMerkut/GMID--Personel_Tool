# GMID Tool — Game Mansion In Discord
## Tasarım Dokümanı · v0.3 (taslak)

> **Durum:** Fikir / karar aşaması. Kod yazılmadı.
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

---

## 1. Vizyon ve temel ilkeler

### 1.1 İsim ve hikâye

**GMID = Game Mansion In Discord.** Discord'da kurulmuş bir oyun malikânesi; Butler'ı, yapay zekâ
botu, müzik botu olan kendi oyun evrenimiz. **GMID Tool** da bu malikânenin **alet çantası**:
malikâne sakinlerinin bilgisayarında duran; oyunlarını, film-dizilerini, notlarını, kliplerini,
indirdiği videoları ve Windows'taki küçük can sıkıcı işleri tek bir şık pencerede toplayan uygulama.

### 1.2 Temel ilkeler

| İlke | Pratikte ne demek? |
|------|--------------------|
| **Optimizasyon** | Tepside beklerken ve oyun oynarken neredeyse sıfır kaynak (hedef: **< 30 MB RAM, ~%0 CPU**). Hızlı açılış (< 1 sn). |
| **Güzel görünüm** | Arka plan resmi/GIF, cam efektli paneller, oyun kapakları, yumuşak geçişler. Açtığında "vay" dedirtmeli. |
| **Anlaşılabilirlik** | Her ekran tek bir işi yapar. En fazla 2 tıkla her şeye ulaşılır. Türkçe arayüz, net ikonlar, boş ekranlarda "ne yapmalıyım" ipucu. Elle veri girmek en aza iner (bilgiler internetten otomatik gelir). |
| **Derli toplu** | Uygulamanın ürettiği **her şey tek bir ana klasörde** durur (videolar, klipler, notlar, arka planlar, veritabanı, yedekler). Hiçbir şey "İndirilenler"e ya da masaüstüne dağılmaz. |

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
- Ortak arşiv geldiğinde (§8) hiçbir şeyi yeniden yazmak gerekmeyecek; istenirse sunucusuz bir ara çözüm de var (öneri #12).

---

## 2. Uygulama yapısı (kategoriler ve sekmeler)

Sol tarafta daraltılabilir bir menü, sağda içerik:

```
🏠 Ana Sayfa             → Özet: bugün/bu hafta oynanan süre, son klipler, yaklaşan
                            hatırlatıcılar, son notlar, devam eden indirmeler

🎮 Oyun
   ├─ Kütüphanem         → Steam / Epic / EA / elle eklenen oyunlar, kapaklar, oynama süreleri
   ├─ Oturum Geçmişi     → Hangi gün hangi oyunu ne kadar oynadım (grafik)
   ├─ Galeri             → Ekran görüntüleri ve klipler (NVIDIA Instant Replay vb.), oyuna göre
   └─ Oyun Modu          → Tek tıkla "oyuna hazırlan" profili

🍿 Eğlence
   ├─ Oyun Arşivi        → Puanladığımız oyunlar, yorumlar, notlar + tür/etiket filtreleme
   ├─ Film & Dizi Arşivi → Aynısı film ve diziler için
   └─ Videolar           → İndirilen videolar (IDM benzeri indiricinin kütüphanesi)

🧰 Araçlar
   ├─ Notlar
   ├─ Hatırlatıcılar
   ├─ Sayaç              → Kronometre + geri sayım
   ├─ Ses & Mikrofon     → Kulaklık/hoparlör geçişi, mikrofon seviyesi kilidi, kısayollar
   └─ Format Sonrası Kurulum

⚙️ Ayarlar              → Görünüm (arka plan, kalite), kısayol tuşları, başlangıçta açılma,
                            veri klasörü, yedekleme, eklenti bağlantısı
```

**Pencereyi açmadan erişim:**
- **Oyun İçi Panel** (Steam'deki `Shift+Tab` gibi): oyundan çıkmadan açılan yarı saydam panel — bkz. §3.11.
- **Sistem tepsisi ikonu** (saatin yanı): sağ tık → ses cihazı, mikrofon, hızlı not, hatırlatıcı, sayaç.
- **Genel kısayol tuşları** — tam liste §3.8'de, hepsi ayarlardan değiştirilebilir.

✅ Oyun arşivine hem **Eğlence → Oyun Arşivi**'nden hem de **oyunun kütüphane kartından** ulaşılır (veri tek yerde).
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

**MVP**
- YouTube, Twitter/X, Instagram (ve yt-dlp'nin desteklediği yüzlerce site, ücretsiz bonus).
- Çözünürlük seçimi, sadece ses seçeneği, indirme kuyruğu, duraklat/iptal.
- Videolar sekmesi: küçük resim, başlık, kaynak site, süre, boyut, tarih; arama ve filtre; tek tıkla oynat / klasörde göster / sil.
- Dosyalar otomatik olarak siteye göre alt klasörlere: `Videolar\YouTube\`, `Videolar\Twitter\`…

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

❓ Varsayılan kalite: "her seferinde sor" mu, "hep en iyisi" mi? Videonun üstüne düğme eklensin mi? (soru 7)

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

**İki ayrı süre:** Steam'in tuttuğu süre, kart toplama / süre artırma (ör. Steam Card Idler) ile şişebilir.
Bu yüzden her oyunda iki sayı görünür: **"Gerçek oynama"** (GMID'in ölçtüğü, oyun gerçekten açıkken) ve
**"Steam'deki süre"**. İstatistikler ve Wrapped gerçek oynamayı kullanır.

**Teknik not:** Süreç kontrolü 5 sn aralıkla, sadece program adı karşılaştırmasıyla yapılır — işlemciye
etkisi ölçülemeyecek kadar az. Oyunların içine **hiçbir şey enjekte edilmez** (anti-cheat güvenliği).
Ayrıntılar: [Ek B](ARASTIRMA-TEKNIK.md) §2.

❓ Steam Web API anahtarı / AFK süresi (soru 6)

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

❓ Notlar .md dosyası olarak da dursun mu? (soru 8)

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

❓ Mikrofon kilidi için hedef seviye %100 mü? (soru 5)

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

❓ Listene kesin girecek programlar (soru 9)

---

### 3.10 Görünüm ve Arka Plan

**MVP**
- Arka plan: **düz renk / degrade / resim (JPG, PNG, WebP) / hareketli (GIF, WebM, MP4)**.
- Bulanıklık ve karartma kaydırıcıları (yazılar hep okunur kalsın diye), cam efektli paneller.
- Vurgu rengi; koyu tema varsayılan, açık tema seçeneği.
- Arka planlar `Arkaplanlar\` klasörüne kopyalanır (orijinal silinse de bozulmaz).
- Notlar ve hızlı not penceresi için ayrı arka plan seçimi.
- **Görünüm kalitesi:** Otomatik / Yüksek / Hafif (§1.3).

**Optimizasyon (denge kuralı gereği):**
- Pencere küçültülünce / arkada kalınca / **bir oyun algılanınca hareketli arka plan otomatik durur**.
- GIF seçersen, GMID onu arka planda aynı görünen ama 5–10 kat daha hafif bir videoya çevirir (görüntü aynı, yük daha az).

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

### 3.12 Galeri — Ekran Görüntüleri ve Klipler ✅

**Amaç:** NVIDIA Instant Replay, Xbox Game Bar, Steam vb.'nin her yere dağıttığı klip ve ekran
görüntülerini **tek yerde, oyuna göre düzenli** toplamak.

**Nasıl çalışır?**
1. GMID, kayıt programlarının klasörlerini sessizce izler (NVIDIA App / Instant Replay, Xbox Game Bar, Steam, AMD, OBS, Medal…).
2. Yeni bir klip kaydedilince (ör. NVIDIA'da `Alt+F10`), dosyanın yazılması bitene kadar bekler.
3. **Hangi oyunda çekildiğini bilir** — çünkü o an hangi oyunun açık olduğunu zaten takip ediyor (§3.2).
   Böylece "Desktop" ya da garip isimli klasörlere düşen klipler bile doğru oyuna gider.
4. Dosyayı `Galeri\<Oyun adı>\Klipler\` veya `...\Ekran Goruntuleri\` altına taşır
   (Steam ekran görüntüleri Steam'in kendi yükleme özelliği bozulmasın diye **kopyalanır**).
5. Küçük bir gösterge: "🎬 Klip kaydedildi — Valorant (0:30)".

**Galeri ekranı (MVP)**
- Oyuna göre gruplanmış küçük resimler, tarih sırası; tıklayınca oynatıcı.
- Favori ⭐, sil, yeniden adlandır, klasörde göster.
- **"Discord'a sığdır"** düğmesi: Klibi Discord'un ücretsiz yükleme sınırına (10 MB) sığacak şekilde sıkıştırır, panoya kopyalar → Discord'a direkt yapıştırırsın.
- Disk koruması: "Favori olmayan, X günden eski klipleri sil" seçeneği (varsayılan kapalı); Galeri başka bir diske taşınabilir.

**Sonra:** Klip kırpma (başlangıç–bitiş), birden fazla klibi birleştirme, GIF'e çevirme.

> **Bilmen gereken:** NVIDIA App ve Medal kendi galerilerini tutuyor. Klipleri GMID'e taşırsak, onlar
> **GMID galerisinde** görünür ama **NVIDIA'nın kendi galerisinde (Alt+Z) artık görünmez**. NVIDIA'da
> kayıt klasörünü bir kez GMID klasörü olarak seçmek de mümkün (GMID adım adım gösterir).
> Ayrıntılar: [Ek C](ARASTIRMA-OVERLAY-METADATA.md) §4.

✅ **Karar: Klipler GMID klasörüne taşınacak** (Steam ekran görüntüleri hariç — onlar kopyalanır).
İlk kurulumda GMID, NVIDIA'nın kayıt klasörünü de doğrudan `GMID Tool\Galeri\` yapmayı adım adım gösterecek.

---

## 4. Ek özellikler (öneriler)

**Durum:** ✅ = onaylandı · ⭐ = önerim · 👍 = eklenirse güzel olur · 🤔 = sonra düşünürüz · ❌ = eklemeyelim

**Onaylananlar:** Oyun Modu, mikrofon kısayolları (+ seviye kilidi), Oyun İçi Panel, otomatik kapak ve bilgi, Galeri.

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
| 9 | **Butler entegrasyonu** *(çevrim içi fazında)* | Discord'da `/oyun-oner roguelike` → Butler, GMID arşivinden puanlarımızla cevap verir; biri yeni oyun puanlayınca kanala duyuru. | Orta | ⭐ |
| 10 | **FPS / donanım izleme** | CPU/GPU sıcaklık, FPS. Sıcaklık sürücüsü bazı anti-cheat'lerle sorun çıkarabiliyor; MSI Afterburner zaten iyi yapıyor. | Orta–Zor | 🤔 |
| 11 | **Sürücü güncelleme hatırlatıcısı** | "Ekran kartı sürücünü 60 gündür güncellemedin." | Orta | 🤔 |
| 12 | **Arşiv paylaşım dosyası** | Sunucu olmadan arkadaş puanları: puan ve yorumlarını tek bir dosya olarak dışa aktar, Discord'dan arkadaşına at; o içe aktarınca kartlarda "Ben 9 · Ali 7" görünür. VDS gelene kadar köprü. | Kolay | ⭐ |
| 13 | **Butler'ın ağzından bildirimler** | İsteğe bağlı tema: "Efendim, çay vaktiniz geldi.", "Efendim, mikrofonunuz kapatıldı." — malikâne havası. Kapalıyken düz bildirimler. | Kolay | 👍 |
| 14 | **Steam Kart Toplayıcı modülü** | Senin Steam Card Idler'ını GMID Tool'un içine almak (§8.2). | Orta | 🤔 |
| 15 | Pano geçmişi / ekran görüntüsü aracı | Windows'ta zaten var (`Win+V`, `Win+Shift+S`). | — | ❌ |
| 16 | Makro, otomatik tıklama, nişangâh (crosshair), oyunun **içine** çizim yapan overlay | Online oyunlarda **ban riski**. | — | ❌ |

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
│  │  Galeri İzleyici    → kayıt klasörlerini izler, klipleri taşır  │  │
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

- **Malikâne kimliği:** Koyu, şık bir tema; GMID'in renkleri ve logosu (❓ soru 1). Yuvarlatılmış köşeler, cam paneller → arka plan görünür ama yazılar okunur.
- **Kapak odaklı:** Oyun ve film listeleri tablo değil, kapaklı kart vitrini.
- Her ekranda tek bir ana eylem düğmesi (ör. Notlar'da "+ Yeni not").
- Boş ekranlar boş kalmaz: "Henüz notun yok — `Ctrl+Alt+N` ile her yerden hızlı not alabilirsin" gibi yol gösterici mesajlar.
- Klavye dostu: ana işlemlerin kısayolu var.
- Silinen her şey önce çöp kutusuna gider; "Geri al" bildirimi.

---

## 6. Veri klasörü düzeni (derli toplu yapı)

Varsayılan konum `Belgeler\GMID Tool\` (ayarlardan başka diske, ör. `D:\GMID Tool\` taşınabilir).
Format atarken **bu tek klasörü yedeklemen yeterli.**

```
GMID Tool\
├─ gmid.db                  ← Tek SQLite veritabanı (oyunlar, puanlar, notlar, hatırlatıcılar, ayarlar)
├─ Videolar\                ← İndirilen videolar
│  ├─ YouTube\  ├─ Twitter\  ├─ Instagram\  └─ Diger\
├─ Galeri\                  ← Klipler ve ekran görüntüleri
│  └─ <Oyun adı>\
│     ├─ Klipler\
│     └─ Ekran Goruntuleri\
├─ Notlar\                  ← Notların .md kopyaları (uygulama olmadan da okunabilir)
├─ Arkaplanlar\
├─ Kapaklar\                ← Oyun/film kapak önbelleği (silinirse yeniden indirilir)
├─ Kurulum\                 ← Format sonrası kurulum listeleri + dışa aktarılan .bat
├─ Yedekler\                ← Veritabanının otomatik günlük yedekleri (son 7 gün)
└─ Araclar\                 ← yt-dlp, ffmpeg, deno (uygulama kendisi günceller)
```

---

## 7. Veritabanı taslağı

İlk sürüm için kabaca tablolar (kodlama aşamasında kesinleşecek):

| Tablo | Temel alanlar |
|-------|---------------|
| `people` | id, ad, avatar — şimdilik sadece "ben"; ortak arşivde arkadaşlar |
| `games` | id, ad, kaynak (steam/igdb/rawg/manual), kaynak_id, exe_adları, kapak, geliştirici, yayıncı, çıkış_yılı, açıklama, oyun_modları |
| `tags` / `game_tags` | Türkçe etiket listesi (Roguelike, Co-op…) · oyun↔etiket |
| `play_sessions` | id, game_id, başlangıç, bitiş, süre_sn, kaynak (canlı/içe aktarma) |
| `reviews` | id, yazar (people), konu (oyun/film/dizi + id), puan, soru_puanları (JSON: soru→puan veya "yok"), genel_puan, elle_düzeltme, tek_cümle, notlar, durum, tavsiye, tarih |
| `titles` | id, tmdb_id, tür (film/dizi), ad, orijinal_ad, yıl, afiş, özet, yönetmen, oyuncular, sezon_sayısı |
| `captures` | id, game_id, dosya_yolu, tür (klip/ekran görüntüsü), kaynak (nvidia/xbox/steam…), süre, boyut, favori, tarih |
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
- **Butler bağlantısı:** `/oyun-oner roguelike` → Butler arşivden puanlarımızla öneri yapar; biri yeni oyun puanlayınca kanala duyuru; "şu an kim ne oynuyor" panosu.
- Yerel veriler her zaman yerelde kalır; neyin paylaşılacağını herkes kendisi seçer.
- Sunucu gelene kadar arkadaş puanları için **paylaşım dosyası** (öneri #12) kullanılabilir.

### 8.2 GMID evreninin mevcut parçaları (kodları incelendi)

| Proje | Teknoloji | Ne yapıyor? | GMID Tool ile bağlantı |
|-------|-----------|-------------|------------------------|
| [GMID Butler](https://github.com/BalamanMerkut/GMID-Butler) | Python · discord.py · Gemini 2.5 Flash · SQLite · Render'da (Flask ile "uyanık tutma") | Asil uşak kişilikli yapay zekâ sohbeti, geçici özel kanallar, otomatik kanal temizliği, 8 dil | Ortak arşivden öneri ve duyurular (§8.1); uşak kişiliği GMID Tool bildirimlerinde de kullanılabilir (öneri #13) |
| [GMID Music Bot](https://github.com/BalamanMerkut/GMID-Discord-MusicBot) | Python · discord.py · yt-dlp · OpenAI (şarkı sözleri) · 8 dil | YouTube'dan müzik, düğmeli kontrol paneli, çalma listeleri | İleride Oyun İçi Panel'den bota "geç / durdur" komutu (çevrim içi fazında) |
| [Steam Card Idler](https://github.com/BalamanMerkut/Steam-Card-Idler) | C# · .NET 8 · WPF · SteamKit2 | Kart düşürme, süre artırma, VAC kontrolü, piyasa fiyatı, tepside çalışma | GMID Tool modülü olabilir (öneri #14) |

**Notlar:**
- Botlar sandığın kadar ilkel değil: düzenli dosya yapısı, çok dil desteği, SQLite kullanımı var. Toparlama zamanı geldiğinde iyi bir temel.
- **Bilmekte fayda var:** Müzik botu da GMID Tool'un video indiricisi gibi **yt-dlp** kullanıyor. yt-dlp, 2025 sonundan beri
  YouTube için ek olarak **deno** istiyor. Müzik botu bir gün YouTube'dan çalmayı bırakırsa sebebi büyük ihtimalle bu:
  bot sunucusuna deno kurulup yt-dlp güncellenmeli.
- **VDS'e geçildiğinde:** Butler, müzik botu ve GMID sunucusu aynı VDS'te çalışır; Render'daki "uyanık tutma" hilesine gerek kalmaz.
  Butler zaten Python + SQLite olduğu için GMID sunucusuna küçük bir API üzerinden kolayca bağlanır.
- **Steam Card Idler'ı GMID'e almak için** iki yol var:
  (a) GMID Tool, senin mevcut idler'ını yardımcı program olarak çalıştırıp yönetir (hızlı, kod tekrar yazılmaz);
  (b) Rust'a yeniden yazmak (Rust'ta SteamKit2 kadar olgun bir kütüphane olmadığı için zor).
  Önerim **(a)**. Her iki durumda Steam şifren GMID'de saklanmaz; kart toplama süreleri "gerçek oynama" sayılmaz (§3.2).
- Botların düzenlenmesi ayrı bir iş; bu dokümanın kapsamı dışında, zamanı gelince ele alınır.

---

## 9. Yol haritası

Mantık: **önce her gün kullanacağın, kolay ve hızlı biten modüller** → uygulamayı erkenden
kullanmaya başlarsın, eksikleri yaşarken görürsün. Zor modüller (video indirici) sonraya.

| Faz | İçerik | Sonunda elinde ne olacak? |
|-----|--------|---------------------------|
| **0 — Tasarım** *(şu an)* | Bu doküman, kararlar, ekran taslakları | Onaylanmış plan |
| **1 — İskelet** | Tauri projesi, sol menü, Ayarlar, veri klasörü, SQLite, tepsi ikonu, Windows ile başlatma, arka plan (resim/GIF/video), otomatik yedek | Boş ama şık, tepside duran uygulama |
| **2 — Günlük araçlar** | Ses & Mikrofon (kilit, kısayollar, ekran göstergesi), Notlar (+ hızlı not, yapışkan not), Hatırlatıcılar, Sayaç | Her gün kullandığın araçlar |
| **3 — Oyun** | Oyun algılama, Steam/Epic/EA içe aktarma, kütüphane, otomatik bilgi ve kapak, Oyun Arşivi (puan, yorum, keşfet), Galeri | Discord gibi oyun yakalayan, puanlayıp filtreleyebildiğin arşiv |
| **4 — Oyun içi deneyim** | Oyun İçi Panel, Oyun Modu | Oyundan çıkmadan kontrol |
| **5 — Video İndirici** | İndirme yöneticisi, Videolar sekmesi, Chrome/Opera eklentisi | IDM benzeri indirici |
| **6 — Film & Dizi + Kurulum** | Film & Dizi Arşivi, format sonrası kurulum + dışa aktarılan .bat | Tüm istenen modüller tamam (**v1.0**) |
| **7 — Öneriler** | Onaylanan ek özellikler (ücretsiz oyunlar, Wrapped, Rich Presence…) | v1.x |
| **8 — Çevrim içi** *(tarih belirsiz)* | Sunucu (VDS), ortak arşiv, arkadaşlar, sohbet, Butler entegrasyonu | v2.0 |

Her faz sonunda: çalışan bir sürüm (`.exe` kurulum dosyası), kısa değişiklik notu, performans ölçümü.
*(Sunucu beklemeden arkadaş puanları istenirse paylaşım dosyası — öneri #12 — Faz 7'de yapılabilir.)*

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
| Klipler diski doldurur | Disk alanı | Galeri başka diske taşınabilir; isteğe bağlı otomatik temizlik |
| Anti-cheat sistemleri | Ban riski | Oyunlara enjeksiyon, bellek okuma, oyun içine çizim **yok** (§4 güvenlik kuralı) |
| Veri kaybı | Notlar, puanlar kaybolur | Günlük otomatik yedek (son 7 gün) + tek klasör + notların .md kopyası |
| Çerezlerin (Instagram/X) uygulamaya geçmesi | Hassas veri | Sadece ilgili site, sadece indirme sırasında, iş bitince silinir |

---

## 11. Kesinleşen kararlar ve açık sorular

### ✅ Kesinleşenler
- **İsim:** GMID Tool — *Game Mansion In Discord*'un alet çantası.
- **Kapsam:** Windows 10/11, yerel; başta sadece sen. Veri modeli baştan çok kişili; sunucu (VDS) tarihi belirsiz (§1.4, §8).
- İlkeler: optimizasyon, güzel görünüm, anlaşılabilirlik, derli toplu — **denge kuralı** (§1.3).
- Teknoloji: Tauri 2 + Rust + Svelte + SQLite (kodlama tamamen benden).
- Tüm uygulama verisi tek bir ana klasörde.
- **Puanlama: 5 kısa soru** (oyunda: oynanış, hikâye, görsellik & ses, performans, keyif) → otomatik genel puan + tek cümle + notlar + durum + tavsiye (§3.3).
- **Oyun bilgileri (tür, etiket, geliştirici, yayıncı, yıl, kapak) internetten otomatik**; tür/etiket ile filtrelenebilir Keşfet ekranı.
- Film'in yanında **dizi** de var; bilgiler TMDB'den otomatik, kendi 5 sorusu var.
- Oyun arşivine hem Eğlence menüsünden hem oyun kartından ulaşılır.
- Onaylanan ekler: **Oyun Modu, mikrofon kısayolları + seviye kilidi, Oyun İçi Panel, otomatik kapak, Galeri (NVIDIA Instant Replay vb.)**.
- **Galeri:** Klipler GMID klasörüne taşınır (Steam ekran görüntüleri kopyalanır).
- **Oyun İçi Panel:** Çerçevesiz oyunlar ana hedef; tam ekranda kısayollar + sesli geri bildirim.
- Video indirici: tarayıcı eklentisi + masaüstü ikilisi (Chrome öncelikli, Opera aynı eklentiyle).
- Kodlamaya bu doküman onaylandıktan sonra geçilecek.

### ❓ Açık sorular
**Görünüm ve genel**
1. **Görsel kimlik:** GMID'in bir logosu, renkleri var mı? (Discord sunucusunun ikonu/renkleri de olur.) Menülerde malikâne temalı isimler ister misin
   (ör. "Oyun Salonu", "Sinema Odası")? Önerim: anlaşılırlık için düz isimler + malikâne temalı ikon ve renkler.
2. Arayüz sadece Türkçe mi olsun, ileride İngilizce de gerekir mi? (Botların 8 dili var; arkadaşların hepsi Türkçe mi?)
3. Veri klasörü: `Belgeler\GMID Tool` mu, yoksa ayrı bir disk (ör. `D:\GMID Tool`) mi? (Format sonrası kaybolmaması için D: daha güvenli.)

**Modüller**
4. Puanlama soruları (§3.3 ve §3.4) ve yüzdeleri içine sindi mi? Değiştirmek istediğin soru var mı?
5. Mikrofon kilidi için hedef seviye %100 mü?
6. Steam Web API anahtarı almak ister misin? (Ücretsiz, 1 dakika; silinen oyunların sürelerini de getirir.) AFK süresi oyun süresine sayılsın mı?
7. Video indirmede varsayılan kalite "her seferinde sor" mu, "hep en iyisi" mi? Videonun üstüne düğme eklensin mi?
8. Notlar ayrıca **.md dosyası** olarak da dursun mu? (Önerim: evet.)
9. Format sonrası kurulum listene kesin girecek programlar neler? (Chrome, Steam, Discord, Claude, WinRAR… başka?)

**Öneriler**
10. §4'teki öneriler (#2–#14) arasından istediklerin hangileri? Özellikle: **paylaşım dosyası (#12)**, **Butler'ın ağzından bildirimler (#13)**, **Steam Kart Toplayıcı modülü (#14)**.
11. Aklına gelen başka özellik var mı? 🙂

---

### Değişiklik geçmişi
| Sürüm | Tarih | Değişiklik |
|-------|-------|------------|
| v0.1 | 2026-10-10 | İlk taslak: istekler + araştırma sonuçları + öneriler |
| v0.3 | 2026-10-10 | Puanlama 5 kısa soruya çevrildi (oyun ve film); kapsam: önce yerel, sunucu tarihi belirsiz; Galeri taşıma kararı; tam ekran oyunlar için sesli geri bildirim; "gerçek oynama" ve "Steam süresi" ayrımı; GMID evreninin mevcut parçaları (Butler, Music Bot, Card Idler) incelendi; yeni öneriler: paylaşım dosyası, Butler'ın ağzından bildirimler, Steam Kart Toplayıcı |
| v0.2 | 2026-10-10 | İsim ve hikâye (Game Mansion In Discord); denge kuralı; sade puanlama + otomatik oyun bilgisi + Keşfet ekranı; dizi eklendi; Ses & Mikrofon (seviye kilidi, kısayollar, ekran göstergesi); yeni modüller: Oyun İçi Panel, Galeri; Oyun Modu onaylandı; teknoloji seçimi kesinleşti; Butler entegrasyonu fikri; yol haritası güncellendi |
