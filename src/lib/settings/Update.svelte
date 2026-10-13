<script lang="ts">
  // Uygulama içinden güncelleme: GitHub'daki en son derlemeye bakar, tek tıkla indirip kurar.
  import { onMount } from "svelte";
  import { api, formatSize } from "../api";
  import { on } from "../bus";
  import Switch from "../components/Switch.svelte";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { UpdateStatus } from "../types";

  let status = $state<UpdateStatus | null>(null);
  let checking = $state(false);
  let installing = $state(false);
  let auto = $state(getJson<boolean>("update.auto", true));
  let now = $state(Date.now());

  onMount(() => {
    void api.update.status().then((s) => (status = s)).catch(() => {});
    const stop = on<UpdateStatus>("update", (s) => (status = s));
    const t = setInterval(() => (now = Date.now()), 30_000);
    return () => {
      clearInterval(t);
      void stop.then((f) => f());
    };
  });

  async function check() {
    checking = true;
    try {
      status = await api.update.check();
      if (status.error) notify(status.error, "info");
      else if (!status.available) notify("GMID Tool güncel", "check");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      checking = false;
    }
  }

  async function install() {
    installing = true;
    try {
      await api.update.install();
    } catch (e) {
      installing = false;
      notify(String(e), "info");
    }
  }
</script>

<section class="panel upd">
  <div class="spread">
    <h3 class="h2"><Icon name="download" />Güncelleme</h3>
    <span class="sub small tnum">
      {status?.current ? `Derleme ${status.current}` : "Geliştirme derlemesi"}{status?.checkedAt ? ` · son bakış ${ago(status.checkedAt, now)}` : ""}
    </span>
  </div>

  {#if status?.available && status.latest}
    <div class="new">
      <div class="nt">
        <b>Yeni sürüm: derleme {status.latest.build}</b>
        <span>{status.latest.title}</span>
        {#if status.latest.notes}<pre>{status.latest.notes}</pre>{/if}
      </div>
      {#if status.progress != null}
        <div class="prog"><span class="small">İndiriliyor · %{Math.round(status.progress * 100)}</span><span class="bar"><i style:width="{status.progress * 100}%"></i></span></div>
      {/if}
      <div class="row">
        <button class="btn" disabled={installing} onclick={install}>
          <Icon name="download" size={15} />{installing ? "Güncelleniyor…" : `Güncelle (${formatSize(status.latest.size)})`}
        </button>
        <span class="sub small">GMID kapanır, kurulum ilerleme penceresiyle çalışır ve bitince GMID yeniden açılır. Verilerin olduğu gibi kalır.</span>
      </div>
    </div>
  {:else}
    <p class="sub">
      {#if status && !status.current}
        Bu kopya elle derlenmiş; güncellemeler GitHub'dan indirilen kurulumlarda çalışır.
      {:else if status?.latest}
        GMID Tool güncel (derleme {status.latest.build}).
      {:else}
        Yeni bir sürüm çıkınca burada ve Windows bildirimiyle haber verilir.
      {/if}
    </p>
  {/if}
  {#if status?.error}<p class="err small">{status.error}</p>{/if}

  <div class="spread">
    <div class="auto">
      <Switch label="Kendiliğinden bak" checked={auto} onchange={(v) => ((auto = v), void setJson("update.auto", v))} />
      <span class="small">Açılışta ve 6 saatte bir yeni sürüme bak</span>
    </div>
    <button class="btn ghost sm" disabled={checking || installing} onclick={check}><Icon name="refresh" size={14} />{checking ? "Bakılıyor…" : "Şimdi denetle"}</button>
  </div>
</section>

<style>
  .upd { display: flex; flex-direction: column; gap: 12px; }
  .spread { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
  .h2 { margin: 0; }
  .new { display: flex; flex-direction: column; gap: 12px; padding: 14px 16px; border-radius: 12px; border: 1px solid var(--gold-dim); background: rgba(227, 194, 112, 0.07); }
  .nt { display: flex; flex-direction: column; gap: 4px; }
  .nt b { color: var(--gold-glow); }
  pre { margin: 4px 0 0; white-space: pre-wrap; font: inherit; font-size: 13px; color: var(--muted); max-height: 160px; overflow: auto; }
  .row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .prog { display: flex; flex-direction: column; gap: 6px; }
  .bar { display: block; height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--gold); transition: width 0.2s; }
  .auto { display: flex; align-items: center; gap: 10px; }
  .err { color: var(--bad); margin: 0; }
  p { margin: 0; }
</style>
