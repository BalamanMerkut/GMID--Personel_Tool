<script lang="ts">
  // Oyun gecesi planla: oyun (kütüphanenden öneri), gün, saat, kişi sınırı, not. Planlanınca akışa kart düşer,
  // arkadaşlarına davet bildirimi gider.
  import { onMount } from "svelte";
  import { api, inTauri } from "../../api";
  import Icon from "../../Icon.svelte";
  import { notify } from "../../toast.svelte";
  import { geceAc, sosyal } from "../../social.svelte";

  let { onkapat }: { onkapat: () => void } = $props();

  const iki = (n: number) => String(n).padStart(2, "0");
  const gunYaz = (d: Date) => `${d.getFullYear()}-${iki(d.getMonth() + 1)}-${iki(d.getDate())}`;
  // Varsayılan: bugün 21:00 (geçtiyse yarın).
  const ilk = new Date();
  if (ilk.getHours() >= 20) ilk.setDate(ilk.getDate() + 1);

  let oyun = $state(sosyal.ben ? (sosyal.cevrimici[sosyal.ben.id]?.oyun ?? "") : "");
  let gun = $state(gunYaz(ilk));
  let saat = $state("21:00");
  let kontenjan = $state<string>("5");
  let not = $state("");
  let oyunlar = $state<string[]>([]);
  let gonderiliyor = $state(false);

  const zaman = $derived(new Date(`${gun}T${saat || "21:00"}`).getTime());
  const gecerli = $derived(oyun.trim().length > 0 && Number.isFinite(zaman) && zaman > Date.now() - 5 * 60_000);

  onMount(() => {
    if (inTauri)
      void api.games
        .list()
        .then((l) => (oyunlar = l.filter((g) => g.inLibrary).map((g) => g.name).sort((a, b) => a.localeCompare(b, "tr"))))
        .catch(() => {});
  });

  async function planla(e: SubmitEvent) {
    e.preventDefault();
    if (!gecerli || gonderiliyor) return;
    gonderiliyor = true;
    try {
      await geceAc({ oyun: oyun.trim(), zaman, kontenjan: kontenjan ? Number(kontenjan) : null, not: not.trim() || null });
      notify("Oyun gecesi planlandı · arkadaşlarına haber gitti", "bell");
      onkapat();
    } catch (err) {
      notify(String(err instanceof Error ? err.message : err), "info");
    } finally {
      gonderiliyor = false;
    }
  }
</script>

<form class="plan" onsubmit={planla}>
  <div class="baslik">
    <span><Icon name="bell" size={16} /><b>Oyun gecesi planla</b></span>
    <button type="button" class="x" aria-label="Kapat" onclick={onkapat}><Icon name="x" size={15} /></button>
  </div>
  <div class="alanlar">
    <label class="genis">
      <span>Oyun</span>
      <input bind:value={oyun} list="gece-oyunlar" maxlength="80" placeholder="CS2, Valorant, LoL…" required />
      <datalist id="gece-oyunlar">{#each oyunlar as o (o)}<option value={o}></option>{/each}</datalist>
    </label>
    <label>
      <span>Gün</span>
      <input type="date" bind:value={gun} min={gunYaz(new Date())} required />
    </label>
    <label>
      <span>Saat</span>
      <input type="time" bind:value={saat} required />
    </label>
    <label>
      <span>Kişi sınırı</span>
      <select bind:value={kontenjan}>
        <option value="">Sınırsız</option>
        {#each [2, 3, 4, 5, 6, 8, 10] as n (n)}<option value={String(n)}>{n} kişi</option>{/each}
      </select>
    </label>
    <label class="genis">
      <span>Not <em>(isteğe bağlı)</em></span>
      <input bind:value={not} maxlength="300" placeholder="Ranked, Discord'da Oyun Odası 1" />
    </label>
  </div>
  <div class="alt">
    <span class="sub small">Arkadaşlarına davet gider; geleceklere başlamadan 15 dakika önce hatırlatılır.</span>
    <button class="btn" disabled={!gecerli || gonderiliyor}><Icon name="check" size={15} />{gonderiliyor ? "Planlanıyor…" : "Planla"}</button>
  </div>
</form>

<style>
  .plan { display: flex; flex-direction: column; gap: 12px; padding: 12px; border-radius: 12px; border: 1px solid var(--line-2); background: rgba(0, 0, 0, 0.2); }
  .baslik { display: flex; align-items: center; justify-content: space-between; }
  .baslik span { display: inline-flex; align-items: center; gap: 8px; color: var(--gold-glow); }
  .x { border: 0; background: none; color: var(--dim); width: 28px; height: 28px; border-radius: 8px; display: grid; place-items: center; cursor: pointer; }
  .x:hover { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .alanlar { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; }
  label { display: flex; flex-direction: column; gap: 5px; min-width: 0; }
  label.genis { grid-column: 1 / -1; }
  label > span { font-size: 11.5px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--dim); }
  label em { font-style: normal; text-transform: none; letter-spacing: 0; font-weight: 400; }
  input, select { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 10px; padding: 8px 10px; color: var(--text); font: inherit; font-size: 14px; outline: none; color-scheme: dark; min-width: 0; }
  input:focus, select:focus { border-color: var(--gold-dim); }
  .alt { display: flex; align-items: center; justify-content: space-between; gap: 10px; flex-wrap: wrap; }
  .btn { display: inline-flex; align-items: center; gap: 6px; }
</style>
