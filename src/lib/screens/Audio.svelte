<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import { debounce, on } from "../bus";
  import Switch from "../components/Switch.svelte";
  import AppMixer from "../components/AppMixer.svelte";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../icons";
  import { getJson, setJson } from "../settings.svelte";
  import { notify } from "../toast.svelte";
  import type { AudioState, Device, DevicePrefs } from "../types";

  let { onnavigate }: { onnavigate: (id: string) => void } = $props();

  let audio: AudioState | null = $state(null);
  let error = $state("");
  let prefs: Record<string, DevicePrefs> = $state(getJson("audio.devices", {}));
  let lockLevel = $state(getJson<number>("audio.micLockLevel", 1));
  let editing: string | null = $state(null);
  let showHidden = $state(false);

  async function refresh() {
    try {
      audio = await api.audio.state();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    void refresh();
    const stop = on("audio-changed", debounce(refresh, 120));
    return () => void stop.then((f) => f());
  });

  function guessIcon(name: string): IconName {
    const n = name.toLowerCase();
    if (/(headphone|headset|kulaklık|earphone|airpods|buds)/.test(n)) return "headphones";
    if (/(hdmi|display|monitor|nvidia high definition|tv)/.test(n)) return "monitor";
    return "speaker";
  }
  const iconOf = (d: Device): IconName => prefs[d.id]?.icon ?? guessIcon(d.name);
  const nameOf = (d: Device) => prefs[d.id]?.name || d.name;
  const pct = (v: number) => Math.round(v * 100);

  async function savePrefs(id: string, p: Partial<DevicePrefs>) {
    prefs = { ...prefs, [id]: { ...prefs[id], ...p } };
    await setJson("audio.devices", prefs);
  }

  async function makeDefault(d: Device) {
    if (d.isDefault) return;
    try {
      await api.audio.setDefault(d.id);
      notify(`Varsayılan: ${nameOf(d)}`, iconOf(d));
      await refresh();
    } catch (e) {
      notify(String(e), "info");
    }
  }

  // Kaydırıcıyı sürüklerken çağrıları seyrelt.
  let pending: Record<string, ReturnType<typeof setTimeout>> = {};
  function volume(d: Device, level: number) {
    d.level = level;
    clearTimeout(pending[d.id]);
    pending[d.id] = setTimeout(() => api.audio.setVolume(d.id, level).catch((e) => notify(String(e), "info")), 60);
  }

  async function mute(d: Device) {
    await api.audio.setMute(d.id, !d.muted).catch((e) => notify(String(e), "info"));
    await refresh();
  }

  async function toggleLock(on: boolean) {
    try {
      await api.audio.setLock(on ? lockLevel : null);
      notify(on ? `Mikrofon %${pct(lockLevel)} seviyesinde kilitlendi` : "Mikrofon kilidi kapalı", "lock");
      await refresh();
    } catch (e) {
      notify(String(e), "info");
    }
  }

  let lockTimer: ReturnType<typeof setTimeout> | undefined;
  function setLockLevel(v: number) {
    lockLevel = v;
    clearTimeout(lockTimer);
    lockTimer = setTimeout(async () => {
      await setJson("audio.micLockLevel", v);
      if (audio?.lock != null) await api.audio.setLock(v).catch(() => {});
    }, 150);
  }

  const mic = $derived.by(() => audio?.inputs.find((d) => d.isDefault));
</script>

{#snippet deviceList(devices: Device[], kind: "out" | "in")}
  {@const visible = devices.filter((d) => showHidden || !prefs[d.id]?.hidden)}
  {#each visible as d (d.id)}
    <div class="dev" class:on={d.isDefault} class:hidden-dev={prefs[d.id]?.hidden}>
      <button class="pick" onclick={() => makeDefault(d)} title={d.isDefault ? "Varsayılan cihaz" : "Varsayılan yap"}>
        <span class="ic"><Icon name={kind === "in" ? "mic" : iconOf(d)} /></span>
        <span class="nm">
          <b>{nameOf(d)}</b>
          <span>{prefs[d.id]?.name ? d.name : d.isDefault ? "Varsayılan" : "Varsayılan yapmak için tıkla"}</span>
        </span>
      </button>
      <div class="vol">
        <button class="mini" title={d.muted ? "Sesi aç" : "Sessize al"} onclick={() => mute(d)}>
          <Icon name={d.muted ? (kind === "in" ? "micoff" : "mute") : kind === "in" ? "mic" : "speaker"} size={17} />
        </button>
        <input type="range" min="0" max="100" value={pct(d.level)} aria-label="{nameOf(d)} ses seviyesi"
          oninput={(e) => volume(d, +e.currentTarget.value / 100)} />
        <span class="tnum pct">{d.muted ? "—" : `%${pct(d.level)}`}</span>
        <button class="mini" title="Düzenle" onclick={() => (editing = editing === d.id ? null : d.id)}><Icon name="edit" size={16} /></button>
      </div>
      {#if editing === d.id}
        <div class="edit">
          <label class="field">
            <span>Takma ad</span>
            <input type="text" value={prefs[d.id]?.name ?? ""} placeholder={d.name}
              onchange={(e) => savePrefs(d.id, { name: e.currentTarget.value.trim() || null })} />
          </label>
          {#if kind === "out"}
            <div class="field">
              <span>Simge</span>
              <div class="seg">
                {#each [["headphones", "Kulaklık"], ["speaker", "Hoparlör"], ["monitor", "Monitör"]] as [ic, l] (ic)}
                  <button aria-pressed={iconOf(d) === ic} onclick={() => savePrefs(d.id, { icon: ic as DevicePrefs["icon"] })}>{l}</button>
                {/each}
              </div>
            </div>
          {/if}
          <label class="row hide-row">
            <Switch label="Listede gizle" checked={!!prefs[d.id]?.hidden} onchange={(v) => savePrefs(d.id, { hidden: v })} />
            <span class="sub small">Listede ve kısayolla geçişte gizle</span>
          </label>
        </div>
      {/if}
    </div>
  {/each}
{/snippet}

<div class="audio">
  {#if error}
    <section class="panel wide warn"><Icon name="info" /><span>{error}</span></section>
  {:else if audio && !audio.supported}
    <section class="panel wide warn"><Icon name="info" /><span>Ses ayarları sadece Windows'ta çalışır.</span></section>
  {/if}

  <section class="panel">
    <div class="spread">
      <h3 class="h2"><Icon name="speaker" />Ses çıkışı</h3>
      <span class="sub small">Kısayol: <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>A</kbd></span>
    </div>
    {#if audio}{@render deviceList(audio.outputs, "out")}{/if}
  </section>

  <section class="panel">
    <div class="spread">
      <h3 class="h2"><Icon name="headphones" />Uygulama sesleri</h3>
      <span class="sub small">Oyunda Ctrl+Shift+G panelinde de var</span>
    </div>
    <AppMixer max={14} />
  </section>

  <section class="panel">
    <div class="spread">
      <h3 class="h2"><Icon name="mic" />Mikrofon</h3>
      <span class="sub small">Kısayol: <kbd>Ctrl</kbd> <kbd>Alt</kbd> <kbd>M</kbd></span>
    </div>
    {#if audio}{@render deviceList(audio.inputs, "in")}{/if}

    <div class="lock" class:active={audio?.lock != null}>
      <div class="line">
        <div>
          <b><Icon name="lock" size={16} /> Seviye kilidi</b>
          <span class="sub small">Bir program (Discord, tarayıcı…) mikrofonu kısarsa anında geri çeker.</span>
        </div>
        <Switch label="Mikrofon seviye kilidi" checked={audio?.lock != null} onchange={toggleLock} />
      </div>
      <label class="field">
        <span>Kilit seviyesi <b class="tnum">%{pct(lockLevel)}</b></span>
        <input type="range" min="10" max="100" value={pct(lockLevel)} oninput={(e) => setLockLevel(+e.currentTarget.value / 100)} />
      </label>
      {#if mic}<span class="sub small tnum">Şu an: {nameOf(mic)} · %{pct(mic.level)}{mic.muted ? " · sessiz" : ""}</span>{/if}
    </div>
  </section>

  <section class="panel">
    <h3 class="h2"><Icon name="keyboard" />Kısayollar</h3>
    <p class="sub small">Oyunun içinden de çalışır.</p>
    <ul class="keys">
      <li><span>Kulaklık ↔ hoparlör</span><span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>A</kbd></span></li>
      <li><span>Mikrofonu sessize al</span><span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>M</kbd></span></li>
      <li><span>Mikrofon kilidi</span><span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>L</kbd></span></li>
      <li><span>Mikrofon ±5 / %100</span><span><kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>PgUp</kbd><kbd>PgDn</kbd><kbd>Home</kbd></span></li>
    </ul>
    <span class="sub small">Varsayılan tuşlar; değiştirdiysen ayarlarda yenisi yazar.</span>
    <button class="btn ghost" onclick={() => onnavigate("settings/kisayollar")}><Icon name="keyboard" />Kısayolları düzenle</button>
  </section>

  <section class="panel">
    <h3 class="h2"><Icon name="search" />Mikrofonun neden kısılıyor?</h3>
    <ul class="check">
      <li><Icon name="check" size={16} /><span>Discord → Ses ve Görüntü → <b>Otomatik Kazanç Kontrolü</b> açık olabilir.</span></li>
      <li><Icon name="check" size={16} /><span>Tarayıcıdaki sesli sohbetler (Meet, web Discord) seviyeyi kendisi ayarlayabilir.</span></li>
      <li><Icon name="check" size={16} /><span>Teams / Zoom'daki "mikrofonu otomatik ayarla" seçeneği.</span></li>
      <li><Icon name="check" size={16} /><span>Windows: Mikrofon özellikleri → Gelişmiş → "Uygulamaların bu cihazın özel denetimini almasına izin ver".</span></li>
    </ul>
    <p class="sub small">Kilit açıkken hangisi olursa olsun seviye geri çekilir; asıl sebebi kapatmak da iyi olur.</p>
    <label class="row hide-row">
      <Switch label="Gizlenen cihazları göster" checked={showHidden} onchange={(v) => (showHidden = v)} />
      <span class="sub small">Gizlenen cihazları göster</span>
    </label>
  </section>
</div>

<style>
  .audio { display: grid; grid-template-columns: repeat(auto-fit, minmax(380px, 1fr)); gap: 16px; max-width: 1400px; margin: 0 auto; align-items: start; }
  .panel { display: flex; flex-direction: column; gap: 10px; }
  .wide { grid-column: 1 / -1; }
  .warn { flex-direction: row; align-items: center; color: var(--gold); }
  .dev { border: 1px solid var(--line); border-radius: 12px; background: rgba(255, 255, 255, 0.02); padding: 8px 10px; display: flex; flex-direction: column; gap: 6px; }
  .dev.on { border-color: var(--gold); background: rgba(227, 194, 112, 0.06); }
  .dev.hidden-dev { opacity: 0.55; }
  .pick { display: flex; align-items: center; gap: 12px; border: 0; background: none; padding: 2px; text-align: left; min-width: 0; }
  .ic { width: 38px; height: 38px; border-radius: 10px; display: grid; place-items: center; background: var(--coal); color: var(--muted); flex: none; }
  .on .ic { color: var(--gold); background: rgba(227, 194, 112, 0.14); }
  .nm { display: flex; flex-direction: column; min-width: 0; }
  .nm b { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nm span { font-size: 12px; color: var(--dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .on .nm span { color: var(--gold-dim); }
  .vol { display: grid; grid-template-columns: auto minmax(0, 1fr) 44px auto; gap: 8px; align-items: center; padding-left: 50px; }
  .pct { font-size: 12px; color: var(--muted); text-align: right; }
  .mini { border: 0; background: transparent; color: var(--dim); padding: 5px; border-radius: 6px; display: grid; place-items: center; }
  .mini:hover { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .edit { display: flex; flex-direction: column; gap: 10px; padding: 10px 0 4px 50px; border-top: 1px solid var(--line); }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field > span { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .field > span b { color: var(--gold); margin-left: 4px; }
  .field input[type="text"] { background: var(--coal-2); border: 1px solid var(--line); border-radius: 8px; padding: 7px 10px; }
  .hide-row { gap: 10px; }
  .lock { border-top: 1px solid var(--line); padding-top: 12px; margin-top: 4px; display: flex; flex-direction: column; gap: 10px; }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  .line > div { display: flex; flex-direction: column; gap: 2px; }
  .line b { display: inline-flex; align-items: center; gap: 6px; }
  .lock.active b { color: var(--gold-glow); }
  .check { margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: 8px; color: var(--muted); }
  .check li { display: flex; gap: 10px; align-items: flex-start; }
  .check li :global(svg) { color: var(--teal); margin-top: 2px; }
  kbd { font-family: var(--f-body); font-size: 11px; font-weight: 600; padding: 1px 6px; border-radius: 5px; background: var(--coal); border: 1px solid var(--line-2); }
  @media (max-width: 1000px) { .audio { grid-template-columns: minmax(0, 1fr); } }
  .keys { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .keys li { display: flex; justify-content: space-between; align-items: center; gap: 10px; color: var(--muted); }
  .keys li span:last-child { display: flex; gap: 3px; }
</style>
