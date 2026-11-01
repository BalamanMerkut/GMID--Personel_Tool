<script lang="ts">
  // Steam'in arkadaş listesi ve sohbet penceresi gibi: her ekranda sağ altta. Arkadaşlar (oyunda / çevrim içi /
  // çevrim dışı), gelen istekler, arkadaş ekleme; sekmeli özel sohbet penceresi.
  import { tick } from "svelte";
  import Icon from "../../Icon.svelte";
  import { player } from "../../player.svelte";
  import {
    arkadasEkle, arkadasSil, istek, mesajlariDinle, profilGoster, sohbetAc, sohbetKapat, sosyal, toplamOkunmamis,
    type Kisi, type Mesaj,
  } from "../../social.svelte";
  import { notify } from "../../toast.svelte";
  import Avatar from "./Avatar.svelte";

  const hata = (e: unknown) => notify(String(e instanceof Error ? e.message : e), "info");
  const SAAT = new Intl.DateTimeFormat("tr-TR", { hour: "2-digit", minute: "2-digit" });
  const GUN = new Intl.DateTimeFormat("tr-TR", { day: "numeric", month: "short" });

  // ---------- arkadaş listesi ----------
  const durumlu = $derived(sosyal.arkadaslar.map((k) => ({ ...k, c: sosyal.cevrimici[k.id] })));
  const oyunda = $derived(durumlu.filter((k) => k.c?.oyun));
  const acik = $derived(durumlu.filter((k) => k.c && !k.c.oyun));
  const kapali = $derived(durumlu.filter((k) => !k.c));
  const cevrimiciSayi = $derived(oyunda.length + acik.length);
  const rozet = $derived(toplamOkunmamis() + sosyal.gelen.length);

  let ekle = $state(false);
  let ara = $state("");
  let herkes = $state<Kisi[]>([]);
  async function ekleAc() {
    ekle = !ekle;
    ara = "";
    if (ekle) herkes = (await istek<{ kisiler: Kisi[] }>("GET", "/api/kisiler").catch(() => ({ kisiler: [] }))).kisiler;
  }
  const adaylar = $derived(
    herkes
      .filter((k) => k.id !== sosyal.ben?.id && !sosyal.arkadaslar.some((x) => x.id === k.id))
      .filter((k) => !ara.trim() || k.ad.toLocaleLowerCase("tr").includes(ara.trim().toLocaleLowerCase("tr")))
      .slice(0, 30),
  );
  const gonderildi = (k: Kisi) => sosyal.giden.some((x) => x.id === k.id);

  // ---------- sohbet ----------
  let mesajlar = $state<Record<string, Mesaj[]>>({});
  let taslak = $state("");
  let akis: HTMLDivElement | undefined = $state();
  const aktif = $derived(sosyal.sohbetler.find((k) => k.id === sosyal.aktifSohbet) ?? null);

  async function sona() {
    await tick();
    akis?.scrollTo({ top: akis.scrollHeight });
  }

  async function yukle(k: Kisi) {
    try {
      const r = await istek<{ mesajlar: Mesaj[] }>("GET", `/api/mesaj/${k.id}`);
      mesajlar = { ...mesajlar, [k.id]: r.mesajlar };
      const { [k.id]: _, ...kalan } = sosyal.okunmamis;
      sosyal.okunmamis = kalan;
      await sona();
    } catch (e) {
      hata(e);
    }
  }

  // Sekme değişince (ya da pencere açılınca) o konuşma yüklenir ve okundu olur.
  $effect(() => {
    if (sosyal.sohbetAcik && aktif) void yukle(aktif);
  });

  $effect(() =>
    mesajlariDinle((m) => {
      const diger = m.kimden === sosyal.ben?.id ? m.kime : m.kimden;
      const l = mesajlar[diger];
      if (l && !l.some((x) => x.id === m.id)) mesajlar = { ...mesajlar, [diger]: [...l, m] };
      if (sosyal.sohbetAcik && sosyal.aktifSohbet === diger) {
        void sona();
        if (m.kimden === diger) void istek("GET", `/api/mesaj/${diger}?once=0`).catch(() => {});
      }
    }),
  );

  async function gonder() {
    const metin = taslak.trim();
    if (!metin || !aktif) return;
    taslak = "";
    try {
      await istek("POST", `/api/mesaj/${aktif.id}`, { metin });
    } catch (e) {
      taslak = metin;
      hata(e);
    }
  }

  const altBosluk = $derived(player.item && !player.open ? 92 : 14);
</script>

{#if sosyal.anahtar}
  <div class="dock" style:bottom="{altBosluk}px" style:--alt="{altBosluk}px">
    <div class="pencereler">
    {#if sosyal.sohbetAcik && aktif}
      <section class="sohbet" aria-label="Sohbet">
        <nav class="sekmeler">
          {#each sosyal.sohbetler as k (k.id)}
            <div class="sekme" class:secili={k.id === sosyal.aktifSohbet}>
              <button class="sekme-ad" onclick={() => (sosyal.aktifSohbet = k.id)}>
                <Avatar kisi={k} boy={20} nokta={false} /><span>{k.ad}</span>
                {#if sosyal.okunmamis[k.id]}<i class="nokta-yeni"></i>{/if}
              </button>
              <button class="kapat" aria-label="Sekmeyi kapat" onclick={() => sohbetKapat(k.id)}><Icon name="x" size={12} /></button>
            </div>
          {/each}
          <button class="ic kucult" title="Küçült" aria-label="Sohbeti küçült" onclick={() => (sosyal.sohbetAcik = false)}><Icon name="shrinkdown" size={15} /></button>
        </nav>
        <header class="kim">
          <button class="kim-btn" onclick={() => aktif && profilGoster(aktif.id)}>
            <Avatar kisi={aktif} boy={36} />
            <span class="kim-yazi">
              <b>{aktif.ad}</b>
              <span class="small" class:oyun={!!sosyal.cevrimici[aktif.id]?.oyun}>
                {sosyal.cevrimici[aktif.id]?.oyun ? `${sosyal.cevrimici[aktif.id]?.oyun} oynuyor` : sosyal.cevrimici[aktif.id] ? "Çevrim içi" : "Çevrim dışı"}
              </span>
            </span>
          </button>
        </header>
        <div class="akis" bind:this={akis}>
          {#each mesajlar[aktif.id] ?? [] as m, i (m.id)}
            {@const l = mesajlar[aktif.id] ?? []}
            {#if i === 0 || m.zaman - l[i - 1].zaman > 30 * 60_000}<div class="zaman">{GUN.format(m.zaman)} {SAAT.format(m.zaman)}</div>{/if}
            <div class="balon" class:benim={m.kimden === sosyal.ben?.id} title={SAAT.format(m.zaman)}>{m.metin}</div>
          {:else}
            <p class="bos">{aktif.ad} ile sohbetin burada başlar.</p>
          {/each}
        </div>
        <form class="yaz" onsubmit={(e) => (e.preventDefault(), gonder())}>
          <input bind:value={taslak} placeholder="Mesaj yaz…" maxlength="1000" aria-label="Mesaj" />
          <button class="ic gonder" disabled={!taslak.trim()} aria-label="Gönder"><Icon name="send" size={17} /></button>
        </form>
      </section>
    {/if}

    {#if sosyal.liste}
      <section class="liste" aria-label="Arkadaşlar">
        <header class="ben">
          {#if sosyal.ben}<Avatar kisi={sosyal.ben} boy={38} />{/if}
          <div class="ben-yazi"><b>{sosyal.ben?.ad}</b><span class="small">{sosyal.bagli ? "Çevrim içi" : "Bağlanıyor…"}</span></div>
          <button class="ic" class:on={ekle} title="Arkadaş ekle" aria-label="Arkadaş ekle" onclick={ekleAc}><Icon name="plus" size={17} /></button>
          <button class="ic" title="Kapat" aria-label="Arkadaş listesini kapat" onclick={() => (sosyal.liste = false)}><Icon name="x" size={16} /></button>
        </header>
        <div class="liste-ic">
          {#if ekle}
            <div class="ekle">
              <input bind:value={ara} placeholder="Üye ara…" aria-label="Üye ara" />
              {#each adaylar as k (k.id)}
                <div class="satir">
                  <button class="satir-ana" onclick={() => profilGoster(k.id)}><Avatar kisi={k} boy={30} /><span class="ad">{k.ad}</span></button>
                  {#if gonderildi(k)}
                    <button class="mini" onclick={() => arkadasSil(k).catch(hata)} title="İsteği geri al">Gönderildi</button>
                  {:else}
                    <button class="mini altin" onclick={() => arkadasEkle(k).catch(hata)}><Icon name="plus" size={12} />Ekle</button>
                  {/if}
                </div>
              {:else}
                <p class="bos">Eklenecek kimse bulunamadı.</p>
              {/each}
            </div>
          {/if}

          {#if sosyal.gelen.length}
            <div class="grup">Arkadaşlık istekleri <span>{sosyal.gelen.length}</span></div>
            {#each sosyal.gelen as k (k.id)}
              <div class="satir istek">
                <button class="satir-ana" onclick={() => profilGoster(k.id)}><Avatar kisi={k} boy={30} /><span class="ad">{k.ad}</span></button>
                <button class="mini altin" onclick={() => arkadasEkle(k).catch(hata)}>Kabul et</button>
                <button class="ic kucuk" title="Reddet" aria-label="Reddet" onclick={() => arkadasSil(k).catch(hata)}><Icon name="x" size={13} /></button>
              </div>
            {/each}
          {/if}

          {#snippet grup(baslik: string, l: typeof durumlu)}
            {#if l.length}
              <div class="grup">{baslik} <span>{l.length}</span></div>
              {#each l as k (k.id)}
                <button class="satir arkadas" class:kapali={!k.c} onclick={() => sohbetAc(k)} title="Sohbet aç">
                  <Avatar kisi={k} boy={32} />
                  <span class="satir-yazi">
                    <span class="ad">{k.ad}</span>
                    <span class="small" class:oyun={!!k.c?.oyun}>{k.c?.oyun ?? (k.c ? "Çevrim içi" : "Çevrim dışı")}</span>
                  </span>
                  {#if sosyal.okunmamis[k.id]}<span class="say tnum">{sosyal.okunmamis[k.id]}</span>{/if}
                </button>
              {/each}
            {/if}
          {/snippet}
          {@render grup("Oyunda", oyunda)}
          {@render grup("Çevrim içi", acik)}
          {@render grup("Çevrim dışı", kapali)}
          {#if !sosyal.arkadaslar.length && !sosyal.gelen.length && !ekle}
            <div class="bos-kutu">
              <Icon name="users" size={26} />
              <span>Henüz arkadaşın yok.</span>
              <button class="mini altin" onclick={ekleAc}><Icon name="plus" size={12} />Arkadaş ekle</button>
            </div>
          {/if}
        </div>
      </section>
    {/if}

    </div>
    <div class="cubuk">
      {#if !sosyal.sohbetAcik && sosyal.sohbetler.length}
        <button class="sohbet-dugme" onclick={() => (sosyal.sohbetAcik = true)} title="Sohbeti aç">
          <Icon name="chat" size={16} />Sohbet
          {#if toplamOkunmamis()}<span class="say tnum">{toplamOkunmamis()}</span>{/if}
        </button>
      {/if}
      <button class="arkadaslar-dugme" class:acik={sosyal.liste} onclick={() => (sosyal.liste = !sosyal.liste)}>
        <Icon name="users" size={16} />
        <span>Arkadaşlar</span>
        <span class="cev tnum"><i></i>{cevrimiciSayi}</span>
        {#if rozet}<span class="say tnum">{rozet}</span>{/if}
      </button>
    </div>
  </div>
{/if}

<style>
  .dock { position: fixed; right: 16px; z-index: 60; display: flex; flex-direction: column; align-items: flex-end; gap: 8px; pointer-events: none; }
  .pencereler { display: flex; align-items: flex-end; gap: 10px; }
  .pencereler > *, .cubuk > * { pointer-events: auto; }
  .cubuk { display: flex; align-items: center; gap: 8px; }
  .arkadaslar-dugme, .sohbet-dugme { display: inline-flex; align-items: center; gap: 8px; padding: 9px 14px; border-radius: 999px; border: 1px solid var(--line-2); background: rgba(24, 26, 31, 0.96); color: var(--text); font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45); }
  .arkadaslar-dugme:hover, .sohbet-dugme:hover, .arkadaslar-dugme.acik { border-color: rgba(227, 194, 112, 0.5); }
  .cev { display: inline-flex; align-items: center; gap: 5px; color: var(--muted); font-weight: 500; }
  .cev i { width: 7px; height: 7px; border-radius: 50%; background: var(--good); }
  .say { min-width: 18px; height: 18px; padding: 0 5px; border-radius: 999px; background: #e5484d; color: #fff; font-size: 10.5px; font-weight: 800; display: inline-grid; place-items: center; }

  .liste, .sohbet { display: flex; flex-direction: column; border-radius: 16px; border: 1px solid var(--line-2); background: rgba(22, 24, 29, 0.98); box-shadow: 0 18px 50px rgba(0, 0, 0, 0.55); overflow: hidden; animation: gel 0.18s ease-out; }
  @keyframes gel { from { opacity: 0; transform: translateY(10px); } }
  .liste { width: 290px; height: min(540px, calc(100vh - 150px - var(--alt))); }
  .sohbet { width: 360px; height: min(500px, calc(100vh - 150px - var(--alt))); }

  .ben { display: flex; align-items: center; gap: 10px; padding: 12px; background: linear-gradient(180deg, rgba(227, 194, 112, 0.1), transparent); border-bottom: 1px solid var(--line); }
  .ben-yazi { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .ben-yazi .small { color: var(--good); }
  .small { font-size: 11.5px; color: var(--dim); }
  .liste-ic { flex: 1; overflow-y: auto; padding: 6px 6px 10px; }
  .grup { display: flex; align-items: center; gap: 6px; margin: 10px 8px 4px; font-size: 10.5px; font-weight: 800; letter-spacing: 0.1em; text-transform: uppercase; color: var(--dim); }
  .grup span { color: var(--muted); }
  .satir { display: flex; align-items: center; gap: 8px; width: 100%; padding: 6px 8px; border-radius: 10px; border: 0; background: none; color: var(--text); font: inherit; text-align: left; }
  .satir.arkadas { cursor: pointer; }
  .satir.arkadas:hover, .satir.istek { background: rgba(255, 255, 255, 0.04); }
  .satir.kapali { opacity: 0.55; }
  .satir-ana { flex: 1; min-width: 0; display: flex; align-items: center; gap: 8px; border: 0; background: none; padding: 0; color: inherit; font: inherit; cursor: pointer; text-align: left; }
  .satir-yazi { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .ad { font-size: 13.5px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .oyun { color: #a99cff; }
  .ekle { display: flex; flex-direction: column; gap: 2px; padding: 4px 2px 8px; border-bottom: 1px solid var(--line); margin-bottom: 4px; }
  .ekle input, .yaz input { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 999px; padding: 7px 12px; color: var(--text); font: inherit; font-size: 13px; outline: none; }
  .ekle input { margin: 4px 4px 6px; }
  .ekle input:focus, .yaz input:focus { border-color: rgba(227, 194, 112, 0.5); }
  .mini { display: inline-flex; align-items: center; gap: 4px; padding: 4px 9px; border-radius: 8px; border: 1px solid var(--line-2); background: none; color: var(--muted); font: inherit; font-size: 11.5px; font-weight: 600; cursor: pointer; white-space: nowrap; }
  .mini.altin { border-color: rgba(227, 194, 112, 0.5); color: var(--gold-glow); }
  .mini:hover { background: rgba(255, 255, 255, 0.05); }
  .ic { display: grid; place-items: center; width: 30px; height: 30px; border-radius: 8px; border: 0; background: none; color: var(--dim); cursor: pointer; flex: none; }
  .ic:hover:not(:disabled), .ic.on { color: var(--text); background: rgba(255, 255, 255, 0.06); }
  .ic:disabled { opacity: 0.35; cursor: default; }
  .ic.kucuk { width: 24px; height: 24px; }
  .bos { color: var(--dim); font-size: 12.5px; text-align: center; padding: 14px; }
  .bos-kutu { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 34px 12px; color: var(--dim); font-size: 13px; }

  .sekmeler { display: flex; align-items: center; gap: 2px; padding: 6px 6px 0; background: rgba(0, 0, 0, 0.25); overflow-x: auto; }
  .sekme { display: flex; align-items: center; border-radius: 9px 9px 0 0; background: rgba(255, 255, 255, 0.03); max-width: 140px; }
  .sekme.secili { background: rgba(255, 255, 255, 0.09); }
  .sekme-ad { display: flex; align-items: center; gap: 6px; padding: 6px 4px 6px 8px; border: 0; background: none; color: var(--muted); font: inherit; font-size: 12px; cursor: pointer; min-width: 0; }
  .sekme.secili .sekme-ad { color: var(--text); }
  .sekme-ad span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .nokta-yeni { width: 7px; height: 7px; border-radius: 50%; background: #e5484d; flex: none; }
  .kapat { display: grid; place-items: center; width: 20px; height: 20px; margin-right: 4px; border-radius: 6px; border: 0; background: none; color: var(--dim); cursor: pointer; }
  .kapat:hover { color: var(--text); background: rgba(255, 255, 255, 0.08); }
  .kucult { margin-left: auto; }
  .kim { padding: 10px 12px; border-bottom: 1px solid var(--line); background: rgba(255, 255, 255, 0.03); }
  .kim-btn { display: flex; align-items: center; gap: 10px; border: 0; background: none; padding: 0; color: var(--text); font: inherit; cursor: pointer; text-align: left; }
  .kim-yazi { display: flex; flex-direction: column; }
  .akis { flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 4px; }
  .zaman { align-self: center; font-size: 11px; color: var(--dim); margin: 8px 0 2px; }
  .balon { align-self: flex-start; max-width: 80%; padding: 7px 11px; border-radius: 14px 14px 14px 4px; background: rgba(255, 255, 255, 0.07); font-size: 13.5px; line-height: 1.4; white-space: pre-wrap; overflow-wrap: anywhere; }
  .balon.benim { align-self: flex-end; border-radius: 14px 14px 4px 14px; background: linear-gradient(160deg, rgba(227, 194, 112, 0.32), rgba(227, 194, 112, 0.2)); color: #fff4d8; }
  .yaz { display: flex; gap: 6px; padding: 10px; border-top: 1px solid var(--line); }
  .yaz input { flex: 1; min-width: 0; }
  .gonder { color: var(--gold); }
  @media (prefers-reduced-motion: reduce) { .liste, .sohbet { animation: none; } }
</style>
