# GMID Tool · Yapılacaklar

Toplu yapılacak işler. Biten iş buradan silinir, `TASARIM.md`'deki sürüm tablosuna yazılır.
Son güncelleme: 31 Ekim 2026.

---

## 1. Ses Paneli (şimdilik bekliyor)

### 1.1 Tek tıkla kablo kurulumu
- **Şimdi:** Kullanıcı VB-CABLE'ı siteden kendisi indirip kuruyor.
- **İstenen:** Ses Paneli'nde tek bir "Kur" düğmesi olsun:
  1. GMID, VB-CABLE'ı resmi siteden (vb-audio.com) indirir; FFmpeg'i indirdiği gibi.
  2. Zip'i açar, kurulumu yönetici izniyle başlatır. Windows bir kez izin ister.
  3. Kurulum bitince kabloyu kendisi tekrar arar.
  4. Windows varsayılan hoparlörü ya da mikrofonu kabloya çevirdiyse eski haline döndürür.
- Kullanıcıya kalan tek iş: Discord'da (ve istenirse Valorant'ta) bir kez Giriş Aygıtı olarak "CABLE Output" seçmek. Discord'un ayarlarına dışarıdan dokunulmaz.
- Kaldırıcı kuralı geçerli: GMID kabloyu kaldırmaz, sadece kendi indirdiği kurulum dosyalarını siler.

### 1.2 Diğer notlar
- Ses Paneli sayfasına not: Valorant'ta da kullanılabilir (ses ayarlarında giriş aygıtı CABLE Output).
- **İsteğe bağlı önlem:** CABLE Output Windows'ta varsayılan mikrofon yapılmışsa, GMID kapanırken ya da panel kapatılınca varsayılan mikrofon gerçek mikrofona geri alınsın.

## 2. Bekleyen (öncekilerden)

- **FPS göstergesi:** Panelde FPS neden çıkmıyor, teşhis ve çözüm (PresentMon).
- **Ses Paneli:** VB-CABLE ile ilk gerçek deneme. Sorun çıkarsa geri bildirime göre düzeltme.
- **Futbol:** Takım sayfasında Avrupa/kupa maçlarının ESPN'den geldiğinin doğrulanması; Türkiye Kupası adı (tur.cup) tutmazsa düzeltilecek.
- **Windows'ta denenecekler (v0.73):**
  - Whisper sunucusunun sözcük güveni verdiği (`verbose_json`): günlükte "net duyuldu, sözlük … uygulanmadı" satırları görünmeli.
  - Fırsatlar'ın Steam/Epic uygulamasında açılması.
  - Ders kaydı: uzun kayıtta yazının akması, bitince notun oluşması.

## Ertelenenler / yapılmayacaklar

- **Ertelendi:** Disk temizliği.
- **Yapılmayacak:** Mola hatırlatıcısı, mikrofon testi.
- **VDS yol haritasında:** Discord botu özellikleri (botlar tek botta birleştirilecek; mevcut botlar taşınmıyor).

## 3. GMID sunucusu (VDS alındı, 2026-11-01)

- **Kullanıcı:** DuckDNS adı al, `docs/SUNUCU.md` §3'teki tek komutla kur, `https://<ad>/saglik` açılıyor mu bak.
- **Kullanıcı:** Discord uygulaması (birleşik bot ile aynı): Client ID/Secret, Redirect `http://127.0.0.1:53682/giris`,
  `/etc/gmid/sunucu.env`.
- **GMID Tool:** Kontrol Odası → "Discord ile giriş" (tarayıcıda Discord, 53682'de geçici dinleyici, kod sunucuya;
  `identify guilds` kapsamı), yan menüde Arkadaşlar (çevrim içi + oynadığı oyun), oyun algılanınca durum gönderimi
  (ayardan kapatılabilir), sunucu adresi ayarı.
- ✅ (v0.86) Topluluk: akış, beğeni/yorum, profiller, özel mesajlar, paylaşım ayarları.
- **Kullanıcı:** Discord uygulaması açılıp sunucuya yazılmadan "Discord ile giriş" çalışmaz (SUNUCU.md §4).
- Sırada: **ortak şarkı çalma** (biri çalar, diğerleri aynı anda dinler; YouTube bağlantısıyla eşzamanlı),
  oyun gecesi planlayıcı, ortak puan arşivi ("Ben 9 · Ali 7").
- Fikir: VDS'te **oyun sunucuları** (Minecraft, CS2…): sunucunun işlemci/belleğine ve Mola projesine göre karar.
