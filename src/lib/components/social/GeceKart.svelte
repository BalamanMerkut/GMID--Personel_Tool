<script lang="ts">
  // Oyun gecesi kartı (akışta ve Sosyal'in sağ sütununda): ne zaman, hangi oyun, kim geliyor; Geliyorum / Belki /
  // Gelemem. Gece sunucudan canlı gelir (sosyal.geceler); iptal edilmiş ya da eskiyse kartın kendi bilgisi gösterilir.
  import Icon from "../../Icon.svelte";
  import { notify } from "../../toast.svelte";
  import { geceIptal, geceKatil, profilGoster, sosyal, type GeceDurum } from "../../social.svelte";
  import Avatar from "./Avatar.svelte";
  import { geceKalan, geceZamani } from "./metin";

  let {
    id,
    yedek,
    kucuk = false,
  }: {
    id: number;
    /** Gece yüklenemezse (bitti / iptal) kartta yazanlar. */
    yedek?: { oyun?: string; zaman?: number; kontenjan?: number | null; not?: string | null; hemen?: boolean };
    /** Sağ sütundaki sıkışık görünüm. */
    kucuk?: boolean;
  } = $props();

  const g = $derived(sosyal.geceler[id] ?? null);
  const oyun = $derived(g?.oyun ?? yedek?.oyun ?? "Oyun gecesi");
  const zaman = $derived(g?.zaman ?? yedek?.zaman ?? 0);
  const kontenjan = $derived(g?.kontenjan ?? yedek?.kontenjan ?? null);
  const not = $derived(g?.not ?? yedek?.not ?? null);
  /** "Oyuncu arıyorum": şimdi oynamak için; 2 saat açık kalır, "Varım" / "Vazgeç". */
  const hemen = $derived(g?.hemen ?? yedek?.hemen ?? false);
  const HEMEN_MS = 2 * 3600 * 1000;
  function hemenKalan(ms: number): string {
    if (ms <= 0) return "bitti";
    const dk = Math.max(1, Math.round(ms / 60000));
    return dk > 90 ? "2 saat açık" : dk > 60 ? `1 sa ${dk - 60} dk daha açık` : `${dk} dk daha açık`;
  }
  const z = $derived(geceZamani(zaman));
  const kalan = $derived(
    hemen ? hemenKalan(zaman + HEMEN_MS - Date.now()) : geceKalan(zaman),
  );
  const bitti = $derived(hemen ? zaman + HEMEN_MS < Date.now() : !g && zaman < Date.now());
  const gelen = $derived(g?.katilanlar.filter((k) => k.durum === "geliyor") ?? []);
  const belki = $derived(g?.katilanlar.filter((k) => k.durum === "belki") ?? []);
  const benim = $derived(g?.katilanlar.find((k) => k.kisi.id === sosyal.ben?.id)?.durum ?? null);
  const dolu = $derived(kontenjan != null && gelen.length >= kontenjan && benim !== "geliyor");
  const iptalEdebilir = $derived(!!g && (g.olusturan.id === sosyal.ben?.id || sosyal.yonetici));
  let bekliyor = $state(false);
  let iptalOnay = $state(false);

  async function sec(d: GeceDurum) {
    if (bekliyor || benim === d) return;
    bekliyor = true;
    try {
      await geceKatil(id, d);
      if (d === "geliyor")
        notify(hemen ? `${oyun}: "Varım" dedin · ${g?.olusturan.ad ?? "arayan"} kişiye haber gitti` : `${oyun}: geliyorsun, 15 dakika önce hatırlatırım`, "bell");
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
    } finally {
      bekliyor = false;
    }
  }

  async function iptal() {
    if (!iptalOnay) {
      iptalOnay = true;
      setTimeout(() => (iptalOnay = false), 4000);
      return;
    }
    try {
      await geceIptal(id);
      notify(hemen ? "Arama kapatıldı" : "Oyun gecesi iptal edildi", "check");
    } catch (e) {
      notify(String(e instanceof Error ? e.message : e), "info");
    }
  }
</script>

<div class="gece" class:kucuk class:bitti class:hemen>
  {#if hemen}
    <div class="takvim simdi" aria-hidden="true">
      <span class="gun">Şimdi</span>
      <Icon name="users" size={kucuk ? 20 : 24} />
      {#if !kucuk}<span class="tarih">oyuncu aranıyor</span>{/if}
    </div>
  {:else}
    <div class="takvim" aria-hidden="true">
      <span class="gun">{z.gun}</span>
      <b class="saat tnum">{z.saat}</b>
      {#if !kucuk}<span class="tarih">{z.tarih}</span>{/if}
    </div>
  {/if}
  <div class="bilgi">
    <div class="ust">
      <b class="oyun">{oyun}</b>
      <span class="kalan" class:canli={kalan === "şimdi oynanıyor"}>{bitti ? "bitti" : kalan}</span>
    </div>
    {#if not && !kucuk}<p class="not">{not}</p>{/if}
    {#if g}
      <div class="kimler">
        <span class="yuzler">
          {#each gelen.slice(0, kucuk ? 4 : 8) as k (k.kisi.id)}
            <button class="yuz" title={k.kisi.ad} onclick={() => profilGoster(k.kisi.id)}><Avatar kisi={k.kisi} boy={kucuk ? 22 : 26} nokta={false} /></button>
          {/each}
        </span>
        <span class="sayi tnum">
          {#if hemen}
            <b>{gelen.length}{kontenjan ? `/${kontenjan}` : ""}</b> kişi{kontenjan && gelen.length < kontenjan ? ` · ${kontenjan - gelen.length} kişi daha lazım` : kontenjan ? " · tamam!" : ""}
          {:else}
            <b>{gelen.length}{kontenjan ? `/${kontenjan}` : ""}</b> geliyor{belki.length ? ` · ${belki.length} belki` : ""}
          {/if}
        </span>
      </div>
      {#if !bitti && kalan !== "bitti"}
        <div class="secim" role="group" aria-label="Katılım">
          <button class:secili={benim === "geliyor"} class="evet" disabled={bekliyor || dolu} title={dolu ? "Kişi sınırı doldu" : ""} onclick={() => sec("geliyor")}>
            <Icon name="check" size={14} />{dolu ? "Dolu" : hemen ? "Varım" : "Geliyorum"}
          </button>
          {#if !hemen}<button class:secili={benim === "belki"} disabled={bekliyor} onclick={() => sec("belki")}>Belki</button>{/if}
          {#if !hemen || benim === "geliyor"}
            <button class:secili={benim === "gelemem"} class="hayir" disabled={bekliyor} onclick={() => sec("gelemem")}>{hemen ? "Vazgeç" : "Gelemem"}</button>
          {/if}
          {#if iptalEdebilir && !kucuk}
            <button class="iptal" class:onay={iptalOnay} onclick={iptal}>{iptalOnay ? (hemen ? "Kapatılsın mı?" : "İptal edilsin mi?") : hemen ? "Aramayı kapat" : "İptal et"}</button>
          {/if}
        </div>
      {/if}
    {:else if !bitti}
      <span class="sub small">İptal edildi</span>
    {/if}
  </div>
</div>

<style>
  .gece { display: flex; gap: 14px; padding: 12px; border-radius: 16px; border: 1px solid rgba(227, 194, 112, 0.3); background: linear-gradient(135deg, rgba(227, 194, 112, 0.09), rgba(255, 255, 255, 0.02) 60%); }
  .gece.bitti { opacity: 0.6; }
  .gece.hemen { border-color: rgba(92, 192, 138, 0.4); background: linear-gradient(135deg, rgba(92, 192, 138, 0.1), rgba(255, 255, 255, 0.02) 60%); }
  .takvim.simdi { color: var(--good); border-color: rgba(92, 192, 138, 0.35); gap: 4px; }
  .takvim.simdi .gun { color: var(--good); }
  .gece.kucuk { padding: 10px; gap: 10px; border-radius: 12px; }
  .takvim { flex: none; width: 76px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1px; padding: 8px 4px; border-radius: 12px; background: rgba(0, 0, 0, 0.3); border: 1px solid rgba(227, 194, 112, 0.25); }
  .kucuk .takvim { width: 60px; padding: 6px 2px; align-self: flex-start; }
  .gun { font-size: 11px; font-weight: 800; letter-spacing: 0.06em; text-transform: uppercase; color: var(--gold); }
  .saat { font-family: var(--f-display); font-size: 22px; line-height: 1.1; }
  .kucuk .saat { font-size: 17px; }
  .tarih { font-size: 11px; color: var(--dim); }
  .bilgi { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 8px; }
  .kucuk .bilgi { gap: 5px; }
  .ust { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; }
  .oyun { font-size: 16px; }
  .kucuk .oyun { font-size: 14px; }
  .kalan { font-size: 12.5px; color: var(--dim); }
  .kalan.canli { color: var(--good); font-weight: 700; }
  .not { margin: 0; color: var(--muted); font-size: 14px; line-height: 1.45; overflow-wrap: anywhere; }
  .kimler { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .yuzler { display: inline-flex; }
  .yuz { border: 0; background: none; padding: 0; margin-right: -6px; border-radius: 50%; box-shadow: 0 0 0 2px var(--night-2, #16181c); cursor: pointer; }
  .sayi { font-size: 13px; color: var(--muted); margin-left: 8px; }
  .sayi b { color: var(--text); }
  .secim { display: flex; gap: 6px; flex-wrap: wrap; }
  .secim button { display: inline-flex; align-items: center; gap: 5px; padding: 6px 12px; border-radius: 9px; border: 1px solid var(--line-2); background: rgba(255, 255, 255, 0.03); color: var(--muted); font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; }
  .kucuk .secim button { padding: 4px 9px; font-size: 12px; }
  .secim button:hover:not(:disabled) { color: var(--text); border-color: var(--gold-dim); }
  .secim button:disabled { opacity: 0.45; cursor: default; }
  .secim .evet.secili { background: var(--gold); border-color: var(--gold); color: #1b1608; }
  .secim button.secili { background: rgba(227, 194, 112, 0.16); border-color: var(--gold-dim); color: var(--gold-glow); }
  .secim .hayir.secili { background: rgba(255, 255, 255, 0.08); border-color: var(--line-2); color: var(--muted); }
  .secim .iptal { margin-left: auto; border-color: transparent; background: none; color: var(--dim); }
  .secim .iptal.onay { background: rgba(229, 72, 77, 0.18); color: #ff8a8d; border-color: transparent; }
</style>
