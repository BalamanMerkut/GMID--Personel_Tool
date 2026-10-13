<script lang="ts">
  // Zamanlı kapatma: "10 dk sonra bilgisayarı kapat". Son bir dakikada ekranın ortasında geri sayım çıkar.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Icon from "../Icon.svelte";
  import { clock } from "../time";
  import { notify } from "../toast.svelte";
  import type { IconName } from "../icons";
  import type { PowerPlan } from "../types";

  const ACTIONS: [PowerPlan["action"], string, IconName][] = [
    ["shutdown", "Kapat", "power"],
    ["restart", "Yeniden başlat", "replay"],
    ["sleep", "Uyut", "clock"],
  ];
  const PRESETS = [10, 30, 60, 90, 120];

  let plan = $state<PowerPlan | null>(null);
  let action = $state<PowerPlan["action"]>("shutdown");
  let custom = $state(45);
  /** Yanlış tıklamaya karşı: önce süre seçilir, sonra onaylanır. */
  let pending = $state<number | null>(null);
  let now = $state(Date.now());

  onMount(() => {
    void api.power.status().then((p) => (plan = p)).catch(() => {});
    const stop = on<PowerPlan | null>("power-changed", (p) => (plan = p ?? null));
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => {
      clearInterval(t);
      void stop.then((f) => f());
    };
  });

  async function start(min: number) {
    pending = null;
    try {
      plan = await api.power.schedule(action, min);
      notify(`Bilgisayar ${min} dk sonra ${verb(action)}`, "power");
    } catch (e) {
      notify(String(e), "info");
    }
  }

  async function cancel() {
    await api.power.cancel().catch(() => {});
    plan = null;
    notify("Zamanlı kapatma iptal edildi", "check");
  }

  const dur = (m: number) => (m < 60 ? `${m} dk` : m % 60 ? `${Math.floor(m / 60)} sa ${m % 60} dk` : `${m / 60} saat`);
  const verb = (a: string) => (a === "restart" ? "yeniden başlatılacak" : a === "sleep" ? "uyku moduna geçecek" : "kapanacak");
  const left = $derived(plan ? Math.max(0, plan.at - now) : 0);
  const at = $derived(plan ? new Date(plan.at).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" }) : "");
</script>

<section class="panel pw">
  <h3 class="h2"><Icon name="power" />Bilgisayarı zamanla kapat</h3>
  {#if plan}
    <div class="active">
      <div>
        <span class="big tnum">{clock(left)}</span>
        <span class="sub">sonra ({at}) bilgisayar {verb(plan.action)}</span>
      </div>
      <button class="btn ghost" onclick={cancel}><Icon name="x" size={15} />İptal et</button>
    </div>
    <span class="sub small">Son bir dakikada ekranın ortasında geri sayım çıkar; oradan da iptal edebilirsin.</span>
  {:else}
    <div class="seg">
      {#each ACTIONS as [a, l, icon] (a)}
        <button aria-pressed={action === a} onclick={() => (action = a)}><Icon name={icon} size={14} />{l}</button>
      {/each}
    </div>
    {#if pending !== null}
      <div class="confirm">
        <span>Bilgisayar <b>{dur(pending)}</b> sonra <b>{verb(action)}</b>. Emin misin?</span>
        <button class="btn ghost sm" onclick={() => (pending = null)}>Vazgeç</button>
        <button class="btn sm" onclick={() => start(pending!)}><Icon name="power" size={14} />Evet, başlat</button>
      </div>
    {:else}
      <div class="row">
        {#each PRESETS as m (m)}
          <button class="btn ghost sm" onclick={() => (pending = m)}>{dur(m)}</button>
        {/each}
        <span class="custom">
          <input type="number" min="1" max="1440" bind:value={custom} aria-label="Dakika" />
          <span class="sub small">dk</span>
          <button class="btn ghost sm" disabled={!custom || custom < 1} onclick={() => (pending = Number(custom))}>Seç</button>
        </span>
      </div>
    {/if}
    <span class="sub small">
      Zamanı gelince açık programlar kapatılır; kaydedilmemiş işin varsa kaybolabilir. GMID kapanırsa plan da iptal olur.
    </span>
  {/if}
</section>

<style>
  .pw { display: flex; flex-direction: column; gap: 12px; }
  .seg { display: flex; gap: 6px; flex-wrap: wrap; }
  .seg button { display: inline-flex; align-items: center; gap: 6px; padding: 7px 12px; border-radius: 10px; border: 1px solid var(--line-2); background: var(--coal-2); color: var(--text); font-weight: 600; }
  .seg button[aria-pressed="true"] { border-color: var(--gold); background: rgba(227, 194, 112, 0.12); color: var(--gold-glow); }
  .row { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
  .confirm { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; padding: 12px 14px; border-radius: 12px; background: rgba(224, 106, 106, 0.08); border: 1px solid rgba(224, 106, 106, 0.45); }
  .confirm span { flex: 1 1 260px; }
  .custom { display: inline-flex; align-items: center; gap: 6px; margin-left: auto; }
  .custom input { width: 72px; background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 9px; padding: 6px 8px; color: var(--text); }
  .active { display: flex; align-items: center; justify-content: space-between; gap: 16px; flex-wrap: wrap; padding: 14px 16px; border-radius: 14px; background: rgba(227, 194, 112, 0.08); border: 1px solid rgba(227, 194, 112, 0.4); }
  .active > div { display: flex; flex-direction: column; gap: 2px; }
  .big { font-family: var(--f-display); font-size: 30px; font-weight: 800; color: var(--gold-glow); }
</style>
