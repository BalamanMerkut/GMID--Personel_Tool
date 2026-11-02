<script lang="ts">
  // Topluluk turnuvası kartı (akışta ve Sosyal'in sağ sütununda): ad, oyun, kim katıldı; kayıt sürerken
  // Katıl / Ayrıl / Başlat; başlayınca eleme ağacı (maçın oyuncuları ya da açan kişi kazananı seçer), bitince şampiyon.
  import Icon from "../../Icon.svelte";
  import { notify } from "../../toast.svelte";
  import { macSonucu, profilGoster, sosyal, turnuvaBaslat, turnuvaKatil, turnuvaSil, type Kisi, type TurnuvaMac } from "../../social.svelte";
  import Avatar from "./Avatar.svelte";

  let {
    id,
    yedek,
    kucuk = false,
  }: {
    id: number;
    /** Turnuva silinmişse kartta yazanlar. */
    yedek?: { ad?: string; oyun?: string; boyut?: number; durum?: string; sampiyon?: Kisi | null };
    kucuk?: boolean;
  } = $props();

  const t = $derived(sosyal.turnuvalar[id] ?? null);
  const benId = $derived(sosyal.ben?.id ?? "");
  const katildim = $derived(!!t?.oyuncular.some((k) => k.id === benId));
  const yetkili = $derived(!!t && (t.olusturan.id === benId || sosyal.yonetici));
  const durum = $derived(t?.durum ?? yedek?.durum ?? "kayit");
  const sampiyon = $derived(t?.sampiyon ?? yedek?.sampiyon ?? null);
  let bekliyor = $state(false);
  let silOnay = $state(false);

  /** Turlar: sütun sütun. */
  const turlar = $derived.by(() => {
    const l: TurnuvaMac[][] = [];
    for (const m of t?.maclar ?? []) (l[m.tur] ??= []).push(m);
    return l;
  });
  function turAdi(i: number, n: number): string {
    const kalan = n - i;
    if (kalan === 1) return "Final";
    if (kalan === 2) return "Yarı final";
    if (kalan === 3) return "Çeyrek final";
    return `${i + 1}. tur`;
  }

  const hata = (e: unknown) => notify(String(e instanceof Error ? e.message : e), "info");
  async function yap(f: () => Promise<void>, mesaj?: string) {
    if (bekliyor) return;
    bekliyor = true;
    try {
      await f();
      if (mesaj) notify(mesaj, "star");
    } catch (e) {
      hata(e);
    } finally {
      bekliyor = false;
    }
  }

  /** Bu maçın kazananını seçebilir miyim? */
  const secebilir = (m: TurnuvaMac) =>
    t?.durum === "suruyor" && !!m.a && !!m.b && (yetkili || (!m.kazanan && (m.a.id === benId || m.b.id === benId)));

  function kazandi(m: TurnuvaMac, k: Kisi | null) {
    if (!k || !secebilir(m) || m.kazanan === k.id) return;
    void yap(() => macSonucu(id, m.tur, m.sira, k.id), `${k.ad} bir üst tura çıktı`);
  }

  function sil() {
    if (!silOnay) {
      silOnay = true;
      setTimeout(() => (silOnay = false), 4000);
      return;
    }
    void yap(() => turnuvaSil(id), "Turnuva silindi");
  }
</script>

<div class="turnuva" class:kucuk class:bitti={durum === "bitti"}>
  <div class="bas">
    <span class="kupa"><Icon name="trophy" size={kucuk ? 18 : 22} /></span>
    <span class="baslik">
      <b>{t?.ad ?? yedek?.ad ?? "Turnuva"}</b>
      <span class="sub small">{t?.oyun ?? yedek?.oyun ?? ""} · {t ? `${t.oyuncular.length}/${t.boyut}` : (yedek?.boyut ?? "")} kişi</span>
    </span>
    <span class="durum {durum}">{durum === "kayit" ? "Kayıt açık" : durum === "suruyor" ? "Sürüyor" : "Bitti"}</span>
  </div>

  {#if sampiyon}
    <button class="sampiyon" onclick={() => profilGoster(sampiyon.id)}>
      <Avatar kisi={sampiyon} boy={kucuk ? 26 : 34} nokta={false} />
      <span><span class="sub small">Şampiyon</span><b>{sampiyon.ad}</b></span>
      <Icon name="trophy" size={18} />
    </button>
  {/if}

  {#if t}
    {#if t.durum === "kayit"}
      <div class="oyuncular">
        {#each t.oyuncular.slice(0, kucuk ? 6 : 16) as k (k.id)}
          <button class="yuz" title={k.ad} onclick={() => profilGoster(k.id)}><Avatar kisi={k} boy={kucuk ? 24 : 30} nokta={false} /></button>
        {/each}
        {#if t.oyuncular.length < t.boyut}<span class="sub small bos-yer">{t.boyut - t.oyuncular.length} yer boş</span>{/if}
      </div>
      <div class="dugmeler">
        {#if katildim}
          {#if t.olusturan.id !== benId}<button class="btn ghost sm" disabled={bekliyor} onclick={() => yap(() => turnuvaKatil(id, false))}>Ayrıl</button>{/if}
        {:else}
          <button class="btn sm" disabled={bekliyor || t.oyuncular.length >= t.boyut} onclick={() => yap(() => turnuvaKatil(id, true), "Turnuvaya katıldın")}>
            <Icon name="plus" size={14} />{t.oyuncular.length >= t.boyut ? "Dolu" : "Katıl"}
          </button>
        {/if}
        {#if yetkili && !kucuk}
          <button class="btn sm" disabled={bekliyor || t.oyuncular.length < 2} title={t.oyuncular.length < 2 ? "En az 2 kişi gerek" : "Kayıtları kapat, eşleşmeleri kur"} onclick={() => yap(() => turnuvaBaslat(id), "Turnuva başladı")}>
            <Icon name="play" size={14} />Başlat
          </button>
        {/if}
        {#if yetkili && !kucuk}<button class="sil" class:onay={silOnay} onclick={sil}>{silOnay ? "Silinsin mi?" : "Sil"}</button>{/if}
      </div>
    {:else if !kucuk}
      <div class="agac">
        {#each turlar as tur, i (i)}
          <div class="sutun">
            <span class="tur-adi">{turAdi(i, turlar.length)}</span>
            <div class="maclar">
              {#each tur as m (m.sira)}
                <div class="mac" class:secilir={secebilir(m)}>
                  {#each [m.a, m.b] as k, j (j)}
                    <button
                      class="oyuncu"
                      class:kazanan={!!k && m.kazanan === k.id}
                      class:kaybeden={!!k && !!m.kazanan && m.kazanan !== k.id}
                      disabled={!k || !secebilir(m)}
                      title={k && secebilir(m) ? `${k.ad} kazandı` : ""}
                      onclick={() => kazandi(m, k)}
                    >
                      {#if k}<Avatar kisi={k} boy={20} nokta={false} /><span>{k.ad}</span>{:else}<span class="sub">{i === 0 ? "bay" : "—"}</span>{/if}
                      {#if k && m.kazanan === k.id}<Icon name="check" size={13} />{/if}
                    </button>
                  {/each}
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
      {#if t.durum === "suruyor"}<span class="sub small">Maç bitince kazananın adına tıkla (maçın oyuncuları ya da turnuvayı açan).</span>{/if}
      {#if yetkili}<div class="dugmeler"><button class="sil" class:onay={silOnay} onclick={sil}>{silOnay ? "Silinsin mi?" : "Turnuvayı sil"}</button></div>{/if}
    {/if}
  {:else if !yedek}
    <span class="sub small">Turnuva bulunamadı.</span>
  {/if}
</div>

<style>
  .turnuva { display: flex; flex-direction: column; gap: 10px; padding: 12px; border-radius: 16px; border: 1px solid rgba(255, 211, 110, 0.35); background: linear-gradient(135deg, rgba(255, 211, 110, 0.1), rgba(255, 255, 255, 0.02) 60%); }
  .turnuva.kucuk { padding: 10px; gap: 8px; border-radius: 12px; }
  .bas { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .kupa { flex: none; width: 40px; height: 40px; border-radius: 12px; display: grid; place-items: center; color: #ffd36e; background: rgba(0, 0, 0, 0.3); border: 1px solid rgba(255, 211, 110, 0.3); }
  .kucuk .kupa { width: 32px; height: 32px; border-radius: 10px; }
  .baslik { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .baslik b { font-size: 15px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .durum { flex: none; font-size: 11px; font-weight: 800; padding: 3px 9px; border-radius: 999px; }
  .durum.kayit { color: var(--good); background: rgba(92, 192, 138, 0.12); }
  .durum.suruyor { color: #ffd36e; background: rgba(255, 211, 110, 0.14); }
  .durum.bitti { color: var(--dim); background: rgba(255, 255, 255, 0.06); }
  .sampiyon { display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 12px; border: 1px solid rgba(255, 211, 110, 0.45); background: rgba(255, 211, 110, 0.12); color: #ffd36e; cursor: pointer; font: inherit; text-align: left; }
  .sampiyon > span { flex: 1; display: flex; flex-direction: column; }
  .sampiyon b { color: var(--text); font-size: 15px; }
  .oyuncular { display: flex; flex-wrap: wrap; align-items: center; gap: 4px; }
  .yuz { border: 0; background: none; padding: 0; border-radius: 50%; cursor: pointer; }
  .bos-yer { margin-left: 6px; }
  .dugmeler { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .sm { padding: 6px 12px; font-size: 13px; display: inline-flex; align-items: center; gap: 5px; }
  .sil { margin-left: auto; border: 0; background: none; color: var(--dim); font: inherit; font-size: 12.5px; padding: 5px 9px; border-radius: 8px; cursor: pointer; }
  .sil:hover { color: var(--text); }
  .sil.onay { background: rgba(229, 72, 77, 0.18); color: #ff8a8d; }
  .agac { display: flex; gap: 14px; overflow-x: auto; padding-bottom: 4px; }
  .sutun { flex: none; width: 160px; display: flex; flex-direction: column; gap: 6px; }
  .tur-adi { font-size: 11px; font-weight: 800; letter-spacing: 0.08em; text-transform: uppercase; color: var(--gold-dim); }
  .maclar { flex: 1; display: flex; flex-direction: column; justify-content: space-around; gap: 8px; }
  .mac { display: flex; flex-direction: column; border-radius: 10px; border: 1px solid var(--line-2); background: rgba(0, 0, 0, 0.25); overflow: hidden; }
  .mac.secilir { border-color: var(--gold-dim); }
  .oyuncu { display: flex; align-items: center; gap: 6px; padding: 6px 8px; border: 0; background: none; color: var(--text); font: inherit; font-size: 12.5px; text-align: left; min-width: 0; }
  .oyuncu + .oyuncu { border-top: 1px solid var(--line); }
  .oyuncu span { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .oyuncu:not(:disabled) { cursor: pointer; }
  .oyuncu:not(:disabled):hover { background: rgba(227, 194, 112, 0.1); }
  .oyuncu.kazanan { color: #ffd36e; font-weight: 800; background: rgba(255, 211, 110, 0.08); }
  .oyuncu.kaybeden { opacity: 0.45; }
</style>
