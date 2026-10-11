<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";
  import type { HotkeyStatus } from "../types";

  let list: HotkeyStatus[] = $state([]);
  /** Şu an yeni tuş beklenen kısayol. */
  let capturing: string | null = $state(null);

  onMount(async () => {
    list = await api.hotkeys.list();
  });

  const MODS = new Set(["Control", "Alt", "Shift", "Meta", "AltGraph", "OS"]);

  /** Klavye olayını "Ctrl+Alt+A" biçimine çevirir. */
  function toAccel(e: KeyboardEvent): string | null {
    if (MODS.has(e.key)) return null;
    let key = e.code;
    if (key.startsWith("Key")) key = key.slice(3);
    else if (key.startsWith("Digit")) key = key.slice(5);
    const isF = /^F\d{1,2}$/.test(key);
    const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
    // Tek başına harf ya da rakam yazarken sürekli tetiklenir; en az bir değiştirici tuş iste (F tuşları hariç).
    if (!mods.length && !isF) return null;
    return [...mods, key].join("+");
  }

  function onKey(e: KeyboardEvent) {
    if (!capturing) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      capturing = null;
      return;
    }
    const accel = toAccel(e);
    if (accel) void save(capturing, accel);
  }

  async function save(id: string, accel: string | null) {
    try {
      list = await api.hotkeys.set(id, accel);
      const h = list.find((x) => x.id === id);
      if (h?.error) notify(h.error, "info");
      else notify(accel === "" ? "Kısayol kapatıldı" : "Kısayol kaydedildi", "keyboard");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      capturing = null;
    }
  }

  /** Gruplar listedeki sırayla. */
  const groups = $derived.by(() => {
    const out: [string, HotkeyStatus[]][] = [];
    for (const h of list) {
      const g = out.find(([name]) => name === h.group);
      if (g) g[1].push(h);
      else out.push([h.group, [h]]);
    }
    return out;
  });
  const changed = $derived(list.filter((h) => h.accel !== h.defaultAccel));

  async function resetAll() {
    for (const h of changed) {
      try {
        list = await api.hotkeys.set(h.id, null);
      } catch (e) {
        notify(String(e), "info");
      }
    }
    notify("Bütün kısayollar varsayılana döndü", "keyboard");
  }

  const keys = (accel: string) => accel.split("+").map((k) => k.replace("PageUp", "PgUp").replace("PageDown", "PgDn"));
</script>

<svelte:window onkeydown={onKey} />

{#if changed.length}
  <div class="reset">
    <span class="sub small">{changed.length} kısayol değiştirildi</span>
    <button class="link" onclick={resetAll}><Icon name="restore" size={14} />Hepsini varsayılana döndür</button>
  </div>
{/if}

<table class="keys">
  {#each groups as [group, items] (group)}
  <tbody>
    <tr class="ghead"><th colspan="3">{group}</th></tr>
    {#each items as h (h.id)}
      <tr>
        <td>
          <div class="label">
            <span>{h.label}</span>
            {#if h.error}<span class="err">{h.error}</span>{/if}
          </div>
        </td>
        <td class="combo">
          {#if capturing === h.id}
            <span class="wait">Tuşlara bas… <kbd>Esc</kbd> vazgeç</span>
          {:else if h.accel}
            {#each keys(h.accel) as k (k)}<kbd>{k}</kbd>{/each}
          {:else}
            <span class="sub small">Kapalı</span>
          {/if}
        </td>
        <td class="acts">
          <button class="mini" title="Değiştir" onclick={() => (capturing = h.id)}><Icon name="edit" size={15} /></button>
          {#if h.accel !== h.defaultAccel}
            <button class="mini" title="Varsayılana döndür ({h.defaultAccel})" onclick={() => save(h.id, null)}><Icon name="restore" size={15} /></button>
          {/if}
          {#if h.accel}
            <button class="mini" title="Kapat" onclick={() => save(h.id, "")}><Icon name="x" size={15} /></button>
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
  {/each}
</table>

<style>
  .keys { width: 100%; border-collapse: collapse; }
  .ghead th { text-align: left; padding: 16px 4px 6px; font-family: var(--f-display); font-size: 11px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; color: var(--gold-dim); border-bottom: 1px solid var(--line); }
  tbody:first-child .ghead th { padding-top: 4px; }
  .reset { display: flex; justify-content: space-between; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 10px; background: rgba(227, 194, 112, 0.06); border: 1px solid rgba(227, 194, 112, 0.25); }
  .link { border: 0; background: none; padding: 0; color: var(--teal); font-weight: 600; display: inline-flex; gap: 5px; align-items: center; }
  .link:hover { color: var(--neon); }
  td { padding: 9px 4px; border-bottom: 1px solid var(--line); vertical-align: middle; }
  .label { display: flex; flex-direction: column; gap: 2px; }
  .err { font-size: 12px; color: var(--bad); }
  .combo { white-space: nowrap; text-align: right; }
  .combo kbd + kbd { margin-left: 4px; }
  .wait { color: var(--neon); font-size: 12.5px; }
  .acts { width: 1%; white-space: nowrap; text-align: right; }
  kbd {
    font-family: var(--f-body); font-size: 12px; font-weight: 600; padding: 2px 7px; border-radius: 6px;
    background: var(--coal); border: 1px solid var(--line-2); border-bottom-width: 2px;
  }
  .mini { border: 0; background: transparent; color: var(--dim); padding: 5px; border-radius: 6px; }
  .mini:hover { color: var(--text); background: rgba(255, 255, 255, 0.06); }
</style>
