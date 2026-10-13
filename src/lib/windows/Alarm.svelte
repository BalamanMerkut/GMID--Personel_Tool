<script lang="ts">
  // Hatırlatıcı / sayaç uyarısı: ekranın ortasında, sakin bir melodiyle. Ertele ya da Kapat; hepsi bitince
  // pencere kendiliğinden kapanır ve melodi susar. Enter: kapat, Esc: 10 dk ertele.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import PowerWarn from "../components/PowerWarn.svelte";
  import { clock, when } from "../time";
  import type { Countdown, Reminder } from "../types";

  let reminders: Reminder[] = $state([]);
  let timers: Countdown[] = $state([]);
  let muted = $state(false);

  async function refresh() {
    const a = await api.overlay.alerts().catch(() => null);
    reminders = a?.reminders ?? [];
    timers = a?.timers ?? [];
    const plan = await api.power.status().catch(() => null);
    const powerSoon = !!plan && !plan.hidden && plan.at - Date.now() < 66_000;
    if (a && !reminders.length && !timers.length && !powerSoon) void api.alarm.hide();
  }

  onMount(() => {
    void refresh();
    const stops = [on("alarm-changed", () => void refresh()), on("reminders-changed", () => void refresh()), on("timers-changed", () => void refresh())];
    return () => stops.forEach((s) => void s.then((f) => f()));
  });

  async function act(p: Promise<unknown>) {
    await p.catch(() => {});
    await refresh();
  }

  function mute() {
    muted = true;
    void api.alarm.mute();
  }

  const first = $derived(reminders[0] ?? null);
  const firstTimer = $derived(first ? null : (timers[0] ?? null));
  const more = $derived(reminders.length + timers.length - 1);

  function onkey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      if (first) void act(api.reminders.complete(first.id));
      else if (firstTimer) void act(api.timers.remove(firstTimer.id));
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (first) void act(api.reminders.snooze(first.id, 10));
      else if (firstTimer) void act(api.timers.remove(firstTimer.id));
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="wrap">
  <div class="card" data-tauri-drag-region>
    <PowerWarn />
    <div class="head" data-tauri-drag-region>
      <span class="ring"><Icon name={first ? "bell" : firstTimer ? "timer" : "power"} size={26} /></span>
      <span class="kind" data-tauri-drag-region>{first ? "Hatırlatıcı" : firstTimer ? "Süre doldu" : "GMID Tool"}</span>
      {#if !muted}
        <button class="ghost mini" onclick={mute} title="Melodiyi sustur"><Icon name="mute" size={15} />Sustur</button>
      {/if}
    </div>

    {#if first}
      <h1>{first.title}</h1>
      {#if first.note}<p class="note">{first.note}</p>{/if}
      <span class="when tnum">{when(first.dueAt)}</span>
      <div class="acts">
        <span class="lbl">Ertele</span>
        <button class="ghost" onclick={() => act(api.reminders.snooze(first.id, 5))}>5 dk</button>
        <button class="ghost" onclick={() => act(api.reminders.snooze(first.id, 10))}>10 dk</button>
        <button class="ghost" onclick={() => act(api.reminders.snooze(first.id, 60))}>1 saat</button>
        <button class="ok" onclick={() => act(api.reminders.complete(first.id))}><Icon name="check" size={16} />Kapat</button>
      </div>
    {:else if firstTimer}
      <h1>{firstTimer.label}</h1>
      <p class="note tnum">{clock(firstTimer.totalMs)} geri sayım bitti</p>
      <div class="acts">
        <button class="ghost" onclick={() => act(api.timers.toggle(firstTimer.id))}><Icon name="replay" size={15} />Yeniden başlat</button>
        <button class="ok" onclick={() => act(api.timers.remove(firstTimer.id))}><Icon name="check" size={16} />Kapat</button>
      </div>
    {/if}

    <span class="foot" class:hide={!first && !firstTimer}>
      {#if more > 0}Sırada {more} uyarı daha · {/if}Enter: kapat · Esc: {first ? "10 dk ertele" : "kapat"}
    </span>
  </div>
</div>

<style>
  :global(html), :global(body) { background: transparent !important; overflow: hidden; }
  .wrap { position: fixed; inset: 0; display: grid; place-items: center; padding: 10px; }
  .card {
    width: 100%; height: 100%; display: flex; flex-direction: column; gap: 10px; padding: 20px 22px 16px;
    border-radius: 20px; overflow: auto; background: rgba(18, 20, 23, 0.97); border: 1px solid rgba(227, 194, 112, 0.6);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.55), 0 0 40px rgba(227, 194, 112, 0.15);
    animation: in 0.35s ease-out;
  }
  .head { display: flex; align-items: center; gap: 12px; }
  .ring {
    display: grid; place-items: center; width: 48px; height: 48px; border-radius: 50%; color: var(--gold);
    background: rgba(227, 194, 112, 0.12); box-shadow: 0 0 0 0 rgba(227, 194, 112, 0.5); animation: pulse 2.4s ease-out infinite;
  }
  .kind { flex: 1; font-family: var(--f-display); font-size: 12px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; color: var(--gold-dim); }
  h1 { margin: 4px 0 0; font-family: var(--f-display); font-size: 26px; line-height: 1.2; overflow-wrap: anywhere; }
  .note { margin: 0; color: var(--muted); font-size: 15px; line-height: 1.45; max-height: 66px; overflow: auto; }
  .when { font-size: 12.5px; color: var(--dim); }
  .acts { margin-top: auto; display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .lbl { font-size: 12px; color: var(--dim); margin-right: 2px; }
  button { display: inline-flex; align-items: center; gap: 6px; border-radius: 11px; padding: 9px 14px; font-size: 14px; font-weight: 700; }
  .ghost { border: 1px solid var(--line-2); background: transparent; color: var(--text); }
  .ghost:hover { border-color: var(--gold-dim); }
  .mini { padding: 5px 10px; font-size: 12px; font-weight: 600; color: var(--muted); }
  .ok { margin-left: auto; border: 0; background: var(--gold); color: #1b1608; padding: 10px 20px; }
  .ok:hover { background: var(--gold-glow); }
  .foot { font-size: 11.5px; color: var(--dim); }
  .foot.hide { display: none; }
  @keyframes in { from { opacity: 0; transform: translateY(10px) scale(0.98); } }
  @keyframes pulse { 0% { box-shadow: 0 0 0 0 rgba(227, 194, 112, 0.45); } 70% { box-shadow: 0 0 0 16px rgba(227, 194, 112, 0); } 100% { box-shadow: 0 0 0 0 rgba(227, 194, 112, 0); } }
  @media (prefers-reduced-motion: reduce) { .card, .ring { animation: none; } }
</style>
