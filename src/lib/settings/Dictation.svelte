<script lang="ts">
  // Sesle yazma: Ctrl + Alt basılı tut, konuş, bırak; yazı odaktaki programa yazılır.
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Switch from "../components/Switch.svelte";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { DictationEntry, DictationStatus, ToolProgress, WhisperStatus } from "../types";

  let status: DictationStatus | null = $state(null);
  let whisper: WhisperStatus | null = $state(null);
  let busy: string | null = $state(null);
  let progress: ToolProgress | null = $state(null);
  let lang = $state(getJson<string>("dictation.lang", "tr"));
  let gpu = $state(getJson("dictation.gpu", true));
  let tidy = $state(getJson("dictation.tidy", false));
  let words = $state(getJson<string>("dictation.words", ""));
  let sounds = $state(getJson("dictation.sounds", true));
  let showRaw: number | null = $state(null);

  async function refresh() {
    [status, whisper] = await Promise.all([api.dictation.status(), api.transcripts.whisperStatus().catch(() => null)]);
  }

  onMount(() => {
    void refresh();
    const stops = [
      on<{ phase: DictationStatus["phase"]; error: string | null }>("dictation-state", (p) => {
        if (status) status = { ...status, phase: p.phase, lastError: p.error };
      }),
      on<DictationEntry>("dictation-entry", (e) => {
        if (status) status = { ...status, history: [e, ...status.history].slice(0, 20) };
      }),
      on<ToolProgress>("tools-progress", (p) => {
        if (busy) progress = p;
      }),
    ];
    return () => stops.forEach((s) => void s.then((f) => f()));
  });

  async function setOpt(key: string, value: unknown) {
    await setJson(key, value);
    status = await api.dictation.apply();
  }

  async function toggle(v: boolean) {
    await setOpt("dictation.enabled", v);
    notify(v ? "Sesle yazma açık: Ctrl + Alt basılı tut, konuş" : "Sesle yazma kapalı", v ? "mic" : "info");
  }

  async function install(part: "whisper" | "gpu" | "llm") {
    busy = part;
    progress = null;
    try {
      status = await api.dictation.install(part);
      whisper = await api.transcripts.whisperStatus().catch(() => whisper);
      notify(part === "gpu" ? "NVIDIA hızlandırma kuruldu" : part === "llm" ? "Toparlayıcı kuruldu" : "Whisper hazır", "check");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      busy = null;
      progress = null;
    }
  }

  async function useModel(id: string) {
    await setJson("transcript.model", id);
    busy = "whisper";
    try {
      whisper = await api.transcripts.installWhisper(id);
      status = await api.dictation.apply();
    } catch (e) {
      notify(String(e), "info");
    } finally {
      busy = null;
      progress = null;
    }
  }

  async function warmUp() {
    busy = "warm";
    try {
      status = await api.dictation.warmUp();
      notify(status.gpuActive ? "Hazır · ekran kartında çalışıyor" : "Hazır · işlemcide çalışıyor", "check");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      busy = null;
    }
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      notify("Kopyalandı", "copy");
    } catch {
      notify(text, "info");
    }
  }

  const PHASES: Record<DictationStatus["phase"], string> = {
    idle: "Hazır",
    listening: "Dinliyor…",
    transcribing: "Yazıya döküyor…",
    tidying: "Toparlıyor…",
  };
  const pct = $derived.by(() => {
    const p = progress as ToolProgress | null;
    return p && p.total ? Math.round((p.done / p.total) * 100) : null;
  });
</script>

{#if status && !status.supported}
  <section class="panel"><p class="sub">Sesle yazma yalnızca Windows'ta çalışır.</p></section>
{:else if status}
  <section class="panel hero" class:live={status.phase !== "idle"}>
    <div class="line">
      <div>
        <h3 class="h2"><Icon name="mic" />Sesle yazma</h3>
        <span class="sub small">
          <kbd>Ctrl</kbd> + <kbd>Alt</kbd> (sol Alt) basılı tut, konuş, bırak: söylediğin, o an hangi programdaysan oraya
          yazılır. Her şey bu bilgisayarda olur; ses hiçbir yere gönderilmez.
        </span>
      </div>
      <Switch label="Sesle yazma" checked={status.enabled} onchange={toggle} disabled={!status.whisperReady} />
    </div>
    {#if status.enabled}
      <div class="state">
        <span class="dot" class:on={status.phase !== "idle"}></span>
        <b>{PHASES[status.phase]}</b>
        {#if status.lastError && status.phase === "idle"}<span class="err small">{status.lastError}</span>{/if}
      </div>
    {/if}
    {#if !status.whisperReady}
      <div class="line">
        <span class="warn small">Sesle yazma için Whisper'ın sunucu parçası ve seçili model gerekiyor (bir kez iner).</span>
        <button class="btn" disabled={busy !== null} onclick={() => install("whisper")}>
          <Icon name="download" size={17} />{busy === "whisper" ? "İndiriliyor…" : "Hazırla"}
        </button>
      </div>
    {/if}
    <div class="line">
      <div><b>Ses efekti</b><span class="sub small">Dinlemeye başlarken ve bırakınca kısa bir "tık" sesi.</span></div>
      <Switch label="Ses efekti" checked={sounds} onchange={(v) => ((sounds = v), void setJson("dictation.sounds", v))} />
    </div>
    <p class="sub small">
      Türkçe klavyede <b>AltGr</b> (@, €, {"{"}…) dikteyi başlatmaz. Ctrl+Alt+N gibi kısayollar da çalışmaya devam eder:
      araya başka bir tuş girerse dikte iptal olur.
    </p>
  </section>

  <section class="panel">
    <h3 class="h2"><Icon name="note" />1 · Yazıya dökme (Whisper)</h3>
    {#if whisper}
      {#each whisper.models as m (m.id)}
        <div class="line tool">
          <div><b>{m.label}{#if whisper.selected === m.id && m.installed} · kullanılıyor{/if}</b><span class="sub small">{m.detail}</span></div>
          {#if m.installed && whisper.program && whisper.selected === m.id}
            <span class="ok"><Icon name="check" size={15} />Hazır</span>
          {:else}
            <button class="btn ghost" disabled={busy !== null} onclick={() => useModel(m.id)}>
              {busy === "whisper" ? "İndiriliyor…" : m.installed ? "Bunu kullan" : `İndir (~${m.sizeMb} MB)`}
            </button>
          {/if}
        </div>
      {/each}
    {/if}

    <div class="line">
      <div>
        <b>NVIDIA ekran kartıyla hızlandır</b>
        <span class="sub small">
          {#if status.gpuError}
            <span class="err">Ekran kartında açılamadı, işlemci kullanılıyor ({status.gpuError}). Ekran kartı sürücüsünü güncellemek çözebilir.</span>
          {:else if status.gpuActive === true}
            Şu an ekran kartında çalışıyor: kısa bir dikte 1 saniyenin altında yazılır.
          {:else if status.gpuInstalled}
            Kurulu. İlk dikteyle (ya da "Hazırla" ile) model ekran kartına yüklenir.
          {:else}
            Dengeli model işlemcide 5–15 sn, ekran kartında ~1 sn. Bir kez iner (~650 MB). Şarkı sözü çıkarmayı da hızlandırır.
          {/if}
        </span>
      </div>
      {#if status.gpuInstalled}
        <Switch label="Ekran kartını kullan" checked={gpu} onchange={(v) => ((gpu = v), void setOpt("dictation.gpu", v))} />
      {:else}
        <button class="btn" disabled={busy !== null} onclick={() => install("gpu")}><Icon name="download" size={17} />{busy === "gpu" ? "İndiriliyor…" : "Kur (~650 MB)"}</button>
      {/if}
    </div>

    <div class="line">
      <div><b>Konuşma dili</b><span class="sub small">Türkçe seçiliyken araya giren İngilizce sözcükler de çoğunlukla doğru yazılır.</span></div>
      <div class="seg">
        {#each [["tr", "Türkçe"], ["en", "İngilizce"], ["auto", "Otomatik"]] as [v, l] (v)}
          <button aria-pressed={lang === v} onclick={() => ((lang = v), void setOpt("dictation.lang", v))}>{l}</button>
        {/each}
      </div>
    </div>

    <label class="field">
      <span>Özel sözcükler</span>
      <textarea
        rows="2"
        placeholder="GMID, Merkut, Valorant, CS2, Discord…"
        bind:value={words}
        onblur={() => void setOpt("dictation.words", words.trim())}
      ></textarea>
      <span class="sub small">Sık kullandığın adlar ve terimler: doğru yazılmaları için Whisper'a ipucu olarak verilir.</span>
    </label>

    <div class="row">
      <button class="btn ghost" disabled={busy !== null || !status.whisperReady} onclick={warmUp}>
        <Icon name="bolt" size={17} />{busy === "warm" ? "Yükleniyor…" : "Hazırla (modeli şimdi yükle)"}
      </button>
      <span class="sub small">15 dakika kullanılmazsa model bellekten çıkar (oyunda ekran kartı boş kalsın); sonraki dikte birkaç saniye geç başlar.</span>
    </div>
  </section>

  <section class="panel">
    <div class="line">
      <div>
        <h3 class="h2"><Icon name="sparkle" />2 · Toparlama (deneme)</h3>
        <span class="sub small">
          Yerel bir yapay zekâ (Gemma 3 4B) yazıyı temize çeker: noktalama, paragraflar, "ıı, şey" gibi dolgular,
          "yarın, yok yok cuma" gibi düzeltmeler ve devrik cümleler. Ekran kartında birkaç saniye sürer; şüpheli bir
          sonuç çıkarsa ham yazı yapıştırılır.
        </span>
      </div>
      {#if status.llmInstalled}
        <Switch label="Toparla" checked={tidy} onchange={(v) => ((tidy = v), void setOpt("dictation.tidy", v))} />
      {:else}
        <button class="btn" disabled={busy !== null} onclick={() => install("llm")}><Icon name="download" size={17} />{busy === "llm" ? "İndiriliyor…" : "Kur (~3 GB)"}</button>
      {/if}
    </div>
  </section>

  {#if busy && progress && progress.stage !== "done"}
    <section class="panel">
      <b>{progress.name}</b>
      <div class="pbar"><i style:width="{progress.stage === 'extract' ? 100 : (pct ?? 5)}%"></i></div>
      <span class="sub small tnum">{progress.stage === "extract" ? "Açılıyor…" : pct !== null ? `%${pct}` : "İndiriliyor…"}</span>
    </section>
  {/if}

  <section class="panel">
    <h3 class="h2"><Icon name="edit" />Dene</h3>
    <textarea class="try" rows="4" placeholder="Buraya tıkla, Ctrl + Alt basılı tutup konuş, bırak…"></textarea>
    {#if status.history.length}
      <h4 class="sub small up">Son dikteler</h4>
      {#each status.history as h, i (h.at)}
        <div class="hist">
          <div class="htext">
            <p>{showRaw === i ? h.raw : h.text}</p>
            <span class="sub small tnum">
              {ago(h.at)} · {(h.ms / 1000).toFixed(1)} sn{#if h.tidied}{" · toparlandı · "}<button class="link" onclick={() => (showRaw = showRaw === i ? null : i)}>{showRaw === i ? "toparlanmışı göster" : "hamını göster"}</button>{/if}
            </span>
          </div>
          <button class="mini" title="Kopyala" onclick={() => copy(showRaw === i ? h.raw : h.text)}><Icon name="copy" size={16} /></button>
        </div>
      {/each}
      <span class="sub small">Yazı bir yere yapışmadıysa (yönetici olarak açılmış programlar gibi) buradan kopyalayabilirsin.</span>
    {/if}
  </section>
{/if}

<style>
  .panel { display: flex; flex-direction: column; gap: 14px; }
  .hero { border-color: rgba(227, 194, 112, 0.35); transition: box-shadow 0.3s; }
  .hero.live { box-shadow: 0 0 0 1px var(--gold), 0 0 30px rgba(227, 194, 112, 0.2); }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .line > div:first-child { display: flex; flex-direction: column; gap: 4px; min-width: 0; flex: 1 1 260px; }
  .tool { padding: 8px 10px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); }
  .ok { color: var(--good); display: inline-flex; gap: 4px; align-items: center; font-weight: 600; font-size: 13px; }
  .state { display: flex; align-items: center; gap: 8px; }
  .dot { width: 10px; height: 10px; border-radius: 50%; background: var(--good); box-shadow: 0 0 8px rgba(92, 192, 138, 0.6); }
  .dot.on { background: var(--gold); box-shadow: 0 0 12px rgba(227, 194, 112, 0.9); animation: pulse 1s ease-in-out infinite; }
  @keyframes pulse { 50% { opacity: 0.4; } }
  .err { color: var(--bad); }
  .warn { color: var(--gold); margin: 0; }
  kbd { font-family: ui-monospace, Consolas, monospace; font-size: 11.5px; border: 1px solid var(--line-2); border-bottom-width: 2px; border-radius: 6px; padding: 0 6px; color: var(--text); background: var(--coal-2); }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field > span:first-child { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  textarea { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 10px; padding: 9px 12px; color: var(--text); font: inherit; resize: vertical; }
  textarea:focus { outline: none; border-color: var(--gold-dim); }
  .try { min-height: 96px; }
  .pbar { height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
  .pbar i { display: block; height: 100%; background: linear-gradient(90deg, var(--teal), var(--neon)); transition: width 0.3s; }
  .up { margin: 4px 0 0; text-transform: uppercase; letter-spacing: 0.06em; font-weight: 700; }
  .hist { display: flex; gap: 10px; align-items: flex-start; padding: 10px 12px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); }
  .htext { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .htext p { margin: 0; white-space: pre-line; }
  .mini { border: 0; background: transparent; color: var(--dim); padding: 6px; border-radius: 7px; display: inline-flex; }
  .mini:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; font-size: inherit; }
  .link:hover { color: var(--neon); }
</style>
