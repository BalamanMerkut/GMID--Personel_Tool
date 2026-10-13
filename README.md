# GMID Tool

*Merkut Studios*

**Game Mansion In Discord**'un alet çantası: oyunlarını, film-dizilerini, notlarını, kliplerini,
indirdiğin videoları ve Windows'taki küçük can sıkıcı işleri tek bir şık pencerede toplayan
kişisel Windows uygulaması.

**GMID Discord:** [discord.gg/upu9VQwFrV](https://discord.gg/upu9VQwFrV)

> **Durum:** Faz 1 (İskelet), Faz 2 (Günlük araçlar), Faz 3a (Sunucular), Faz 3b (Oyun: Kütüphanem, Oturum Geçmişi, Oyun Arşivi, Kliplerim), Faz 4 (Oyun Modu, Oyun İçi Panel), Faz 5 (Video İndirici) ve Faz 6 (Film & Dizi Arşivi, Format Sonrası Kurulum) kodlandı. Sonrası için bkz. [Yol haritası](docs/TASARIM.md#9-yol-haritası).

## Kurulum (Windows 10/11)

1. GitHub'da bu deponun **Actions** sekmesine gir, en son başarılı **Derleme** çalışmasını aç.
2. Sayfanın altındaki **Artifacts** bölümünden `GMID-Tool-Kurulum` dosyasını indir (zip içinde `.exe` var).
3. `GMID Tool_x.y.z_x64-setup.exe` dosyasını çalıştır. Yönetici izni istemez; sadece senin kullanıcına kurulur.

Windows SmartScreen "tanınmayan uygulama" uyarısı verebilir (uygulama imzalı değil):
**Ek bilgi → Yine de çalıştır**.

Kolay yol: deponun **Releases** sayfasındaki en son `GMID-Tool-Kurulum.exe`. Bir kez kurduktan sonra yeni
sürümler uygulamanın içinden gelir: yan menüde **Güncelleme var** çıkar, Kontrol Odası → Hakkında → **Güncelle**.

İlk açılışta beş adımlık kısa bir **tanıtım turu** çıkar (Discord'a katıl, kısayollar, oyun içi panel ve FPS izni,
klip klasörleri, Windows ile başlat); sonra Kontrol Odası → Hakkında → **Tanıtım turunu yeniden aç**. Bir sorun olursa
aynı yerde **Sorun bildir**: açıklaman, sürüm, Windows bilgisi ve günlükler tek zip olur (kullanıcı adın gizlenir).

## Neler var?

### Çark & Çekiliş (Eğlence Salonu)

- Bir liste yaz (isimler, filmler, oyunlar; tek tek, toplu yapıştır ya da **İzlenecek filmler / İstediğim oyunlar /
  Kurulu oyunlar** ile tek tıkla doldur), listeyi kaydet ("Film gecesi"), sonra:
  - **Çark:** çevir, ibrenin gösterdiği kazanır (ilk çıkan). İstersen çıkan sonraki çevirişte listeden çıkar.
  - **Eleme:** her çevirişte ibrenin gösterdiği elenir (dilimi yumuşakça daralır), **son kalan** kazanır; "Otomatik"
    ile son kalana kadar kendisi çevirir. Elenme sırası yanda.
  - **Turnuva:** adını koy (ör. "LoL 1v1 Turnuvası"), eşleşmeler çekilsin (sayı 2, 4, 8, 16 değilse ilk turda bay);
    maçı kim kazandıysa ona tıkla, şampiyona kadar; 🎲 ile maçı rastgele bitir. Sonunda kürsü (🥇🥈🥉) ve
    **şampiyon kartı**: görsel panoya gelir, Discord'da Ctrl+V.
  - **Kura:** N kazanan seç, rastgele **sıraya diz** ya da **takımlara böl**; "Seviyeye göre dengele" ile herkese 1–5
    yıldız verip güçleri eşit takımlar kur.
  - **Zar & yazı-tura:** üç kart (yazı-tura, zar, sayı seç); birine tıklayınca ekranın ortasında büyür (Geri / Esc).
    Para havaya atılıp dönerek düşer (3B, kalınlıklı, gölgeli), zarlar noktalı yüzlerle yuvarlanır (1–6 zar, toplam).
- Rastgelelik kriptografik ve eşit. Tık sesi, kutlama, **Büyük göster** (Discord'da ekran paylaşırken), son sonuçlar ve
  **Discord'a kopyala**. Boşluk tuşu çevirir.

### Bağlantı Testi (Oyun Salonu)

- "Lag bende mi, sunucuda mı?": modem, internet (Cloudflare, Google, Discord), GMID sunucuları ve en yakın Steam
  noktalarına aynı anda ping. Ortalama, en iyi/en kötü, **titreme**, **kayıp** ve her ping'in çubuğu; en üstte tek
  cümlelik teşhis (ev ağı / Wi-Fi, internet sağlayıcısı ya da sunucu). Oyundayken **Oyun İçi Panel → Ping** kartından
  tek tıkla (~1 sn), alt-tab yok.
- **Ekran kartı sürücüsü:** yeni NVIDIA sürücüsü çıkınca bir kez bildirir; Sistem özelliklerinde "Yeni sürücü · İndir".

### Faz 3b — Oyun (1. bölüm: Kütüphanem, Oturum Geçmişi)

- **Oyunları tara:** Steam, Epic, Riot (League of Legends, VALORANT), Xbox uygulaması ve Minecraft (Java ve Bedrock) oyunları
  (Steam'in tuttuğu süreyle) bulunur;
  istediklerini seçip eklersin. Kapağı gelmeyen oyunlara oyun penceresinden **Kapak seç** ile kendi resmini koyabilirsin.
  Tanınmayan oyunlar: **Oyun ekle** → çalışan uygulamalardan seç ya da .exe göster.
- **Yeni oyun açılınca sorar:** kütüphanende olmayan bir oyun açılınca sağ altta "Kütüphaneye ekleyeyim mi?" kartı çıkar
  (Ekle · Şimdi değil · Bir daha sorma). Steam'de olmayan oyunların kapağı Wikipedia'dan gelir.
- **Canlı algılama (Discord gibi):** 5 saniyede bir çalışan programlara bakılır; kütüphanedeki bir oyun açılınca
  oturum başlar, kapanınca biter. Oyuna hiçbir şey enjekte edilmez (anti-cheat güvenli). 30 saniyeden kısa açılışlar
  sayılmaz. Kontrol Odası'ndan kapatılabilir.
- **Kütüphanem:** kapak ızgarası (kapak ve bilgiler Steam mağazasından kendiliğinden, Türkçe), arama, kaynak süzgeci,
  sıralama; oyun penceresinde **Oyna**, klasörü aç, **Gerçek oynama** (GMID'in ölçtüğü) ve **Steam'deki süre** ayrı.
- **Oturum Geçmişi:** son 7/30 gün, günlük oynama grafiği (en çok oynanan 5 oyun renkli), gün gün oturum listesi.

### GMID Wrapped

- Ayın ya da yılın özeti tek bir şık görselde: kaç saat oynadın, en çok oynadığın 5 oyun, en uzun oturumun, en
  sevdiğin gün ve saat, rozetlerin (Gece Kuşu, Maratoncu, Kâşif…), puanladıkların ve izlediklerin. PNG olarak kaydet
  ya da panoya kopyala, Discord'a yapıştır.

### Fırsatlar

- Epic, Steam, GOG, PlayStation, Xbox, Ubisoft, EA, Prime Gaming ve itch'te **şu an ücretsiz** oyunlar; Epic'in
  sıradaki ücretsiz oyunu; Oyun Arşivi'nde **İstiyorum** dediğin oyunların indirimleri (Steam, GOG, Epic, EA,
  Ubisoft, Humble…). Yenisi çıkınca Windows bildirimi; hangi mağazalardan ve en az yüzde kaç indirimde haber
  verileceğini sen seçersin.

### Faz 6 — Format Sonrası Kurulum

- **Katalog** (Çalışma Odası → Programlar → Format Kurulumu): tarayıcılar, oyun platformları, iletişim, araçlar, medya,
  donanım, oyun bileşenleri… Tıkla, listene girsin. Katalogda yoksa **Tüm depoda ara** (Microsoft'un paket deposu).
  Bilgisayarında kurulu olanlar "Kurulu" diye işaretlenir.
- **Şimdi kur:** listendeki eksik programlar sırayla ve sessizce kurulur (winget); her biri için ilerleme ve ✅/❌.
- **Format Kiti:** USB'ye ya da bulut klasörüne bir klasör çıkarır. Formattan sonra **Kur.bat**'a çift tıkla:
  programlar kurulur, **GMID'in yedeği** (ayarlar, kısayollar, notlar, hatırlatıcılar, kütüphane, oturum geçmişi,
  puanlar, çalma listeleri, kapaklar, arka planlar; istersen videolar ve modeller) yerine konur, GMID kurulur ve
  formattan önceki haliyle açılır. Kullanıcı adın değişse bile dosya yolları yeni bilgisayara göre düzeltilir.
- Yedeği uygulamanın içinden de geri yükleyebilirsin: **Kontrol Odası → Veri ve yedekler → Yedekten geri yükle**.


### Windows Defender uyarısı

GMID henüz dijital imzalı değil; Defender yeni ve imzasız programları bazen yapay zekâ tahminiyle (`…!ml` ile biten
adlar, ör. `Trojan:Win32/Bearfoos.A!ml`) zararlı sanıp karantinaya alabilir. Bu bir **yanlış alarmdır**:
- Geri almak için: Windows Güvenliği → Virüs ve tehdit koruması → **Koruma geçmişi** → kayıt → **Eylemler → Geri yükle /
  Cihazda izin ver**.
- Microsoft'a bildirmek için: <https://www.microsoft.com/wdsi/filesubmission> → *Software developer* → kurulum dosyası,
  "Incorrectly detected as malware". Genelde birkaç gün içinde tespit kaldırılır.
- Kalıcı çözüm kod imzalama sertifikasıdır; alındığında derleme otomatik imzalanacak şekilde ayarlanır.

### Program Kaldırıcı

- **Programlar → Kaldırıcı:** programı kendi kaldırıcısıyla kaldır; bitince geride kalan klasör ve kayıt defteri artıkları
  **katı kurallarla** bulunur: ad birebir tutmalı; yalnız Program Files, ProgramData, AppData ve Başlat menüsünün birinci
  katı (ya da `Yayıncı\Program`); Windows ve Microsoft klasörleri, hâlâ kurulu başka bir programa ait ya da ortak kullanılan
  her şey ve bağlantılar (junction) asla; kayıt defterinde yalnız `Software\Program` ve `Software\Yayıncı\Program`.
  Dokunulmayanlar nedeniyle gösterilir. Sen seçip onaylarsın: dosyalar **Geri Dönüşüm Kutusu**'na gider, kayıtlar silinmeden
  önce `.reg` olarak yedeklenir (Belgeler › GMID Tool › Yedekler). Listede her programın kendi simgesi görünür; programlar türe (uygulama, oyun, sürücü & donanım, çalışma ortamı, Microsoft) ve
  diske göre süzülebilir, satıra tıklayınca nerede kurulu olduğu, yükleyicisi ve kaldırmanın riski görünür.
- **Sürükle-bırak:** Medya'daki videoları ve şarkıları, Kliplerim'deki klipleri basılı tutup sürükleyerek Masaüstü'ne, bir
  klasöre ya da Discord'a bırakabilirsin (kopyalanır, kütüphanedeki dosya yerinde kalır).
### Faz 6 — Film & Dizi Arşivi

- İzlediğin film ve dizileri **5 kısa soruyla** puanla: hikâye, oyunculuk, yönetmenlik, tempo, keyif. Önce durum:
  İzliyorum · İzledim · Yarıda bıraktım · İzlenecek (puansız, izleme listesi).
- Adını yazman yeter: Türkçe ad, afiş, Türkçe özet, yönetmen, oyuncular, tür, süre ya da sezon sayısı **Wikipedia ve
  Wikidata'dan** kendiliğinden gelir (anahtar gerekmez). Bulunamazsa yalnızca adıyla eklenir.
- Dizilerde "nerede kaldın" (S2 · B5), hepsinde nerede ve kiminle izlediğin.
- Sağdaki süzgeçler: film/dizi, tür, yönetmen ("Christopher Nolan filmleri"), puan, durum, tavsiye. **🎲 Bana bir film seç.**

### Faz 4 — Oyun içi deneyim

- **Oyun Modu** (Oyun Salonu → Oyun Modu, kısayol **Ctrl+Alt+O**): güç planı yüksek performansa geçer, seçtiğin ses
  çıkışına (ör. kulaklık) geçilir, mikrofon seviyesi kilitlenir, GMID'in sesleri ve Windows bildirimleri susar, hareketli
  arka plan durur, seçtiğin uygulamalar (ör. tarayıcı) kapanır. İstersen kütüphanendeki bir oyun açılınca kendiliğinden
  açılır, son oyun kapanınca kapanır. Kapanınca (uygulama kapanırken de) her şey eski haline döner.
- **Oyun İçi Panel** (**Ctrl+Shift+G**): oyunun bulunduğu ekranın üstünde yarı saydam panel — üstte FPS, işlemci,
  ekran kartı (yük, sıcaklık, VRAM), RAM, işlemci saat hızı, disk etkinliği, ağ indirme/yükleme ve (bilgisayar
  veriyorsa) ACPI sıcaklığı (FPS için bir kez **FPS ölçümünü aç** — panelde ya da Oyun Modu'nda; bir kez
  Windows izni ister, oturumu kapatıp açmak gerekmez; RivaTuner/RTSS açıksa ona da gerek yok). FPS yoksa nedeni
  panelde yazar. Kenarlıksız/pencereli oyunlarda Steam'deki Shift+Tab gibi fare serbest kalır, oyun arkada görünür.
  **Özel tam ekranda** Windows hiçbir pencereyi oyunun üstünde göstermez: oyunu **Pencereli** moda al, GMID onu
  **kenarlıksız tam ekrana** çevirir (Oyun Modu → Kenarlıksız tam ekran; panel bir oyunda açılamazsa o oyun listeye
  kendiliğinden eklenir ve Windows bildirimiyle anlatılır). Ses çıkışı ve seviyesi, **uygulama başına ses**
  (oyun, Discord, Spotify, tarayıcı ayrı ayrı; Ses & Mikrofon ekranında da var),
  mikrofon (seviye, sessize alma, kilit), Oyun Modu, sayaç (5/15/30 dk mola), o oyuna not (Notlar'da da görünür), son
  klip (Discord'a sığdır), yaklaşan hatırlatıcılar, **Şimdi çalan** (Spotify, tarayıcıda YouTube, VLC… Windows'un
  medya tuşlarına bağlanan her program: kapak, ad, ilerleme çubuğu (tıkla → oraya git), karıştır, önceki /
  oynat-duraklat / sonraki, tekrar, o programın ses seviyesi; GMID'in kendi oynatıcısında hepsi doğrudan). Esc ya da aynı tuşla kapanır,
  oyuna geri dönersin. Oyuna hiçbir şey eklenmez; çerçevesiz ya da pencereli oyunlarda görünür.
- **Discord'da durum** (oyundayken profilde "X oynuyor · GMID'e katıl"): kodu hazır; GMID'in Discord botu (VDS) açılınca
  kendiliğinden gelecek.
- NVIDIA (Alt+F10), Xbox Game Bar, AMD, OBS, Steam (F12) ve Medal kayıtları **kendiliğinden bulunur**; dosyalar
  taşınmaz, yerinde kalır. Başka bir klasöre kaydediyorsan Kontrol Odası → Oyunlar'dan ekle.
- Her klip **hangi oyunda çekildiyse** onun altında görünür (o an açık oyun, Steam'in kendi bilgisi ya da klasör adı);
  yanlışsa büyük görünümden değiştir. Yeni klipte köşede "Klip kaydedildi — Valorant (0:30)".
- Üzerine gelince sessiz önizleme, tıklayınca büyük görünüm (← → ile gezin), çift tıklayınca varsayılan oynatıcı.
  Favori, yeniden adlandır, klasörde göster, sil (Geri Dönüşüm Kutusu'na; geri alınabilir).
- **Kapaklar** videonun içinden rastgele bir kareden (başındaki siyah ekran değil). Kapak için gereken ffmpeg yoksa bir
  kez kendiliğinden iner (~90 MB); çıkarılamayan kapaklar sonra yeniden denenir.
- **Discord'a sığdır (tek tık):** klip **çözünürlüğü korunarak** küçültülür (AV1, görsel kalite ayarı; gerekirse
  30 kare) ve panoya konur — Discord'da **Ctrl+V**. Ölçümde aynı boyutta küçültmekten daha net çıktı. Klip o kadar
  uzunsa ki çözünürlük korunursa görüntü dağılacak, o zaman küçültülür. Yanındaki ⚙ seçenekleri açar:
- **Seçenekler:** yan yana — **Orijinal çözünürlük**, 10 MB'ta **Net** (AV1, en yüksek çözünürlük), **Akıcı** (60 fps),
  **Uyumlu** (H.264, her telefonda), **Nitro Basic** (50 MB), **Nitro** (500 MB) ya da sığıyorsa **Olduğu gibi**;
  her birinin yanında çözünürlük, kare hızı, **beklenen MB** ve netlik yazar. Uzun klipte "Son 30 sn" önerilir (kısa =
  çok daha net). Hazırlanan kopya pencerede oynar; beğenmezsen başka seçenekle dene, hepsi listede kalır. Seçilen kopya
  panoya konur; Discord'da **Ctrl+V** yeter.
  (Kopyalar `Belgeler\GMID Tool\Discord'a Hazır` içinde, 3 gün sonra silinir. ffmpeg gerekir: Video İndirici'nin araçları.)

### Faz 3b — Oyun (2. bölüm: Oyun Arşivi)

- **Önce durum:** Oynuyorum, Bitirdim, %100, Bıraktım ya da İstiyorum. İstiyorum'da puan sorulmaz (istek listesi).
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
  **GMID ile indir** düğmesi; videoya sağ tık → **GMID Tool ile indir** → kalite. Akış sayfalarında (X, Instagram,
  YouTube ana sayfa, **TikTok Sana Özel/Takip/Keşfet**) üstünde durulan videonun kendi bağlantısı bulunur.
- **Medya Oynatıcı (ayrı ekran):** indirilen müzikler ve videolar. **Müzik:** solda listeler (Tüm şarkılar,
  **Favoriler** ♥, **Son eklenenler**, **En çok dinlenenler**, kendi listelerin, indirilen listeler); üstte **Şimdi
  çalan** kartı (kapağın bulanık hali arka planda, büyük kapak, oynat/duraklat, **Karıştırarak çal**, o an söylenen söz
  satırı); sade satırlar (ekle/klasör/sil üzerine gelince, kapağın üstünde oynat, çalan şarkı altın ve ses dalgalı);
  ada / sanatçıya / süreye / en çok dinlenene göre sıralama; **Liste / Kapak** görünümü. Şarkıya tıklamak oynatıcıyı
  öne getirmez, altta çalar. **Videolar:** büyük kapaklı kartlar, üstünde süre ve kalite; site ve liste süzgeci.
  Oynatıcıyı küçültünce (Esc) başka ekranlarda altta çalmaya devam eder, video köşede küçük oynar.
- **Karaoke** (oynatıcıda **Karaoke** düğmesi): müzik iki yoldan gelir. **İnternetten temiz müzik bul** (en temiz):
  şarkının resmî enstrümantal / karaoke sürümü YouTube'da aranır, seçtiğin indirilir ve sözlere göre kendiliğinden
  hizalanır. Ya da şarkıcının sesi yapay zekâyla (Meta'nın **Demucs** modeli) müzikten ayrılır (ses girince müzik biraz
  boğuklaşabilir); yalnız müzik çalar, sözler ortada kelime kelime altınla dolar (önceki/sonraki satırlar, girişte geri sayım),
  istersen **Orijinal ses** kaydırıcısıyla şarkıcıyı hafifçe duyarsın. **Kaydet**: mikrofonla söylediğin müzikle
  karıştırılır (hafif yankı, ses seviyesi ve gecikme düzeltmesi sonradan ayarlanır) → `Belgeler\GMID Tool\Karaoke\Kayıtlar`.
  Ayırma bu bilgisayarda, şarkı başına bir kez (ekran kartıyla DirectML: NVIDIA/AMD/Intel; yoksa işlemciyle 1–3 dk);
  ilk kullanımda ses ayırma programı (ONNX Runtime, ~25 MB) ve model (~98 MB) iner. Kulaklıkla en temiz kayıt;
  hoparlörde "Kulaklık takılı"yı kapat (müziğin mikrofona girmesi engellenir).
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
- **Açılışta hazırla:** GMID açıldıktan biraz sonra modeller arka planda yüklenir ve ekran kartı bir kez ısındırılır;
  ilk dikte beklemez. Model yüklenirken gösterge "Hazırlanıyor" der.
- 15 dakika kullanılmazsa modeller bellekten çıkar (oyunda ekran kartı boş kalsın). Kontrol Odası → Sesle yazma:
  özel sözcükler, dil, deneme alanı, son dikteler (yapışmadıysa kopyala).
- **Sözlük (kullandıkça öğrenir):** Whisper bir sözcüğü hep yanlış yazıyorsa ("GMID" → "GMD", "VDS" → "videoyu")
  "şunu duyunca bunu yaz" kuralı olur; her diktede uygulanır ("GMD'ye" → "GMID'ye", ek korunur) ve doğru yazılış
  Whisper'a ipucu olarak da verilir. Öğrenme: dikteden sonra yazıyı yapıştırıldığı yerde (Discord, tarayıcı, Word…)
  düzeltirsen GMID gönderilmeden önceki halini okuyup karşılaştırır ve sağ altta **"Bunu böyle mi yazayım?"** diye
  sorar; "Son dikteler"de ✎ ile de düzeltebilirsin. Yazı hiçbir yere kaydedilmez, şifre alanlarına bakılmaz.

### Hatırlatıcı, sayaç ve zamanlı kapatma

- Zamanı gelen hatırlatıcı ya da biten sayaç **ekranın ortasında** açılır, kısık ve sakin bir müzik kutusu melodisiyle:
  **Ertele** (5 dk, 10 dk, 1 saat) ya da **Kapat** (Enter: kapat, Esc: 10 dk ertele). Oyundaysan pencere yerine
  **Oyun İçi Panel** açılır, uyarı en üstte: alt-tab yapmadan kapatırsın, panel kapanınca oyuna dönersin. Kontrol
  Odası → Bildirimler: açılır pencere, oyunda panel, melodi ve ses düzeyi (Dinle).
- **Bilgisayarı zamanla kapat** (Kontrol Odası → Genel; yanlış tıklamaya karşı onay sorar): Kapat / Yeniden başlat / Uyut — 10 dk, 30 dk, 1 saat… ya da elle.
  Son bir dakikada ortada geri sayım çıkar (İptal / Şimdi / sağ üstteki × ile gizle: süre işlemeye devam eder).
  Zamanı gelince açık programlar kapatılır.
- **Sayaç:** kronometre ve geri sayımların rakamları kayarak değişir; salise ve ilerleme çubuğu her karede akar.
- **Sistem özellikleri** (Kontrol Odası; bir kez okunur ve saklanır, değiştiyse Yenile): bilgisayarın özeti ve dört ana parça (işlemci, ekran kartı, bellek, depolama),
  altında sürücü doluluğu, bellek modülleri, ekranlar (inç, çözünürlük, Hz), fare/klavye/kulaklık, kamera, yazıcı, anakart ve ağ;
  **Discord'a kopyala** ile düzenli bir özet yapıştır.

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
- **Sunucu bildirimi:** kartın zil düğmesiyle "sunucu açılınca" ya da "en az N kişi olunca" Windows bildirimi
  (yalnızca izlenen sunucular, 2 dakikada bir; aynı şey için tekrar tekrar gelmez)

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
