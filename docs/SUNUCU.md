# GMID Sunucusu · Kurulum Rehberi

GMID Tool'un çevrim içi kısmı (kim çevrim içi, kim ne oynuyor; ileride sohbet, ortak puan arşivi, oyun gecesi)
bu sunucuda çalışır. **GMID Tool sunucu olmadan da tam çalışır**: sunucu kapanırsa yalnız çevrim içi özellikler
durur, notların, oyunların, ayarların hep senin bilgisayarında kalır.

- Sunucu programı: `sunucu/` klasörü (Rust). Her derlemede GitHub bunu Linux için hazırlar ve sürüme ekler
  (`gmid-sunucu-linux-x64`).
- Stabil yedek (2026-11-01): GitHub'da **derleme 93** sürümü (kurulum dosyası) + kaynak kod zip'i.
- GMID Tool'un güncellemeleri eskisi gibi GitHub'dan gelir; sunucu bununla ilgilenmez.

---

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

Sunucuda şunu yapıştır (`gmid.duckdns.org` yerine kendi adını yaz):

```
curl -fsSL https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/sunucu/kurulum/kur.sh | sudo bash -s -- gmid.duckdns.org
```

Ne yapar:
- GMID sunucu programını indirir, `gmid` adlı ayrı bir kullanıcıyla arka planda çalıştırır (çökerse kendiliğinden
  yeniden başlar, sunucu yeniden açılınca kendiliğinden açılır).
- **Caddy** kurar: https sertifikasını (Let's Encrypt) kendisi alır ve yeniler.
- Güvenlik duvarında yalnız SSH (22), 80 ve 443 açık kalır.
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
| Güncelle (en son sürüm; açılmazsa eskisine döner) | `sudo gmid-guncelle` |
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

## 7. Sıradaki adımlar

1. **GMID Tool tarafı:** Kontrol Odası'nda "Discord ile giriş", yan menüde **Arkadaşlar** (kim çevrim içi, kim ne
   oynuyor); oyun algılandığında durum kendiliğinden sunucuya gider (istenirse kapatılır).
2. Sohbet penceresi, ortak puan arşivi, oyun gecesi planlayıcı (§8 TASARIM.md).
