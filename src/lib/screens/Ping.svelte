<script lang="ts">
  // Bağlantı Testi: modem, internet, GMID sunucuları ve Steam'e aynı anda ping; en üstte tek cümlelik teşhis
  // ("sorun ev ağında / sağlayıcıda / sunucuda"). Her satırda ortalama, en iyi/kötü, titreme, kayıp ve
  // her ping'in küçük çubuğu (kırmızı çizgi: yanıt gelmedi).
  import { onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { GROUPS, msText, tone } from "../ping";
  import { notify } from "../toast.svelte";
  import type { NetProbe, NetReport } from "../types";

  let report = $state<NetReport | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function run() {
    if (busy) return;
    busy = true;
    error = null;
    try {
      report = await api.net.test(false);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => void run());

  const grouped = $derived(GROUPS.map((g) => ({ ...g, list: (report?.probes ?? []).filter((p) => p.group === g.id) })).filter((g) => g.list.length));
  const peak = (p: NetProbe) => Math.max(40, ...p.samples.map((s) => s ?? 0));

  async function copy() {
    if (!report) return;
    const lines = report.probes
      .filter((p) => p.avg != null)
      .map((p) => `${p.label}: ${msText(p.avg)} ms · titreme ${msText(p.jitter)} · kayıp %${p.loss}`);
    const text = `📶 **Bağlantı testi** · ${report.verdict.title}\n${report.verdict.detail}\n${lines.join("\n")}`;
    try {
      await navigator.clipboard.writeText(text);
      notify("Kopyalandı · Discord'da Ctrl+V", "copy");
    } catch (e) {
      notify(String(e), "info");
    }
  }
</script>

<div class="ping">
  <section class="panel hero {busy ? 'busy' : (report?.verdict.level ?? '')}">
    <span class="mark">
      {#if busy}<span class="pulse"><Icon name="network" size={28} /></span>
      {:else if report?.verdict.level === "good"}<Icon name="check" size={30} />
      {:else}<Icon name="network" size={28} />{/if}
    </span>
    <div class="tx">
      <b class="title">{busy ? "Ölçülüyor…" : report ? report.verdict.title : "Bağlantı testi"}</b>
      <span class="sub">
        {#if busy}Modem, internet, GMID sunucuları ve Steam'e aynı anda ping atılıyor (birkaç saniye).
        {:else if error}<span class="err">{error}</span>
        {:else if report}{report.verdict.detail}{/if}
      </span>
      {#if report?.link.kind && !busy}
        <span class="chip link"><Icon name={report.link.kind === "wifi" ? "network" : "link"} size={13} />{report.link.kind === "wifi" ? "Wi-Fi" : report.link.kind === "ethernet" ? "Kablo (Ethernet)" : "Ağ"}{report.link.name && !/^(wi-?fi|ethernet)$/i.test(report.link.name) ? ` · ${report.link.name}` : ""}</span>
      {/if}
    </div>
    <div class="acts">
      <button class="btn" disabled={busy} onclick={run}><Icon name="refresh" size={15} />{busy ? "Ölçülüyor…" : "Yeniden test et"}</button>
      <button class="btn ghost sm" disabled={busy || !report} onclick={copy}><Icon name="discord" size={14} />Kopyala</button>
    </div>
  </section>

  {#each grouped as g (g.id)}
    <section class="panel grp" class:dim={busy}>
      <div class="ghead">
        <h3 class="h2">{g.title}</h3>
        <span class="sub small">{g.hint}</span>
      </div>
      {#each g.list as p (p.id)}
        {@const t = tone(p)}
        <div class="row-p">
          <div class="who">
            <b>{p.label}</b>
            <span class="sub small">{p.host}{p.via === "game" ? " · oyun sorgusuyla (ping'e kapalı)" : ""}</span>
          </div>
          <div class="bars" aria-hidden="true">
            {#each p.samples as s, i (i)}
              {#if s == null}<i class="lost"></i>{:else}<i style:height="{Math.max(8, (s / peak(p)) * 100)}%"></i>{/if}
            {/each}
          </div>
          <div class="nums tnum">
            <span class="big {t}">{msText(p.avg)}<small>ms</small></span>
            <span class="sub small">
              {#if p.avg != null}en iyi {msText(p.min)} · en kötü {msText(p.max)} · titreme {msText(p.jitter)}{:else}yanıt yok{/if}
            </span>
          </div>
          <span class="loss tnum" class:bad={p.loss >= 3}>%{p.loss}<small>kayıp</small></span>
        </div>
      {/each}
    </section>
  {/each}

  <p class="sub small foot">
    50 ms altı çok iyi, 50–100 ms oynanır, 100 ms üstü gecikme hissedilir. <b>Titreme</b> (ping'in oynaması) ve <b>kayıp</b>,
    ping'in kendisinden bile önemlidir: takılma ve ışınlanma bunlardan olur. Oyundayken Oyun İçi Panel'deki
    <b>Ping</b> kartından tek tıkla ölçebilirsin.
  </p>
</div>

<style>
  .ping { display: flex; flex-direction: column; gap: 14px; max-width: 1100px; margin: 0 auto; }
  .hero { display: flex; align-items: center; gap: 18px; flex-wrap: wrap; border-width: 1px; }
  .hero.good { border-color: rgba(92, 192, 138, 0.55); background: linear-gradient(120deg, rgba(92, 192, 138, 0.1), rgba(34, 37, 42, 0.72) 60%); }
  .hero.warn { border-color: rgba(227, 194, 112, 0.6); background: linear-gradient(120deg, rgba(227, 194, 112, 0.1), rgba(34, 37, 42, 0.72) 60%); }
  .hero.bad { border-color: rgba(224, 106, 106, 0.6); background: linear-gradient(120deg, rgba(224, 106, 106, 0.1), rgba(34, 37, 42, 0.72) 60%); }
  .mark { width: 60px; height: 60px; border-radius: 18px; display: grid; place-items: center; flex: none; background: rgba(255, 255, 255, 0.05); color: var(--muted); }
  .good .mark { color: var(--good); background: rgba(92, 192, 138, 0.14); }
  .warn .mark { color: var(--gold); background: rgba(227, 194, 112, 0.14); }
  .bad .mark { color: var(--bad); background: rgba(224, 106, 106, 0.14); }
  .busy .mark { color: var(--teal); }
  .pulse { display: grid; animation: pulse 1.1s ease-in-out infinite; }
  .tx { flex: 1 1 320px; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .title { font-family: var(--f-display); font-size: 21px; }
  .err { color: var(--bad); }
  .link { align-self: flex-start; margin-top: 4px; }
  .acts { display: flex; gap: 8px; align-items: center; }
  .grp { display: flex; flex-direction: column; gap: 4px; transition: opacity 0.2s; }
  .grp.dim { opacity: 0.5; }
  .ghead { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; margin-bottom: 6px; }
  .row-p {
    display: grid; grid-template-columns: minmax(150px, 1.2fr) minmax(120px, 1fr) minmax(170px, auto) 64px; gap: 16px; align-items: center;
    padding: 10px 0; border-top: 1px solid rgba(68, 74, 81, 0.45);
  }
  .ghead + .row-p { border-top: 0; }
  .who { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .who b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .who .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bars { display: flex; align-items: flex-end; gap: 3px; height: 30px; }
  .bars i { flex: 1; max-width: 9px; border-radius: 2px; background: var(--teal); opacity: 0.85; }
  .bars i.lost { height: 100%; background: repeating-linear-gradient(180deg, var(--bad) 0 3px, transparent 3px 6px); opacity: 0.8; }
  .nums { display: flex; flex-direction: column; align-items: flex-end; gap: 2px; }
  .big { font-family: var(--f-display); font-size: 26px; font-weight: 800; line-height: 1; }
  .big small { font-size: 12px; margin-left: 3px; color: var(--dim); font-weight: 600; }
  .big.good { color: var(--good); }
  .big.warn { color: var(--gold); }
  .big.bad { color: var(--bad); }
  .big.none { color: var(--dim); }
  .loss { display: flex; flex-direction: column; align-items: flex-end; font-weight: 700; color: var(--muted); }
  .loss small { font-size: 10.5px; color: var(--dim); font-weight: 600; }
  .loss.bad { color: var(--bad); }
  .foot { line-height: 1.55; }
  @keyframes pulse { 50% { transform: scale(1.12); opacity: 0.6; } }
  @media (max-width: 760px) { .row-p { grid-template-columns: minmax(0, 1fr) auto; } .bars { display: none; } .loss { display: none; } }
</style>
