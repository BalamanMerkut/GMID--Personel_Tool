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
- `GET /api/gundem?arkadaslar=1` (0.8): son 7 günde en çok geçen oyun/film/dizi ve #etiketler `{ad, etiket, paylasim, begeni}` (puan = paylaşım + beğeni); `GET /api/akis?konu=#cs2` ya da `konu=Hades II`: o konuya süzülmüş akış.
- **Yönetici** (0.9): `GMID_YONETICILER=discordId1,discordId2` (`/etc/gmid/sunucu.env`); boşsa sunucuya **ilk giriş yapan kişi** yöneticidir. Yönetici herkesin paylaşımını ve yorumunu silebilir, akışta şikâyet sayısını görür; `POST /api/akis/{id}/sikayet {neden?}` çevrim içi yöneticilere bildirim gönderir. `GET /api/ben` → `yonetici`.
- **Gece yedeği** (0.9): açılışta ve 6 saatte bir bakılır, o günün yedeği yoksa `yedek/gmid-YYYY-AA-GG.db` yazılır (SQLite `VACUUM INTO`, çalışırken tutarlı); en yeni 7 gün kalır. Geri yükleme: `systemctl stop gmid-sunucu`, yedeği `/var/lib/gmid/gmid.db` yerine kopyala (`chown gmid:gmid`), `systemctl start gmid-sunucu`. Resimler ve klipler yedeklenmez.
- **Oyun gecesi** (0.10): `POST /api/gece {oyun, zaman, kontenjan?, not?}` (kişi başı en çok 5 yaklaşan), `GET /api/geceler`, `POST /api/gece/{id}/katil {durum: geliyor|belki|gelemem}` (sınır doluysa 409), `DELETE /api/gece/{id}` (açan ya da yönetici). Akışa `gece` olayı; arkadaşlara `davet`, 15 dk önce `hatirlat` bildirimi (dakikada bir denetlenir). WS `{"tip":"gece"}` değişince.
- **@anma ve profil sekmeleri** (0.11): gönderi/yanıtta `@Ad` geçen üyelere `anma` bildirimi; `GET /api/akis?kisi=&tur=gonderi|klip` ve `?begenen=<id>`.
- **Birlikte dinleme** (0.12, bellekte): `GET /api/dinle` (açık odalar + `simdi`), `POST /api/dinle {baslik, url, kapak?, sure?, konum, caliyor}` (sahip; oda yoksa açılır), `DELETE /api/dinle` (kapat), `POST|DELETE /api/dinle/{sahip}/katil`. WS: `{"tip":"dinle","oda":…,"simdi":…}` / `{"tip":"dinle","kapandi":"<sahip>"}`. Son bağlantısı kapanan kişinin odası kapanır.
- **Dinleme odaları 2.0** (0.13, bellekte, `oda.rs`): `GET /api/odalar` (görebildiğin odalar + `simdi`), `POST /api/oda {ad?, gizlilik?: herkes|arkadaslar, sinir?: 0=sınırsız, yetki?: sahip|dj|herkes}` (kişi başına tek oda; 409), `PATCH /api/oda/{id} {ad?, gizlilik?, sinir?, yetki?, dj?: {kisi, ver}}` (sahip) · `{tekrar?: yok|sarki|liste, karisik?: bool}` (sahip/DJ), `DELETE /api/oda/{id}` (sahip ya da yönetici), `POST|DELETE /api/oda/{id}/katil` (dolu: 409), `POST /api/oda/{id}/sira {baslik, url, kapak?, sure?, video}` (yetkiye göre; sıra en çok 50), `DELETE /api/oda/{id}/sira/{oge}` (sahip/DJ ya da ekleyen), `POST /api/oda/{id}/oynat {konum, caliyor}` (sahip/DJ), `POST /api/oda/{id}/sonraki {oge?, bitti?, sec?}` (sahip/DJ; şarkı bittiyse her üye — aynı öğe iki kez geçilmez; `bitti: true` şarkı tekrarında baştan çaldırır; `sec`: sıradaki o öğeye hemen geç, sahip/DJ), `POST /api/oda/{id}/at/{kisi}` (sahip). WS: `{"tip":"oda","oda":…,"simdi":…}`, `{"tip":"oda","kapandi":"<id>"}`, çıkarılana `{"tip":"oda","atildi":"<id>","ad":…}`. "Arkadaşlarım" odaları yalnız üyelere ve sahibin arkadaşlarına gider. Sahip ayrılınca oda en eski üyeye geçer; boşalınca kapanır; son bağlantısı kapanan kişi odasından çıkar.
- **Oyuncu arıyorum** (0.14): `POST /api/gece {oyun, kontenjan?, not?, hemen: true}` (zaman yok sayılır, şimdi; `kontenjan` açanı da sayar). Kart 2 saat açık kalır (`GET /api/geceler`'de `hemen: true`), hatırlatma gitmez; arkadaşlara `bildirim` `tur: "ariyor"`, "geliyor" diyenin haberi arayana `tur: "varim"`. Eski veritabanına `geceler.hemen` sütunu kendiliğinden eklenir.
- **Sunucuya girdi** (0.14): `POST /api/sunucuya-girdi {sunucu, adres?}` → arkadaşlara `bildirim` `tur: "sunucu"`, `metin`: sunucu adı. Aynı kişi için 10 dakikada bir (bellekte).
- **Anket** (0.14): `POST /api/gonderi {metin, anket: [2–4 seçenek], anket_saat?: 1–168 (24)}` → `veri.anket = {secenekler, bitis}`; akışta sunucu `veri.anket.durum = {oylar: [..], benim}` ekler. `POST /api/akis/{id}/oy {secim}` (fikir değiştirilebilir; bitince 410). Tablo `anket_oylar`.
- **Rozetler** (0.14): `GET /api/profil/{id}` yanıtında `rozetler: [{id, ad, aciklama, zaman, kez}]`. Hesaplananlar: kurucu (ilk 25 üye), sevilen (25+ beğeni) / yildiz (100+), gececi (3+ oyun gecesi), sosyal (10+ arkadaş), yazar (20+ gönderi). Kalıcılar `rozetler` tablosunda (haftanin-klibi, sampiyon).
- **Haftalık özet** (0.14, `hafta.rs`): dakikalık döngüde pazartesi 12:00 (TR) geçtiyse ve o hafta yapılmadıysa (`haftalik` tablosu) geçen 7 günün özetini sistem hesabı `gmid` adına akışa `tur: "ozet"` olarak yazar; en çok beğenilen klibin sahibine `haftanin-klibi` rozeti + `bildirim` `tur: "haftanin-klibi"`. İsteğe bağlı `GMID_DISCORD_WEBHOOK=https://discord.com/api/webhooks/…` (sunucu.env) → özet Discord'a da gider.
- nginx: klip yükleme için `client_max_body_size 64m` gerekir; `kur.sh` yazar, eski kurulumlarda `sudo gmid-guncelle` bir kez çalışınca kendi ekler (yalnız GMID'in site dosyasına, `nginx -t` ile denetleyip).
- WebSocket'te ek: `akis` (akış değişti), `mesaj`, `okundu`, `bildirim` (paylaşımıma beğeni/yorum)
- Oyuna girince akışa "oynuyor" (aynı oyun 12 saatte bir)

## 7. Sıradaki adımlar

1. **GMID Tool tarafı:** Kontrol Odası'nda "Discord ile giriş", yan menüde **Arkadaşlar** (kim çevrim içi, kim ne
   oynuyor); oyun algılandığında durum kendiliğinden sunucuya gider (istenirse kapatılır).
2. Sohbet penceresi, ortak puan arşivi, oyun gecesi planlayıcı (§8 TASARIM.md).
