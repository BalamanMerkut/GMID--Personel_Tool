<script lang="ts">
  // Ders kaydı: hoca konuşurken kayıt; söylenenler ~25 saniyelik parçalar halinde canlı yazıya dökülür,
  // bitince dakika işaretleriyle Notlar'a kaydedilir.
  import { onMount, tick } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Switch from "../components/Switch.svelte";
  import { requestFocus } from "../focus.svelte";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { notify } from "../toast.svelte";
  import type { Device, LectureStatus } from "../types";

  let { onnavigate }: { onnavigate: (route: string) => void } = $props();

  let st = $state<LectureStatus | null>(null);
  let whisperReady = $state(true);
  let mics = $state<Device[]>([]);
  let title = $state(getJson<string>("lecture.title", ""));
  /** "system": bilgisayarın sesi (online ders); "": varsayılan mikrofon; diğerleri mikrofon kimliği. */
  let mic = $state(getJson<string>("lecture.source", "system"));
  let keepAudio = $state(getJson<boolean>("lecture.keepAudio", false));
  let starting = $state(false);
  let list = $state<HTMLElement | null>(null);
  let follow = $state(true);

  async function refresh() {
    try {
      const before = st?.segments.length ?? 0;
      st = await api.lecture.status();
      if (st.segments.length > before && follow) {
        await tick();
        list?.scrollTo({ top: list.scrollHeight, behavior: "smooth" });
      }
    } catch (e) {
      notify(String(e), "info");
    }
  }

  onMount(() => {
    void refresh();
    void api.dictation.status().then((s) => (whisperReady = s.whisperReady)).catch(() => {});
    void api.audio.state().then((a) => (mics = a.inputs)).catch(() => {});
    const stop = on("lecture-changed", () => void refresh());
    // Süre ve seviye göstergesi için kayıt sürerken sık bakılır.
    const t = setInterval(() => {
      if (st?.recording) void refresh();
    }, 1000);
    return () => {
      clearInterval(t);
      void stop.then((f) => f());
    };
  });

  async function start() {
    starting = true;
    void setJson("lecture.title", title.trim());
    void setJson("lecture.source", mic);
    void setJson("lecture.keepAudio", keepAudio);
    try {
      st = await api.lecture.start(title.trim(), mic, keepAudio);
      follow = true;
    } catch (e) {
      notify(String(e), "info");
    } finally {
      starting = false;
    }
  }
  async function pause() {
    if (st) st = await api.lecture.pause(!st.paused);
  }
  async function stop() {
    st = await api.lecture.stop();
  }
  async function again() {
    st = await api.lecture.clear();
  }
  function openNote() {
    if (!st?.noteId) return;
    requestFocus("note", st.noteId);
    onnavigate("notes");
  }
  function onScroll() {
    if (!list) return;
    follow = list.scrollHeight - list.scrollTop - list.clientHeight < 60;
  }

  const clock = (s: number) => {
    const h = Math.floor(s / 3600);
    const m = Math.floor((s / 60) % 60);
    const sec = Math.floor(s % 60);
    const p = (n: number) => String(n).padStart(2, "0");
    return h ? `${p(h)}:${p(m)}:${p(sec)}` : `${p(m)}:${p(sec)}`;
  };
  const active = $derived(!!st && (st.recording || st.finishing));
  const done = $derived(!!st && !st.recording && !st.finishing && (!!st.noteId || st.segments.length > 0));
  const words = $derived(st?.segments.reduce((n, s) => n + s.text.split(/\s+/).filter(Boolean).length, 0) ?? 0);
</script>

<div class="lec">
  {#if st && !st.supported}
    <section class="panel warn"><Icon name="info" /><span>Ders kaydı yalnız Windows'ta çalışır.</span></section>
  {/if}

  {#if !active && !done}
    <!-- Başlamadan önce -->
    <section class="panel setup">
      <div class="hd">
        <span class="ic"><Icon name="mic" size={22} /></span>
        <div>
          <h2 class="h2">Ders kaydı</h2>
          <p class="sub small">
            Ders başlarken başlat: hocanın söyledikleri <b>burada, arka planda</b> canlı yazıya dökülür (imlecin olduğu yere
            yazmaz; DEBSIS, Zoom, Teams ya da tarayıcıda ders açıkken rahatça başka şey yapabilirsin). Bitince dakika
            işaretleriyle <b>Notlar</b>'a kaydedilir. Ses bu bilgisayardan çıkmaz.
          </p>
        </div>
      </div>
      {#if !whisperReady}
        <div class="warnline">
          <Icon name="info" size={16} /><span>Önce sesle yazmanın Whisper modeli kurulmalı (bir kez iner).</span>
          <button class="btn ghost" onclick={() => onnavigate("settings/dikte")}>Sesle yazma ayarları</button>
        </div>
      {/if}
      <label class="field">
        <span>Ders adı</span>
        <input bind:value={title} placeholder="Örn. Fizik · Türev" maxlength="80" onkeydown={(e) => e.key === "Enter" && whisperReady && start()} />
      </label>
      <div class="field">
        <span>Neyi dinlesin?</span>
        <div class="src">
          <button class="srcbtn" aria-pressed={mic === "system"} onclick={() => (mic = "system")}>
            <Icon name="headphones" size={18} />
            <span><b>Bilgisayarın sesi</b><em>Online ders (DEBSIS, Zoom, Teams, YouTube…). Kulaklık takılıyken de hocayı duyar.</em></span>
          </button>
          <button class="srcbtn" aria-pressed={mic !== "system"} onclick={() => mic === "system" && (mic = "")}>
            <Icon name="mic" size={18} />
            <span><b>Mikrofon</b><em>Sınıfta, yüz yüze ders. Hocayı en iyi bilgisayarın kendi mikrofonu duyar.</em></span>
          </button>
        </div>
        {#if mic !== "system"}
          <select bind:value={mic}>
            <option value="">Windows'un varsayılan mikrofonu</option>
            {#each mics as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
          </select>
        {/if}
      </div>
      <div class="line">
        <div>
          <b>Ses kaydını da sakla</b>
          <span class="sub small">Açıksa ders sesi <code>Dersler</code> klasörüne WAV olarak kaydedilir (1 saat ≈ 115 MB). Kapalıyken yalnız yazı kalır.</span>
        </div>
        <Switch label="Ses kaydını da sakla" checked={keepAudio} onchange={(v) => (keepAudio = v)} />
      </div>
      <button class="btn big" disabled={!whisperReady || starting} onclick={start}>
        <span class="rec"></span>{starting ? "Başlıyor…" : "Kaydı başlat"}
      </button>
      <p class="sub small tip">
        Uzun derste bilgisayar yorulmasın diye NVIDIA kuruluysa ekran kartı kullanılır. Kayıt sürerken başka ekranlara geçebilirsin;
        GMID'i kapatırsan o ana kadar yazılan kısım yine not olarak saklanır.
      </p>
    </section>
  {:else if st}
    <!-- Kayıt sürüyor ya da bitti -->
    <section class="panel bar" class:live={st.recording && !st.paused} class:paused={st.paused}>
      <div class="hd">
        <span class="dotrec" class:on={st.recording && !st.paused}></span>
        <div>
          <h2 class="h2">{st.title}</h2>
          <span class="sub small">
            {#if st.finishing}Son kısım yazıya dökülüyor…
            {:else if st.recording && st.paused}Duraklatıldı
            {:else if st.recording}Kaydediliyor{st.busy ? " · yazıya dökülüyor…" : ""}
            {:else}Kaydedildi · {words} sözcük{/if}
          </span>
        </div>
      </div>
      <span class="time tnum">{clock(st.elapsed)}</span>
      {#if st.recording}
        <span class="meter" title="Mikrofon seviyesi"><i style:width="{Math.round((st.paused ? 0 : st.level) * 100)}%"></i></span>
        <div class="row">
          <button class="btn ghost" onclick={pause}><Icon name={st.paused ? "play" : "pause"} size={15} />{st.paused ? "Devam" : "Duraklat"}</button>
          <button class="btn" onclick={stop}><Icon name="check" size={15} />Bitir ve kaydet</button>
        </div>
      {:else if st.noteId}
        <div class="row">
          <button class="btn" onclick={openNote}><Icon name="note" size={15} />Notu aç</button>
          <button class="btn ghost" onclick={again}><Icon name="plus" size={15} />Yeni ders</button>
        </div>
      {/if}
    </section>
    {#if st.error}
      <section class="panel warn"><Icon name="info" /><span>{st.error}</span></section>
    {/if}

    <section class="panel text">
      <div class="spread">
        <h3 class="h2"><Icon name="note" />Yazı</h3>
        {#if st.recording}<span class="sub small">Her ~25 saniyede bir, konuşmanın duraksadığı yerde eklenir</span>{/if}
      </div>
      <div class="segs" bind:this={list} onscroll={onScroll}>
        {#each st.segments as s, i (i)}
          <p><b class="tnum">{clock(s.at)}</b>{s.text}</p>
        {:else}
          <p class="sub empty">{st.recording ? "Konuşma bekleniyor… İlk yazı yaklaşık 25 saniye sonra gelir." : "Yazıya dökülecek konuşma duyulmadı."}</p>
        {/each}
        {#if st.busy || st.finishing}<p class="sub typing"><span></span><span></span><span></span></p>{/if}
      </div>
      {#if st.audio && !st.recording}<span class="sub small">Ses kaydı: <code>{st.audio}</code></span>{/if}
    </section>
  {/if}
</div>

<style>
  .lec { display: flex; flex-direction: column; gap: 16px; max-width: 980px; margin: 0 auto; }
  .panel { display: flex; flex-direction: column; gap: 14px; }
  .warn { flex-direction: row; align-items: center; gap: 10px; color: var(--gold); }
  .hd { display: flex; align-items: center; gap: 14px; min-width: 0; }
  .hd > div { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .ic { width: 46px; height: 46px; border-radius: 12px; display: grid; place-items: center; background: rgba(227, 194, 112, 0.12); color: var(--gold); flex: none; }
  .warnline { display: flex; align-items: center; gap: 10px; color: var(--gold); flex-wrap: wrap; }
  .warnline span { flex: 1; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field > span { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  .field input { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 10px; padding: 10px 12px; font-size: 15px; }
  .field input:focus { border-color: var(--gold-dim); outline: none; }
  .field em { font-style: normal; }
  .src { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 10px; }
  .srcbtn { display: flex; gap: 12px; align-items: flex-start; text-align: left; padding: 12px 14px; border-radius: 12px; border: 1px solid var(--line-2); background: rgba(255, 255, 255, 0.02); color: var(--muted); }
  .srcbtn:hover { border-color: var(--gold-dim); }
  .srcbtn[aria-pressed="true"] { border-color: var(--gold); background: rgba(227, 194, 112, 0.08); color: var(--gold-glow); }
  .srcbtn span { display: flex; flex-direction: column; gap: 3px; }
  .srcbtn b { color: var(--text); font-size: 14px; }
  .srcbtn em { font-style: normal; font-size: 12px; color: var(--muted); }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 12px; }
  .line > div { display: flex; flex-direction: column; gap: 2px; }
  .btn.big { align-self: flex-start; padding: 12px 22px; font-size: 15px; }
  .rec { width: 12px; height: 12px; border-radius: 50%; background: #c0392b; box-shadow: 0 0 0 3px rgba(192, 57, 43, 0.25); }
  .tip { margin: 0; }

  .bar { flex-direction: row; align-items: center; gap: 18px; flex-wrap: wrap; }
  .bar .hd { flex: 1; }
  .bar.live { border-color: rgba(224, 106, 106, 0.5); }
  .bar.paused { border-color: var(--gold-dim); }
  .dotrec { width: 14px; height: 14px; border-radius: 50%; background: var(--dim); flex: none; }
  .dotrec.on { background: var(--bad); animation: pulse 1.4s ease-in-out infinite; }
  @keyframes pulse { 50% { box-shadow: 0 0 0 7px rgba(224, 106, 106, 0.18); } }
  .time { font-family: var(--f-display); font-size: 26px; font-weight: 700; letter-spacing: 0.02em; }
  .meter { width: 90px; height: 6px; border-radius: 999px; background: rgba(255, 255, 255, 0.08); overflow: hidden; }
  .meter i { display: block; height: 100%; background: var(--good); transition: width 0.25s; }

  .text { min-height: 320px; }
  .segs { display: flex; flex-direction: column; gap: 12px; max-height: 58vh; overflow: auto; padding-right: 6px; }
  .segs p { margin: 0; line-height: 1.6; font-size: 15px; display: grid; grid-template-columns: 64px minmax(0, 1fr); gap: 10px; }
  .segs p b { color: var(--gold-dim); font-size: 12.5px; padding-top: 3px; }
  .segs .empty, .segs .typing { display: block; }
  .typing span { display: inline-block; width: 6px; height: 6px; margin-right: 4px; border-radius: 50%; background: var(--muted); animation: blink 1.2s infinite; }
  .typing span:nth-child(2) { animation-delay: 0.2s; }
  .typing span:nth-child(3) { animation-delay: 0.4s; }
  @keyframes blink { 50% { opacity: 0.2; } }
</style>
