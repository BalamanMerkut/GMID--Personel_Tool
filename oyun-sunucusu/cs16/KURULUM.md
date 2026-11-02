# GMID CS 1.6 sunucusu — kurulum (sunucuda çalıştırılır)

**Pro Public** sunucusu: klasik kurallar, eklenti yok (JailBreak'ten vazgeçildi). Harita sırası (`haritalar.txt`):
`35hp_2_s_arena`, `fy_iceworld16`, `awp_india`, `de_dust2` — **her harita 45 dakika** (`mp_timelimit 45`).

## Kurallar (sunucudaki diğer işler)

- Sunucuda başka bir proje (Mola, molayazilim.com.tr) var: **ona, nginx/Caddy ayarlarına ve başka hizmetlere dokunma.**
- Her şey `/opt/gmid-cs16` altında, ayrı `cs16` kullanıcısıyla, `gmid-cs16` hizmetiyle çalışır. Bellek en çok 768 MB, 1 çekirdek, düşük öncelik.
- Güvenlik duvarı (ufw) açıksa yalnız 27015 (UDP+TCP) açılır; kapalıysa açılmaz.
- **Başka bir kurulum (ör. Valheim) sürerken `kur.sh` çalıştırma.** Sunucu şu an çalışır durumda; küçük ayar değişiklikleri
  (ör. harita süresi) için betiği yeniden çalıştırmak yerine `server.cfg` elle düzenlenip hizmet yeniden başlatılır (aşağıda).

## 1. Kur (yalnız sıfırdan kurulum için)

```
curl -fsSL https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/oyun-sunucusu/cs16/kur.sh | sudo bash
```

Betik: SteamCMD ile HLDS (uygulama 90, cstrike, `-beta steam_legacy`) → ReHLDS + ReGameDLL + Metamod-R → AMX Mod X 1.10 →
eski GMID eklentilerini (varsa) kaldırır → `server.cfg`, harita sırası, `gmid-cs16` hizmeti.

## 2. Haritalar

Oyunla gelmeyen haritaların `.bsp` dosyaları (varsa `.wad`, `gfx/`, `sound/`, `models/` ile) `/opt/gmid-cs16/hlds/cstrike/`
altına konur, `chown -R cs16:cs16 /opt/gmid-cs16` yapılır. Bulunamayan harita sıradan atlanır.

## 3. Harita süresini değiştirme (betiksiz)

```
sudo sed -i 's/^mp_timelimit .*/mp_timelimit 45/' /opt/gmid-cs16/hlds/cstrike/server.cfg
sudo systemctl restart gmid-cs16
```

## Günlük işler

- Durum: `systemctl status gmid-cs16`
- Günlük: `journalctl -u gmid-cs16 -n 50`
- Yeniden başlat: `systemctl restart gmid-cs16`
- Rcon şifresi: `/etc/gmid/cs16-rcon` (yalnız root okur)
- Bağlan: oyunda konsol → `connect gmid.molayazilim.com.tr:27015`
