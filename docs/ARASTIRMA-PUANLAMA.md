# Ek A — Oyun ve Film Puanlama Araştırması

> Soru: *"Bir oyun değerlendirilirken, puanlama yapılırken nelere bakılmalı ve puanlanmalı; bu puanlama puanın yüzde kaçını etkiliyor?"*
> Ana doküman: [TASARIM.md](TASARIM.md) §3.3 ve §3.4

> **v0.2 notu:** Karar gereği uygulamadaki puanlama **sade** tutuldu (1–10 puan + tek cümle + notlar).
> Buradaki ayrıntılı kriterler ve yüzdeler, puanlama kartındaki isteğe bağlı ve varsayılan olarak
> kapalı **"Detaylı puanla"** bölümünde ve puan rehberinde (1–10 çapaları) kullanılacak.

## 1. Sektör puanlamayı nasıl yapıyor?

| Sistem | Yöntem | Ağırlıklı kategori var mı? |
|---|---|---|
| **IGN (bugün)** | 1–10 arası tam sayı, her puanın bir etiketi var (10 Masterpiece, 8 Great, 7 Good, 5 Mediocre…). IGN'e göre eleştiri bir bilim değil ve teknik değerler puanı tek başına belirlemiyor. | Hayır, tek bir bütünsel puan |
| **Eski IGN (~2000–2011)** | Presentation, Graphics, Sound, Gameplay ve Lasting Appeal için ayrı puanlar verilirdi. Genel puanın yanında "not an average" yazardı: genel puan alt puanların ortalaması değildi, ayrıca verilirdi. | Kategori vardı, ağırlık yoktu |
| **GamePro / Nintendo Power (90'lar)** | GamePro: Graphics, Sound, Control, FunFactor (5 üzerinden). Nintendo Power: Graphics & Sound, Play Control, Challenge, Theme & Fun. | Kategori vardı, sabit ağırlık yoktu |
| **GameSpot** | 1–10, eleştiri metniyle birlikte bütünsel puan | Hayır |
| **Famitsu** | 4 eleştirmen × 10 puan = 40 üzerinden. 40/40 çok nadir. | Hayır; birden fazla kişinin görüşü birleşiyor |
| **Metacritic** | Eleştirmen puanları 0–100'e çevrilir, ağırlıkları açıklanmayan bir ortalama alınır. Renk bantlarıyla gösterilir. | Yayın bazında gizli ağırlık |
| **OpenCritic** | "Top Critic Average": yalnızca seçkin eleştirmenlerin puanlarının basit ortalaması + "% Recommended". | Hayır, bilerek şeffaf |
| **Steam** | Kullanıcı yalnızca "Tavsiye ederim / etmem" der; sonuç yüzdeye ve etikete dönüşür (Overwhelmingly Positive…). | Hayır, ikili oy |

**Akademik çerçeveler (kategori seçerken yol gösterici):**
- **MDA** (Hunicke, LeBlanc, Zubek 2004): Mechanics (kurallar) → Dynamics (oynarken ortaya çıkan davranış) → Aesthetics (oyuncunun duygusal deneyimi). Rubrikte "Oynanış" mekanik+dinamiği, "Kişisel keyif" estetiği karşılar.
- **GameFlow** (Sweetser & Wyeth 2005): Keyfi sekiz unsurla tanımlar — konsantrasyon, meydan okuma, oyuncu becerisi, kontrol, net hedefler, geri bildirim, içine çekme (immersion), sosyal etkileşim.
- **PENS** (Ryan, Rigby, Przybylski 2006): Yetkinlik, özerklik, ilişkisellik, içine çekme (presence) ve sezgisel kontroller.

**Çıkarım:** Profesyoneller bugün çoğunlukla **bütünsel tek puan** kullanıyor; kategori puanları yalnızca destek. Kişisel uygulama için en iyisi ikisini birleştirmek: kategorilerden **hesaplanan puan** + elle girilebilen **nihai puan**.

---

## 2. Önerilen oyun rubriği (varsayılan, toplam %100)

| Kriter | Ağırlık | Neye bakılır |
|---|---|---|
| **Oynanış / Mekanikler** | %25 | Ana oyun döngüsü eğlenceli mi, mekaniklerde derinlik var mı, zorluk dengesi, ilerleme hissi |
| **Hikâye & Dünya** | %15 | Senaryo, karakterler, diyaloglar, lore, anlatımın oynanışla uyumu |
| **Sanat Yönetimi & Görsellik** | %10 | Görsel stilin tutarlılığı ve özgünlüğü, teknik grafik kalitesi (stil, teknikten önemli) |
| **Ses & Müzik** | %8 | Müzik, ses efektleri, seslendirme, ses tasarımının oynanışa katkısı |
| **Kontroller & Arayüz (UI/UX)** | %7 | Kontrollerin tepkiselliği, menü ve HUD netliği, erişilebilirlik |
| **Performans & Teknik** | %7 | FPS kararlılığı, hata/çökme, yükleme süreleri, optimizasyon |
| **İçerik & Tekrar Oynanabilirlik** | %8 | Süre/doluluk, yan içerik, oyun sonu içeriği, tekrar oynama sebebi |
| **Fiyat / Değer** | %5 | Ödenen para ve harcanan zamana göre verdiği değer |
| **Kişisel Keyif / Etki** | %15 | Aklımda kaldı mı, ne kadar içine çekti, "yine oynar mıydım?" |
| **Toplam** | **%100** | |

**Hesaplama:**
`Hesaplanan puan = Σ(puanᵢ × ağırlıkᵢ) / Σ(ağırlıkᵢ)` — bölen yalnızca **puanlanmış** kriterlerin ağırlık toplamıdır; böylece "Yok/Uygulanamaz" (N/A) işaretli kriterler kendiliğinden düşer.

### 1–10 ölçek çapaları (tüm kriterlerde ortak)

| Puan | Anlamı |
|---|---|
| **1–2** | Kırık. Oynanamaz ya da deneyimi aktif olarak bozuyor. |
| **3** | Kötü. Belirgin kusurlar baskın, olumlu yan çok az. |
| **4** | Zayıf. İşliyor ama sık sık rahatsız ediyor. |
| **5** | Vasat. Ne iyi ne kötü, unutulup gidiyor. |
| **6** | İdare eder. Birkaç iyi yanı var, kusurları göze batıyor. |
| **7** | İyi. Keyifli ve sağlam ama türünde öne çıkmıyor. |
| **8** | Çok iyi. Türünün iyi örneklerinden, kusurları küçük. |
| **9** | Mükemmel. Türün en iyileri arasında, uzun süre akılda kalıyor. |
| **10** | Başyapıt. Ölçüt oluşturuyor. Çok nadir verilmeli. |

> Profesyonel puanların büyük kısmı 6–9 arasına yığılıyor. "7 = iyi" sabitlenmezse kişisel puanlar da zamanla yukarı kayar. Çapalar uygulamada kaydırıcının yanında ipucu olarak gösterilecek.

---

## 3. Türe göre ağırlık şablonları (ön ayarlar)

| Kriter | Varsayılan | Hikâye odaklı RPG / Aksiyon-Macera | Rekabetçi Multiplayer (FPS/MOBA) | Bulmaca / Indie | Yarış / Spor / Simülasyon |
|---|---|---|---|---|---|
| Oynanış | 25 | 20 | **35** | **30** | **30** |
| Hikâye & Dünya | 15 | **25** | N/A (0) | 10 | 5 |
| Sanat & Görsellik | 10 | 10 | 5 | **15** | 10 |
| Ses & Müzik | 8 | 8 | 7 | 10 | 7 |
| Kontroller & UI/UX | 7 | 5 | **10** | 7 | **12** |
| Performans & Teknik | 7 | 5 | **13** | 3 | 10 |
| İçerik & Tekrar | 8 | 10 | **15** | 5 | 10 |
| Fiyat / Değer | 5 | 2 | 5 | 5 | 3 |
| Kişisel Keyif | 15 | 15 | 10 | 15 | 13 |
| **Toplam** | 100 | 100 | 100 | 100 | 100 |

Ek şablon önerisi — **Korku / Anlatı / Walking-sim:** Hikâye 30, Ses 15, Sanat/Atmosfer 15, Oynanış 15, Keyif 15, diğerleri toplam 10.

**N/A (uygulanamaz) kuralı**
- Her kriterin yanında bir **N/A** kutusu. İşaretlenen kriter hesaba girmez; ağırlığı kalanlara **oransal** dağılır:
  `yeni ağırlıkᵢ = ağırlıkᵢ / (100 − N/A ağırlıkları toplamı) × 100`
- Örnek: Hikâyesiz bir multiplayer oyunda Hikâye (%15) N/A → Oynanış %25 yerine 25/85 ≈ **%29,4** olur.
- **N/A ≠ 0 puan.** Hikâyesi berbat olan oyun 2 alır; hikâyesi hiç olmayan oyun N/A olur.
- Şablon sonradan değişirse eski kayıtlar bozulmasın: her değerlendirme, puanlandığı andaki **şablon kimliğini ve ağırlıkların kopyasını** saklar.

---

## 4. Film rubriği

**Siteler nasıl yapıyor?**
- **IMDb:** Kullanıcılar 1–10 verir; gösterilen sonuç manipülasyona karşı ağırlıklandırılmış (formülü gizli) bir ortalama.
- **Letterboxd:** 0,5–5 yarım yıldız + ayrı bir "beğen (kalp)" + izleme günlüğü. Bütünsel.
- **Rotten Tomatoes:** Tomatometer = olumlu eleştirilerin yüzdesi (her eleştiri olumlu/olumsuz sayılır, %60+ "Fresh"). Popcornmeter seyirci karşılığı.
- **Metacritic:** Eleştirmen puanlarının 0–100 ağırlıklı ortalaması, renk bantlarıyla.

Bunların **hiçbiri kategori puanı kullanmıyor**. Kişisel uygulama için önerilen kategoriler:

| Kriter | Ağırlık | Neye bakılır |
|---|---|---|
| **Senaryo & Hikâye** | %22 | Yapı, diyalog, karakter gelişimi, özgünlük, sonun tatmin ediciliği |
| **Oyunculuk** | %15 | İnandırıcılık, oyuncular arası uyum, yan rollerin kalitesi |
| **Yönetmenlik** | %15 | Vizyonun bütünlüğü, ton kontrolü, sahnelerin kuruluşu |
| **Görüntü & Sanat Yönetimi** | %10 | Sinematografi, ışık, set ve kostüm, görsel efektler |
| **Kurgu & Tempo** | %10 | Ritim, sürenin hakkını verme, geçişler |
| **Müzik & Ses** | %8 | Film müziği, ses tasarımı, sessizliğin kullanımı |
| **Duygusal Etki / Kişisel Keyif** | %20 | Bende ne bıraktı, aklımda kalıyor mu, tekrar izler miyim |
| **Toplam** | **%100** | |

Film şablonları:
- **Aksiyon / Gişe filmi:** Görüntü-efekt ve Kurgu yükselir, Senaryo düşer.
- **Belgesel:** Oyunculuk N/A; yerine "Bilgi / Konu" kriteri.
- **Animasyon:** Oyunculuk → "Seslendirme" olarak adlandırılır.
- **Dizi (istenirse):** + "Sezonlar arası tutarlılık" kriteri.

---

## 5. Kişisel puanlama uygulaması için arayüz fikirleri

- **Hesaplanan puan + Nihai puan.** Kategorilerden çıkan puan otomatik görünür, istersen nihai puanı elle değiştirirsin (eski IGN'in "ortalama değildir" yaklaşımı). Fark büyükse (±1,5) "Neden?" diye kısa bir not ister.
- **Durum alanı** (Backloggd'dan esinlenerek): *İstek listesi · Sırada · Oynanıyor · Bitti (ana hikâye) · Bitti + yan içerik · %100 · Ara verildi · Bırakıldı*. Filmler: *İzlenecek · İzlendi · Yarıda bırakıldı · Tekrar izlendi*.
- **Süre alanları** (HowLongToBeat mantığı): Ana hikâye / Ana + ekstra / %100. "Saat başına değer" türetilebilir.
- **"Tekrar oynar / izler miydim?"** Evet / Belki / Hayır — puandan bağımsız hızlı sinyal.
- **Favori (kalp)** ve **"Tavsiye ederim"** bayrakları (Letterboxd'taki gibi puandan ayrı beğeni).
- **Meta veriler:** Platform, mağaza, başlama/bitirme tarihi, fiyat (indirimli mi), zorluk seviyesi, oynanan sürüm/yama.
- **Etiketler:** "soulslike", "co-op", "kısa", "rahatlatıcı"… Filtre ve şablon önerisi için.
- **Yeniden değerlendirme geçmişi:** Yıllar sonra tekrar puanlanırsa eski puan silinmez, zaman çizelgesinde ikisi de görünür.
- **Kısa not ve "en iyi an".** Puanın neden verildiğini hatırlatır.
- **Kalibrasyon görünümü:** Puan dağılımı histogramı — her şey 8–9'a yığılıyorsa fark edersin.
- **Dış puan karşılaştırması (isteğe bağlı):** Metacritic/OpenCritic/IMDb ortalamasına göre "benim puanım − ortalama".
- **Hızlı mod / Detaylı mod:** Hızlı modda yalnızca nihai puan + durum; detaylıda tüm kategoriler. Kategori puanlamak zorunluluk olmaz.

---

### Kaynaklar
- IGN puan ölçeği ve inceleme ilkeleri: https://corp.ign.com/review-practices
- IGN'in eski kategori sistemi: https://en.wikipedia.org/wiki/IGN
- Profesyonel puanların 6–9 aralığına yığılması: https://www.pcinvasion.com/?p=29849
- OpenCritic Top Critic Average: https://en.wikipedia.org/wiki/OpenCritic
- Metacritic'in gizli ağırlıklı ortalaması: https://padandpixel.com/is-review-aggregation-worth-fixing/
- Rotten Tomatoes: https://www.rottentomatoes.com/about
- GameFlow (Sweetser & Wyeth 2005): https://ntnu.no/wiki/download/attachments/112986425/GameFlow.pdf?api=v2
- PENS (Ryan, Rigby, Przybylski 2006): https://selfdeterminationtheory.org/player-experience-of-needs-satisfaction-pens/
- MDA (Hunicke, LeBlanc, Zubek 2004)
- Famitsu, GamePro, Nintendo Power, Steam, Letterboxd, Backloggd ve HowLongToBeat ayrıntıları genel bilgiden yazıldı (kaynak sayfası açılmadı); özellikle GamePro/Nintendo Power kategori adları teyit edilmeli.

> **Not:** Önerilen ağırlıklar ve şablonlar hiçbir kaynaktan birebir alınmadı; yukarıdaki çerçevelere ve yaygın inceleme pratiğine dayanan, kişisel kullanım için önerilmiş tasarım tercihleridir. İstediğin gibi değiştirebiliriz.
