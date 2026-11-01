<script lang="ts">
  // Sosyal: topluluğun son olayları (puanladı, izledi, oynamaya başladı); beğeni ve yorum.
  import { onMount } from "svelte";
  import Icon from "../Icon.svelte";
  import FeedCard from "../components/social/FeedCard.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { istek, sosyal, type Olay } from "../social.svelte";
  import { nav } from "../player.svelte";
  import { notify } from "../toast.svelte";

  let olaylar = $state<Olay[]>([]);
  let yukleniyor = $state(false);
  let son = $state(false);
  let yalnizArkadas = $state(getJson<boolean>("social.feedFriends", false));

  async function yukle(devam = false) {
    if (!sosyal.anahtar) return;
    yukleniyor = true;
    try {
      const q = new URLSearchParams();
      if (devam && olaylar.length) q.set("once", String(olaylar[olaylar.length - 1].zaman));
      if (yalnizArkadas) q.set("arkadaslar", "1");
      const { olaylar: yeni } = await istek<{ olaylar: Olay[] }>("GET", `/api/akis?${q}`);
      olaylar = devam ? [...olaylar, ...yeni] : yeni;
      son = yeni.length < 30;
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
    } finally {
      yukleniyor = false;
    }
  }

  /** Kart sunucudan sildi; listeden de çıkar. */
  function sil(o: Olay) {
    olaylar = olaylar.filter((x) => x.id !== o.id);
  }

  function suzgec(v: boolean) {
    yalnizArkadas = v;
    void setJson("social.feedFriends", v);
    void yukle();
  }

  // Akış değişince (biri puanladı, beğendi, yorum yaptı) ilk sayfa sessizce yenilenir.
  let surum = sosyal.akisSurum;
  $effect(() => {
    if (sosyal.akisSurum !== surum) {
      surum = sosyal.akisSurum;
      void yukle();
    }
  });

  onMount(() => {
    sosyal.bildirim = 0;
    void yukle();
  });
</script>

<div class="akis">
  {#if !sosyal.anahtar}
    <section class="panel bos">
      <Icon name="sparkle" size={30} />
      <b>Arkadaşlarının son puanları ve oyunları burada</b>
      <span class="sub">Görmek için Topluluk'tan Discord ile giriş yap.</span>
      <button class="btn" onclick={() => nav.go("social")}><Icon name="users" size={16} />Topluluk'a git</button>
    </section>
  {:else}
    <div class="ust">
      <div class="seg" role="tablist">
        <button aria-pressed={!yalnizArkadas} onclick={() => suzgec(false)}>Herkes</button>
        <button aria-pressed={yalnizArkadas} onclick={() => suzgec(true)}>Arkadaşlarım</button>
      </div>
      <button class="btn ghost sm" disabled={yukleniyor} onclick={() => yukle()}><Icon name="refresh" size={15} />Yenile</button>
    </div>
    {#if yukleniyor && !olaylar.length}
      {#each Array(3) as _, i (i)}<div class="iskelet"><span class="sk a"></span><span class="sk s"></span><span class="sk k"></span></div>{/each}
    {:else}
      {#each olaylar as o (o.id)}
        <FeedCard {o} ondegisti={() => yukle()} onsil={sil} />
      {:else}
        <section class="panel bos">
          <Icon name="sparkle" size={28} />
          <b>Akış şimdilik sessiz</b>
          <span class="sub">{yalnizArkadas ? "Arkadaşların bir şey puanlayınca ya da bir oyuna girince burada görünür." : "Biri bir oyunu, filmi ya da diziyi puanlayınca ya da bir oyuna girince burada görünür."}</span>
        </section>
      {/each}
      {#if olaylar.length && !son}
        <button class="btn ghost devam" disabled={yukleniyor} onclick={() => yukle(true)}>{yukleniyor ? "Yükleniyor…" : "Daha eski"}</button>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .akis { display: flex; flex-direction: column; gap: 12px; max-width: 760px; margin: 0 auto; }
  .ust { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .bos { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; padding: 34px; color: var(--muted); }
  .devam { align-self: center; }
  .sm { padding: 6px 11px; font-size: 13px; }
  .iskelet { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; padding: 14px 16px; border-radius: 16px; border: 1px solid var(--line); }
  .sk { background: linear-gradient(90deg, rgba(255, 255, 255, 0.05) 25%, rgba(255, 255, 255, 0.1) 50%, rgba(255, 255, 255, 0.05) 75%); background-size: 200% 100%; animation: parilti 1.4s infinite linear; }
  .sk.a { width: 40px; height: 40px; border-radius: 50%; }
  .sk.s { width: 45%; height: 12px; border-radius: 6px; }
  .sk.k { width: 100%; height: 84px; border-radius: 12px; }
  @keyframes parilti { to { background-position: -200% 0; } }
</style>
