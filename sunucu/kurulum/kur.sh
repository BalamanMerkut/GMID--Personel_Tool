#!/usr/bin/env bash
# GMID sunucusu kurulumu (Ubuntu 22.04 / 24.04). Başka projelerin çalıştığı sunucuda da güvenle kurulur:
#   - Mevcut web sunucusunun (nginx / Caddy) ayarlarının ÜZERİNE YAZILMAZ; yalnız GMID'in alan adı için ayrı bir
#     bölüm eklenir, önce denetlenir (nginx -t / caddy validate), hata varsa geri alınır.
#   - Başka projelerin dosyalarına, portlarına ve güvenlik duvarı kurallarına dokunulmaz (güvenlik duvarı kapalıysa
#     açılmaz; açıksa yalnız 80/443 izni eklenir).
#   - GMID ayrı bir kullanıcıyla, yalnız kendi klasörlerinde çalışır: /opt/gmid, /var/lib/gmid, /etc/gmid.
#
# Önce yalnız kontrol (hiçbir şey değiştirmez):
#   curl -fsSL https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/sunucu/kurulum/kur.sh | sudo bash -s -- --kontrol gmid.ornek.com
# Kurulum:
#   curl -fsSL https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/sunucu/kurulum/kur.sh | sudo bash -s -- gmid.ornek.com
# Yeniden çalıştırmak güvenlidir (güncelleme gibi davranır).
set -euo pipefail

KONTROL=0
if [ "${1:-}" = "--kontrol" ]; then KONTROL=1; shift; fi
ALAN="${1:-}"
PORT=8780
DEPO="BalamanMerkut/GMID--Personel_Tool"
PROGRAM_URL="https://github.com/${DEPO}/releases/latest/download/gmid-sunucu-linux-x64"
HAM="https://raw.githubusercontent.com/${DEPO}/${GMID_DAL:-HEAD}/sunucu/kurulum"

[ "$(id -u)" -eq 0 ] || { echo "root olarak çalıştır (sudo)."; exit 1; }
[ -n "$ALAN" ] || { echo "Alan adı ver: ... | sudo bash -s -- gmid.ornek.com"; exit 1; }
[[ "$ALAN" =~ ^[A-Za-z0-9.-]+\.[A-Za-z]{2,}$ ]] || { echo "Geçersiz alan adı: $ALAN"; exit 1; }

etkin() { systemctl is-active --quiet "$1" 2>/dev/null; }
port_sahibi() { ss -tlnpH "sport = :$1" 2>/dev/null | sed -n 's/.*users:(("\([^"]*\)".*/\1/p' | head -1 || true; }

# ---------- Durum ----------
WEB=""
if etkin nginx; then WEB=nginx
elif etkin caddy; then WEB=caddy
elif etkin apache2; then WEB=apache
fi
P80="$(port_sahibi 80)"; P443="$(port_sahibi 443)"; PG="$(port_sahibi $PORT)"
SUNUCU_IP="$(curl -fsS4 --max-time 5 https://api.ipify.org 2>/dev/null || true)"
ALAN_IP="$(getent ahostsv4 "$ALAN" 2>/dev/null | awk 'NR==1{print $1}' || true)"

echo "== GMID kurulum kontrolü =="
echo "Sistem      : $(. /etc/os-release && echo "$PRETTY_NAME") · $(uname -m)"
echo "Disk (/)    : $(df -h / | awk 'NR==2{print $4" boş / "$2}')"
echo "Bellek      : $(free -h | awk '/Mem:/{print $7" boş / "$2}')"
echo "Web sunucusu: ${WEB:-yok}  (80: ${P80:-boş} · 443: ${P443:-boş})"
echo "GMID portu  : $PORT → ${PG:-boş}"
echo "Güvenlik d. : $(ufw status 2>/dev/null | head -1 || echo 'ufw yok')"
echo "Alan adı    : $ALAN → ${ALAN_IP:-çözülmedi} (sunucu: ${SUNUCU_IP:-?})"
if etkin gmid-sunucu; then echo "GMID        : zaten kurulu ve çalışıyor (güncellenecek)"; fi

SORUN=0
if [ -n "$PG" ] && [ "$PG" != "gmid-sunucu" ]; then
  echo "SORUN: $PORT portunu '$PG' kullanıyor."; SORUN=1
fi
if [ "$WEB" = "apache" ]; then
  echo "SORUN: Apache otomatik ayarlanmıyor; elle ayar gerekir (bana yaz)."; SORUN=1
fi
if [ -z "$WEB" ] && { [ -n "$P80" ] || [ -n "$P443" ]; }; then
  echo "SORUN: 80/443 portunu tanımadığım bir program kullanıyor (${P80:-}${P443:+ / $P443}); ona dokunmuyorum (bana yaz)."; SORUN=1
fi
# Cloudflare gibi bir vekil arkasındaysa IP farklı görünür: GMID_DNS_ATLA=1 ile bu denetim atlanır.
if [ "${GMID_DNS_ATLA:-0}" != 1 ] && { [ -z "$ALAN_IP" ] || { [ -n "$SUNUCU_IP" ] && [ "$ALAN_IP" != "$SUNUCU_IP" ]; }; }; then
  echo "SORUN: $ALAN bu sunucuyu göstermiyor. Alan adı panelinde A kaydı ekle: ${ALAN%%.*} → ${SUNUCU_IP:-sunucu IP adresi}"; SORUN=1
fi
if [ "$KONTROL" = 1 ]; then
  [ "$SORUN" = 0 ] && echo "Kontrol tamam: kurulum yapılabilir (hiçbir şey değiştirilmedi)."
  exit 0
fi
[ "$SORUN" = 0 ] || { echo "Kurulum yapılmadı; yukarıdaki sorun çözülünce yeniden çalıştır."; exit 1; }

# ---------- GMID programı ----------
echo "==> GMID programı"
export DEBIAN_FRONTEND=noninteractive
command -v sqlite3 >/dev/null || { apt-get update -q; apt-get install -y -q sqlite3; }
id gmid >/dev/null 2>&1 || useradd --system --home /var/lib/gmid --shell /usr/sbin/nologin gmid
install -d -o gmid -g gmid -m 750 /var/lib/gmid /var/lib/gmid/yedek
install -d -m 755 /opt/gmid
install -d -m 750 /etc/gmid
if [ ! -f /etc/gmid/sunucu.env ]; then
  cat > /etc/gmid/sunucu.env <<ENV
# GMID sunucusu ayarları. Değiştirince: systemctl restart gmid-sunucu
GMID_ADRES=127.0.0.1:$PORT
GMID_VERITABANI=/var/lib/gmid/gmid.db
# Discord Developer Portal → uygulaman → OAuth2 (doldurulana kadar giriş kapalı kalır)
DISCORD_CLIENT_ID=
DISCORD_CLIENT_SECRET=
# GMID Discord sunucusunun kimliği (yalnız üyeler girebilsin)
DISCORD_SUNUCU_ID=
ENV
fi
chmod 600 /etc/gmid/sunucu.env
curl -fsSL "$PROGRAM_URL" -o /opt/gmid/gmid-sunucu.yeni
chmod 755 /opt/gmid/gmid-sunucu.yeni
mv -f /opt/gmid/gmid-sunucu.yeni /opt/gmid/gmid-sunucu
curl -fsSL "$HAM/gmid-sunucu.service" -o /etc/systemd/system/gmid-sunucu.service
curl -fsSL "$HAM/guncelle.sh" -o /usr/local/bin/gmid-guncelle
chmod 755 /usr/local/bin/gmid-guncelle
cat > /etc/cron.daily/gmid-yedek <<'CRON'
#!/bin/sh
# GMID veritabanının gecelik yedeği (son 14 gün)
D=/var/lib/gmid/yedek
[ -f /var/lib/gmid/gmid.db ] || exit 0
sqlite3 /var/lib/gmid/gmid.db ".backup '$D/gmid-$(date +%F).db'" && chown gmid:gmid "$D"/*.db
find "$D" -name 'gmid-*.db' -mtime +14 -delete
CRON
chmod 755 /etc/cron.daily/gmid-yedek
systemctl daemon-reload
systemctl enable --now gmid-sunucu >/dev/null 2>&1
systemctl restart gmid-sunucu
sleep 2
curl -fsS "http://127.0.0.1:$PORT/saglik" >/dev/null || { echo "GMID açılmadı: journalctl -u gmid-sunucu -n 50"; exit 1; }

# ---------- https ----------
echo "==> https ($ALAN, web sunucusu: ${WEB:-yok → Caddy kurulacak})"
case "$WEB" in
  nginx)
    # Ubuntu düzeni (sites-enabled) yoksa conf.d kullanılır.
    if grep -rqs "sites-enabled" /etc/nginx/nginx.conf; then
      CONF=/etc/nginx/sites-available/gmid-sunucu; ETKIN=/etc/nginx/sites-enabled/gmid-sunucu
    else
      CONF=/etc/nginx/conf.d/gmid-sunucu.conf; ETKIN=""
    fi
    cat > "$CONF" <<NGINX
# GMID sunucusu (kur.sh ekledi; yalnız $ALAN için)
map \$http_upgrade \$gmid_baglanti { default upgrade; '' close; }
server {
    listen 80;
    listen [::]:80;
    server_name $ALAN;
    location / {
        proxy_pass http://127.0.0.1:$PORT;
        proxy_http_version 1.1;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection \$gmid_baglanti;
        proxy_set_header Host \$host;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        proxy_read_timeout 3600s;
    }
}
NGINX
    [ -n "$ETKIN" ] && ln -sf "$CONF" "$ETKIN"
    if ! nginx -t 2>/dev/null; then
      rm -f "${ETKIN:-$CONF}"
      echo "nginx ayarı denetimden geçmedi; GMID bölümü geri alındı, sitene dokunulmadı."; nginx -t || true; exit 1
    fi
    systemctl reload nginx
    command -v certbot >/dev/null || apt-get install -y -q certbot python3-certbot-nginx
    # Yalnız bu alan adının sertifikası; certbot yalnız GMID bölümüne https ekler.
    certbot --nginx -d "$ALAN" --non-interactive --agree-tos --register-unsafely-without-email --redirect
    ;;
  caddy)
    F=/etc/caddy/Caddyfile
    if ! grep -q "^$ALAN " "$F" 2>/dev/null; then
      cp "$F" "$F.gmid-yedek-$(date +%s)"
      printf '\n# GMID sunucusu (kur.sh ekledi)\n%s {\n\tencode gzip\n\treverse_proxy 127.0.0.1:%s\n}\n' "$ALAN" "$PORT" >> "$F"
      if ! caddy validate --config "$F" --adapter caddyfile >/dev/null 2>&1; then
        cp "$(ls -t "$F".gmid-yedek-* | head -1)" "$F"
        echo "Caddy ayarı denetimden geçmedi; eski haline döndürüldü."; exit 1
      fi
    fi
    systemctl reload caddy
    ;;
  "")
    apt-get install -y -q debian-keyring debian-archive-keyring apt-transport-https gnupg curl
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor --yes -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' > /etc/apt/sources.list.d/caddy-stable.list
    apt-get update -q; apt-get install -y -q caddy
    printf '%s {\n\tencode gzip\n\treverse_proxy 127.0.0.1:%s\n}\n' "$ALAN" "$PORT" > /etc/caddy/Caddyfile
    systemctl enable caddy >/dev/null 2>&1; systemctl restart caddy
    ;;
esac

if ufw status 2>/dev/null | grep -q "Status: active"; then
  ufw allow 80/tcp >/dev/null; ufw allow 443/tcp >/dev/null
fi

sleep 3
if curl -fsS "https://$ALAN/saglik" >/dev/null 2>&1; then
  echo; echo "Tamam. GMID sunucusu çalışıyor: https://$ALAN/saglik"
else
  echo; echo "GMID çalışıyor; https birkaç dakika içinde hazır olur: https://$ALAN/saglik"
fi
echo "Discord girişi için: sudo nano /etc/gmid/sunucu.env  sonra  sudo systemctl restart gmid-sunucu"
echo "Güncellemek için: sudo gmid-guncelle"
