# GMID Tool

*Merkut Studios*

**Game Mansion In Discord**'un alet çantası: oyunlarını, film-dizilerini, notlarını, kliplerini,
indirdiğin videoları ve Windows'taki küçük can sıkıcı işleri tek bir şık pencerede toplayan
kişisel Windows uygulaması.

> **Durum:** Faz 1 (İskelet) kodlandı. Sonraki fazlar için bkz. [Yol haritası](docs/TASARIM.md#9-yol-haritası).

## Kurulum (Windows 10/11)

1. GitHub'da bu deponun **Actions** sekmesine gir, en son başarılı **Derleme** çalışmasını aç.
2. Sayfanın altındaki **Artifacts** bölümünden `GMID-Tool-Kurulum` dosyasını indir (zip içinde `.exe` var).
3. `GMID Tool_x.y.z_x64-setup.exe` dosyasını çalıştır. Yönetici izni istemez; sadece senin kullanıcına kurulur.

Windows SmartScreen "tanınmayan uygulama" uyarısı verebilir (uygulama imzalı değil):
**Ek bilgi → Yine de çalıştır**.

## Faz 1'de neler var?

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
| `src-tauri/` | Çekirdek (Rust): veri klasörü, veritabanı, yedekler, tepsi, pencere |
| `docs/` | Tasarım dokümanı, araştırmalar, görsel taslak |
| `assets/marka/` | Merkut Studios, GMID ve GMID AI Butler logoları |

## Dokümanlar
- [Tasarım Dokümanı](docs/TASARIM.md) — vizyon, modüller, mimari, yol haritası, açık sorular
- [Ek A — Oyun ve film puanlama araştırması](docs/ARASTIRMA-PUANLAMA.md)
- [Ek B — Teknik fizibilite araştırması](docs/ARASTIRMA-TEKNIK.md)
- [Ek C — Oyun içi panel, otomatik bilgi ve galeri araştırması](docs/ARASTIRMA-OVERLAY-METADATA.md)
- [Görsel taslak](docs/taslak/gmid-tool-taslak.html) — tıklanabilir prototip, tarayıcıda açılır
