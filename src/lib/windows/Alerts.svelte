<script lang="ts">
  // Sağ alttaki hatırlatıcı/sayaç kartları. Odak çalmaz; düğmeler tıklanabilir.
  import { onMount, tick } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import { clock, relative, when } from "../time";
  import type { Alerts } from "../types";

  let alerts: Alerts = $state({ reminders: [], timers: [], games: [] });
  let box: HTMLDivElement;

  async function refresh() {
    alerts = await api.overlay.alerts();
    if (!alerts.reminders.length && !alerts.timers.length && !alerts.games.length) {
      await api.overlay.closeAlerts();
      return;
    }
    await tick();
    await api.overlay.fitAlerts(box.offsetHeight + 12);
  }

  onMount(() => {
    void refresh();
    const stop = on("alerts-changed", refresh);
    return () => void stop.then((f) => f());
  });

  async function act(p: Promise<unknown>) {
    await p;
    await refresh();
  }
</script>

<div class="stack" bind:this={box}>
  {#each alerts.games as g (g.path)}
    <div class="card game">
      <div class="top"><span class="ic"><Icon name="library" size={18} /></span><b>Yeni oyun: {g.name}</b></div>
      <span class="meta">{g.exe} · Kütüphaneye ekleyeyim mi? Oynadığın süre tutulur.</span>
      <div class="acts">
        <button class="ghost" onclick={() => act(api.games.answerSuggestion(g.path, "never"))} title="Bu program için bir daha sorulmaz">Bir daha sorma</button>
        <button class="ghost" onclick={() => act(api.games.answerSuggestion(g.path, "later"))}>Şimdi değil</button>
        <button class="ok" onclick={() => act(api.games.answerSuggestion(g.path, "add"))}>Ekle</button>
      </div>
    </div>
  {/each}
  {#each alerts.reminders as r (r.id)}
    <div class="card">
      <div class="top"><span class="ic"><Icon name="bell" size={18} /></span><b>{r.title}</b></div>
      <span class="meta">{when(r.dueAt)} · {relative(r.dueAt)}{r.note ? ` · ${r.note}` : ""}</span>
      <div class="acts">
        <button class="ghost" onclick={() => act(api.reminders.snooze(r.id, 10))}>10 dk ertele</button>
        <button class="ghost" onclick={() => act(api.reminders.snooze(r.id, 60))}>1 saat</button>
        <button class="ok" onclick={() => act(api.reminders.complete(r.id))}>Tamam</button>
      </div>
    </div>
  {/each}
  {#each alerts.timers as t (t.id)}
    <div class="card">
      <div class="top"><span class="ic"><Icon name="timer" size={18} /></span><b>Süre doldu: {t.label}</b></div>
      <span class="meta tnum">{clock(t.totalMs)} geri sayım bitti</span>
      <div class="acts">
        <button class="ghost" onclick={() => act(api.timers.toggle(t.id))}>Baştan başlat</button>
        <button class="ok" onclick={() => act(api.timers.remove(t.id))}>Tamam</button>
      </div>
    </div>
  {/each}
</div>

<style>
  :global(html), :global(body) { background: transparent !important; overflow: hidden; }
  .stack { position: fixed; left: 6px; right: 6px; bottom: 6px; display: flex; flex-direction: column; gap: 8px; }
  .card {
    display: flex; flex-direction: column; gap: 6px; padding: 12px 14px; border-radius: 14px;
    background: rgba(18, 20, 23, 0.96); border: 1px solid rgba(227, 194, 112, 0.55);
    box-shadow: 0 8px 26px rgba(0, 0, 0, 0.45), 0 0 18px rgba(227, 194, 112, 0.12);
    animation: in 0.22s ease-out;
  }
  .top { display: flex; align-items: center; gap: 10px; }
  .top b { font-weight: 700; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ic { color: var(--gold); display: grid; place-items: center; }
  .meta { font-size: 12.5px; color: var(--muted); padding-left: 28px; }
  .acts { display: flex; gap: 6px; justify-content: flex-end; flex-wrap: wrap; }
  button { border-radius: 9px; padding: 6px 12px; font-size: 13px; font-weight: 600; }
  .ghost { border: 1px solid var(--line-2); background: transparent; color: var(--text); }
  .ghost:hover { border-color: var(--gold-dim); }
  .ok { border: 0; background: var(--gold); color: #1b1608; }
  .ok:hover { background: var(--gold-glow); }
  @keyframes in { from { opacity: 0; transform: translateY(8px); } }
</style>
