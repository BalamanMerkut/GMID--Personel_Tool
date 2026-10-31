# GMID Tool · Yapılacaklar

Toplu yapılacak işler. Biten iş buradan silinir, `TASARIM.md`'deki sürüm tablosuna yazılır.
Son güncelleme: 31 Ekim 2026.

---

## 1. Fırsatlar

### 1.1 Yeni geleni ayır
- Fırsatlar'a girince, son girişten beri gelen oyunlar ilk bakışta belli olsun. Örneğin kırmızı çerçeve ve köşede **YENİ** etiketi kullanılabilir.
- "En son ne gelmiş" görünsün. Yeniler en üstte ayrı bir şerit olabilir ("Son gelenler · 3"). Her kartta ne zaman geldiği yazsın ("2 saat önce").
- Etiket şu durumlarda kalksın: kart görülünce mi, sayfadan çıkınca mı, yoksa 24 saat sonra mı? Bu karar yapılırken verilecek.
- Kenar menüdeki kırmızı rozet (v0.6x) aynı "görülmedi" bilgisini kullansın. Böylece rozet sayısı ile sayfadaki YENİ etiketleri hep tutar.

### 1.2 Mağaza sırası
- Kartlar gösterilirken mağaza sırası: **1. Steam, 2. Epic, 3. GOG**, sonra diğerleri.
- Sonrası için önerilen sıra: PlayStation, Xbox/Microsoft Store, Ubisoft, EA, Prime Gaming, diğerleri.
- Bu sıra filtreden bağımsız. Filtre (Epic / Steam …) yalnız neyin görüneceğini seçer, sıralamayı değiştirmez.
- Aynı mağaza içinde önce ücretsizler, sonra indirim oranı büyük olanlar gelsin.

## 2. Sesle yazma

### 2.1 Fikir değiştirmek öğrenilmesin
- **Sorun:** Yanlış konuşup geri dönüp düzeltince (söylediğin sözü değiştirince) bu da sözlüğe giriyor.
- **İstenen:** Düzeltme, yazılan cümlenin bütünüyle değerlendirilsin.
  - Yalnız "Whisper yanlış duymuş" olanlar öğrenilsin.
  - "Ben yanlış söyledim" olanlar öğrenilmesin.
- **Nasıl ayırt edilebilir:**
  - Whisper her sözcük için ne kadar emin olduğunu verir. Emin olduğu sözcüğün değiştirilmesi büyük ihtimalle fikir değişikliğidir; öğrenilmez.
  - Düzeltilen sözcük yazılandan çok farklıysa öğrenilmez. Yanlış duyulan söz, yazılana sesçe yakın olur ("Esad" → "Esat", "videoyu" → "VDS").
  - Bir düzeltmede birden çok yer değişmişse ya da cümlenin yapısı değişmişse hiçbiri öğrenilmez.
  - Aynı düzeltme en az 2 kez görülmeden kalıcı kural olmasın. İlk seferinde "aday" olarak beklesin.

### 2.2 Sözlüğe yalnız gerektiğinde bakılsın
- **Sorun:** Sözlükteki kural her seferinde uygulanıyor.
- **İstenen:**
  - Whisper sözcüğü net duyduysa (yüksek güven) duyduğunu yazsın.
  - Sözlüğe yalnız Whisper bocaladığında (düşük güven) danışsın; o kelimeye en uygun sözlük karşılığını koysun.
- **Nasıl:** Sözcük başına güven değeri ve bir eşik kullanılır. Sözlükteki özel adlar Whisper'a da önceden ipucu olarak verilir; doğru yazma ihtimali artar.

### 2.3 Ders kaydı / not alma kipi
- Derste hoca konuşurken kaydı başlat; söylenenler sürekli yazıya dökülsün ve ders notu olarak kullanılabilsin.
- Bu, Ctrl+Alt'a basılı tutarak yazmaktan ayrı bir kip: uzun kayıt, durdur/devam et.
- Yazı canlı olarak parça parça akar (her ~20–30 sn'lik bölüm çözülür).
- Bitince Notlar'a kaydedilir: başlık (ders adı + tarih), dakika işaretleri, istenirse ses dosyası.
- Mikrofon seçilebilsin (bilgisayarın kendi mikrofonu ya da kulaklık).
- Ses dosyası saklama seçmeli, varsayılanı kapalı.
- Uzun derste bilgisayar yorulmasın; GPU varsa onu kullansın.

### 2.4 Hızlı açma/kapama
- Sesle yazma arayüzden tek tıkla açılıp kapansın (kenar menü ya da üst çubukta görünür bir düğme).
- Kısayolla da açılıp kapansın; kısayol Kısayollar listesinde değiştirilebilsin.

### 2.5 Oyundayken kapansın (ayar)
- Ayar: "Oyundayken sesle yazma kapalı olsun".
- **Varsayılan: kapalı** (ayar açılmadıkça oyunda da çalışır).
- Açılırsa oyun algılanınca Ctrl+Alt dinlenmez, oyun kapanınca kendiliğinden geri gelir.

## 3. Oyunlar

### 3.1 Yeni oyunu sormadan ekle
- **Şimdiki durum:** Yeni bir oyuna girince Oyunlar'da "Bu oyunu ekleyeyim mi?" diye soruyor.
- **İstenen:** Sormasın, doğrudan eklesin ki oynama süresi hemen işlesin.
- Eklenince kısa bir bildirim yeter ("X kütüphaneye eklendi"); istenirse oradan geri alınsın.
- Oyun olmayan programlar (başlatıcılar, araçlar) yanlışlıkla eklenmesin; bugünkü algılama kuralları korunur. Yanlış eklenen tek tıkla kaldırılabilsin ve bir daha eklenmesin.

## 4. Bekleyen (öncekilerden)

- **FPS göstergesi:** Panelde FPS neden çıkmıyor, teşhis ve çözüm (PresentMon).
- **Ses Paneli:** VB-CABLE ile ilk gerçek deneme. Sorun çıkarsa geri bildirime göre düzeltme.
- **Futbol:** Takım sayfasında Avrupa/kupa maçlarının ESPN'den geldiğinin doğrulanması; Türkiye Kupası adı (tur.cup) tutmazsa düzeltilecek.

## Ertelenenler / yapılmayacaklar

- **Ertelendi:** Disk temizliği.
- **Yapılmayacak:** Mola hatırlatıcısı, mikrofon testi.
- **VDS yol haritasında:** Discord botu özellikleri.
