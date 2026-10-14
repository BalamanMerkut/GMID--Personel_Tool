<script lang="ts">
  import logo from "../../assets/gmid-logo.webp";
  import Icon from "../Icon.svelte";
  import { activeSummary } from "../downloads.svelte";
  import { groupOf, NAV } from "../nav";
  import { player } from "../player.svelte";
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import type { UpdateStatus } from "../types";
  import { community, joinDiscord, startCommunity } from "../community.svelte";
  import { dealsBadge, markDealsSeen, startDealsBadge } from "../dealsBadge.svelte";

  let { current, version, onnavigate }: { current: string; version: string; onnavigate: (id: string) => void } = $props();
  const downloading = $derived(activeSummary().count);
  // Bir girişe yeniden tıklayınca en son açık olan sekmesine gidilir.
  const last: Record<string, string> = {};
  const currentGroup = $derived(groupOf(current)?.id);
  $effect(() => {
    if (currentGroup) last[currentGroup] = current;
  });
  // Fırsatlar açıkken gelen yenilikler de görülmüş sayılır.
  $effect(() => {
    if (current === "deals" && dealsBadge.count) markDealsSeen();
  });
  let update = $state<UpdateStatus | null>(null);
  onMount(() => {
    startCommunity();
    startDealsBadge();
    void api.update.status().then((s) => (update = s)).catch(() => {});
    const stop = on<UpdateStatus>("update", (s) => (update = s));
    return () => void stop.then((f) => f());
  });
</script>

<aside class="side" aria-label="Malikâne odaları">
  <div class="brand">
    <img src={logo} alt="GMID" />
    <div class="name"><b>GMID TOOL</b><span lang="en">Game Mansion In Discord</span></div>
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
          {#if g.id === "deals" && dealsBadge.count}<span class="badge new tnum" title={dealsBadge.wish ? "İstek listende indirime giren oyun var" : "Yeni ücretsiz oyun var"}>{dealsBadge.count}</span>{/if}
          {#if g.id === "media" && downloading}<span class="badge tnum" title="{downloading} indirme sürüyor">{downloading}</span>{/if}
        </button>
      {/each}
    {/each}
  </nav>
  {#if community.discord}
    <button class="dc" onclick={joinDiscord} title="GMID Discord sunucusuna katıl ({community.discord.replace('https://', '')})">
      <i class="logo"><Icon name="discord" size={17} /></i><span>GMID'e katıl</span>
    </button>
  {/if}
  {#if update?.available}
    <button class="upd" onclick={() => onnavigate("settings/hakkinda")} title={update.latest?.title ?? ""}>
      <Icon name="download" size={15} /><span>Güncelleme var</span>
    </button>
  {/if}
  <div class="foot"><span>Merkut Studios</span><span class="tnum">v{version}{update?.current ? ` · ${update.current}` : ""}</span></div>
</aside>

<style>
  .side {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: rgba(23, 26, 30, 0.86);
    border-right: 1px solid var(--line);
  }
  .brand { display: flex; align-items: center; gap: 12px; padding: 18px 18px 12px; }
  .brand img { width: 44px; height: 44px; border-radius: 50%; box-shadow: 0 0 18px rgba(227, 194, 112, 0.25); }
  .brand b { font-family: var(--f-display); font-weight: 800; font-size: 17px; letter-spacing: 0.04em; color: var(--gold); display: block; }
  .brand span { display: block; font-size: 10.5px; color: var(--dim); letter-spacing: 0.08em; text-transform: uppercase; }
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
  .dc, .upd { width: auto; margin: 0 10px 6px; }
  .dc { display: flex; align-items: center; gap: 10px; padding: 6px 12px 6px 6px; border-radius: 12px; border: 1px solid rgba(88, 101, 242, 0.45); background: rgba(88, 101, 242, 0.12); color: #dfe2ff; font-weight: 700; font-size: 13px; }
  .dc .logo { display: grid; place-items: center; width: 30px; height: 30px; border-radius: 9px; background: #5865f2; color: #fff; flex: none; font-style: normal; }
  .dc:hover { background: rgba(88, 101, 242, 0.24); color: #fff; }
  .upd { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-radius: 10px; border: 1px solid var(--gold-dim); background: rgba(227, 194, 112, 0.1); color: var(--gold-glow); font-weight: 600; font-size: 13px; }
  .upd:hover { background: rgba(227, 194, 112, 0.18); }
  .foot {
    margin-top: auto; padding: 12px 18px 14px; border-top: 1px solid var(--line);
    font-size: 11px; color: var(--dim); display: flex; justify-content: space-between; gap: 8px;
  }

  /* Dar pencerede sadece ikonlar */
  @media (max-width: 1100px) {
    .brand { justify-content: center; padding: 16px 8px; }
    .name, .room, .label, .foot, .upd span, .dc span { display: none; }
    nav { padding: 6px 10px; gap: 4px; }
    .sep { display: block; height: 1px; background: var(--line); margin: 8px 6px; }
    button { justify-content: center; padding: 11px; position: relative; }
    .dc { padding: 6px; justify-content: center; }
    .badge { position: absolute; top: 3px; right: 3px; min-width: 16px; height: 16px; font-size: 10px; padding: 0 4px; }
  }
</style>
