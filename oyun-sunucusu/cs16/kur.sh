#!/usr/bin/env bash
# GMID CS 1.6 sunucusu: JailBreak + 35 HP (bunny hop) + Pro Public (fy_iceworld) tek sunucuda, harita türüne göre.
#
#   sudo bash kur.sh                 → kur ya da güncelle (ayarlar, rcon şifresi, yöneticiler korunur)
#   GMID_CS_YONETICI="STEAM_0:1:123" sudo -E bash kur.sh   → bu Steam kimliğini tam yetkili yönetici yap
#
# Güvenlik: sunucudaki başka işlere DOKUNMAZ. Her şey /opt/gmid-cs16 altında, ayrı "cs16" kullanıcısıyla çalışır;
# nginx/Caddy ve başka projeler değişmez. Güvenlik duvarı (ufw) açıksa yalnız 27015 açılır, kapalıysa açılmaz.
# Hizmet sınırlı: en çok 768 MB bellek, 1 çekirdek, düşük öncelik (diğer projeler önce gelir).
set -euo pipefail

DIZIN=/opt/gmid-cs16
HLDS=$DIZIN/hlds
CS=$HLDS/cstrike
KULLANICI=cs16
PORT="${GMID_CS_PORT:-27015}"
SLOT="${GMID_CS_SLOT:-24}"
AD="${GMID_CS_AD:-GMID | JailBreak - 35HP - IceWorld | discord.gg/upu9VQwFrV}"
DEPO_HAM="https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/${GMID_DAL:-HEAD}/oyun-sunucusu/cs16"
HIZMET=gmid-cs16

[ "$(id -u)" -eq 0 ] || { echo "sudo ile çalıştır"; exit 1; }
gecici=$(mktemp -d)
trap 'rm -rf "$gecici"' EXIT
adim() { echo; echo "==> $*"; }

# ---------- 0. port boş mu (başka bir şey kullanıyorsa dokunma) ----------
if ss -lun 2>/dev/null | grep -q ":$PORT " && ! systemctl is-active --quiet "$HIZMET"; then
  echo "UDP $PORT başka bir program tarafından kullanılıyor; GMID_CS_PORT=27016 ile farklı port seç."
  exit 1
fi

# ---------- 1. gerekli paketler (32 bit kütüphaneler) ----------
adim "Gerekli paketler"
export DEBIAN_FRONTEND=noninteractive
paket_gerek=""
for p in lib32gcc-s1 lib32stdc++6 curl unzip tar ca-certificates; do
  dpkg -s "$p" >/dev/null 2>&1 || paket_gerek="$paket_gerek $p"
done
if [ -n "$paket_gerek" ]; then
  apt-get update -q
  # shellcheck disable=SC2086
  apt-get install -y -q $paket_gerek
fi

# ---------- 2. kullanıcı ve klasör ----------
adim "Kullanıcı ve klasör ($DIZIN)"
id "$KULLANICI" >/dev/null 2>&1 || useradd --system --home-dir "$DIZIN" --create-home --shell /usr/sbin/nologin "$KULLANICI"
mkdir -p "$DIZIN/steamcmd" "$HLDS"
chown -R "$KULLANICI:$KULLANICI" "$DIZIN"
olarak() { runuser -u "$KULLANICI" -- "$@"; }

# ---------- 3. HLDS (SteamCMD, uygulama 90) ----------
adim "Half-Life sunucusu (SteamCMD)"
if [ ! -x "$DIZIN/steamcmd/steamcmd.sh" ]; then
  curl -fsSL https://steamcdn-a.akamaihd.net/client/installer/steamcmd_linux.tar.gz -o "$gecici/steamcmd.tgz"
  tar -xzf "$gecici/steamcmd.tgz" -C "$DIZIN/steamcmd"
  chown -R "$KULLANICI:$KULLANICI" "$DIZIN/steamcmd"
fi
# Uygulama 90 bazen ilk denemede eksik iner (bilinen Steam sorunu): cstrike gelene kadar birkaç kez.
for deneme in 1 2 3 4; do
  olarak "$DIZIN/steamcmd/steamcmd.sh" +force_install_dir "$HLDS" +login anonymous \
    +app_set_config 90 mod cstrike +app_update 90 validate +quit >"$gecici/steam.log" 2>&1 || true
  [ -f "$CS/liblist.gam" ] && [ -f "$HLDS/hlds_linux" ] && break
  echo "  deneme $deneme eksik kaldı, yeniden…"
done
[ -f "$CS/liblist.gam" ] || { echo "HLDS indirilemedi:"; tail -20 "$gecici/steam.log"; exit 1; }

# ---------- 4. ReHLDS, ReGameDLL, Metamod-R (GitHub'daki son sürüm) ----------
son_surum() { # $1: depo, $2: dosya adında geçen
  curl -fsSL "https://api.github.com/repos/$1/releases/latest" \
    | grep -o '"browser_download_url": *"[^"]*"' | cut -d'"' -f4 | grep -i "$2" | grep -i '\.zip$' | head -1
}
indir_ac() { # $1: url, $2: hedef klasör
  curl -fsSL "$1" -o "$gecici/p.zip"
  rm -rf "$2"; mkdir -p "$2"
  unzip -q -o "$gecici/p.zip" -d "$2"
}

adim "ReHLDS"
u=$(son_surum rehlds/ReHLDS rehlds-bin) || true
[ -n "${u:-}" ] || { echo "ReHLDS sürümü bulunamadı"; exit 1; }
indir_ac "$u" "$gecici/rehlds"
kaynak=$(dirname "$(find "$gecici/rehlds" -name engine_i486.so | head -1)")
cp -rf "$kaynak"/. "$HLDS"/

adim "ReGameDLL"
u=$(son_surum rehlds/ReGameDLL_CS regamedll-bin) || true
[ -n "${u:-}" ] || { echo "ReGameDLL sürümü bulunamadı"; exit 1; }
indir_ac "$u" "$gecici/regame"
csso=$(find "$gecici/regame" -path '*linux32*' -name cs.so | head -1)
cp -f "$csso" "$CS/dlls/cs.so"
gcfg=$(find "$gecici/regame" -path '*linux32*' -name game.cfg | head -1 || true)
if [ -n "$gcfg" ] && [ ! -f "$CS/game.cfg" ]; then cp "$gcfg" "$CS/game.cfg"; fi

adim "Metamod-R"
u=$(son_surum rehlds/Metamod-R metamod-bin) || true
[ -n "${u:-}" ] || { echo "Metamod-R sürümü bulunamadı"; exit 1; }
indir_ac "$u" "$gecici/metamod"
mm=$(find "$gecici/metamod" -name 'metamod_i386.so' | head -1)
mkdir -p "$CS/addons/metamod"
cp -f "$mm" "$CS/addons/metamod/metamod_i386.so"
sed -i 's#^gamedll_linux .*#gamedll_linux "addons/metamod/metamod_i386.so"#' "$CS/liblist.gam"
grep -q '^gamedll_linux' "$CS/liblist.gam" || echo 'gamedll_linux "addons/metamod/metamod_i386.so"' >>"$CS/liblist.gam"

# ---------- 5. AMX Mod X 1.10 ----------
adim "AMX Mod X"
liste=$(curl -fsSL https://www.amxmodx.org/amxxdrop/1.10/ || true)
for paket in base cstrike; do
  dosya=$(echo "$liste" | grep -o "amxmodx-1\.10\.0-git[0-9]*-$paket-linux\.tar\.gz" | sort -V | uniq | tail -1)
  [ -n "$dosya" ] || { echo "AMX Mod X ($paket) bulunamadı"; exit 1; }
  curl -fsSL "https://www.amxmodx.org/amxxdrop/1.10/$dosya" -o "$gecici/amxx-$paket.tgz"
  # Ayar dosyaları (users.ini, plugins.ini) varsa korunur.
  tar -xzf "$gecici/amxx-$paket.tgz" -C "$CS" --skip-old-files
done
tar -xzf "$gecici/amxx-base.tgz" -C "$CS" --wildcards --overwrite 'addons/amxmodx/dlls/*' 'addons/amxmodx/modules/*' 'addons/amxmodx/scripting/amxxpc*' 'addons/amxmodx/scripting/include/*' 2>/dev/null || true
echo "linux addons/amxmodx/dlls/amxmodx_mm_i386.so" >"$CS/addons/metamod/plugins.ini"

# ---------- 6. GMID eklentileri (bu depodan, sunucuda derlenir) ----------
adim "GMID eklentileri (mod seçici + JailBreak)"
SCR="$CS/addons/amxmodx/scripting"
for e in gmid_mod gmid_jail; do
  curl -fsSL "$DEPO_HAM/eklentiler/$e.sma" -o "$SCR/$e.sma"
  rm -f "$CS/addons/amxmodx/plugins/$e.amxx"
  ( cd "$SCR" && ./amxxpc "$e.sma" -o"../plugins/$e.amxx" ) >"$gecici/$e.log" 2>&1 || true
  if [ ! -s "$CS/addons/amxmodx/plugins/$e.amxx" ]; then
    echo "$e derlenemedi:"; cat "$gecici/$e.log"; exit 1
  fi
  grep -q "^$e.amxx" "$CS/addons/amxmodx/configs/plugins.ini" || echo "$e.amxx" >>"$CS/addons/amxmodx/configs/plugins.ini"
done

# Tam yetkili yönetici (isteğe bağlı).
if [ -n "${GMID_CS_YONETICI:-}" ]; then
  u="$CS/addons/amxmodx/configs/users.ini"
  grep -q "\"$GMID_CS_YONETICI\"" "$u" || echo "\"$GMID_CS_YONETICI\" \"\" \"abcdefghijklmnopqrstu\" \"ce\"" >>"$u"
  echo "  Yönetici eklendi: $GMID_CS_YONETICI"
fi

# ---------- 7. haritalar ve ayarlar ----------
adim "Haritalar"
mkdir -p "$CS/maps"
# İstenen haritalar: yoksa listelenir (Mola sohbetinde indirilir; .bsp dosyaları $CS/maps içine).
curl -fsSL "$DEPO_HAM/haritalar.txt" -o "$gecici/haritalar.txt" || true
eksik=""
: >"$CS/mapcycle.txt"
while read -r h; do
  h="${h%%#*}"; h="$(echo "$h" | tr -d '[:space:]')"
  [ -n "$h" ] || continue
  if [ -f "$CS/maps/$h.bsp" ]; then echo "$h" >>"$CS/mapcycle.txt"; else eksik="$eksik $h"; fi
done <"$gecici/haritalar.txt"
# Hiçbiri yoksa oyunla gelen haritalarla açılır.
if [ ! -s "$CS/mapcycle.txt" ]; then
  printf 'de_dust2\ncs_assault\nde_inferno\n' >"$CS/mapcycle.txt"
fi
ilk=$(head -1 "$CS/mapcycle.txt")
if [ -n "$eksik" ]; then echo "  Eksik haritalar:$eksik"; fi

adim "server.cfg"
RCON_DOSYA=/etc/gmid/cs16-rcon
mkdir -p /etc/gmid
if [ ! -s "$RCON_DOSYA" ]; then
  head -c 18 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 20 >"$RCON_DOSYA"
  chmod 600 "$RCON_DOSYA"
fi
RCON=$(cat "$RCON_DOSYA")
cat >"$CS/server.cfg" <<CFG
// GMID CS 1.6 (kur.sh yazar; elle değiştirme, yeniden kurulumda yenilenir). Harita türüne göre ayarları
// gmid_mod eklentisi yapar: 35hp_* bunny hop açık, jail_*/jb_* JailBreak, diğerleri Pro Public.
hostname "$AD"
rcon_password "$RCON"
sv_lan 0
sv_region 3
sv_password ""
sv_maxrate 100000
sv_minrate 20000
sv_maxupdaterate 101
sv_minupdaterate 30
sys_ticrate 1000
sv_allowdownload 1
sv_allowupload 0
mp_timelimit 25
mp_friendlyfire 0
mp_footsteps 1
mp_flashlight 1
mp_tkpunish 0
mp_autokick 0
mp_chattime 5
sv_alltalk 0
decalfrequency 60
log on
exec banned.cfg
exec listip.cfg
CFG
touch "$CS/banned.cfg" "$CS/listip.cfg"
chown -R "$KULLANICI:$KULLANICI" "$DIZIN"

# ---------- 8. hizmet ----------
adim "Hizmet ($HIZMET)"
cat >/etc/systemd/system/$HIZMET.service <<UNIT
[Unit]
Description=GMID CS 1.6 sunucusu (JailBreak / 35HP / Pro Public)
After=network-online.target
Wants=network-online.target

[Service]
User=$KULLANICI
Group=$KULLANICI
WorkingDirectory=$HLDS
Environment=LD_LIBRARY_PATH=$HLDS
ExecStart=$HLDS/hlds_linux -game cstrike -strictportbind -port $PORT +ip 0.0.0.0 +maxplayers $SLOT +map $ilk +sys_ticrate 1000 -pingboost 3
Restart=always
RestartSec=5
# Diğer projeler önce gelsin.
Nice=5
MemoryMax=768M
CPUQuota=100%
NoNewPrivileges=true
ProtectSystem=full
ProtectHome=true
PrivateTmp=true
ReadWritePaths=$DIZIN

[Install]
WantedBy=multi-user.target
UNIT
systemctl daemon-reload
systemctl enable "$HIZMET" >/dev/null 2>&1
systemctl restart "$HIZMET"

# Güvenlik duvarı: yalnız açıksa ve yalnız oyun portu.
if command -v ufw >/dev/null && ufw status 2>/dev/null | grep -q "Status: active"; then
  ufw allow "$PORT"/udp >/dev/null
  ufw allow "$PORT"/tcp >/dev/null
  echo "  ufw: $PORT açıldı"
fi

sleep 6
if systemctl is-active --quiet "$HIZMET" && ss -lun | grep -q ":$PORT "; then
  ip=$(curl -fsS -4 https://api.ipify.org 2>/dev/null || hostname -I | awk '{print $1}')
  echo
  echo "Hazır. Oyunda konsola:  connect $ip:$PORT"
  echo "Harita sırası: $(tr '\n' ' ' <"$CS/mapcycle.txt")"
  echo "Rcon şifresi: $RCON_DOSYA dosyasında (yalnız root okur)."
  if [ -n "$eksik" ]; then
    echo "Eksik haritalar indirilip $CS/maps içine konunca kur.sh yeniden çalıştırılmalı:$eksik"
  fi
else
  echo "Sunucu açılmadı; son günlük:"
  journalctl -u "$HIZMET" -n 40 --no-pager || true
  exit 1
fi
