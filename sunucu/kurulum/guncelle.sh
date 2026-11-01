#!/usr/bin/env bash
# GMID sunucusunu en son sürüme günceller (ayarlar ve veritabanı korunur). Kullanım: sudo gmid-guncelle
set -euo pipefail
[ "$(id -u)" -eq 0 ] || { echo "sudo ile çalıştır"; exit 1; }
URL="https://github.com/BalamanMerkut/GMID--Personel_Tool/releases/latest/download/gmid-sunucu-linux-x64"
curl -fsSL "$URL" -o /opt/gmid/gmid-sunucu.yeni
chmod 755 /opt/gmid/gmid-sunucu.yeni
cp -f /opt/gmid/gmid-sunucu /opt/gmid/gmid-sunucu.onceki 2>/dev/null || true
mv -f /opt/gmid/gmid-sunucu.yeni /opt/gmid/gmid-sunucu
systemctl restart gmid-sunucu
sleep 2
if curl -fsS http://127.0.0.1:8780/saglik; then
  echo; echo "Güncellendi."
else
  echo "Yeni sürüm açılmadı, eskisine dönülüyor."
  mv -f /opt/gmid/gmid-sunucu.onceki /opt/gmid/gmid-sunucu
  systemctl restart gmid-sunucu
  exit 1
fi
