/*
 * GMID JailBreak (sade)
 *
 * Yalnız jail_* ve jb_* haritalarında çalışır (diğerlerinde kendini durdurur).
 *
 *   Mahkûmlar (T)      : yalnız bıçakla doğar.
 *   Gardiyanlar (CT)   : M4A1 + Deagle + zırh. Her 3 mahkûma 1 gardiyan; fazlası mahkûm olur.
 *   Komutan            : "/simon" ya da "/komutan" yazan ilk gardiyan (10 sn içinde kimse yazmazsa rastgele).
 *                        Mavi parlar. "/ac" ile hücreleri açar, "/fd" ile bir mahkûma serbest gün verir.
 *   Serbest gün (FD)   : yeşil parlar; gardiyan ona hasar veremez. Mahkûm gardiyana vurursa FD biter.
 *   Son istek (LR)     : tek mahkûm kalınca "/lr": Bıçak düellosu ya da Deagle düellosu (komutana ya da ilk
 *                        gardiyana karşı). Düelloya başkası karışamaz.
 *   Kurallar           : "/kurallar". Yöneticiler (kick yetkisi) komutan komutlarını da kullanabilir.
 */

#include <amxmodx>
#include <amxmisc>
#include <cstrike>
#include <fun>
#include <fakemeta>
#include <hamsandwich>

#define PLUGIN  "GMID JailBreak"
#define VERSION "1.0"
#define AUTHOR  "GMID"

#define ON  "[GMID JB]"
#define TASK_EKIPMAN 1000
#define TASK_KOMUTAN 2000

new g_simon
new bool:g_fd[33]
new bool:g_lr
new g_lrT
new g_lrCT

public plugin_init()
{
	register_plugin(PLUGIN, VERSION, AUTHOR)

	new map[32]
	get_mapname(map, charsmax(map))
	strtolower(map)
	if (!equal(map, "jail_", 5) && !equal(map, "jb_", 3))
	{
		// Bu harita JailBreak değil: eklenti durur, hiçbir şeye karışmaz.
		pause("ad")
		return
	}

	RegisterHam(Ham_Spawn, "player", "fw_Spawn", 1)
	RegisterHam(Ham_Killed, "player", "fw_Killed", 1)
	RegisterHam(Ham_TakeDamage, "player", "fw_TakeDamage")
	register_event("HLTV", "ev_YeniTur", "a", "1=0", "2=0")
	register_logevent("ev_TurBasladi", 2, "1=Round_Start")

	komut("simon", "cmd_simon")
	komut("komutan", "cmd_simon")
	komut("ac", "cmd_ac")
	komut("fd", "cmd_fd")
	komut("lr", "cmd_lr")
	komut("kurallar", "cmd_kurallar")
}

komut(const ad[], const fonk[])
{
	new s[32]
	formatex(s, charsmax(s), "say /%s", ad)
	register_clcmd(s, fonk)
	formatex(s, charsmax(s), "say_team /%s", ad)
	register_clcmd(s, fonk)
}

bool:yetkili(id)
{
	return id == g_simon || (get_user_flags(id) & ADMIN_KICK) != 0
}

say(const fmt[], any:...)
{
	new m[190]
	vformat(m, charsmax(m), fmt, 2)
	client_print(0, print_chat, "%s %s", ON, m)
}

say1(id, const fmt[], any:...)
{
	new m[190]
	vformat(m, charsmax(m), fmt, 3)
	client_print(id, print_chat, "%s %s", ON, m)
}

parla(id, r, g, b)
{
	if (r || g || b)
		set_user_rendering(id, kRenderFxGlowShell, r, g, b, kRenderNormal, 18)
	else
		set_user_rendering(id, kRenderFxNone, 0, 0, 0, kRenderNormal, 0)
}

// ---------------- tur ----------------

public ev_YeniTur()
{
	g_simon = 0
	g_lr = false
	g_lrT = 0
	g_lrCT = 0
	for (new i = 1; i <= MaxClients; i++)
	{
		g_fd[i] = false
		if (is_user_connected(i))
			parla(i, 0, 0, 0)
	}
	remove_task(TASK_KOMUTAN)
}

public ev_TurBasladi()
{
	say("Gardiyanlar: komutan olmak icin /simon yazin. Kurallar: /kurallar")
	set_task(10.0, "komutan_sec", TASK_KOMUTAN)
}

public komutan_sec()
{
	if (g_simon && is_user_alive(g_simon))
		return
	new oyuncular[32], n
	get_players(oyuncular, n, "ae", "CT")
	if (!n)
		return
	komutan_yap(oyuncular[random(n)])
}

komutan_yap(id)
{
	g_simon = id
	parla(id, 0, 80, 255)
	new ad[32]
	get_user_name(id, ad, charsmax(ad))
	say("Komutan: %s. Hucreleri /ac ile acar, /fd ile serbest gun verir.", ad)
}

// ---------------- doğma ve takım dengesi ----------------

public fw_Spawn(id)
{
	if (!is_user_alive(id))
		return HAM_IGNORED

	if (cs_get_user_team(id) == CS_TEAM_CT && gardiyan_fazla())
	{
		cs_set_user_team(id, CS_TEAM_T)
		say1(id, "Gardiyan sayisi dolu (her 3 mahkuma 1 gardiyan). Mahkum oldun.")
		ExecuteHamB(Ham_CS_RoundRespawn, id)
		return HAM_IGNORED
	}
	// Haritanın verdiği silahlardan sonra.
	remove_task(id + TASK_EKIPMAN)
	set_task(0.2, "ekipman", id + TASK_EKIPMAN)
	return HAM_IGNORED
}

bool:gardiyan_fazla()
{
	new ct, t
	for (new i = 1; i <= MaxClients; i++)
	{
		if (!is_user_connected(i))
			continue
		switch (cs_get_user_team(i))
		{
			case CS_TEAM_CT: ct++
			case CS_TEAM_T: t++
		}
	}
	return ct > t / 3 + 1
}

public ekipman(task)
{
	new id = task - TASK_EKIPMAN
	if (!is_user_alive(id))
		return
	strip_user_weapons(id)
	give_item(id, "weapon_knife")
	if (cs_get_user_team(id) == CS_TEAM_CT)
	{
		give_item(id, "weapon_m4a1")
		cs_set_user_bpammo(id, CSW_M4A1, 90)
		give_item(id, "weapon_deagle")
		cs_set_user_bpammo(id, CSW_DEAGLE, 35)
		cs_set_user_armor(id, 100, CS_ARMOR_VESTHELM)
	}
	else
	{
		cs_set_user_armor(id, 0, CS_ARMOR_NONE)
	}
}

// ---------------- komutan komutları ----------------

public cmd_simon(id)
{
	if (cs_get_user_team(id) != CS_TEAM_CT || !is_user_alive(id))
	{
		say1(id, "Komutan yalniz yasayan bir gardiyan olabilir.")
		return PLUGIN_HANDLED
	}
	if (g_simon && is_user_alive(g_simon) && g_simon != id)
	{
		say1(id, "Bu turun zaten bir komutani var.")
		return PLUGIN_HANDLED
	}
	remove_task(TASK_KOMUTAN)
	komutan_yap(id)
	return PLUGIN_HANDLED
}

public cmd_ac(id)
{
	if (!yetkili(id))
	{
		say1(id, "Hucreleri yalniz komutan acabilir.")
		return PLUGIN_HANDLED
	}
	new acilan = 0
	acilan += kapilari_ac("func_door", id)
	acilan += kapilari_ac("func_door_rotating", id)
	say("Hucreler acildi (%d kapi).", acilan)
	return PLUGIN_HANDLED
}

kapilari_ac(const sinif[], id)
{
	new ent = -1, n = 0
	while ((ent = engfunc(EngFunc_FindEntityByString, ent, "classname", sinif)) > 0)
	{
		dllfunc(DLLFunc_Use, ent, id)
		n++
	}
	return n
}

public cmd_fd(id)
{
	if (!yetkili(id))
	{
		say1(id, "Serbest gunu yalniz komutan verebilir.")
		return PLUGIN_HANDLED
	}
	new menu = menu_create("Serbest gun kime?", "fd_secim")
	new oyuncular[32], n, ad[32], bilgi[4]
	get_players(oyuncular, n, "ae", "TERRORIST")
	for (new i = 0; i < n; i++)
	{
		get_user_name(oyuncular[i], ad, charsmax(ad))
		num_to_str(oyuncular[i], bilgi, charsmax(bilgi))
		menu_additem(menu, ad, bilgi)
	}
	if (!n)
	{
		menu_destroy(menu)
		say1(id, "Yasayan mahkum yok.")
		return PLUGIN_HANDLED
	}
	menu_display(id, menu)
	return PLUGIN_HANDLED
}

public fd_secim(id, menu, item)
{
	if (item == MENU_EXIT)
	{
		menu_destroy(menu)
		return PLUGIN_HANDLED
	}
	new bilgi[4], ad[32], erisim, geri
	menu_item_getinfo(menu, item, erisim, bilgi, charsmax(bilgi), ad, charsmax(ad), geri)
	menu_destroy(menu)
	new hedef = str_to_num(bilgi)
	if (!is_user_alive(hedef) || cs_get_user_team(hedef) != CS_TEAM_T)
		return PLUGIN_HANDLED
	g_fd[hedef] = true
	parla(hedef, 0, 255, 0)
	say("%s bu tur serbest gunde (yesil). Gardiyanlar ona ates edemez.", ad)
	return PLUGIN_HANDLED
}

// ---------------- hasar kuralları ----------------

public fw_TakeDamage(kurban, silah, saldiran, Float:hasar, tur)
{
	if (!(1 <= saldiran <= MaxClients) || saldiran == kurban || !is_user_connected(saldiran))
		return HAM_IGNORED

	// Son istek düellosuna başkası karışamaz.
	if (g_lr && (kurban == g_lrT || kurban == g_lrCT) && saldiran != g_lrT && saldiran != g_lrCT)
		return HAM_SUPERCEDE

	new CsTeams:ts = cs_get_user_team(saldiran)
	new CsTeams:tk = cs_get_user_team(kurban)

	// Serbest gündeki mahkûma gardiyan vuramaz.
	if (ts == CS_TEAM_CT && tk == CS_TEAM_T && g_fd[kurban])
		return HAM_SUPERCEDE

	// Serbest gündeki mahkûm gardiyana vurursa serbest günü biter.
	if (ts == CS_TEAM_T && tk == CS_TEAM_CT && g_fd[saldiran])
	{
		g_fd[saldiran] = false
		parla(saldiran, 0, 0, 0)
		new ad[32]
		get_user_name(saldiran, ad, charsmax(ad))
		say("%s gardiyana saldirdi: serbest gunu bitti.", ad)
	}
	return HAM_IGNORED
}

// ---------------- son istek ----------------

public fw_Killed(kurban, saldiran, gib)
{
	if (kurban == g_simon)
	{
		g_simon = 0
		say("Komutan oldu. Baska bir gardiyan /simon yazabilir.")
	}
	if (g_lr)
		return HAM_IGNORED
	new t[32], nt, ct[32], nct
	get_players(t, nt, "ae", "TERRORIST")
	get_players(ct, nct, "ae", "CT")
	if (nt == 1 && nct >= 1)
	{
		new ad[32]
		get_user_name(t[0], ad, charsmax(ad))
		say("Son mahkum %s! Son istegini secmek icin /lr yazsin.", ad)
	}
	return HAM_IGNORED
}

public cmd_lr(id)
{
	new t[32], nt, ct[32], nct
	get_players(t, nt, "ae", "TERRORIST")
	get_players(ct, nct, "ae", "CT")
	if (g_lr || nt != 1 || t[0] != id || !nct)
	{
		say1(id, "Son istek yalniz son kalan mahkum icindir.")
		return PLUGIN_HANDLED
	}
	new menu = menu_create("Son istegin?", "lr_secim")
	menu_additem(menu, "Bicak duellosu", "1")
	menu_additem(menu, "Deagle duellosu", "2")
	menu_display(id, menu)
	return PLUGIN_HANDLED
}

public lr_secim(id, menu, item)
{
	if (item == MENU_EXIT)
	{
		menu_destroy(menu)
		return PLUGIN_HANDLED
	}
	new bilgi[4], ad[32], erisim, geri
	menu_item_getinfo(menu, item, erisim, bilgi, charsmax(bilgi), ad, charsmax(ad), geri)
	menu_destroy(menu)

	new ct[32], nct
	get_players(ct, nct, "ae", "CT")
	if (g_lr || !is_user_alive(id) || !nct)
		return PLUGIN_HANDLED

	new rakip = (g_simon && is_user_alive(g_simon)) ? g_simon : ct[0]
	g_lr = true
	g_lrT = id
	g_lrCT = rakip

	new bool:deagle = (str_to_num(bilgi) == 2)
	duelloya_hazirla(id, deagle)
	duelloya_hazirla(rakip, deagle)
	parla(id, 255, 0, 0)
	parla(rakip, 0, 80, 255)

	new a1[32], a2[32]
	get_user_name(id, a1, charsmax(a1))
	get_user_name(rakip, a2, charsmax(a2))
	say("SON ISTEK: %s ile %s arasinda %s! Kimse karisamaz.", a1, a2, deagle ? "Deagle duellosu" : "Bicak duellosu")
	return PLUGIN_HANDLED
}

duelloya_hazirla(id, bool:deagle)
{
	strip_user_weapons(id)
	give_item(id, "weapon_knife")
	set_user_health(id, 100)
	cs_set_user_armor(id, 0, CS_ARMOR_NONE)
	if (deagle)
	{
		give_item(id, "weapon_deagle")
		cs_set_user_bpammo(id, CSW_DEAGLE, 21)
	}
}

// ---------------- kurallar ----------------

public cmd_kurallar(id)
{
	say1(id, "Mahkum: komutana uy, izinsiz hucreden cikma. Gardiyan: sebepsiz mahkum oldurme.")
	say1(id, "Komutan: /simon, hucre: /ac, serbest gun: /fd. Son mahkum: /lr (son istek).")
	return PLUGIN_HANDLED
}
