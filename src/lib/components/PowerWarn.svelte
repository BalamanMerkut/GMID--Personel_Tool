<script lang="ts">
  // Zamanlı kapatmaya bir dakika kala: geri sayım, İptal / Şimdi. Uyarı penceresinde ve Oyun İçi Panel'de.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import type { PowerPlan } from "../types";

  let plan = $state<PowerPlan | null>(null);
  let now = $state(Date.now());

  onMount(() => {
    const load = () => void api.power.status().then((p) => (plan = p)).catch(() => {});
    load();
    const stops = [on<PowerPlan | null>("power-changed", (p) => (plan = p ?? null)), on("alarm-changed", load)];
    const t = setInterval(() => (now = Date.now()), 250);
    return () => {
      clearInterval(t);
      stops.forEach((s) => void s.then((f) => f()));
    };
  });

  const left = $derived(plan ? Math.max(0, Math.ceil((plan.at - now) / 1000)) : 0);
  const show = $derived(!!plan && !plan.hidden && left <= 65);
  const verb = $derived(plan?.action === "restart" ? "yeniden başlatılacak" : plan?.action === "sleep" ? "uyku moduna geçecek" : "kapanacak");
</script>

{#if show && plan}
  <div class="pwarn">
    <span class="ic"><Icon name="power" size={22} /></span>
    <div class="tx">
      <b>Bilgisayar <span class="tnum">{left}</span> saniye içinde {verb}</b>
      <span>{plan.action === "sleep" ? "İşin varsa iptal et." : "Açık programlar kapanır; kaydetmediğin bir şey varsa iptal et."}</span>
    </div>
    <button class="cancel" onclick={() => api.power.cancel()}>İptal et</button>
    <button class="nowb" onclick={() => api.power.now()}>Şimdi</button>
    <button class="hide" onclick={() => api.power.dismiss()} title="Uyarıyı kapat: süre işlemeye devam eder" aria-label="Uyarıyı kapat"><Icon name="x" size={14} /></button>
  </div>
{/if}

<style>
  .pwarn { position: relative; display: flex; align-items: center; gap: 12px; padding: 12px 14px; border-radius: 14px; background: rgba(224, 106, 106, 0.1); border: 1px solid rgba(224, 106, 106, 0.6); }
  .ic { color: var(--bad); display: grid; place-items: center; }
  .tx { flex: 1; display: flex; flex-direction: column; gap: 2px; min-width: 0; font-size: 13px; color: var(--muted); }
  .tx b { color: var(--text); font-size: 15px; }
  button { border-radius: 10px; padding: 8px 14px; font-weight: 700; font-size: 13.5px; }
  .cancel { border: 0; background: var(--gold); color: #1b1608; }
  .nowb { border: 1px solid var(--line-2); background: transparent; color: var(--muted); }
  .hide {
    position: absolute; top: -9px; right: -9px; width: 24px; height: 24px; padding: 0; border-radius: 50%;
    display: grid; place-items: center; border: 1px solid var(--line-2); background: #1b1e22; color: var(--muted);
  }
  .hide:hover { color: var(--text); border-color: var(--gold-dim); }
</style>
