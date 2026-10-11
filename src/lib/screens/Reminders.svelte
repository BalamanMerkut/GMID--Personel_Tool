<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import { on } from "../bus";
  import Switch from "../components/Switch.svelte";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { fromInputs, relative, toInputs, when } from "../time";
  import { notify } from "../toast.svelte";
  import type { Reminder, Repeat } from "../types";

  const REPEATS: [Repeat, string][] = [
    ["none", "Bir kez"],
    ["daily", "Her gün"],
    ["weekdays", "Hafta içi"],
    ["weekly", "Her hafta"],
    ["monthly", "Her ay"],
  ];
  const repeatLabel = (r: Repeat) => REPEATS.find(([k]) => k === r)?.[1] ?? "";

  let list: Reminder[] = $state([]);
  let now = $state(Date.now());
  let sound = $state(getJson("reminders.sound", true));
  let toast = $state(getJson("reminders.toast", true));
  let showDone = $state(false);

  // form
  let editingId: string | null = $state(null);
  let title = $state("");
  let note = $state("");
  let repeat: Repeat = $state("none");
  const start = toInputs(Date.now() + 30 * 60_000);
  let date = $state(start.date);
  let time = $state(start.time);

  const due = $derived(list.filter((r) => !r.done && r.notified));
  const upcoming = $derived(list.filter((r) => !r.done && !r.notified));
  const done = $derived(list.filter((r) => r.done));
  const formDue = $derived(fromInputs(date, time));

  async function refresh() {
    list = await api.reminders.list();
  }

  onMount(() => {
    void refresh();
    const stop = on("reminders-changed", refresh);
    // "5 dk sonra" yazıları güncel kalsın.
    const clock = setInterval(() => (now = Date.now()), 30_000);
    return () => {
      clearInterval(clock);
      void stop.then((f) => f());
    };
  });

  function setDue(ts: number) {
    ({ date, time } = toInputs(ts));
  }

  function quick(kind: string) {
    const d = new Date();
    if (kind === "10") setDue(Date.now() + 10 * 60_000);
    if (kind === "30") setDue(Date.now() + 30 * 60_000);
    if (kind === "60") setDue(Date.now() + 60 * 60_000);
    if (kind === "evening") {
      d.setHours(20, 0, 0, 0);
      if (d.getTime() < Date.now()) d.setDate(d.getDate() + 1);
      setDue(d.getTime());
    }
    if (kind === "tomorrow") {
      d.setDate(d.getDate() + 1);
      d.setHours(9, 0, 0, 0);
      setDue(d.getTime());
    }
  }

  function resetForm() {
    editingId = null;
    title = "";
    note = "";
    repeat = "none";
    setDue(Date.now() + 30 * 60_000);
  }

  function editItem(r: Reminder) {
    editingId = r.id;
    title = r.title;
    note = r.note;
    repeat = r.repeat;
    setDue(r.dueAt);
  }

  async function save(e: SubmitEvent) {
    e.preventDefault();
    if (!formDue) return notify("Tarih ve saati kontrol et.", "info");
    if (formDue < Date.now() - 60_000 && repeat === "none") return notify("Geçmiş bir zamana hatırlatıcı kurulamaz.", "info");
    try {
      await api.reminders.save({ id: editingId, title, note, dueAt: formDue, repeat });
      notify(editingId ? "Hatırlatıcı güncellendi" : `Hatırlatıcı kuruldu · ${when(formDue)}`, "bell");
      resetForm();
      await refresh();
    } catch (err) {
      notify(String(err), "info");
    }
  }

  /** Gerçek yoldan geçen bir deneme: şu an için hatırlatıcı kurar, birkaç saniyede çalar. */
  async function testAlert() {
    try {
      await api.reminders.save({ id: null, title: "Deneme uyarısı", note: "Bunu görüyorsan uyarılar çalışıyor 🎉", dueAt: Date.now(), repeat: "none" });
      notify("Birkaç saniye içinde sağ altta kart ve Windows bildirimi çıkmalı", "bell");
      await refresh();
    } catch (err) {
      notify(String(err), "info");
    }
  }

  async function act(fn: () => Promise<void>, msg: string, icon: "check" | "bell" | "trash") {
    try {
      await fn();
      notify(msg, icon);
      await refresh();
    } catch (err) {
      notify(String(err), "info");
    }
  }
</script>

<div class="rem">
  <form class="panel form" onsubmit={save}>
    <h3 class="h2"><Icon name="bell" />{editingId ? "Hatırlatıcıyı düzenle" : "Yeni hatırlatıcı"}</h3>
    <label class="field">
      <span>Ne hatırlatılsın?</span>
      <input id="rem-title" type="text" maxlength="120" placeholder="ör. Çay demle" bind:value={title} required />
    </label>
    <div class="field">
      <span>Ne zaman?</span>
      <div class="chips">
        <button type="button" onclick={() => quick("10")}>10 dk sonra</button>
        <button type="button" onclick={() => quick("30")}>30 dk sonra</button>
        <button type="button" onclick={() => quick("60")}>1 saat sonra</button>
        <button type="button" onclick={() => quick("evening")}>Akşam 20:00</button>
        <button type="button" onclick={() => quick("tomorrow")}>Yarın 09:00</button>
      </div>
      <div class="row">
        <input id="rem-date" type="date" bind:value={date} aria-label="Tarih" />
        <input id="rem-time" type="time" bind:value={time} aria-label="Saat" />
        {#if formDue}<span class="sub small">{when(formDue, now)} · {relative(formDue, now)}</span>{/if}
      </div>
    </div>
    <label class="field">
      <span>Tekrar</span>
      <select id="rem-repeat" bind:value={repeat}>
        {#each REPEATS as [k, l] (k)}<option value={k}>{l}</option>{/each}
      </select>
    </label>
    <label class="field">
      <span>Not (isteğe bağlı)</span>
      <textarea id="rem-note" rows="2" maxlength="300" bind:value={note}></textarea>
    </label>
    <div class="row">
      <button class="btn" type="submit"><Icon name="check" />{editingId ? "Güncelle" : "Kur"}</button>
      {#if editingId}<button class="btn ghost" type="button" onclick={resetForm}>Vazgeç</button>{/if}
    </div>
    <div class="line">
      <span class="sub small">Zamanı gelince ekranın sağ altında küçük bir kart çıkar; oyunu ya da yazdığın şeyi kesmez.</span>
    </div>
    <label class="row">
      <Switch label="Uyarıda ses çal" checked={sound} onchange={(v) => { sound = v; void setJson("reminders.sound", v); }} />
      <span class="small">Uyarıda Windows bildirim sesi çal</span>
    </label>
    <label class="row">
      <Switch label="Windows bildirimi" checked={toast} onchange={(v) => { toast = v; void setJson("reminders.toast", v); }} />
      <span class="small">Windows bildirimi de gönder (bildirim merkezinde kalır)</span>
    </label>
    <button class="btn ghost sm" type="button" onclick={testAlert}><Icon name="bell" size={16} />Uyarıyı dene</button>
  </form>

  <div class="lists">
    {#if due.length}
      <section class="panel">
        <h3 class="h2 hot"><Icon name="bell" />Zamanı gelenler</h3>
        {#each due as r (r.id)}
          <div class="item hot">
            <div class="txt"><b>{r.title}</b><span class="sub small">{when(r.dueAt, now)} · {relative(r.dueAt, now)}{r.note ? ` · ${r.note}` : ""}</span></div>
            <div class="acts">
              <button class="btn ghost sm" onclick={() => act(() => api.reminders.snooze(r.id, 10), "10 dakika ertelendi", "bell")}>10 dk ertele</button>
              <button class="btn sm" onclick={() => act(() => api.reminders.complete(r.id), "Tamamlandı", "check")}>Tamam</button>
            </div>
          </div>
        {/each}
      </section>
    {/if}

    <section class="panel">
      <h3 class="h2"><Icon name="clock" />Yaklaşanlar</h3>
      {#each upcoming as r (r.id)}
        <div class="item">
          <div class="when tnum"><b>{when(r.dueAt, now)}</b><span>{relative(r.dueAt, now)}</span></div>
          <div class="txt">
            <b>{r.title}</b>
            <span class="sub small">{r.repeat !== "none" ? repeatLabel(r.repeat) : "Bir kez"}{r.note ? ` · ${r.note}` : ""}</span>
          </div>
          <div class="acts">
            <button class="mini" title="Düzenle" onclick={() => editItem(r)}><Icon name="edit" size={16} /></button>
            <button class="mini" title="Sil" onclick={() => act(() => api.reminders.remove(r.id), "Hatırlatıcı silindi", "trash")}><Icon name="trash" size={16} /></button>
          </div>
        </div>
      {:else}
        <p class="empty">Yaklaşan hatırlatıcı yok. Soldan bir tane kur.</p>
      {/each}
    </section>

    {#if done.length}
      <section class="panel">
        <button class="toggle" onclick={() => (showDone = !showDone)}>
          <Icon name="check" />Tamamlananlar <span class="sub small">({done.length})</span>
        </button>
        {#if showDone}
          {#each done as r (r.id)}
            <div class="item muted">
              <div class="txt"><b>{r.title}</b><span class="sub small">{when(r.dueAt, now)}</span></div>
              <button class="mini" title="Sil" onclick={() => act(() => api.reminders.remove(r.id), "Silindi", "trash")}><Icon name="trash" size={16} /></button>
            </div>
          {/each}
        {/if}
      </section>
    {/if}
  </div>
</div>

<style>
  .rem { display: grid; grid-template-columns: minmax(320px, 420px) minmax(0, 1fr); gap: 16px; max-width: 1400px; margin: 0 auto; align-items: start; }
  .form, .panel { display: flex; flex-direction: column; gap: 12px; }
  .lists { display: flex; flex-direction: column; gap: 16px; min-width: 0; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field > span { font-size: 12px; font-weight: 600; letter-spacing: 0.05em; text-transform: uppercase; color: var(--dim); }
  input[type="text"], input[type="date"], input[type="time"], textarea, select {
    background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 8px 10px;
  }
  textarea { resize: vertical; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chips button { border: 1px solid var(--line-2); background: var(--coal-2); color: var(--muted); border-radius: 999px; padding: 5px 11px; font-size: 12.5px; }
  .chips button:hover { color: var(--gold-glow); border-color: var(--gold-dim); }
  .item { display: flex; align-items: center; gap: 14px; padding: 10px 12px; border-radius: 12px; background: rgba(255, 255, 255, 0.03); }
  .item.hot { border: 1px solid rgba(227, 194, 112, 0.45); background: rgba(227, 194, 112, 0.08); }
  .item.muted { opacity: 0.7; }
  .when { display: flex; flex-direction: column; min-width: 120px; }
  .when b { font-weight: 600; }
  .when span { font-size: 12px; color: var(--teal); }
  .txt { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .txt b { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .acts { display: flex; gap: 6px; align-items: center; }
  .sm { padding: 6px 11px; font-size: 13px; }
  .mini { border: 0; background: transparent; color: var(--dim); padding: 6px; border-radius: 6px; display: grid; place-items: center; }
  .mini:hover { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .hot { color: var(--gold-glow); }
  .empty { color: var(--dim); margin: 0; }
  .toggle { border: 0; background: none; padding: 0; display: flex; align-items: center; gap: 8px; font-family: var(--f-display); font-weight: 700; font-size: 15px; text-align: left; }
  .line { border-top: 1px solid var(--line); padding-top: 10px; }
  @media (max-width: 1000px) { .rem { grid-template-columns: minmax(0, 1fr); } }
</style>
