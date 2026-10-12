<script lang="ts">
  // Sorun bildir: kısa açıklama + günlük dosyaları tek zip; özet panoya, GMID Discord'u açılır.
  import { api } from "../api";
  import { community, joinDiscord } from "../community.svelte";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";

  let text = $state("");
  let logs = $state(true);
  let busy = $state(false);
  let done = $state<{ path: string; summary: string } | null>(null);

  async function create() {
    busy = true;
    try {
      done = await api.report.create(text, logs);
      try {
        await navigator.clipboard.writeText(done.summary);
        notify("Rapor hazır · özet panoya kopyalandı", "check");
      } catch {
        notify("Rapor hazır", "check");
      }
    } catch (e) {
      notify(String(e), "info");
    } finally {
      busy = false;
    }
  }
</script>

<section class="panel rep">
  <h3 class="h2"><Icon name="flag" />Sorun bildir</h3>
  {#if done}
    <div class="ok">
      <Icon name="check" size={18} />
      <div>
        <b>Rapor hazır.</b>
        <span class="sub small">
          Klasörde açıldı. Özet panoya kopyalandı: {community.discord ? "GMID Discord'unda yapıştır (Ctrl+V) ve zip dosyasını sürükleyip bırak." : "Bana gönderirken yapıştır ve zip dosyasını ekle."}
        </span>
      </div>
    </div>
    <div class="row">
      {#if community.discord}<button class="btn" onclick={joinDiscord}><Icon name="discord" size={15} />GMID Discord'u aç</button>{/if}
      <button class="btn ghost" onclick={() => (done = null, text = "")}>Yeni rapor</button>
    </div>
  {:else}
    <p class="sub small">
      Bir şey bozulduysa ne olduğunu kısaca yaz. Sürüm, Windows bilgisi ve günlük dosyası tek bir zip'e konur; kullanıcı adın
      gibi kişisel klasör yolları gizlenir.
    </p>
    <textarea bind:value={text} rows="3" placeholder="Ör. Oyun paneli açılınca oyun küçülüyor (VALORANT, kenarlıksız)…"></textarea>
    <div class="spread">
      <label class="chk"><input type="checkbox" bind:checked={logs} />Günlük dosyasını ekle (önerilir)</label>
      <button class="btn" disabled={busy} onclick={create}><Icon name="flag" size={15} />{busy ? "Hazırlanıyor…" : "Raporu hazırla"}</button>
    </div>
  {/if}
</section>

<style>
  .rep { display: flex; flex-direction: column; gap: 10px; }
  .h2 { margin: 0; }
  p { margin: 0; }
  textarea { width: 100%; resize: vertical; background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 10px; color: var(--text); font: inherit; }
  .spread { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
  .chk { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--muted); }
  .chk input { accent-color: var(--gold); }
  .ok { display: flex; gap: 10px; color: var(--good); }
  .ok div { display: flex; flex-direction: column; gap: 2px; color: var(--text); }
  .row { display: flex; gap: 8px; flex-wrap: wrap; }
</style>
