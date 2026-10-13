<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import RollingClock from "../components/RollingClock.svelte";
  import { clock } from "../time";
  import { notify } from "../toast.svelte";
  import type { Countdown, TimersSnapshot } from "../types";

  let snap: TimersSnapshot | null = $state(null);
  /** Çekirdeğin saatiyle aradaki fark (ms). */
  let offset = 0;
  let now = $state(Date.now());
  let label = $state("");
  let minutes = $state(5);
  let seconds = $state(0);

  const PRESETS = [1, 5, 10, 15, 25, 45, 60];

  function apply(s: TimersSnapshot) {
    snap = s;
    offset = s.now - Date.now();
  }

  async function refresh() {
    apply(await api.timers.state());
  }

  onMount(() => {
    void refresh();
    const stop = on("timers-changed", refresh);
    // Sadece çalışan bir sayaç varken ve bu ekran açıkken, her karede güncelle (salise ve çubuk akıcı aksın).
    let raf = requestAnimationFrame(function frame() {
      if (snap && (snap.stopwatch.running || snap.countdowns.some((c) => c.endsAt))) now = Date.now() + offset;
      raf = requestAnimationFrame(frame);
    });
    return () => {
      cancelAnimationFrame(raf);
      void stop.then((f) => f());
    };
  });

  const elapsed = $derived.by(() => (snap ? snap.stopwatch.elapsedMs + (snap.stopwatch.running ? now - snap.now : 0) : 0));
  const remaining = (c: Countdown) => (c.endsAt ? Math.max(0, c.endsAt - now) : c.remainingMs);

  async function add(ms: number, name = label) {
    try {
      apply(await api.timers.add(name, ms));
      label = "";
      notify(`Geri sayım başladı · ${clock(ms)}`, "timer");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  function addCustom(e: SubmitEvent) {
    e.preventDefault();
    void add((Math.max(0, minutes) * 60 + Math.max(0, seconds)) * 1000);
  }
</script>

<div class="timers">
  <section class="panel sw">
    <h3 class="h2"><Icon name="lap" />Kronometre</h3>
    <div class="digits tnum" class:running={snap?.stopwatch.running}><RollingClock text={clock(elapsed, true)} fast={2} /></div>
    <div class="row center">
      <button class="btn" onclick={async () => apply(await api.timers.stopwatchToggle())}>
        <Icon name={snap?.stopwatch.running ? "pause" : "play"} />{snap?.stopwatch.running ? "Duraklat" : elapsed > 0 ? "Devam" : "Başlat"}
      </button>
      <button class="btn ghost" disabled={!snap?.stopwatch.running} onclick={async () => apply(await api.timers.stopwatchLap())}><Icon name="flag" />Tur</button>
      <button class="btn ghost" disabled={elapsed === 0} onclick={async () => apply(await api.timers.stopwatchReset())}><Icon name="restore" />Sıfırla</button>
    </div>
    {#if snap?.stopwatch.laps.length}
      <ol class="laps">
        {#each [...snap.stopwatch.laps].reverse() as lap, i (i)}
          {@const n = snap.stopwatch.laps.length - i}
          {@const prev = n > 1 ? snap.stopwatch.laps[n - 2] : 0}
          <li class="tnum"><span>Tur {n}</span><span>+{clock(lap - prev, true)}</span><b>{clock(lap, true)}</b></li>
        {/each}
      </ol>
    {/if}
  </section>

  <section class="panel">
    <h3 class="h2"><Icon name="timer" />Geri sayımlar</h3>
    <div class="presets">
      {#each PRESETS as m (m)}
        <button onclick={() => add(m * 60_000, label || `${m} dakika`)}>{m} dk</button>
      {/each}
    </div>
    <form class="custom" onsubmit={addCustom}>
      <input id="cd-label" type="text" maxlength="40" placeholder="Ad (ör. Fırın)" bind:value={label} />
      <label class="num"><input id="cd-min" type="number" min="0" max="1440" bind:value={minutes} aria-label="Dakika" /><span>dk</span></label>
      <label class="num"><input id="cd-sec" type="number" min="0" max="59" bind:value={seconds} aria-label="Saniye" /><span>sn</span></label>
      <button class="btn" type="submit"><Icon name="play" />Başlat</button>
    </form>

    <div class="cds">
      {#each snap?.countdowns ?? [] as c (c.id)}
        {@const left = remaining(c)}
        <div class="cd" class:done={c.finished} class:paused={!c.endsAt && !c.finished}>
          <div class="cd-top">
            <b>{c.label}</b>
            <span class="sub small">{c.finished ? "Bitti" : c.endsAt ? "Çalışıyor" : "Duraklatıldı"}</span>
          </div>
          <div class="cd-time tnum"><RollingClock text={c.finished ? "00:00" : clock(left + 999)} dir={-1} /></div>
          <div class="bar"><i style:width="{c.finished ? 100 : 100 - (left / c.totalMs) * 100}%"></i></div>
          <div class="row">
            <button class="btn ghost sm" onclick={async () => apply(await api.timers.toggle(c.id))}>
              <Icon name={c.finished ? "restore" : c.endsAt ? "pause" : "play"} size={16} />{c.finished ? "Baştan" : c.endsAt ? "Duraklat" : "Devam"}
            </button>
            <button class="btn ghost sm" onclick={async () => apply(await api.timers.remove(c.id))}><Icon name="x" size={16} />Kaldır</button>
          </div>
        </div>
      {:else}
        <p class="empty">Çalışan geri sayım yok. Yukarıdan bir süre seç. Bitince ekranın sağ altında bir kart çıkar.</p>
      {/each}
    </div>
  </section>
</div>

<style>
  .timers { display: grid; grid-template-columns: minmax(320px, 1fr) minmax(0, 1.3fr); gap: 16px; max-width: 1400px; margin: 0 auto; align-items: start; }
  .panel { display: flex; flex-direction: column; gap: 14px; }
  .digits { font-family: var(--f-display); font-weight: 800; font-size: clamp(40px, 6vw, 64px); text-align: center; letter-spacing: 0.02em; color: var(--muted); padding: 10px 0; }
  .digits.running { color: var(--text); text-shadow: 0 0 24px rgba(96, 216, 216, 0.18); }
  .center { justify-content: center; }
  .laps { list-style: none; margin: 0; padding: 0; max-height: 260px; overflow: auto; display: flex; flex-direction: column; gap: 4px; }
  .laps li { display: grid; grid-template-columns: 1fr 1fr 1fr; padding: 6px 10px; border-radius: 8px; background: rgba(255, 255, 255, 0.03); font-size: 13px; }
  .laps li span:first-child { color: var(--dim); }
  .laps li b { text-align: right; font-weight: 600; }
  .presets { display: flex; flex-wrap: wrap; gap: 6px; }
  .presets button { border: 1px solid var(--line-2); background: var(--coal-2); color: var(--muted); border-radius: 999px; padding: 6px 13px; font-weight: 600; }
  .presets button:hover { color: var(--gold-glow); border-color: var(--gold-dim); }
  .custom { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .custom input { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 8px 10px; }
  .custom input[type="text"] { flex: 1 1 160px; min-width: 0; }
  .num { display: flex; align-items: center; gap: 4px; color: var(--dim); font-size: 12.5px; }
  .num input { width: 70px; }
  .cds { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 12px; }
  .cd { border: 1px solid var(--line); border-radius: 14px; padding: 12px 14px; display: flex; flex-direction: column; gap: 8px; background: rgba(255, 255, 255, 0.02); }
  .cd.done { border-color: var(--gold); background: rgba(227, 194, 112, 0.08); }
  .cd-top { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  .cd-top b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cd-time { font-family: var(--f-display); font-weight: 800; font-size: 34px; }
  .paused .cd-time { color: var(--muted); }
  .done .cd-time { color: var(--gold-glow); }
  .bar { height: 6px; border-radius: 999px; background: var(--coal); overflow: hidden; }
  .bar i { display: block; height: 100%; background: linear-gradient(90deg, var(--teal), var(--neon)); }
  .done .bar i { background: var(--gold); }
  .sm { padding: 5px 10px; font-size: 12.5px; }
  .empty { color: var(--dim); margin: 0; grid-column: 1 / -1; }
  @media (max-width: 1000px) { .timers { grid-template-columns: minmax(0, 1fr); } }
</style>
