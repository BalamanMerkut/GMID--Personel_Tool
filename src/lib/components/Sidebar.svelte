<script lang="ts">
  import logo from "../../assets/gmid-logo.webp";
  import Icon from "../Icon.svelte";
  import { activeSummary } from "../downloads.svelte";
  import { ROOMS } from "../nav";

  let { current, version, onnavigate }: { current: string; version: string; onnavigate: (id: string) => void } = $props();
  const downloading = $derived(activeSummary().count);
</script>

<aside class="side" aria-label="Malikâne odaları">
  <div class="brand">
    <img src={logo} alt="GMID" />
    <div class="name"><b>GMID TOOL</b><span lang="en">Game Mansion In Discord</span></div>
  </div>
  <nav>
    {#each ROOMS as room, i (i)}
      {#if room.name}
        <div class="room">{room.name}</div>
      {/if}
      {#if i > 0}<div class="sep"></div>{/if}
      {#each room.screens as s (s.id)}
        <button
          class:top={!room.name}
          aria-current={current === s.id ? "page" : undefined}
          title={s.label}
          onclick={() => onnavigate(s.id)}
        >
          <Icon name={s.icon} />
          <span class="label">{s.label}</span>
          {#if s.phase}<span class="soon">Faz {s.phase}</span>{/if}
          {#if s.id === "videos" && downloading}<span class="badge tnum" title="{downloading} indirme sürüyor">{downloading}</span>{/if}
        </button>
      {/each}
    {/each}
  </nav>
  <div class="foot"><span>Merkut Studios</span><span class="tnum">v{version}</span></div>
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
  .soon { font-size: 10px; color: var(--dim); border: 1px solid var(--line); border-radius: 999px; padding: 0 6px; white-space: nowrap; }
  .badge { min-width: 20px; height: 20px; padding: 0 6px; border-radius: 999px; background: var(--teal); color: #071415; font-size: 11px; font-weight: 800; display: grid; place-items: center; }
  .foot {
    margin-top: auto; padding: 12px 18px 14px; border-top: 1px solid var(--line);
    font-size: 11px; color: var(--dim); display: flex; justify-content: space-between; gap: 8px;
  }

  /* Dar pencerede sadece ikonlar */
  @media (max-width: 1100px) {
    .brand { justify-content: center; padding: 16px 8px; }
    .name, .room, .label, .soon, .foot { display: none; }
    nav { padding: 6px 10px; gap: 4px; }
    .sep { display: block; height: 1px; background: var(--line); margin: 8px 6px; }
    button { justify-content: center; padding: 11px; position: relative; }
    .badge { position: absolute; top: 3px; right: 3px; min-width: 16px; height: 16px; font-size: 10px; padding: 0 4px; }
  }
</style>
