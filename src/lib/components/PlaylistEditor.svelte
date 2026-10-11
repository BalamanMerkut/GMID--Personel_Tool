<script lang="ts">
  // Çalma listesi oluştur / düzenle: ad ver, şarkıları işaretle (seçim sırası liste sırası olur).
  import { untrack } from "svelte";
  import { api } from "../api";
  import { formatDuration } from "../downloads.svelte";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";
  import type { Download, UserPlaylist } from "../types";

  let {
    songs,
    initial = null,
    onclose,
    onsaved,
  }: { songs: Download[]; initial?: UserPlaylist | null; onclose: () => void; onsaved: (p: UserPlaylist) => void } =
    $props();

  // Açılıştaki değerler (pencere açıkken dışarıdaki liste değişse de düzenleme bozulmasın).
  const start = untrack(() => initial);
  /** Var olan bir liste mi (yoksa önceden seçili şarkıyla gelen yeni liste)? */
  const existing = !!start?.id;
  let name = $state(start?.name ?? "");
  let picked: string[] = $state(start ? [...start.items] : []);
  let search = $state("");
  let saving = $state(false);
  let nameEl: HTMLInputElement | undefined = $state();

  $effect(() => {
    nameEl?.focus();
  });

  const shown = $derived.by(() => {
    const q = search.trim().toLocaleLowerCase("tr-TR");
    return songs.filter((d) => !q || `${d.title} ${d.uploader}`.toLocaleLowerCase("tr-TR").includes(q));
  });
  const total = $derived(picked.reduce((s, id) => s + (songs.find((d) => d.id === id)?.duration ?? 0), 0));

  function toggle(id: string) {
    picked = picked.includes(id) ? picked.filter((x) => x !== id) : [...picked, id];
  }

  async function save() {
    if (!name.trim()) {
      nameEl?.focus();
      return notify("Listeye bir ad ver", "info");
    }
    saving = true;
    try {
      const p = await api.videos.savePlaylist(existing ? start!.id : null, name.trim(), picked);
      notify(existing ? "Liste güncellendi" : `"${p.name}" oluşturuldu`, "check");
      onsaved(p);
    } catch (e) {
      notify(String(e), "info");
    } finally {
      saving = false;
    }
  }

  const thumb = (d: Download) =>
    d.thumbnail ? (/^(https?:|data:)/.test(d.thumbnail) ? d.thumbnail : api.fileSrc(d.thumbnail)) : "";
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="back" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="dlg panel" role="dialog" aria-label="Çalma listesi">
    <div class="head">
      <h3 class="h2"><Icon name="list" />{existing ? "Listeyi düzenle" : "Yeni çalma listesi"}</h3>
      <button class="x" title="Kapat (Esc)" onclick={onclose}><Icon name="x" size={18} /></button>
    </div>
    <input
      class="name"
      bind:this={nameEl}
      bind:value={name}
      maxlength="80"
      placeholder="Liste adı — ör. Yol şarkıları"
      onkeydown={(e) => e.key === "Enter" && void save()}
    />
    <div class="bar">
      <input class="search" type="search" placeholder="Şarkı ara…" bind:value={search} />
      <span class="sub small tnum">{picked.length} şarkı{total ? ` · ${formatDuration(total)}` : ""}</span>
    </div>
    <div class="songs">
      {#each shown as d (d.id)}
        {@const n = picked.indexOf(d.id)}
        <button class="song" class:on={n >= 0} onclick={() => toggle(d.id)}>
          <span class="box">{#if n >= 0}<b class="tnum">{n + 1}</b>{/if}</span>
          <span class="th">{#if thumb(d)}<img src={thumb(d)} alt="" loading="lazy" />{/if}</span>
          <span class="tx"><b>{d.title}</b><span class="sub small">{d.uploader}</span></span>
          <span class="sub small tnum">{formatDuration(d.duration ?? 0)}</span>
        </button>
      {:else}
        <p class="sub small">{songs.length ? "Aramaya uyan şarkı yok." : "Önce birkaç şarkı indir (MP3)."}</p>
      {/each}
    </div>
    <div class="foot">
      <span class="sub small">İşaretleme sırası listedeki sıra olur.</span>
      <div class="row">
        <button class="btn ghost" onclick={onclose}>Vazgeç</button>
        <button class="btn" disabled={saving} onclick={save}><Icon name="check" size={17} />{existing ? "Kaydet" : "Oluştur"}</button>
      </div>
    </div>
  </div>
</div>

<style>
  .back { position: fixed; inset: 0; z-index: 60; background: rgba(0, 0, 0, 0.55); display: grid; place-items: center; padding: 24px; }
  .dlg { width: min(620px, 100%); max-height: min(720px, 100%); display: flex; flex-direction: column; gap: 12px; background: var(--night-2); box-shadow: 0 30px 80px rgba(0, 0, 0, 0.6); }
  .head { display: flex; justify-content: space-between; align-items: center; }
  .x { border: 0; background: none; color: var(--muted); padding: 6px; border-radius: 8px; display: grid; }
  .x:hover { color: var(--text); background: rgba(255, 255, 255, 0.07); }
  .name { font-size: 16px; padding: 11px 14px; border-radius: 12px; background: var(--coal-2); border: 1px solid var(--line-2); color: var(--text); }
  .name:focus, .search:focus { outline: none; border-color: var(--gold-dim); }
  .bar { display: flex; justify-content: space-between; align-items: center; gap: 10px; }
  .search { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; width: 240px; color: var(--text); }
  .songs { overflow: auto; min-height: 120px; display: flex; flex-direction: column; gap: 2px; border: 1px solid var(--line); border-radius: 12px; padding: 6px; }
  .song { display: grid; grid-template-columns: 26px 40px minmax(0, 1fr) auto; gap: 10px; align-items: center; text-align: left; border: 0; background: transparent; padding: 6px 8px; border-radius: 9px; color: var(--text); }
  .song:hover { background: rgba(255, 255, 255, 0.05); }
  .song.on { background: rgba(227, 194, 112, 0.1); }
  .box { width: 22px; height: 22px; border-radius: 7px; border: 1.5px solid var(--line-2); display: grid; place-items: center; font-size: 11px; }
  .on .box { background: var(--gold); border-color: var(--gold); color: #1b1608; }
  .th { width: 40px; height: 40px; border-radius: 7px; overflow: hidden; background: var(--coal-2); }
  .th img { width: 100%; height: 100%; object-fit: cover; }
  .tx { display: flex; flex-direction: column; min-width: 0; }
  .tx b, .tx .sub { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .tx b { font-weight: 600; font-size: 13.5px; }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; }
</style>
