# GMID Tool

*Merkut Studios*

**Game Mansion In Discord**'un alet çantası: oyunlarını, film-dizilerini, notlarını, kliplerini,
indirdiğin videoları ve Windows'taki küçük can sıkıcı işleri tek bir şık pencerede toplayan
kişisel Windows uygulaması.

> **Durum:** Faz 1 (İskelet), Faz 2 (Günlük araçlar), Faz 3a (Sunucular) ve Faz 5 (Video İndirici) kodlandı; Faz 3b (Oyun) sürüyor — Kütüphanem, Oturum Geçmişi ve Oyun Arşivi hazır. Sonraki fazlar için bkz. [Yol haritası](docs/TASARIM.md#9-yol-haritası).

## Kurulum (Windows 10/11)

1. GitHub'da bu deponun **Actions** sekmesine gir, en son başarılı **Derleme** çalışmasını aç.
2. Sayfanın altındaki **Artifacts** bölümünden `GMID-Tool-Kurulum` dosyasını indir (zip içinde `.exe` var).
3. `GMID Tool_x.y.z_x64-setup.exe` dosyasını çalıştır. Yönetici izni istemez; sadece senin kullanıcına kurulur.

Windows SmartScreen "tanınmayan uygulama" uyarısı verebilir (uygulama imzalı değil):
**Ek bilgi → Yine de çalıştır**.

## Neler var?

### Faz 3b — Oyun (1. bölüm: Kütüphanem, Oturum Geçmişi)

- **Oyunları tara:** Steam ve Epic'te kurulu oyunlar (Steam'in tuttuğu süreyle) bulunur; istediklerini seçip eklersin.
  Tanınmayan oyunlar: **Oyun ekle** → çalışan uygulamalardan seç ya da .exe göster.
- **Canlı algılama (Discord gibi):** 5 saniyede bir çalışan programlara bakılır; kütüphanedeki bir oyun açılınca
  oturum başlar, kapanınca biter. Oyuna hiçbir şey enjekte edilmez (anti-cheat güvenli). 30 saniyeden kısa açılışlar
  sayılmaz. Kontrol Odası'ndan kapatılabilir.
- **Kütüphanem:** kapak ızgarası (kapak ve bilgiler Steam mağazasından kendiliğinden, Türkçe), arama, kaynak süzgeci,
  sıralama; oyun penceresinde **Oyna**, klasörü aç, **Gerçek oynama** (GMID'in ölçtüğü) ve **Steam'deki süre** ayrı.
- **Oturum Geçmişi:** son 7/30 gün, günlük oynama grafiği (en çok oynanan 5 oyun renkli), gün gün oturum listesi.

### Faz 3b — Oyun (2. bölüm: Oyun Arşivi)

- **5 kısa soruyla puanla:** oynanış, hikâye, keyif, görsellik, performans (kaydırıcılar 7'den başlar). Hikâyesi olmayan
  oyunda **Yok** de: puanı düşürmez. Genel puan kendiliğinden hesaplanır; içine sinmezse ±1 oynatabilirsin. Tek cümle,
  durum (Oynuyorum, Bitirdim, %100, Bıraktım, İstiyorum), tavsiye ve notlar (`||spoiler||` gizlenir).
- **Keşfet:** sağdaki filtre kutusundan tür ve etiket (Roguelike, Co-op, Açık dünya… Steam'den Türkçe), puan, durum,
  tavsiye, mağaza. Her seçeneğin yanında kaç oyun kalacağı yazar. **🎲 Bana bir oyun seç** filtreye uyanlardan birini önerir.
- Kütüphanende olmayan oyunlar (konsolda oynadığın, sildiğin) Steam'de adıyla aratılıp eklenir. Son oynayıp
  puanlamadığın oyunlar arşivin üstünde hatırlatılır.

### Faz 5 — Video İndirici

- **Bağlantı yapıştır → kaliteyi seç → indir:** YouTube, X (Twitter), Instagram, TikTok ve yt-dlp'nin desteklediği
  yüzlerce site. Her kalitenin tahmini boyutu görünür; 1080p önerilir; "Sadece ses (MP3)" da var.
- **İndirmeler:** aynı anda 1–4 indirme, hız ve kalan süre, duraklat / devam ettir / iptal, hata olursa "Tekrar dene".
  Üst çubukta ve menüde her ekrandan görünen indirme göstergesi; bitince Windows bildirimi.
- **Videolarım:** küçük resimli kartlar, siteye göre sekmeler (YouTube · X · Instagram · Sesler · Diğer), arama,
  üzerine gelince önizleme, çift tıkla oynat, klasörde göster, sil. Dosyalar `Videolar\<Site>\` altında.
- **İlk kullanımda** yt-dlp, FFmpeg ve Deno (toplam ~150 MB) bir kez `Araclar\` klasörüne iner; yt-dlp kendini günceller.
- **Chrome / Opera eklentisi:** video sayfalarında "GMID ile indir" düğmesi, eklenti penceresinde kalite kartları,
  sağ tık menüsü. Kurulum: Video İndirici → Tarayıcı eklentisi → adımlar (klasörü hazırla → `chrome://extensions` →
  Geliştirici modu → Paketlenmemiş öğe yükle → 6 haneli kodla eşleştir).

### Video İndirici · çalma listeleri, her sitede yakalama, yazılar

- **Çalma listeleri:** YouTube listesi bağlantısı yapıştır (ya da videodaki "listenin tamamını getir"); videoları
  tek tek seç, kalite (1080p/720p/480p/MP3) seç, toplu indir. Dosyalar liste adıyla ayrı klasöre iner;
  Videolarım'da listeye göre süzgeç.
- **Eklenti her sitede:** yt-dlp'nin tanıdığı 1800+ site sayfa adresinden; tanınmayan sitelerde oynatıcının
  yüklediği video (mp4, m3u8, mpd) yakalanır ve sayfa bilgisiyle indirilir; adı/türü gizlenmiş yayın listeleri de
  (içeriğinden) tanınır. Videonun üstüne gelince sağ üstte
  **GMID ile indir** düğmesi; videoya sağ tık → **GMID Tool ile indir** → kalite.
- **Medya Oynatıcı (ayrı ekran):** indirilen müzikler ve videolar; Müzik / Videolar / site / çalma listesi süzgeci,
  "Hepsini çal". Oynatıcıyı küçültünce (Esc) başka ekranlarda altta çalmaya devam eder, video köşede küçük oynar.
- **Dublaj ve altyazı:** sitede birden çok ses dili ya da altyazı varsa indirirken seçilir (varsayılan: Türkçe
  dublaj, Türkçe altyazı; Kontrol Odası → Video İndirici). Altyazılar videonun yanına `<ad>.tr.vtt` olarak iner;
  eklenti, film sitelerinin ayrıca yüklediği altyazı dosyalarını da yakalar. Oynatıcıda **CC** ile açılır.
- **Uygulama içi oynatıcı:** Medya Oynatıcı'da çift tıkla. **Sahne** görünümü (video/kapak solda, yazılar sağda) ya da
  **Liste** görünümü (parçalar solda, sözler sağda). Altta çalma çubuğu: oynat/duraklat, önceki/sonraki, karıştır,
  tekrar (liste/parça), ilerleme, hız (0.5×–2×), ses kaydırıcısı, tam ekran; kapağa ya da videoya tıklayınca durur.
  O an söylenen satır altınla, kelime zamanları varsa kelime kelime işaretlenir; satıra tıkla → oraya git.
  Kısayollar: Boşluk, ←/→ (5 sn), ↑/↓ (ses), M (sessiz), F (tam ekran), N/P (sonraki/önceki); Windows medya tuşları.
- **Yazıya dök:** önce **şarkı sözü veritabanı** (LRCLIB; satır satır zamanlı, şarkılarda en doğrusu), sonra sitenin
  kendi altyazısı (YouTube'da kelime zamanlı), yoksa **Whisper** tamamen bu bilgisayarda (model bir kez iner,
  ~190–550 MB; müzik aralarında uydurduğu satırlar ayıklanır). Yazılar dosyanın yanına `.srt` (video) / `.lrc`
  (müzik) olarak da kaydedilir.
- **Kendiliğinden yeniden deneme:** sitenin anlık reddi ya da kopan bağlantıda indirme 3 kez kendi kendine yeniden
  denenir ("Tekrar deneniyor").

### Sesle yazma (Ctrl + Alt)

- **Ctrl + sol Alt** basılı tut, konuş, bırak: söylediğin yazı o an hangi programdaysan (Discord, Word, tarayıcı…)
  oraya yapıştırılır. Her şey bu bilgisayarda olur; ses hiçbir yere gönderilmez.
- Türkçe klavyede **AltGr** (@, €, {…) dikteyi başlatmaz; araya başka bir tuş girerse (Ctrl+Alt+N gibi kısayollar)
  dikte başlamaz ya da iptal olur.
- **Whisper** (whisper-server) modeli bir kez yükler; **NVIDIA ekran kartı** varsa CUDA sürümüyle kısa bir dikte
  1 saniyenin altında yazılır (bir kez ~650 MB iner; şarkı sözü çıkarmayı da hızlandırır). Ekran kartında açılmazsa
  kendiliğinden işlemciye geçer.
- **Toparlama (deneme):** yerel dil modeli (Gemma 3 4B, llama.cpp, ~3 GB) noktalama, paragraf, dolgu sözleri,
  "yarın, yok yok cuma" gibi düzeltmeler ve devrik cümleler. Şüpheli sonuçta ham yazı yapıştırılır.
- 15 dakika kullanılmazsa modeller bellekten çıkar (oyunda ekran kartı boş kalsın). Kontrol Odası → Sesle yazma:
  özel sözcükler, dil, deneme alanı, son dikteler (yapışmadıysa kopyala).

### Kontrol Odası (ayarlar merkezi)

- Bütün ayarlar tek yerde, soldan sekmeli: Genel · Görünüm · **Kısayollar** · Ses & Mikrofon · Bildirimler ·
  Video İndirici · Sunucular · Veri ve yedekler · Hakkında; üstte **ayarlarda arama**.
- Kısayollar gruplu (Ses, Mikrofon, Uygulama, Video İndirici), her biri değiştirilebilir / kapatılabilir /
  varsayılana döndürülebilir; "hepsini varsayılana döndür".
- Yeni kısayollar: `Ctrl+Alt+G` GMID Tool penceresini aç, `Ctrl+Alt+D` **panodaki video bağlantısını indir** (IDM gibi).
- Ekran göstergesi açılıp kapatılabilir; günlük dosyası (`gmid.log`) ayarlardan açılabilir.

### Kararlılık düzeltmeleri

- Türkçe Windows'ta başlığında özel karakter olan videolarda (`［EDIT］`, ş, ğ…) "İndirilen dosya bulunamadı"
  hatası giderildi (video iniyordu ama adı yanlış okunuyordu).

- Windows'ta tepsiden pencere açarken / hızlı not açarken uygulamanın **kilitlenmesi** giderildi (hatırlatıcıların
  çalmamasının sebebi buydu).
- Hatırlatıcı ve geri sayımlar artık **Windows bildirimi** de gönderiyor; Hatırlatıcılar ekranında **Uyarıyı dene** düğmesi var.
- Sorun olursa `Belgeler\GMID Tool\gmid.log` dosyasında ne olduğu yazar.

### Faz 3a — Sunucular

- **GMID topluluğunun oyun sunucuları** tek ekranda: logo, açık/kapalı, oyuncu sayısı, harita, oyun modu, gecikme;
  içerideki oyuncuların adları. Ekran açıkken her 30 saniyede yenilenir.
- **Bağlan:** Call of Duty 1/UO/2/4 oyunu açıp doğrudan sunucuya sokar (oyunun yeri bir kez sorulur);
  CS ve diğer Steam oyunları Steam üzerinden, Minecraft Bedrock / SA-MP / MTA kendi istemcileriyle açılır;
  Minecraft Java ve Valheim için adres kopyalanır ve adım adım anlatılır
- **Topluluk listesi** bu depodaki [`topluluk/sunucular.json`](topluluk/sunucular.json) dosyasından gelir;
  dosya GitHub'da düzenlenince herkesin uygulamasında kendiliğinden güncellenir ([nasıl?](topluluk/README.md))
- **Kendi sunucuların:** arkadaşının sunucusunu adres, logo ve notla ekleyebilirsin

### Faz 2 — Günlük araçlar

- **Ses & Mikrofon:** kulaklık / hoparlör / monitör arasında tek tıkla geçiş, cihaz başına ses seviyesi ve sessiz,
  takma ad, simge ve gizleme; **mikrofon seviye kilidi** (bir program mikrofonu kısarsa anında geri çeker)
- **Her yerden çalışan kısayollar** (değiştirilebilir): `Ctrl+Alt+A` ses çıkışı, `Ctrl+Alt+M` mikrofon sessiz,
  `Ctrl+Alt+L` mikrofon kilidi, `Ctrl+Alt+PgUp/PgDn` mikrofon ±5, `Ctrl+Alt+Home` mikrofon %100, `Ctrl+Alt+N` hızlı not
- **Ekran göstergesi:** kısayollara sağ üstte küçük, odak çalmayan bir bilgiyle karşılık verir
- **Notlar:** biçimlendirme (kalın, başlık, liste, yapılacaklar kutusu, kod), not başına renk ve yazı tipi, sabitleme,
  arama, 30 günlük çöp kutusu; her not `Notlar\` klasöründe `.md` dosyası olarak da durur
- **Hızlı not penceresi** (`Ctrl+Alt+N`) ve masaüstünde her zaman üstte duran **yapışkan notlar**
- **Hatırlatıcılar:** hızlı süreler, tekrar (her gün / hafta içi / her hafta / her ay), zamanı gelince sağ altta
  odak çalmayan kart (10 dk / 1 saat ertele, Tamam); kapalıyken kaçırılanlar açılışta gösterilir
- **Sayaç:** turlu kronometre ve aynı anda birden çok geri sayım; pencere kapalıyken de çalışır, bitince kart çıkar
- Tepsi menüsü: hızlı not, ses çıkışı değiştir, mikrofonu sessize al, 5/10/25 dk geri sayım

### Faz 1 — İskelet

- Merkut Studios açılış animasyonu (tıkla ya da Esc ile atla; tepsiden açarken oynamaz)
- Malikâne teması, malikâne odası isimli sol menü; dar pencerede sadece ikonlar
- **Kontrol Odası:** arka plan (Malikâne / düz renk / resim, GIF ya da video), bulanıklık, karartma,
  odak noktası, görünüm kalitesi, açılış animasyonu, Windows ile başlatma, kapatınca tepside kalma,
  veri klasörü ve yedekler
- `Belgeler\GMID Tool` veri klasörü ve tek dosyalık veritabanı (`gmid.db`)
- Her gün otomatik yedek (son 7 yedek saklanır) + "Şimdi yedekle"
- Sistem tepsisi simgesi ve menüsü; ikinci kez açınca yeni kopya yerine mevcut pencere öne gelir
- Pencere boyutu ve konumu hatırlanır; monitör değişirse pencere ekran dışında açılmaz
- Sonraki fazlarda gelecek ekranların her biri ne yapacağını anlatan bir sayfa gösteriyor

## Geliştirme

Gerekenler: Node.js 22, Rust (stable), Windows'ta WebView2 (Windows 10/11'de hazır gelir).

```bash
npm install
npm run tauri dev      # uygulamayı geliştirme modunda açar
npm run tauri build    # kurulum dosyasını üretir (src-tauri/target/release/bundle/nsis)
npm run check          # arayüz tip kontrolü
```

Arayüz Tauri olmadan tarayıcıda da açılır (`npm run dev`); bu durumda ayarlar tarayıcıda saklanır ve
dosya seçme gibi masaüstü işlemleri çalışmaz.

| Klasör | İçerik |
|--------|--------|
| `src/` | Arayüz (Svelte 5 + TypeScript) |
| `src-tauri/` | Çekirdek (Rust): veri klasörü, veritabanı, yedekler, tepsi, pencereler, ses, kısayollar, zamanlayıcı |
| `docs/` | Tasarım dokümanı, araştırmalar, görsel taslak |
| `assets/marka/` | Merkut Studios, GMID ve GMID AI Butler logoları |

## Dokümanlar
- [Tasarım Dokümanı](docs/TASARIM.md) — vizyon, modüller, mimari, yol haritası, açık sorular
- [Ek A — Oyun ve film puanlama araştırması](docs/ARASTIRMA-PUANLAMA.md)
- [Ek B — Teknik fizibilite araştırması](docs/ARASTIRMA-TEKNIK.md)
- [Ek C — Oyun içi panel, otomatik bilgi ve galeri araştırması](docs/ARASTIRMA-OVERLAY-METADATA.md)
- [Görsel taslak](docs/taslak/gmid-tool-taslak.html) — tıklanabilir prototip, tarayıcıda açılır
