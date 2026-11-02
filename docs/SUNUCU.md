# GMID Sunucusu · Kurulum Rehberi

GMID Tool'un çevrim içi kısmı (kim çevrim içi, kim ne oynuyor; ileride sohbet, ortak puan arşivi, oyun gecesi)
bu sunucuda çalışır. **GMID Tool sunucu olmadan da tam çalışır**: sunucu kapanırsa yalnız çevrim içi özellikler
durur, notların, oyunların, ayarların hep senin bilgisayarında kalır.

- Sunucu programı: `sunucu/` klasörü (Rust). Her derlemede GitHub bunu Linux için hazırlar ve sürüme ekler
  (`gmid-sunucu-linux-x64`).
- Stabil yedek (2026-11-01): GitHub'da **derleme 93** sürümü (kurulum dosyası) + kaynak kod zip'i.
- GMID Tool'un güncellemeleri eskisi gibi GitHub'dan gelir; sunucu bununla ilgilenmez.

---

## 0. Senin sunucun (2026-11-01)

- IP: `31.57.187.132`, Ubuntu. Üzerinde başka bir proje ve `molayazilim.com.tr` çalışıyor: kurulum ona dokunmaz
  (web sunucusunu algılar, yalnız GMID'in alt alan adı için ayrı bölüm ekler, önce denetler).
- Alan adı: `gmid.molayazilim.com.tr` → alan adı panelinde **A kaydı**: ad `gmid`, değer `31.57.187.132`
  (Cloudflare kullanıyorsan bulut simgesi gri "DNS only" olsun). DuckDNS gerekmez (§1'i atla).

## 1. Ücretsiz alan adı (DuckDNS, 2 dakika)

Sunucuyla GMID arasındaki bağlantı şifreli (https) olsun diye bir ad gerekiyor. DuckDNS sunucunda çalışan bir
program değil, yalnız bir isim kaydı: "gmid.duckdns.org" → sunucunun IP adresi.

1. <https://www.duckdns.org> → sağ üstten Google ya da GitHub ile giriş.
2. **sub domain** kutusuna bir ad yaz (ör. `gmid`) → **add domain**.
3. Satırdaki **current ip** kutusuna sunucunun IP adresini yaz (sunucuyu aldığın sitenin panelinde yazar) →
   **update ip**.

Bundan sonra adın `gmid.duckdns.org` olur (seçtiğin ada göre).

## 2. Sunucuya bağlan

Windows'ta **Terminal** (ya da PowerShell) aç:

```
ssh root@SUNUCU_IP
```

İlk bağlantıda "yes" yaz, sonra sunucu sitesinin verdiği şifreyi gir (yazarken görünmez, normal).
Kullanıcı adın root değilse (ör. `ubuntu`) `ssh ubuntu@SUNUCU_IP` yaz.

## 3. Tek komutla kurulum

Önce yalnız kontrol (hiçbir şey değiştirmez; ne bulduğunu yazar):

```
curl -fsSL https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/sunucu/kurulum/kur.sh | sudo bash -s -- --kontrol gmid.molayazilim.com.tr
```

"Kontrol tamam" yazınca kurulum (`gmid.molayazilim.com.tr` yerine kendi adını yaz):

```
curl -fsSL https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/sunucu/kurulum/kur.sh | sudo bash -s -- gmid.molayazilim.com.tr
```

Ne yapar:
- GMID sunucu programını indirir, `gmid` adlı ayrı bir kullanıcıyla arka planda çalıştırır (çökerse kendiliğinden
  yeniden başlar, sunucu yeniden açılınca kendiliğinden açılır).
- https: sunucuda **nginx** varsa ona yalnız GMID için ayrı bir bölüm ekler (`nginx -t` ile denetler, hata varsa geri
  alır) ve certbot ile yalnız bu alt alan adına sertifika alır; **Caddy** varsa Caddyfile'ın sonuna bölüm ekler (yedekler,
  denetler); hiçbiri yoksa Caddy kurar. Apache ya da tanımadığı bir program 80/443'teyse hiçbir şey yapmaz, söyler.
- Güvenlik duvarı kapalıysa açmaz (başka projenin portlarını kesmesin); açıksa yalnız 80/443 izni ekler.
- Her gece veritabanını yedekler (`/var/lib/gmid/yedek`, son 14 gün).

Sonunda "Tamam. GMID sunucusu çalışıyor" yazar. Tarayıcıda `https://gmid.duckdns.org/saglik` açınca
`{"durum":"ok", …}` görmelisin.

## 4. Discord ile giriş (bot birleştirilirken)

Herkes Discord hesabıyla girer (şifre yok); yalnız GMID Discord sunucusunun üyeleri. Birleşik bot için açılacak
Discord uygulaması bunun için de kullanılır:

1. <https://discord.com/developers/applications> → uygulama (yoksa **New Application**).
2. **OAuth2** sayfası → **Client ID** ve **Client Secret** (Reset Secret) kopyala.
3. Aynı sayfada **Redirects** → **Add Redirect** → `http://127.0.0.1:53682/giris` → Save.
4. GMID Discord sunucusunun kimliği: Discord → Ayarlar → Gelişmiş → Geliştirici Modu açık; sunucunun adına sağ tık
   → **Sunucu Kimliğini Kopyala**.
5. Sunucuda:
   ```
   sudo nano /etc/gmid/sunucu.env
   ```
   `DISCORD_CLIENT_ID=`, `DISCORD_CLIENT_SECRET=`, `DISCORD_SUNUCU_ID=` satırlarını doldur, Ctrl+O, Enter, Ctrl+X.
   ```
   sudo systemctl restart gmid-sunucu
   ```
   `https://gmid.duckdns.org/saglik` artık `"giris": true` gösterir.

> Client Secret bir şifredir: kimseyle paylaşma, ekran görüntüsüne çıkmasın. Yalnız bu dosyada durur
> (yalnız root okuyabilir).

## 5. Günlük işler

| Ne | Komut |
|----|-------|
| Güncelleme | **Kendiliğinden:** her gece ~04:30 GitHub'a bakar, yeni sürüm varsa kurar (yoksa dokunmaz) |
| Hemen güncelle (açılmazsa eskisine döner) | `sudo gmid-guncelle` |
| Gece güncellemesi ne zaman çalıştı? | `systemctl list-timers gmid-guncelle` · `journalctl -u gmid-guncelle -n 20` |
| Gece güncellemesini kapat | `sudo systemctl disable --now gmid-guncelle.timer` |
| Çalışıyor mu? | `systemctl status gmid-sunucu` |
| Günlük (son 50 satır) | `journalctl -u gmid-sunucu -n 50` |
| Yeniden başlat | `sudo systemctl restart gmid-sunucu` |
| Yedekler | `ls /var/lib/gmid/yedek` |

## 6. Teknik özet

- Program yalnız `127.0.0.1:8780`'den dinler; dışarıya Caddy (https) açar.
- Veritabanı: SQLite, `/var/lib/gmid/gmid.db`. Oturum anahtarlarının kendisi değil özeti (SHA-256) saklanır;
  90 gün kullanılmayan oturum geçersiz olur.
- systemd güvenliği: program yalnız kendi veri klasörüne yazabilir (`ProtectSystem=strict`, `NoNewPrivileges` …).
- Uçlar:
  - `GET /saglik` — durum, sürüm, giriş açık mı, kaç kişi çevrim içi
  - `POST /api/giris` `{kod, donus}` — Discord kodu → `{anahtar, kullanici}` (dönüş adresi yalnız 127.0.0.1/localhost)
  - `GET /api/ben`, `POST /api/cikis`, `GET /api/cevrimici` — `Authorization: Bearer <anahtar>`
  - `GET /ws` (WebSocket) — ilk mesaj `{"tip":"merhaba","anahtar":"…"}`; sonra `{"tip":"durum","oyun":"CS2"|null}`.
    Sunucudan: `liste`, `geldi`, `gitti`, `durum`, `hata`.

### Sosyal uçlar (0.2)

- `GET /api/kisiler` — kayıtlı herkes (çevrim içi mi, ne oynuyor)
- `PUT /api/profil` `{oyunlar:[{ad,kapak,dakika,son}], puanlar:[{anahtar,tur,ad,kapak,puan,yorum,durum,yil,guncelleme}]}` —
  kişinin paylaşmayı seçtikleri; son 7 günde yeni/değişen puanlar akışa düşer (ilk paylaşımda yalnız son 2 gün, en çok 5)
- `GET /api/profil/{id}` — profil (oyunlar, puanlar, çevrim içi, oyun)
- `GET /api/akis?once=<ms>&kisi=<id>` — son olaylar (puanladı / oynuyor), beğeni sayısı, beğendim mi, yorumlar
- `POST /api/akis/{id}/begen` (aç/kapa), `POST /api/akis/{id}/yorum {metin}`, `DELETE /api/akis/{id}` (kendi),
  `DELETE /api/yorum/{id}` (kendi yorumu ya da kendi paylaşımının altındaki)
- `GET /api/konusmalar`, `GET /api/mesaj/{id}?once=<mesaj id>` (okundu yapar), `POST /api/mesaj/{id} {metin}`
- `POST /api/resim` (≤400 KB JPEG/PNG/WebP/GIF, gövde = resim) → `{url:"/resim/<sha256>.jpg"}`; `GET /resim/{ad}`
- `POST /api/klip?ad=&oyun=&sure=&kapak=` (0.5; gövde = MP4, ≤60 MB) → akışa `klip` olayı, `{id, url:"/klip/<sha256>.mp4", bitis}`; `GET /klip/{ad}` (Range ile ileri sarılır). Klipler **7 gün** sonra video ve paylaşım olarak silinir (saatte bir denetlenir); başkalarından aldığı beğeni/yorumlar `kalici_puan` tablosunda kalır, `GET /api/profil/{id}` → `begeni`, `yorum`. Kişi başı aynı anda en çok 10 klip, toplam 5 GB.
- `POST /api/gonderi {metin, resim?}` (0.7): serbest gönderi, en çok 500 harf; `resim` önce `/api/resim` ile yüklenmiş "/resim/…" olmalı; 10 dakikada en çok 10 gönderi → akışta `gonderi` olayı (süresiz).
- nginx: klip yükleme için `client_max_body_size 64m` gerekir; `kur.sh` yazar, eski kurulumlarda `sudo gmid-guncelle` bir kez çalışınca kendi ekler (yalnız GMID'in site dosyasına, `nginx -t` ile denetleyip).
- WebSocket'te ek: `akis` (akış değişti), `mesaj`, `okundu`, `bildirim` (paylaşımıma beğeni/yorum)
- Oyuna girince akışa "oynuyor" (aynı oyun 12 saatte bir)

## 7. Sıradaki adımlar

1. **GMID Tool tarafı:** Kontrol Odası'nda "Discord ile giriş", yan menüde **Arkadaşlar** (kim çevrim içi, kim ne
   oynuyor); oyun algılandığında durum kendiliğinden sunucuya gider (istenirse kapatılır).
2. Sohbet penceresi, ortak puan arşivi, oyun gecesi planlayıcı (§8 TASARIM.md).
