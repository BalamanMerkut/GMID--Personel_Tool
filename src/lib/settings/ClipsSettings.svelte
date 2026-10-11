<script lang="ts">
  // Kontrol Odası → Oyunlar → Klipler: izlenen klasörler, yeni klip göstergesi, disk koruması.
  import { onMount } from "svelte";
  import { api } from "../api";
  import Switch from "../components/Switch.svelte";
  import Icon from "../Icon.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { notify } from "../toast.svelte";
  import type { ClipRoot } from "../types";

  let roots: ClipRoot[] = $state([]);
  let osd = $state(getJson("clips.osd", true));
  let cleanup = $state(getJson("clips.cleanupDays", 0));

  async function load() {
    roots = await api.clips.roots().catch(() => []);
  }
  onMount(() => void load());

  async function saveFolders(list: string[]) {
    await setJson("clips.folders", list);
    await load();
    await api.clips.rescan().catch(() => {});
  }

  async function addFolder() {
    const p = await api.clips.pickFolder();
    if (!p) return;
    const custom = roots.filter((r) => r.custom).map((r) => r.path);
    if (custom.includes(p)) return notify("Bu klasör zaten izleniyor", "info");
    await saveFolders([...custom, p]);
    notify("Klasör eklendi · klipler taranıyor", "check");
  }

  async function removeFolder(p: string) {
    await saveFolders(roots.filter((r) => r.custom && r.path !== p).map((r) => r.path));
  }
</script>

<section class="panel">
  <h3 class="h2"><Icon name="clip" />Klip klasörleri</h3>
  <p class="sub small">
    NVIDIA (Alt+F10), Xbox Game Bar, AMD, OBS, Steam ve Medal'ın varsayılan klasörleri kendiliğinden izlenir. Kayıtların
    başka bir yere gidiyorsa (ör. NVIDIA'da D:\Klipler seçtiysen) o klasörü ekle. Dosyalar taşınmaz, yerinde kalır.
  </p>
  <div class="roots">
    {#each roots as r (r.path)}
      <div class="root" class:missing={!r.exists}>
        <Icon name="folder" size={16} />
        <div class="rtx">
          <b>{r.label}</b>
          <span class="sub small path">{r.path}{r.exists ? "" : " · bulunamadı"}</span>
        </div>
        {#if r.custom}<button class="x" onclick={() => removeFolder(r.path)} title="İzlemeyi bırak"><Icon name="x" size={15} /></button>{/if}
      </div>
    {/each}
  </div>
  <div class="row">
    <button class="btn ghost" onclick={addFolder}><Icon name="plus" size={16} />Klasör ekle</button>
    <button class="btn ghost" onclick={() => api.clips.rescan().then(() => notify("Klasörler tarandı", "check"))}><Icon name="refresh" size={16} />Şimdi tara</button>
  </div>
</section>

<section class="panel">
  <h3 class="h2"><Icon name="bell" />Klipler</h3>
  <div class="line">
    <div>
      <b>Yeni klip göstergesi</b>
      <span class="sub small">Kayıt bitince köşede "Klip kaydedildi — Valorant (0:30)".</span>
    </div>
    <Switch label="Yeni klip göstergesi" checked={osd} onchange={(v) => ((osd = v), void setJson("clips.osd", v))} />
  </div>
  <div class="line">
    <div>
      <b>Eski klipleri temizle</b>
      <span class="sub small">Favori olmayan, seçilen süreden eski klipler Geri Dönüşüm Kutusu'na gider (geri alınabilir). Ekran görüntülerine dokunulmaz.</span>
    </div>
    <select bind:value={cleanup} onchange={() => void setJson("clips.cleanupDays", Number(cleanup))} aria-label="Eski klipleri temizle">
      <option value={0}>Kapalı</option>
      <option value={30}>30 günden eski</option>
      <option value={60}>60 günden eski</option>
      <option value={90}>90 günden eski</option>
    </select>
  </div>
</section>

<style>
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .panel p { margin: 0; }
  .roots { display: flex; flex-direction: column; gap: 4px; }
  .root { display: flex; align-items: center; gap: 10px; padding: 8px 10px; border-radius: 10px; background: rgba(255, 255, 255, 0.03); border: 1px solid var(--line); color: var(--gold); }
  .root.missing { opacity: 0.55; }
  .rtx { flex: 1; display: flex; flex-direction: column; min-width: 0; color: var(--text); }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: ui-monospace, Consolas, monospace; font-size: 12px; }
  .x { border: 0; background: none; color: var(--muted); padding: 6px; border-radius: 8px; display: grid; }
  .x:hover { color: var(--bad); background: rgba(255, 255, 255, 0.07); }
  .line { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  .line > div:first-child { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1 1 260px; }
  select { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; color: var(--text); }
</style>
