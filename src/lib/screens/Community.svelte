<script lang="ts">
  // Topluluk: GMID'in tanıtımı, Discord sunucusuna katılma, Discord ile giriş, üyeler (arkadaş ekle, profil).
  import { onMount } from "svelte";
  import logo from "../../assets/gmid-logo.webp";
  import Icon from "../Icon.svelte";
  import Avatar from "../components/social/Avatar.svelte";
  import { once } from "../components/social/metin";
  import { community, joinDiscord } from "../community.svelte";
  import { arkadasEkle, arkadasSil, girisYap, istek, profilGoster, sohbetAc, sosyal, type Kisi } from "../social.svelte";
  import { notify } from "../toast.svelte";

  let kisiler = $state<Kisi[]>([]);
  let ara = $state("");

  const hata = (e: unknown) => notify(String(e instanceof Error ? e.message : e), "info");

  async function yukle() {
    if (!sosyal.anahtar) return;
    try {
      kisiler = (await istek<{ kisiler: Kisi[] }>("GET", "/api/kisiler")).kisiler;
    } catch (e) {
      hata(e);
    }
  }

  async function giris() {
    try {
      await girisYap();
      await yukle();
    } catch (e) {
      hata(e);
    }
  }

  /** Arkadaşlık durumu arkadaş listesinden (anlık güncel). */
  function iliski(k: Kisi): Kisi["iliski"] {
    if (sosyal.arkadaslar.some((x) => x.id === k.id)) return "arkadas";
    if (sosyal.gelen.some((x) => x.id === k.id)) return "gelen";
    if (sosyal.giden.some((x) => x.id === k.id)) return "giden";
    return null;
  }

  const liste = $derived(
    kisiler
      .filter((k) => !ara.trim() || k.ad.toLocaleLowerCase("tr").includes(ara.trim().toLocaleLowerCase("tr")))
      .map((k) => ({ ...k, c: sosyal.cevrimici[k.id] }))
      .sort((a, b) => Number(!!b.c) - Number(!!a.c) || Number(!!b.c?.oyun) - Number(!!a.c?.oyun) || a.ad.localeCompare(b.ad, "tr")),
  );
  const cevrimici = $derived(Object.keys(sosyal.cevrimici).length);
  const oyunda = $derived(Object.values(sosyal.cevrimici).filter((k) => k.oyun).length);

  onMount(() => void yukle());
</script>

<div class="topluluk">
  <section class="hero">
    <div class="parilti" aria-hidden="true"></div>
    <img class="logo" src={logo} alt="GMID" />
    <div class="hero-yazi">
      <span class="ust-baslik">Game Mansion In Discord</span>
      <h2>Malikâneye hoş geldin</h2>
      <p>
        GMID, oyun oynayan, film ve dizi konuşan arkadaşların buluştuğu bir Discord malikânesi. Akşam kim hangi oyunda,
        kim hangi diziyi bitirip kaç puan verdi, hepsi burada. Bir maça birlikte girmek, bir film önermek ya da sadece
        sohbet etmek için kapımız açık.
      </p>
      <div class="hero-dugmeler">
        {#if community.discord}
          <button class="btn discord" onclick={joinDiscord}><Icon name="discord" size={18} />Discord sunucusuna katıl</button>
        {/if}
        {#if !sosyal.anahtar}
          <button class="btn" disabled={sosyal.giriliyor} onclick={giris}>
            <Icon name="users" size={17} />{sosyal.giriliyor ? "Tarayıcıda onay bekleniyor…" : "Discord ile giriş yap"}
          </button>
        {/if}
      </div>
    </div>
    {#if sosyal.anahtar}
      <div class="sayilar">
        <div><b class="tnum">{kisiler.length}</b><span>üye</span></div>
        <div><b class="tnum yesil">{cevrimici}</b><span>çevrim içi</span></div>
        <div><b class="tnum mor">{oyunda}</b><span>oyunda</span></div>
      </div>
    {/if}
  </section>

  {#if !sosyal.anahtar}
    <section class="panel bilgi">
      <h3 class="h2"><Icon name="sparkle" />Giriş yapınca neler var?</h3>
      <ul>
        <li><Icon name="users" size={16} /><span><b>Arkadaşlar ve sohbet:</b> her ekranda sağ altta arkadaş listesi; kim çevrim içi, kim ne oynuyor; özel mesaj.</span></li>
        <li><Icon name="sparkle" size={16} /><span><b>Sosyal akış:</b> kim neyi kaç puanladı, kim hangi oyuna girdi; beğen, yorum yap.</span></li>
        <li><Icon name="user" size={16} /><span><b>Profil:</b> favori oyunların, en çok oynadıkların, film ve dizi vitrinlerin.</span></li>
        <li><Icon name="shield" size={16} /><span>Discord hesabınla girersin, şifre yok. Neyin paylaşılacağını Kontrol Odası → Topluluk'tan sen seçersin; notların ve dosyaların hiç paylaşılmaz.</span></li>
      </ul>
    </section>
  {:else}
    <section class="panel">
      <div class="spread">
        <h3 class="h2"><Icon name="users" />Üyeler</h3>
        <input class="ara" bind:value={ara} placeholder="Üye ara…" aria-label="Üye ara" />
      </div>
      <div class="uyeler">
        {#each liste as k (k.id)}
          {@const il = iliski(k)}
          <div class="uye" class:kapali={!k.c}>
            <button class="uye-ana" onclick={() => profilGoster(k.id)}>
              <Avatar kisi={k} boy={46} />
              <span class="uye-yazi">
                <b>{k.ad}{k.id === sosyal.ben?.id ? " (sen)" : ""}</b>
                <span class="sub small" class:oyun={!!k.c?.oyun}>{k.c?.oyun ? `${k.c.oyun} oynuyor` : k.c ? "Çevrim içi" : k.son ? `Son görülme ${once(k.son)}` : "Çevrim dışı"}</span>
              </span>
            </button>
            {#if k.id !== sosyal.ben?.id}
              {#if il === "arkadas"}
                <button class="btn ghost sm" onclick={() => sohbetAc(k)}><Icon name="chat" size={14} />Mesaj</button>
              {:else if il === "gelen"}
                <button class="btn sm" onclick={() => arkadasEkle(k).catch(hata)}><Icon name="check" size={14} />Kabul et</button>
              {:else if il === "giden"}
                <button class="btn ghost sm" title="İsteği geri al" onclick={() => arkadasSil(k).catch(hata)}>İstek gönderildi</button>
              {:else}
                <button class="btn ghost sm" onclick={() => arkadasEkle(k).catch(hata)}><Icon name="plus" size={14} />Arkadaş ekle</button>
              {/if}
            {/if}
          </div>
        {:else}
          <p class="sub">Kimse bulunamadı.</p>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .topluluk { display: flex; flex-direction: column; gap: 16px; max-width: 1000px; margin: 0 auto; }
  .hero { position: relative; overflow: hidden; display: flex; align-items: center; gap: 26px; padding: 30px 32px; border-radius: 22px; border: 1px solid rgba(227, 194, 112, 0.25); background: linear-gradient(135deg, rgba(42, 36, 22, 0.9), rgba(24, 26, 31, 0.92) 55%, rgba(30, 28, 52, 0.9)); }
  .parilti { position: absolute; inset: -40% auto auto -10%; width: 420px; height: 420px; border-radius: 50%; background: radial-gradient(circle, rgba(227, 194, 112, 0.22), transparent 65%); pointer-events: none; }
  .logo { position: relative; width: 132px; height: 132px; flex: none; filter: drop-shadow(0 10px 28px rgba(0, 0, 0, 0.5)); }
  .hero-yazi { position: relative; display: flex; flex-direction: column; gap: 8px; flex: 1; min-width: 0; }
  .ust-baslik { font-size: 11px; font-weight: 800; letter-spacing: 0.16em; text-transform: uppercase; color: var(--gold); }
  h2 { margin: 0; font-family: var(--f-display); font-size: 28px; }
  p { margin: 0; color: var(--muted); line-height: 1.55; max-width: 560px; }
  .hero-dugmeler { display: flex; gap: 10px; flex-wrap: wrap; margin-top: 6px; }
  .btn.discord { background: #5865f2; border-color: #5865f2; color: #fff; }
  .btn.discord:hover { background: #4752c4; }
  .sayilar { position: relative; display: flex; flex-direction: column; gap: 8px; flex: none; }
  .sayilar div { display: flex; align-items: baseline; gap: 8px; padding: 8px 14px; border-radius: 12px; background: rgba(0, 0, 0, 0.25); min-width: 130px; }
  .sayilar b { font-size: 22px; font-family: var(--f-display); }
  .sayilar span { color: var(--muted); font-size: 13px; }
  .yesil { color: var(--good); }
  .mor { color: #a99cff; }
  .bilgi ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 12px; }
  .bilgi li { display: flex; gap: 10px; align-items: flex-start; color: var(--muted); line-height: 1.45; }
  .bilgi li :global(svg) { color: var(--gold); flex: none; margin-top: 2px; }
  .bilgi b { color: var(--text); }
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .ara { background: var(--coal-2); border: 1px solid var(--line-2); border-radius: 999px; padding: 7px 14px; color: var(--text); font: inherit; font-size: 13px; width: 220px; outline: none; }
  .uyeler { display: grid; grid-template-columns: repeat(auto-fill, minmax(290px, 1fr)); gap: 10px; }
  .uye { display: flex; align-items: center; gap: 8px; padding: 10px 12px; border-radius: 14px; border: 1px solid var(--line); background: rgba(255, 255, 255, 0.025); }
  .uye:hover { border-color: var(--line-2); }
  .uye.kapali .uye-ana { opacity: 0.6; }
  .uye-ana { flex: 1; min-width: 0; display: flex; align-items: center; gap: 12px; border: 0; background: none; padding: 0; color: inherit; font: inherit; text-align: left; cursor: pointer; }
  .uye-yazi { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .uye-yazi b, .uye-yazi span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .oyun { color: #a99cff; }
  .sm { padding: 6px 11px; font-size: 12.5px; white-space: nowrap; }
  @media (max-width: 760px) {
    .hero { flex-direction: column; text-align: center; }
    .hero-dugmeler { justify-content: center; }
    .sayilar { flex-direction: row; }
  }
</style>
