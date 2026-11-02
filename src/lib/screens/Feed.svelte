<script lang="ts">
  // Sosyal (Twitter düzeni): üstte "Neler oluyor?" yazma kutusu (yazı + resim), ortada tek sütun akış (gönderiler,
  // puanlar, oyuna girişler, klipler; beğeni ve yanıt), sağda şu an oyunda olanlar, gündem ve tanıyor olabileceklerin.
  import { onMount } from "svelte";
  import Icon from "../Icon.svelte";
  import Avatar from "../components/social/Avatar.svelte";
  import FeedCard from "../components/social/FeedCard.svelte";
  import { getJson, setJson } from "../settings.svelte";
  import { arkadasEkle, gonderiPaylas, istek, profilGoster, sosyal, type Kisi, type Olay } from "../social.svelte";
  import { nav } from "../player.svelte";
  import { notify } from "../toast.svelte";

  const SINIR = 500;

  let olaylar = $state<Olay[]>([]);
  let yukleniyor = $state(false);
  let son = $state(false);
  let yalnizArkadas = $state(getJson<boolean>("social.feedFriends", false));
  let kisiler = $state<Kisi[]>([]);

  // Yazma kutusu
  let taslak = $state("");
  let resim = $state<File | null>(null);
  let onizleme = $state<string | null>(null);
  let gonderiliyor = $state(false);
  let dosyaSec = $state<HTMLInputElement | null>(null);
  const kalanHarf = $derived(SINIR - [...taslak].length);
  const gonderilebilir = $derived(!gonderiliyor && kalanHarf >= 0 && (taslak.trim().length > 0 || !!resim));

  const hata = (e: unknown) => notify(String(e instanceof Error ? e.message : e), "info");

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
      hata(e);
    } finally {
      yukleniyor = false;
    }
  }

  function resimSecildi(e: Event) {
    const f = (e.currentTarget as HTMLInputElement).files?.[0] ?? null;
    (e.currentTarget as HTMLInputElement).value = "";
    if (!f) return;
    if (!f.type.startsWith("image/")) return notify("Yalnız resim eklenebilir", "info");
    resimKaldir();
    resim = f;
    onizleme = URL.createObjectURL(f);
  }

  function resimKaldir() {
    if (onizleme) URL.revokeObjectURL(onizleme);
    onizleme = null;
    resim = null;
  }

  async function paylas() {
    if (!gonderilebilir) return;
    gonderiliyor = true;
    try {
      await gonderiPaylas(taslak.trim(), resim);
      taslak = "";
      resimKaldir();
      if (yalnizArkadas) void yukle();
    } catch (e) {
      hata(e);
    } finally {
      gonderiliyor = false;
    }
  }

  function tus(e: KeyboardEvent) {
    // Ctrl+Enter ile gönder (Twitter'daki gibi).
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      void paylas();
    }
  }

  /** Kart sunucudan sildi; listeden de çıkar. */
  function sil(o: Olay) {
    olaylar = olaylar.filter((x) => x.id !== o.id);
  }

  function suzgec(v: boolean) {
    if (yalnizArkadas === v) return;
    yalnizArkadas = v;
    void setJson("social.feedFriends", v);
    void yukle();
  }

  // Sağ sütun
  const oyunda = $derived(
    Object.entries(sosyal.cevrimici)
      .filter(([, k]) => k.oyun)
      .map(([id, k]) => ({ ...k, id }))
      .slice(0, 6),
  );
  /** Akışta en çok geçen oyun / film / dizi (yüklü sayfadan). */
  const gundem = $derived.by(() => {
    const say = new Map<string, number>();
    for (const o of olaylar) {
      const ad = o.tur === "klip" ? o.veri.oyun : o.tur === "gonderi" ? null : o.veri.ad;
      if (ad) say.set(ad, (say.get(ad) ?? 0) + 1);
    }
    return [...say.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5);
  });
  const oneriler = $derived(
    kisiler
      .filter((k) => k.id !== sosyal.ben?.id)
      .filter((k) => ![...sosyal.arkadaslar, ...sosyal.gelen, ...sosyal.giden].some((x) => x.id === k.id))
      .slice(0, 3),
  );

  async function ekle(k: Kisi) {
    try {
      await arkadasEkle(k);
      notify(`${k.ad}: arkadaşlık isteği gönderildi`, "check");
    } catch (e) {
      hata(e);
    }
  }

  // Akış değişince (biri paylaştı, beğendi, yanıtladı) ilk sayfa sessizce yenilenir.
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
    if (sosyal.anahtar) {
      void istek<{ kisiler: Kisi[] }>("GET", "/api/kisiler")
        .then((r) => (kisiler = r.kisiler))
        .catch(() => {});
    }
    return () => resimKaldir();
  });
</script>

{#if !sosyal.anahtar}
  <section class="panel bos yalniz">
    <Icon name="sparkle" size={30} />
    <b>Topluluğun gönderileri, puanları ve klipleri burada</b>
    <span class="sub">Görmek ve paylaşmak için Topluluk'tan Discord ile giriş yap.</span>
    <button class="btn" onclick={() => nav.go("social")}><Icon name="users" size={16} />Topluluk'a git</button>
  </section>
{:else}
  <div class="duzen">
    <main class="zaman-tuneli">
      <div class="sekmeler" role="tablist">
        <button role="tab" aria-selected={!yalnizArkadas} onclick={() => suzgec(false)}><span>Herkes</span></button>
        <button role="tab" aria-selected={yalnizArkadas} onclick={() => suzgec(true)}><span>Arkadaşlarım</span></button>
        <button class="yenile" title="Yenile" aria-label="Yenile" disabled={yukleniyor} onclick={() => yukle()}><Icon name="refresh" size={16} /></button>
      </div>

      <form class="yaz" onsubmit={(e) => (e.preventDefault(), paylas())}>
        {#if sosyal.ben}<Avatar kisi={sosyal.ben} boy={44} nokta={false} />{/if}
        <div class="yaz-sag">
          <textarea bind:value={taslak} onkeydown={tus} placeholder="Neler oluyor?" rows="2" aria-label="Gönderi" disabled={gonderiliyor}></textarea>
          {#if onizleme}
            <div class="onizleme">
              <img src={onizleme} alt="Eklenen resim" />
              <button type="button" class="kaldir" title="Resmi kaldır" aria-label="Resmi kaldır" onclick={resimKaldir}><Icon name="x" size={15} /></button>
            </div>
          {/if}
          <div class="yaz-alt">
            <button type="button" class="arac" title="Resim ekle" aria-label="Resim ekle" onclick={() => dosyaSec?.click()} disabled={gonderiliyor}><Icon name="image" size={19} /></button>
            <input bind:this={dosyaSec} type="file" accept="image/*" hidden onchange={resimSecildi} />
            <span class="bosluk"></span>
            {#if taslak.length}
              <span class="sayac tnum" class:az={kalanHarf <= 40} class:asti={kalanHarf < 0}>{kalanHarf}</span>
            {/if}
            <button class="btn paylas" disabled={!gonderilebilir} title="Ctrl+Enter">{gonderiliyor ? "Paylaşılıyor…" : "Paylaş"}</button>
          </div>
        </div>
      </form>

      {#if yukleniyor && !olaylar.length}
        {#each Array(3) as _, i (i)}<div class="iskelet"><span class="sk a"></span><div class="sk-sag"><span class="sk s"></span><span class="sk k"></span></div></div>{/each}
      {:else}
        {#each olaylar as o (o.id)}
          <FeedCard {o} ondegisti={() => yukle()} onsil={sil} />
        {:else}
          <div class="bos">
            <Icon name="sparkle" size={28} />
            <b>Akış şimdilik sessiz</b>
            <span class="sub">{yalnizArkadas ? "Arkadaşların bir şey paylaşınca burada görünür." : "İlk gönderiyi sen yaz: bu akşam kim ne oynuyor?"}</span>
          </div>
        {/each}
        {#if olaylar.length && !son}
          <button class="devam" disabled={yukleniyor} onclick={() => yukle(true)}>{yukleniyor ? "Yükleniyor…" : "Daha fazla göster"}</button>
        {/if}
      {/if}
    </main>

    <aside class="yan">
      <section class="kutu">
        <h3>Şu an oyunda</h3>
        {#each oyunda as k (k.id)}
          <button class="satir" onclick={() => profilGoster(k.id)}>
            <Avatar kisi={k} boy={36} />
            <span class="satir-yazi"><b>{k.ad}</b><span class="mor">{k.oyun}</span></span>
          </button>
        {:else}
          <p class="sub small">Şu an kimse oyunda değil.</p>
        {/each}
      </section>

      {#if gundem.length}
        <section class="kutu">
          <h3>Gündem</h3>
          {#each gundem as [ad, n], i (ad)}
            <div class="gundem">
              <span class="sub small">{i + 1} · Toplulukta</span>
              <b>{ad}</b>
              <span class="sub small">{n} paylaşım</span>
            </div>
          {/each}
        </section>
      {/if}

      {#if oneriler.length}
        <section class="kutu">
          <h3>Tanıyor olabileceklerin</h3>
          {#each oneriler as k (k.id)}
            <div class="satir">
              <button class="satir-sol" onclick={() => profilGoster(k.id)}>
                <Avatar kisi={k} boy={36} />
                <span class="satir-yazi"><b>{k.ad}</b><span class="sub small">GMID üyesi</span></span>
              </button>
              <button class="btn sm takip" onclick={() => ekle(k)}>Ekle</button>
            </div>
          {/each}
        </section>
      {/if}
    </aside>
  </div>
{/if}

<style>
  .duzen { display: grid; grid-template-columns: minmax(0, 620px) 300px; gap: 28px; justify-content: center; align-items: start; }
  .zaman-tuneli { border-left: 1px solid var(--line); border-right: 1px solid var(--line); min-height: 100%; background: rgba(0, 0, 0, 0.12); }
  .sekmeler { position: sticky; top: 0; z-index: 2; display: flex; border-bottom: 1px solid var(--line); background: rgba(20, 22, 26, 0.88); backdrop-filter: blur(10px); }
  .sekmeler [role="tab"] { flex: 1; border: 0; background: none; color: var(--muted); font: inherit; font-weight: 600; font-size: 15px; padding: 0; height: 52px; cursor: pointer; display: grid; place-items: center; }
  .sekmeler [role="tab"]:hover { background: rgba(255, 255, 255, 0.04); }
  .sekmeler [role="tab"] span { position: relative; height: 100%; display: grid; place-items: center; }
  .sekmeler [role="tab"][aria-selected="true"] { color: var(--text); font-weight: 800; }
  .sekmeler [role="tab"][aria-selected="true"] span::after { content: ""; position: absolute; left: 0; right: 0; bottom: 0; height: 4px; border-radius: 999px; background: var(--gold); }
  .yenile { flex: none; width: 52px; border: 0; background: none; color: var(--dim); display: grid; place-items: center; cursor: pointer; }
  .yenile:hover { color: var(--text); background: rgba(255, 255, 255, 0.04); }
  .yaz { display: grid; grid-template-columns: 44px minmax(0, 1fr); gap: 12px; padding: 14px 18px 10px; border-bottom: 1px solid var(--line); }
  .yaz-sag { display: flex; flex-direction: column; gap: 10px; min-width: 0; }
  textarea { width: 100%; border: 0; background: none; color: var(--text); font: inherit; font-size: 19px; line-height: 1.4; resize: none; outline: none; padding: 8px 0; field-sizing: content; min-height: 52px; max-height: 300px; }
  textarea::placeholder { color: var(--dim); }
  .onizleme { position: relative; align-self: flex-start; }
  .onizleme img { display: block; max-width: 100%; max-height: 300px; border-radius: 16px; border: 1px solid var(--line); }
  .kaldir { position: absolute; top: 8px; left: 8px; width: 30px; height: 30px; border-radius: 50%; border: 0; background: rgba(0, 0, 0, 0.7); color: #fff; display: grid; place-items: center; cursor: pointer; }
  .yaz-alt { display: flex; align-items: center; gap: 10px; padding-top: 8px; border-top: 1px solid var(--line); }
  .arac { width: 36px; height: 36px; border-radius: 50%; border: 0; background: none; color: var(--gold); display: grid; place-items: center; cursor: pointer; }
  .arac:hover:not(:disabled) { background: rgba(227, 194, 112, 0.12); }
  .bosluk { flex: 1; }
  .sayac { font-size: 13px; color: var(--dim); }
  .sayac.az { color: var(--gold-glow); }
  .sayac.asti { color: #ff6b6b; font-weight: 700; }
  .paylas { border-radius: 999px; padding: 8px 20px; font-weight: 800; }
  .bos { display: flex; flex-direction: column; align-items: center; text-align: center; gap: 8px; padding: 40px 24px; color: var(--muted); }
  .bos.yalniz { max-width: 620px; margin: 0 auto; }
  .devam { width: 100%; border: 0; border-bottom: 1px solid var(--line); background: none; color: #6cb6ff; font: inherit; font-size: 14px; padding: 16px; cursor: pointer; }
  .devam:hover { background: rgba(255, 255, 255, 0.03); }
  .iskelet { display: grid; grid-template-columns: 44px 1fr; gap: 12px; padding: 14px 18px; border-bottom: 1px solid var(--line); }
  .sk-sag { display: flex; flex-direction: column; gap: 10px; }
  .sk { display: block; background: linear-gradient(90deg, rgba(255, 255, 255, 0.05) 25%, rgba(255, 255, 255, 0.1) 50%, rgba(255, 255, 255, 0.05) 75%); background-size: 200% 100%; animation: parilti 1.4s infinite linear; }
  .sk.a { width: 44px; height: 44px; border-radius: 50%; }
  .sk.s { width: 45%; height: 12px; border-radius: 6px; }
  .sk.k { width: 100%; height: 84px; border-radius: 16px; }
  @keyframes parilti { to { background-position: -200% 0; } }
  .yan { position: sticky; top: 0; display: flex; flex-direction: column; gap: 16px; }
  .kutu { display: flex; flex-direction: column; border-radius: 16px; border: 1px solid var(--line); background: rgba(255, 255, 255, 0.025); overflow: hidden; }
  .kutu h3 { margin: 0; padding: 12px 16px 8px; font-family: var(--f-display); font-size: 17px; font-weight: 800; }
  .kutu > .sub { margin: 0; padding: 4px 16px 14px; }
  .satir { display: flex; align-items: center; gap: 10px; padding: 10px 16px; border: 0; background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; width: 100%; }
  .satir:hover, .gundem:hover { background: rgba(255, 255, 255, 0.04); }
  .satir-sol { flex: 1; min-width: 0; display: flex; align-items: center; gap: 10px; border: 0; background: none; color: inherit; font: inherit; text-align: left; cursor: pointer; padding: 0; }
  .satir-yazi { display: flex; flex-direction: column; min-width: 0; line-height: 1.3; }
  .satir-yazi b, .satir-yazi span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mor { color: #a99cff; font-size: 13px; }
  .gundem { display: flex; flex-direction: column; padding: 10px 16px; gap: 1px; }
  .gundem b { font-size: 15px; }
  .takip { border-radius: 999px; padding: 6px 16px; font-weight: 800; }
  @media (max-width: 1180px) {
    .duzen { grid-template-columns: minmax(0, 640px); }
    .yan { display: none; }
  }
</style>
