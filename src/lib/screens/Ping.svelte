<script lang="ts">
  // Bağlantı Testi: modem, internet, GMID sunucuları ve Steam'e aynı anda ping; en üstte tek cümlelik teşhis
  // ("sorun ev ağında / sağlayıcıda / sunucuda"). Her satırda ortalama, en iyi/kötü, titreme, kayıp ve
  // her ping'in küçük çubuğu (kırmızı çizgi: yanıt gelmedi).
  import { onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { GROUPS, msText, tone } from "../ping";
  import { notify } from "../toast.svelte";
  import type { DnsBench, DnsStatus, NetProbe, NetReport } from "../types";

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

  onMount(() => {
    void run();
    void api.net.dns().then((d) => (dns = d)).catch(() => {});
  });

  // ---- DNS seçici ----
  let dns = $state<DnsStatus | null>(null);
  let bench = $state<DnsBench[]>([]);
  let benching = $state(false);
  let setting = $state<string | null>(null);
  async function measure() {
    benching = true;
    try {
      bench = await api.net.dnsBench();
    } finally {
      benching = false;
    }
  }
  const msOf = (id: string) => bench.find((b) => b.id === id)?.ms;
  const fastest = $derived(
    bench.filter((b) => b.id !== "current" && b.ms != null).sort((a, b) => (a.ms ?? 0) - (b.ms ?? 0))[0]?.id ?? null,
  );
  async function useDns(id: string) {
    setting = id;
    try {
      dns = await api.net.setDns(id);
      notify(id === "auto" ? "DNS otomatiğe döndü" : `DNS: ${dns.presets.find((p) => p.id === id)?.name}`, "check");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      setting = null;
    }
  }

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

  <section class="panel dns">
    <div class="ghead">
      <h3 class="h2"><Icon name="network" size={18} />DNS</h3>
      <span class="sub small">
        Şu an: <b>{dns ? (dns.preset ? dns.presets.find((p) => p.id === dns!.preset)?.name : "Otomatik (internet sağlayıcın)") : "…"}</b>
        {#if dns?.servers.length}<span class="tnum"> · {dns.servers.filter((s) => !s.includes(":")).join(", ")}</span>{/if}
        {#if msOf("current") != null}<span class="tnum"> · {msOf("current")} ms</span>{/if}
      </span>
      <button class="btn ghost sm" disabled={benching} onclick={measure}><Icon name="refresh" size={14} />{benching ? "Ölçülüyor…" : "Hızlarını ölç"}</button>
    </div>
    <div class="dlist">
      {#each dns?.presets ?? [] as p (p.id)}
        {@const ms = msOf(p.id)}
        <div class="drow" class:on={dns?.preset === p.id}>
          <div class="who">
            <b>{p.name}{#if fastest === p.id}<span class="best">en hızlı</span>{/if}</b>
            <span class="sub small">{p.detail}</span>
          </div>
          <span class="tnum dms" class:none={ms === null}>{ms === undefined ? "" : ms === null ? "yanıt yok" : `${ms} ms`}</span>
          {#if dns?.preset === p.id}
            <span class="cur"><Icon name="check" size={14} />Kullanılıyor</span>
          {:else}
            <button class="btn sm" disabled={!!setting} onclick={() => useDns(p.id)}>{setting === p.id ? "Uygulanıyor…" : "Kullan"}</button>
          {/if}
        </div>
      {/each}
    </div>
    <div class="dfoot">
      <span class="sub small">
        Bütün bilgisayara uygulanır ve Windows yeniden açılınca da kalır; her seferinde açıp kapatmana gerek yok. Yönetici
        izni ister. <b>Oyun açıkken değiştirilmez.</b> DNS yalnız site ve sunucu adlarının adrese çevrilmesini etkiler; oyun
        sunucusuna giden bağlantıyı ve ping'i değiştirmez.
      </span>
      {#if dns?.preset}
        <button class="btn ghost sm" disabled={!!setting} onclick={() => useDns("auto")}>{setting === "auto" ? "Uygulanıyor…" : "Otomatiğe dön"}</button>
      {/if}
    </div>
  </section>

  <p class="sub small foot">
    50 ms altı çok iyi, 50–100 ms oynanır, 100 ms üstü gecikme hissedilir. <b>Titreme</b> (ping'in oynaması) ve <b>kayıp</b>,
    ping'in kendisinden bile önemlidir: takılma ve ışınlanma bunlardan olur. Oyundayken Oyun İçi Panel'deki
    <b>Ping</b> kartından tek tıkla ölçebilirsin.
  </p>
</div>

<style>
  .dns { display: flex; flex-direction: column; gap: 10px; }
  .dns .ghead { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .dns .ghead .sub { flex: 1; }
  .dlist { display: flex; flex-direction: column; gap: 6px; }
  .drow { display: grid; grid-template-columns: minmax(0, 1fr) 90px 130px; align-items: center; gap: 12px; padding: 10px 12px; border-radius: 12px; background: rgba(255, 255, 255, 0.03); border: 1px solid transparent; }
  .drow.on { border-color: var(--gold); background: rgba(227, 194, 112, 0.07); }
  .drow .who { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .best { margin-left: 8px; padding: 1px 7px; border-radius: 999px; font-size: 11px; font-weight: 700; color: var(--good); background: rgba(92, 192, 138, 0.14); }
  .dms { text-align: right; font-weight: 700; }
  .dms.none { color: var(--dim); font-weight: 500; font-size: 12px; }
  .drow .btn, .cur { justify-self: end; }
  .cur { display: inline-flex; align-items: center; gap: 5px; color: var(--gold-glow); font-weight: 700; font-size: 13px; }
  .dfoot { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  .dfoot .sub { flex: 1 1 320px; line-height: 1.5; }
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
