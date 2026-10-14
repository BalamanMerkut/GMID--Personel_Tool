<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { ago } from "../time";
  import { notify } from "../toast.svelte";
  import type { BridgeStatus, ToolsState, WhisperStatus } from "../types";

  let { onnavigate }: { onnavigate: (id: string) => void } = $props();

  let parallel = $state(getJson("videos.parallel", 2));
  let lyricsDb = $state(getJson("transcript.lyricsDb", true));
  let audioLang = $state(getJson<string>("videos.audioLang", "tr"));
  let subLangs = $state(getJson<string>("videos.subLangs", "tr"));
  let tools: ToolsState | null = $state(null);
  let bridge: BridgeStatus | null = $state(null);
  let busy: "update" | "reinstall" | null = $state(null);
  let whisper: WhisperStatus | null = $state(null);
  let whisperBusy: string | null = $state(null);

  onMount(async () => {
    [tools, bridge, whisper] = await Promise.all([
      api.videos.tools(),
      api.videos.bridge().catch(() => null),
      api.transcripts.whisperStatus().catch(() => null),
    ]);
  });

  async function useModel(id: string) {
    whisperBusy = id;
    try {
      whisper = await api.transcripts.installWhisper(id);
      notify("Whisper modeli hazır", "check");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      whisperBusy = null;
    }
  }

  async function update() {
    busy = "update";
    try {
      const before = tools?.version;
      tools = await api.videos.update(true);
      notify(tools.version === before ? `yt-dlp zaten güncel (${tools.version})` : `yt-dlp güncellendi: ${tools.version}`, "check");
    } finally {
      busy = null;
    }
  }

  async function reinstall() {
    busy = "reinstall";
    try {
      tools = await api.videos.installTools(true);
      notify("Araçlar yeniden indirildi", "check");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      busy = null;
    }
  }

  async function unpair() {
    await api.videos.unpair();
    bridge = await api.videos.bridge();
    notify("Eklenti bağlantısı kaldırıldı", "check");
  }
</script>

<section class="panel">
  <h3 class="h2"><Icon name="download" />İndirme</h3>
  <div class="line">
    <div>
      <b>Aynı anda indirme</b>
      <span class="sub small">Fazlası sırada bekler. İnternetin yavaşsa 1–2 iyi.</span>
    </div>
    <div class="seg grid2">
      {#each [1, 2, 3, 4] as n (n)}
        <button aria-pressed={parallel === n} onclick={() => { parallel = n; void setJson("videos.parallel", n); }}>{n}</button>
      {/each}
    </div>
  </div>
  <div class="line">
    <div>
      <b>İndirme klasörü</b>
      <span class="sub small">Videolar siteye göre ayrılır: <code>Videolar\YouTube</code>, <code>Videolar\Twitter</code>…</span>
    </div>
    <button class="btn ghost" onclick={() => api.openDataFolder("Videolar")}><Icon name="folder" />Aç</button>
  </div>
</section>

<section class="panel">
  <h3 class="h2"><Icon name="install" />Araçlar</h3>
  {#if tools}
    {#each tools.tools as t (t.id)}
      <div class="line tool">
        <div><b>{t.name}</b><span class="sub small">{t.purpose}</span></div>
        {#if t.installed}<span class="ok"><Icon name="check" size={15} />Kurulu</span>{:else}<span class="sub small">Kurulu değil</span>{/if}
      </div>
    {/each}
    {#if tools.ready}
      <div class="line">
        <div>
          <b>yt-dlp {tools.version ?? ""}</b>
          <span class="sub small">YouTube sık değiştiği için günde bir kez kendiliğinden güncellenir.</span>
        </div>
        <div class="row">
          <button class="btn ghost" onclick={update} disabled={busy !== null}><Icon name="refresh" />{busy === "update" ? "Bakılıyor…" : "Şimdi güncelle"}</button>
          <button class="btn ghost" onclick={reinstall} disabled={busy !== null}>{busy === "reinstall" ? "İndiriliyor…" : "Yeniden indir"}</button>
        </div>
      </div>
    {:else}
      <button class="btn" onclick={() => onnavigate("videos")}><Icon name="download" />Video İndirici'de kur</button>
    {/if}
  {/if}
</section>

<section class="panel">
  <h3 class="h2"><Icon name="film" />Ses dili (dublaj) ve altyazı</h3>
  <p class="sub small">
    Bazı videoların içinde birden çok <b>ses parçası</b> olur: dizi/film sitelerindeki dublajlar ya da YouTube'un
    kendiliğinden yaptığı <b>yapay zekâ dublajları</b> (videoyu başka dile çevrilmiş sesle okur). Burada hangisinin
    ineceğini seçersin. Bu, sitenin kendi ses parçalarından seçimdir; <b>Whisper ile ilgisi yoktur</b> (Whisper yalnız
    şarkı sözü ve konuşmaları yazıya döker, sesi değiştirmez). Seçtiğin dil videoda yoksa her zaman orijinal ses iner.
    Uygulamada indirirken video başına değiştirebilirsin; tarayıcı eklentisi bu tercihi kullanır.
  </p>
  <div class="line">
    <div>
      <b>Ses dili</b>
      <span class="sub small">
        {audioLang === "orig"
          ? "Videonun kendi sesi; dublaj parçaları yok sayılır. Arkadaşına atacaksan en güvenlisi."
          : audioLang === "tr"
            ? "Türkçe ses parçası varsa o iner (YouTube'un yapay zekâ dublajı da olabilir), yoksa orijinali."
            : "İngilizce ses parçası varsa o iner (Türkçe bir video bile İngilizce dublajla inebilir), yoksa orijinali."}
      </span>
    </div>
    <div class="seg">
      {#each [["orig", "Orijinal"], ["tr", "Türkçe dublaj"], ["en", "İngilizce"]] as [v, l] (v)}
        <button aria-pressed={audioLang === v} onclick={() => ((audioLang = v), void setJson("videos.audioLang", v))}>{l}</button>
      {/each}
    </div>
  </div>
  <div class="line">
    <div><b>Altyazı</b><span class="sub small">Sitenin altyazısı varsa ayrı dosya olarak videonun yanına iner; videoya gömülmez, oynatıcıda CC ile açılır.</span></div>
    <div class="seg">
      {#each [["tr", "Türkçe"], ["tr,en", "Türkçe + İngilizce"], ["none", "İndirme"]] as [v, l] (v)}
        <button aria-pressed={subLangs === v} onclick={() => ((subLangs = v), void setJson("videos.subLangs", v))}>{l}</button>
      {/each}
    </div>
  </div>
</section>

<section class="panel">
  <h3 class="h2"><Icon name="note" />Şarkı sözleri ve yazıya dökme</h3>
  <div class="line">
    <div>
      <b>Şarkı sözü veritabanı (LRCLIB)</b>
      <span class="sub small">
        Şarkılarda önce lrclib.net'in satır satır zamanlanmış sözlerine bakılır; en doğru sonuç budur. Yalnızca şarkının
        adı, sanatçısı ve süresi gönderilir.
      </span>
    </div>
    <div class="seg">
      {#each [[true, "Açık"], [false, "Kapalı"]] as const as [v, l] (l)}
        <button aria-pressed={lyricsDb === v} onclick={() => ((lyricsDb = v), void setJson("transcript.lyricsDb", v))}>{l}</button>
      {/each}
    </div>
  </div>
  <p class="sub small">
    <b>Whisper</b>: sözü ya da altyazısı bulunamayan videoların konuşmalarını ve şarkı sözlerini tamamen bu bilgisayarda
    yazıya döker. Program ve model ilk kullanımda bir kez iner; ses hiçbir yere gönderilmez.
  </p>
  {#if whisper}
    {#each whisper.models as m (m.id)}
      <div class="line tool">
        <div><b>{m.label}{#if whisper.selected === m.id && m.installed} · kullanılıyor{/if}</b><span class="sub small">{m.detail}</span></div>
        {#if m.installed && whisper.program && whisper.selected === m.id}
          <span class="ok"><Icon name="check" size={15} />Hazır</span>
        {:else}
          <button class="btn ghost" disabled={whisperBusy !== null} onclick={() => useModel(m.id)}>
            {whisperBusy === m.id ? "İndiriliyor…" : m.installed ? "Bunu kullan" : `İndir (~${m.sizeMb} MB)`}
          </button>
        {/if}
      </div>
    {/each}
  {/if}
</section>

<section class="panel">
  <h3 class="h2"><Icon name="sparkle" />Tarayıcı eklentisi</h3>
  <div class="line">
    <div>
      {#if bridge?.paired}
        <b class="on"><span class="dot"></span>Bağlı{bridge.browser ? ` · ${bridge.browser}` : ""}</b>
        <span class="sub small">{bridge.lastSeen ? `Son görülme ${ago(bridge.lastSeen)}` : "Bu oturumda henüz bağlanmadı"} · port {bridge.port || "—"}</span>
      {:else}
        <b>Bağlı değil</b>
        <span class="sub small">Kurulum ve eşleştirme adımları Video İndirici ekranında.</span>
      {/if}
    </div>
    {#if bridge?.paired}
      <button class="btn ghost" onclick={unpair}>Bağlantıyı kaldır</button>
    {:else}
      <button class="btn ghost" onclick={() => onnavigate("videos")}>Kur</button>
    {/if}
  </div>
</section>

<style>
  /* 1 2 / 3 4: toplu dursun. */
  .grid2 { display: grid; grid-template-columns: repeat(2, 40px); gap: 6px; flex: none; }
  .grid2 button { padding: 6px 0; border-radius: 10px; text-align: center; font-weight: 600; }
  .panel { display: flex; flex-direction: column; gap: 14px; }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .line > div:first-child { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1 1 240px; }
  .tool { padding: 8px 10px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); }
  .ok { color: var(--good); display: inline-flex; gap: 4px; align-items: center; font-weight: 600; font-size: 13px; }
  .on { color: var(--good); display: inline-flex; align-items: center; gap: 8px; }
  .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--good); box-shadow: 0 0 8px rgba(92, 192, 138, 0.6); }
  code { padding: 1px 6px; }
</style>
