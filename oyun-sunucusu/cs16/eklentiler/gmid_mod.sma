/*
 * GMID Mod Seçici
 *
 * Harita adına göre sunucunun "modunu" ayarlar; tek sunucuda 35 HP ve Pro Public haritaları döner:
 *
 *   35hp_*            → 35 HP: herkes 35 canla doğar, zırh yok, otomatik bunny hop AÇIK (boşluğu basılı tut),
 *                       sv_airaccelerate 100.
 *   diğerleri         → Pro Public (fy_iceworld vb.): bunny hop KAPALI, sv_airaccelerate 10, kısa donma,
 *                       otomatik takım dengesi.
 *
 * Komut: say /mod → şu anki mod. Her 4 dakikada bir sohbete Discord ve GMID Tool duyurusu düşer.
 */

#include <amxmodx>
#include <engine>
#include <fun>
#include <cstrike>
#include <hamsandwich>

#define PLUGIN  "GMID Mod Secici"
#define VERSION "1.0"
#define AUTHOR  "GMID"

enum
{
	MOD_PUB = 0,
	MOD_35HP
}

new g_mod
new bool:g_bhop

public plugin_init()
{
	register_plugin(PLUGIN, VERSION, AUTHOR)

	new map[32]
	get_mapname(map, charsmax(map))
	strtolower(map)

	if (equal(map, "35hp", 4))
		g_mod = MOD_35HP
	else
		g_mod = MOD_PUB

	// Bunny hop yalnız 35 HP haritalarında.
	g_bhop = (g_mod == MOD_35HP)

	RegisterHam(Ham_Spawn, "player", "fw_Spawn", 1)
	register_clcmd("say /mod", "cmd_mod")
	register_clcmd("say_team /mod", "cmd_mod")
	register_clcmd("say /discord", "cmd_discord")
	register_clcmd("say_team /discord", "cmd_discord")
	set_task(240.0, "duyuru", _, _, _, "b")
}

public duyuru()
{
	client_print(0, print_chat, "[GMID] Discord: discord.gg/upu9VQwFrV  |  GMID Tool: gmid.molayazilim.com.tr")
}

public cmd_discord(id)
{
	client_print(id, print_chat, "[GMID] Discord: discord.gg/upu9VQwFrV  |  GMID Tool: gmid.molayazilim.com.tr")
	return PLUGIN_CONTINUE
}

public plugin_cfg()
{
	// server.cfg ve harita ayarlarından sonra uygulansın.
	set_task(1.5, "ayarla")
}

public ayarla()
{
	switch (g_mod)
	{
		case MOD_35HP:
		{
			server_cmd("sv_airaccelerate 100")
			server_cmd("mp_freezetime 0")
			server_cmd("mp_roundtime 3")
			server_cmd("mp_buytime 0.1")
			server_cmd("mp_autoteambalance 1")
			server_cmd("mp_limitteams 2")
		}
		default:
		{
			// Pro Public
			server_cmd("sv_airaccelerate 10")
			server_cmd("mp_freezetime 3")
			server_cmd("mp_roundtime 2.5")
			server_cmd("mp_buytime 0.25")
			server_cmd("mp_autoteambalance 1")
			server_cmd("mp_limitteams 2")
		}
	}
	server_cmd("mp_friendlyfire 0")
	server_exec()
}

public cmd_mod(id)
{
	static const adlar[][] = { "Pro Public (bunny hop kapali)", "35 HP (bunny hop acik)" }
	client_print(id, print_chat, "[GMID] Bu haritanin modu: %s", adlar[g_mod])
	return PLUGIN_CONTINUE
}

public fw_Spawn(id)
{
	if (g_mod != MOD_35HP || !is_user_alive(id))
		return HAM_IGNORED
	// Haritanın kendi ayarından sonra (hemen ardından) 35 can.
	set_task(0.2, "can_35", id)
	return HAM_IGNORED
}

public can_35(id)
{
	if (!is_user_alive(id))
		return
	set_user_health(id, 35)
	cs_set_user_armor(id, 0, CS_ARMOR_NONE)
}

// Otomatik bunny hop: yerdeyken zıplama tuşu basılıysa yeniden zıplatır, inişteki yavaşlamayı kaldırır.
public client_PreThink(id)
{
	if (!g_bhop || !is_user_alive(id))
		return PLUGIN_CONTINUE

	// İnişteki hız kaybı yok.
	entity_set_float(id, EV_FL_fuser2, 0.0)

	if (!(entity_get_int(id, EV_INT_button) & IN_JUMP))
		return PLUGIN_CONTINUE

	new flags = entity_get_int(id, EV_INT_flags)
	if (flags & FL_WATERJUMP || entity_get_int(id, EV_INT_waterlevel) >= 2 || !(flags & FL_ONGROUND))
		return PLUGIN_CONTINUE

	new Float:hiz[3]
	entity_get_vector(id, EV_VEC_velocity, hiz)
	hiz[2] += 250.0
	entity_set_vector(id, EV_VEC_velocity, hiz)
	entity_set_int(id, EV_INT_gaitsequence, 6)
	return PLUGIN_CONTINUE
}
