#!/usr/bin/env bash
# GMID sunucusunu GitHub'daki en son sürüme günceller; ayarlar ve veritabanı korunur.
#   sudo gmid-guncelle             → şimdi güncelle (yeni sürüm yoksa bir şey yapmaz)
#   gmid-guncelle --otomatik       → gece zamanlayıcısının çağırdığı hali (aynısı, sessiz)
#   sudo gmid-guncelle --kur       → yalnız bu betiği ve gece zamanlayıcısını kur/tazele
# Her çalışmada önce kendini ve gece zamanlayıcısını (her gece ~04:30) tazeler. Yeni program indirilince
# SHA-256 özeti denetlenir; yeni sürüm açılmazsa eskisine dönülür.
set -euo pipefail

KIP="${1:-}"
DEPO="BalamanMerkut/GMID--Personel_Tool"
URL="https://github.com/${DEPO}/releases/latest/download/gmid-sunucu-linux-x64"
HAM="https://raw.githubusercontent.com/${DEPO}/${GMID_DAL:-HEAD}/sunucu/kurulum"
PROGRAM=/opt/gmid/gmid-sunucu

[ "$(id -u)" -eq 0 ] || { echo "sudo ile çalıştır: sudo gmid-guncelle"; exit 1; }
gecici=$(mktemp -d)
trap 'rm -rf "$gecici"' EXIT
yaz() { [ "$KIP" = "--otomatik" ] || echo "$@"; }

# ---------- kendini ve zamanlayıcıyı tazele ----------
if curl -fsSL "$HAM/guncelle.sh" -o "$gecici/guncelle.sh" && bash -n "$gecici/guncelle.sh"; then
  install -m 755 "$gecici/guncelle.sh" /usr/local/bin/gmid-guncelle.yeni
  # Yeni dosya (yeni inode): şu an çalışan bu betik bozulmaz.
  mv -f /usr/local/bin/gmid-guncelle.yeni /usr/local/bin/gmid-guncelle
fi
if [ ! -f /etc/systemd/system/gmid-guncelle.timer ]; then
  cat > /etc/systemd/system/gmid-guncelle.service <<'UNIT'
[Unit]
Description=GMID sunucusu: yeni sürüm varsa güncelle
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/gmid-guncelle --otomatik
UNIT
  cat > /etc/systemd/system/gmid-guncelle.timer <<'UNIT'
[Unit]
Description=GMID sunucusu: gece güncelleme denetimi

[Timer]
OnCalendar=*-*-* 04:30
RandomizedDelaySec=20m
Persistent=true

[Install]
WantedBy=timers.target
UNIT
  systemctl daemon-reload
  systemctl enable --now gmid-guncelle.timer >/dev/null 2>&1
  yaz "Gece otomatik güncelleme açıldı (her gece ~04:30, yalnız yeni sürüm varsa)."
fi
[ "$KIP" = "--kur" ] && exit 0

# ---------- yeni sürüm var mı? ----------
curl -fsSL "$URL.sha256" -o "$gecici/ozet" || { echo "GitHub'a ulaşılamadı"; exit 1; }
yeni=$(cut -d' ' -f1 "$gecici/ozet")
eski=$(sha256sum "$PROGRAM" 2>/dev/null | cut -d' ' -f1 || true)
if [ -n "$yeni" ] && [ "$yeni" = "$eski" ]; then
  yaz "Zaten en son sürüm."
  exit 0
fi

curl -fsSL "$URL" -o "$gecici/gmid-sunucu"
gercek=$(sha256sum "$gecici/gmid-sunucu" | cut -d' ' -f1)
[ "$gercek" = "$yeni" ] || { echo "İndirilen program bozuk (özet tutmadı); güncellenmedi."; exit 1; }
install -m 755 "$gecici/gmid-sunucu" "$PROGRAM.yeni"
cp -f "$PROGRAM" "$PROGRAM.onceki" 2>/dev/null || true
mv -f "$PROGRAM.yeni" "$PROGRAM"
# Hizmet dosyası da değişmiş olabilir.
if curl -fsSL "$HAM/gmid-sunucu.service" -o "$gecici/gmid-sunucu.service"; then
  install -m 644 "$gecici/gmid-sunucu.service" /etc/systemd/system/gmid-sunucu.service
  systemctl daemon-reload
fi
systemctl restart gmid-sunucu
sleep 2
if curl -fsS http://127.0.0.1:8780/saglik >/dev/null; then
  echo "GMID sunucusu güncellendi: $(curl -fsS http://127.0.0.1:8780/saglik)"
else
  echo "Yeni sürüm açılmadı, eskisine dönülüyor."
  mv -f "$PROGRAM.onceki" "$PROGRAM"
  systemctl restart gmid-sunucu
  exit 1
fi
