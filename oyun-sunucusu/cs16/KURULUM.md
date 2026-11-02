# GMID CS 1.6 sunucusu — kurulum (sunucuda çalıştırılır)

Tek sunucuda üç mod, **harita adına göre kendiliğinden** seçilir (`gmid_mod` eklentisi):

| Harita | Mod | Bunny hop | Can |
|---|---|---|---|
| `jail_*`, `jb_*` | JailBreak (`gmid_jail`: komutan, hücre aç, serbest gün, son istek) | kapalı | normal |
| `35hp_*` | 35 HP | **açık** (boşluğu basılı tut), `sv_airaccelerate 100` | 35, zırh yok |
| diğerleri (`fy_iceworld` …) | Pro Public | **kapalı**, `sv_airaccelerate 10` | normal |

## Kurallar (sunucudaki diğer işler)

- Sunucuda başka bir proje (Mola, molayazilim.com.tr) var: **ona, nginx/Caddy ayarlarına ve başka hizmetlere dokunma.**
- Her şey `/opt/gmid-cs16` altında, ayrı `cs16` kullanıcısıyla, `gmid-cs16` hizmetiyle çalışır. Bellek en çok 768 MB, 1 çekirdek, düşük öncelik.
- Güvenlik duvarı (ufw) açıksa yalnız 27015 (UDP+TCP) açılır; kapalıysa açılmaz. Sağlayıcının panelinde ayrı bir güvenlik duvarı varsa 27015 UDP orada da açık olmalı.

## 1. Kur

```
curl -fsSL https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/oyun-sunucusu/cs16/kur.sh | sudo bash
```

Betik: SteamCMD ile HLDS (uygulama 90, cstrike) → ReHLDS + ReGameDLL + Metamod-R (GitHub'daki son sürümler) → AMX Mod X 1.10 → `gmid_mod` ve `gmid_jail` eklentilerini **sunucuda derler** → `server.cfg`, harita sırası, `gmid-cs16` hizmeti.

Eklentilerden biri derlenmezse betik hatayı gösterip durur; hata satırına göre `.sma` düzeltilip betik yeniden çalıştırılabilir.

## 2. Haritalar

Oyunla gelmeyen haritalar (`haritalar.txt`): `jail_xmf`, `jail_ms_shawshank`, `jail_alcatraz`, `35hp_2`, `35hp_rmx`, `fy_iceworld`, `fy_iceworld2k`.

1. Bunların `.bsp` dosyalarını (varsa aynı paketteki `.wad`, `gfx/`, `sound/`, `models/` dosyalarıyla) güvenilir bir kaynaktan (ör. GameBanana) indir.
2. Dosyaları `/opt/gmid-cs16/hlds/cstrike/` altına koy (`maps/*.bsp`, `.wad` dosyaları cstrike'ın köküne).
3. `chown -R cs16:cs16 /opt/gmid-cs16` ve kur.sh'yi yeniden çalıştır. Harita sırası, bulunan haritalardan yeniden yazılır.

Bulunamayan harita atlanır. Hiçbiri yoksa sunucu oyunla gelen haritalarla açılır.

## 3. Yönetici (isteğe bağlı)

```
GMID_CS_YONETICI="STEAM_0:1:12345678" sudo -E bash kur.sh
```

Yönetici, komutan komutlarını (`/ac`, `/fd`) da kullanabilir ve `amx_` komutlarına tam yetkiye sahiptir.

## Günlük işler

- Durum: `systemctl status gmid-cs16`
- Günlük: `journalctl -u gmid-cs16 -n 50`
- Yeniden başlat: `systemctl restart gmid-cs16`
- Rcon şifresi: `/etc/gmid/cs16-rcon` (yalnız root okur)
- Bağlan: oyunda konsol → `connect gmid.molayazilim.com.tr:27015`

## Oyunda komutlar

- Herkes: `/mod` (bu haritanın modu)
- JailBreak: `/simon` (komutan ol), `/ac` (hücreleri aç), `/fd` (serbest gün ver), `/lr` (son mahkûm: bıçak ya da deagle düellosu), `/kurallar`
