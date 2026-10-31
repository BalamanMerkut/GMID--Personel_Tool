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
- **VDS yol haritasında:** Discord botu özellikleri.
