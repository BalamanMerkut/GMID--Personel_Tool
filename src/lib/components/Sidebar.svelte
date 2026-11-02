<script lang="ts">
  import logo from "../../assets/gmid-logo.webp";
  import Icon from "../Icon.svelte";
  import { activeSummary } from "../downloads.svelte";
  import { groupOf, NAV } from "../nav";
  import { player } from "../player.svelte";
  import { onMount } from "svelte";
  import { api, inTauri } from "../api";
  import { onceHazirla } from "../footballCache";
  import { getJson } from "../settings.svelte";
  import { on } from "../bus";
  import type { UpdateStatus } from "../types";
  import { community, joinDiscord, startCommunity } from "../community.svelte";
  import { dealsBadge, startDealsBadge } from "../dealsBadge.svelte";
  import { profilGoster, sosyal, sosyalBaslat } from "../social.svelte";
  import Avatar from "./social/Avatar.svelte";

  let { current, version, onnavigate }: { current: string; version: string; onnavigate: (id: string) => void } = $props();
  const downloading = $derived(activeSummary().count);
  // Bir girişe yeniden tıklayınca en son açık olan sekmesine gidilir.
  const last: Record<string, string> = {};
  const currentGroup = $derived(groupOf(current)?.id);
  $effect(() => {
    if (currentGroup) last[currentGroup] = current;
  });
  let update = $state<UpdateStatus | null>(null);
  onMount(() => {
    startCommunity();
    startDealsBadge();
    sosyalBaslat();
    // Futbol ilk açılışta beklemesin: açılıştan biraz sonra bu haftanın maçları arkada hazırlanır.
    const futbol = setTimeout(() => {
      if (inTauri) void onceHazirla(api.football, getJson<string>("football.league", "all")).catch(() => {});
    }, 15_000);
    void api.update.status().then((s) => (update = s)).catch(() => {});
    const stop = on<UpdateStatus>("update", (s) => (update = s));
    return () => {
      clearTimeout(futbol);
      void stop.then((f) => f());
    };
  });
</script>

<aside class="side" aria-label="Malikâne odaları">
  <div class="brand">
    <img src={logo} alt="GMID" />
    <div class="name">
      <div class="baslik">
        <b>GMID TOOL</b>
        {#if community.discord}
          <!-- GMID Discord sunucusuna katıl: başlığın yanında, malikâne renklerinde. -->
          <button class="dc" onclick={joinDiscord} aria-label="GMID Discord sunucusuna katıl" title="GMID Discord sunucusuna katıl ({community.discord.replace('https://', '')})">
            <Icon name="discord" size={15} />
          </button>
        {/if}
      </div>
      <span lang="en">Game Mansion In Discord</span>
    </div>
    {#if community.discord}
      <button class="dc dar" onclick={joinDiscord} aria-label="GMID Discord sunucusuna katıl" title="GMID Discord sunucusuna katıl"><Icon name="discord" size={15} /></button>
    {/if}
  </div>
  <nav>
    {#each NAV as room, i (i)}
      {#if room.name}
        <div class="room">{room.name}</div>
      {/if}
      {#if i > 0}<div class="sep"></div>{/if}
      {#each room.groups as g (g.id)}
        <button
          class:top={!room.name}
          aria-current={currentGroup === g.id ? "page" : undefined}
          title={g.tabs.length > 1 ? `${g.label}: ${g.tabs.map((t) => t.label).join(", ")}` : g.label}
          onclick={() => onnavigate(last[g.id] ?? g.tabs[0].id)}
        >
          <Icon name={g.icon} />
          <span class="label">{g.label}</span>
          {#if g.id === "media" && player.item}<span class="eq" class:paused={!player.playing} title={player.playing ? "Çalıyor" : "Duraklatıldı"}><i></i><i></i><i></i></span>{/if}
          {#if g.id === "social" && sosyal.gelen.length}<span class="badge new tnum" title="Arkadaşlık isteği">{sosyal.gelen.length}</span>{/if}
          {#if g.id === "feed" && sosyal.bildirim}<span class="badge new tnum" title="Paylaşımına beğeni ya da yorum">{sosyal.bildirim}</span>{/if}
          {#if g.id === "deals" && dealsBadge.count}<span class="badge new tnum" title={dealsBadge.wish ? "İstek listende indirime giren oyun var" : "Yeni ücretsiz oyun var"}>{dealsBadge.count}</span>{/if}
          {#if g.id === "media" && downloading}<span class="badge tnum" title="{downloading} indirme sürüyor">{downloading}</span>{/if}
        </button>
      {/each}
    {/each}
  </nav>
  <div class="alt">
    {#if update?.available}
      <button class="upd" onclick={() => onnavigate("settings/hakkinda")} title={update.latest?.title ?? ""}>
        <Icon name="download" size={15} /><span>Güncelleme var</span>
      </button>
    {/if}
    <div class="foot"><span>Merkut Studios</span><span class="tnum">v{version}{update?.current ? ` · ${update.current}` : ""}</span></div>
    <!-- Kendi kartın, sol altta (Discord ve Steam'deki gibi): profil resmin, adın, durumun; tıklayınca profilin. -->
    {#if sosyal.anahtar && sosyal.ben}
      {@const benOyun = sosyal.cevrimici[sosyal.ben.id]?.oyun}
      <button
        class="ben"
        aria-current={current === "profile" && (!sosyal.profilId || sosyal.profilId === sosyal.ben.id) ? "page" : undefined}
        title="Profilim"
        onclick={() => profilGoster(null)}
      >
        <Avatar kisi={sosyal.ben} boy={36} />
        <span class="ben-yazi">
          <b>{sosyal.ben.ad}</b>
          <span class:oyun={!!benOyun} class:acik={sosyal.bagli && !benOyun}>{benOyun ? `${benOyun} oynuyor` : sosyal.bagli ? "Çevrim içi" : "Bağlanıyor…"}</span>
        </span>
        <span class="ben-ok"><Icon name="next" size={14} /></span>
      </button>
    {:else}
      <button class="ben davet" title="Topluluğa katıl" onclick={() => onnavigate("social")}>
        <span class="ben-ikon"><Icon name="user" size={18} /></span>
        <span class="ben-yazi"><b>Topluluğa katıl</b><span>Discord ile giriş yap</span></span>
        <span class="ben-ok"><Icon name="next" size={14} /></span>
      </button>
    {/if}
  </div>
</aside>

<style>
  .side {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: rgba(23, 26, 30, 0.86);
    border-right: 1px solid var(--line);
  }
  .brand { display: flex; align-items: center; gap: 10px; padding: 18px 14px 12px 16px; }
  .brand img { width: 44px; height: 44px; border-radius: 50%; box-shadow: 0 0 18px rgba(227, 194, 112, 0.25); }
  .brand b { font-family: var(--f-display); font-weight: 800; font-size: 17px; letter-spacing: 0.04em; color: var(--gold); display: block; }
  .name { flex: 1; min-width: 0; }
  .dc.dar { display: none; }
  .baslik { display: flex; align-items: center; gap: 8px; }
  .brand span { display: block; font-size: 10px; color: var(--dim); letter-spacing: 0.05em; text-transform: uppercase; white-space: nowrap; }
  nav { overflow: auto; padding: 6px 10px 16px; display: flex; flex-direction: column; gap: 2px; }
  .room {
    font-family: var(--f-display); font-size: 11px; font-weight: 700;
    letter-spacing: 0.12em; text-transform: uppercase; color: var(--gold-dim);
    padding: 14px 10px 6px;
  }
  .sep { display: none; }
  button {
    display: flex; align-items: center; gap: 12px; width: 100%;
    padding: 9px 10px; border: 0; border-radius: 10px;
    background: transparent; color: var(--muted); text-align: left; font-weight: 500;
  }
  button:hover { background: rgba(255, 255, 255, 0.04); color: var(--text); }
  button[aria-current="page"] {
    background: linear-gradient(90deg, rgba(227, 194, 112, 0.16), rgba(227, 194, 112, 0.03));
    color: var(--gold-glow);
  }
  button[aria-current="page"] :global(svg) { filter: drop-shadow(0 0 6px rgba(227, 194, 112, 0.55)); }
  button.top { font-family: var(--f-display); font-weight: 700; }
  .label { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .badge.new { background: #e5484d; color: #fff; box-shadow: 0 0 0 3px rgba(229, 72, 77, 0.18); animation: pop-badge 0.3s ease-out; }
  @keyframes pop-badge { from { transform: scale(0.4); } }
  .badge { min-width: 20px; height: 20px; padding: 0 6px; border-radius: 999px; background: var(--teal); color: #071415; font-size: 11px; font-weight: 800; display: grid; place-items: center; }
  .eq { display: inline-flex; gap: 2px; align-items: flex-end; height: 12px; }
  .eq i { width: 3px; background: var(--gold); border-radius: 1px; animation: eq 0.9s ease-in-out infinite; }
  .eq i:nth-child(2) { animation-delay: 0.2s; }
  .eq.paused i { animation: none; height: 4px; background: var(--dim); }
  .eq i:nth-child(3) { animation-delay: 0.4s; }
  @keyframes eq { 0%, 100% { height: 3px; } 50% { height: 12px; } }
  /* button { width: 100% } + yan boşluk taşırıyordu: genişlik kenarlardan hesaplanır. */
  .upd, .ben { width: auto; margin: 0 10px 6px; }
  .ben { display: flex; align-items: center; gap: 10px; padding: 7px 10px 7px 7px; border-radius: 14px; border: 1px solid var(--line-2); background: linear-gradient(180deg, rgba(255, 255, 255, 0.05), rgba(255, 255, 255, 0.02)); color: var(--text); }
  .ben:hover { border-color: rgba(227, 194, 112, 0.45); background: rgba(227, 194, 112, 0.08); }
  .ben[aria-current="page"] { border-color: rgba(227, 194, 112, 0.6); background: linear-gradient(90deg, rgba(227, 194, 112, 0.16), rgba(227, 194, 112, 0.04)); }
  .ben-yazi { flex: 1; min-width: 0; display: flex; flex-direction: column; line-height: 1.25; }
  .ben-yazi b { font-size: 13.5px; font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ben-yazi span { font-size: 11.5px; color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ben-yazi .acik { color: var(--good); }
  .ben-yazi .oyun { color: #a99cff; }
  .ben-ok { color: var(--dim); display: grid; }
  .ben-ikon { display: grid; place-items: center; width: 36px; height: 36px; border-radius: 50%; background: rgba(227, 194, 112, 0.12); color: var(--gold-glow); flex: none; }
  .ben.davet { border-style: dashed; }
  /* Discord'a katıl: başlığın yanında, malikâne renklerinde (altın halka, koyu zemin). */
  .dc {
    flex: none; display: grid; place-items: center; width: 28px; height: 28px; padding: 0; border-radius: 50%;
    border: 1px solid rgba(227, 194, 112, 0.45);
    background: radial-gradient(circle at 50% 35%, rgba(227, 194, 112, 0.18), rgba(23, 26, 30, 0.9) 70%);
    color: var(--gold);
    transition: color 0.15s, border-color 0.15s, box-shadow 0.15s, transform 0.15s;
  }
  .dc:hover { color: var(--gold-glow); border-color: var(--gold); background: radial-gradient(circle at 50% 35%, rgba(227, 194, 112, 0.3), rgba(23, 26, 30, 0.9) 70%); box-shadow: 0 0 14px rgba(227, 194, 112, 0.35); transform: translateY(-1px); }
  .upd { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-radius: 10px; border: 1px solid var(--gold-dim); background: rgba(227, 194, 112, 0.1); color: var(--gold-glow); font-weight: 600; font-size: 13px; }
  .upd:hover { background: rgba(227, 194, 112, 0.18); }
  /* Alt bölüm: güncelleme, imza ve en altta kendi kartın. */
  .alt { margin-top: auto; padding-top: 8px; border-top: 1px solid var(--line); background: rgba(0, 0, 0, 0.16); }
  .alt .ben { margin-bottom: 10px; }
  .foot { padding: 2px 18px 8px; font-size: 11px; color: var(--dim); display: flex; justify-content: space-between; gap: 8px; }

  /* Dar pencerede sadece ikonlar */
  @media (max-width: 1100px) {
    .brand { flex-direction: column; justify-content: center; gap: 10px; padding: 16px 8px; }
    .name, .room, .label, .foot, .upd span, .ben-yazi, .ben-ok { display: none; }
    nav { padding: 6px 10px; gap: 4px; }
    .sep { display: block; height: 1px; background: var(--line); margin: 8px 6px; }
    button { justify-content: center; padding: 11px; position: relative; }
    .dc.dar { display: grid; width: 32px; height: 32px; padding: 0; }
    .ben { padding: 6px; justify-content: center; }
    .badge { position: absolute; top: 3px; right: 3px; min-width: 16px; height: 16px; font-size: 10px; padding: 0 4px; }
  }
</style>
