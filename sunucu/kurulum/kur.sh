#!/usr/bin/env bash
# GMID sunucusu kurulumu (Ubuntu 22.04 / 24.04). Sunucuda root olarak:
#   curl -fsSL https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/sunucu/kurulum/kur.sh | sudo bash -s -- gmid.duckdns.org
# Ne yapar: GMID sunucu programını indirir, "gmid" adlı ayrı bir kullanıcıyla systemd hizmeti olarak çalıştırır,
# Caddy ile https (Let's Encrypt sertifikası, kendiliğinden yenilenir) açar, güvenlik duvarında yalnız SSH, 80 ve
# 443'ü açık bırakır, her gece veritabanını yedekler. Yeniden çalıştırmak güvenlidir (güncelleme gibi davranır).
set -euo pipefail

ALAN="${1:-}"
DEPO="BalamanMerkut/GMID--Personel_Tool"
PROGRAM_URL="https://github.com/${DEPO}/releases/latest/download/gmid-sunucu-linux-x64"
HAM="https://raw.githubusercontent.com/${DEPO}/${GMID_DAL:-HEAD}/sunucu/kurulum"

[ "$(id -u)" -eq 0 ] || { echo "root olarak çalıştır: sudo bash kur.sh <alan-adı>"; exit 1; }
if [ -z "$ALAN" ]; then
  read -rp "Alan adı (ör. gmid.duckdns.org): " ALAN </dev/tty
fi
[[ "$ALAN" =~ ^[A-Za-z0-9.-]+$ ]] || { echo "Geçersiz alan adı: $ALAN"; exit 1; }

echo "==> Paketler"
export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y -q curl ca-certificates debian-keyring debian-archive-keyring apt-transport-https gnupg ufw sqlite3

if ! command -v caddy >/dev/null; then
  echo "==> Caddy (https)"
  curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor --yes -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
  curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' > /etc/apt/sources.list.d/caddy-stable.list
  apt-get update -q
  apt-get install -y -q caddy
fi

echo "==> Kullanıcı ve klasörler"
id gmid >/dev/null 2>&1 || useradd --system --home /var/lib/gmid --shell /usr/sbin/nologin gmid
install -d -o gmid -g gmid -m 750 /var/lib/gmid /var/lib/gmid/yedek
install -d -m 755 /opt/gmid
install -d -m 750 /etc/gmid
if [ ! -f /etc/gmid/sunucu.env ]; then
  cat > /etc/gmid/sunucu.env <<ENV
# GMID sunucusu ayarları. Değiştirince: systemctl restart gmid-sunucu
GMID_ADRES=127.0.0.1:8780
GMID_VERITABANI=/var/lib/gmid/gmid.db
# Discord Developer Portal → uygulaman → OAuth2 (doldurulana kadar giriş kapalı kalır)
DISCORD_CLIENT_ID=
DISCORD_CLIENT_SECRET=
# GMID Discord sunucusunun kimliği (yalnız üyeler girebilsin)
DISCORD_SUNUCU_ID=
ENV
fi
chmod 600 /etc/gmid/sunucu.env

echo "==> Program"
curl -fsSL "$PROGRAM_URL" -o /opt/gmid/gmid-sunucu.yeni
chmod 755 /opt/gmid/gmid-sunucu.yeni
mv -f /opt/gmid/gmid-sunucu.yeni /opt/gmid/gmid-sunucu
curl -fsSL "$HAM/gmid-sunucu.service" -o /etc/systemd/system/gmid-sunucu.service
curl -fsSL "$HAM/guncelle.sh" -o /usr/local/bin/gmid-guncelle
chmod 755 /usr/local/bin/gmid-guncelle

echo "==> Gece yedeği (son 14 gün)"
cat > /etc/cron.daily/gmid-yedek <<'CRON'
#!/bin/sh
D=/var/lib/gmid/yedek
[ -f /var/lib/gmid/gmid.db ] || exit 0
sqlite3 /var/lib/gmid/gmid.db ".backup '$D/gmid-$(date +%F).db'" && chown gmid:gmid "$D"/*.db
find "$D" -name 'gmid-*.db' -mtime +14 -delete
CRON
chmod 755 /etc/cron.daily/gmid-yedek

echo "==> https ($ALAN)"
cat > /etc/caddy/Caddyfile <<CADDY
$ALAN {
	encode gzip
	reverse_proxy 127.0.0.1:8780
}
CADDY

echo "==> Güvenlik duvarı"
ufw allow OpenSSH >/dev/null
ufw allow 80/tcp >/dev/null
ufw allow 443/tcp >/dev/null
ufw --force enable >/dev/null

systemctl daemon-reload
systemctl enable --now gmid-sunucu
systemctl restart gmid-sunucu
systemctl enable caddy >/dev/null
systemctl reload caddy || systemctl restart caddy

sleep 2
if curl -fsS http://127.0.0.1:8780/saglik >/dev/null; then
  echo
  echo "Tamam. GMID sunucusu çalışıyor: https://$ALAN/saglik"
  echo "Discord girişini açmak için: nano /etc/gmid/sunucu.env  sonra  systemctl restart gmid-sunucu"
  echo "Güncellemek için: sudo gmid-guncelle"
else
  echo "Sunucu yanıt vermedi. Günlük: journalctl -u gmid-sunucu -n 50"
  exit 1
fi
