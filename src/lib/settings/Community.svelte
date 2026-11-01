<script lang="ts">
  // Kontrol Odası → Topluluk: hesap (Discord ile giriş/çıkış) ve neyin paylaşılacağı.
  import Icon from "../Icon.svelte";
  import Switch from "../components/Switch.svelte";
  import Avatar from "../components/social/Avatar.svelte";
  import { nav } from "../player.svelte";
  import { cikis, esitleSimdi, girisYap, oyunHaberiDegistir, paylasimDegistir, profilGoster, sosyal } from "../social.svelte";
  import { notify } from "../toast.svelte";

  let esitleniyor = $state(false);
  const hata = (e: unknown) => notify(String(e instanceof Error ? e.message : e), "info");

  async function simdiEsitle() {
    esitleniyor = true;
    try {
      await esitleSimdi();
      notify("Profilin güncellendi", "check");
    } catch (e) {
      hata(e);
    } finally {
      esitleniyor = false;
    }
  }
</script>

<section class="panel">
  <h3 class="h2"><Icon name="users" />Hesap</h3>
  {#if sosyal.anahtar && sosyal.ben}
    <div class="hesap">
      <Avatar kisi={sosyal.ben} boy={52} />
      <div class="hesap-yazi"><b>{sosyal.ben.ad}</b><span class="sub small">Discord ile giriş yapıldı · {sosyal.bagli ? "sunucuya bağlı" : "bağlanıyor…"}</span></div>
      <button class="btn ghost sm" onclick={() => profilGoster(null)}>Profilimi gör</button>
      <button class="btn ghost sm" onclick={() => cikis()}>Çıkış yap</button>
    </div>
  {:else}
    <div class="hesap">
      <span class="sub">GMID topluluğuna Discord hesabınla girersin; şifre yok.</span>
      <button class="btn" disabled={sosyal.giriliyor} onclick={() => girisYap().catch(hata)}>
        <Icon name="discord" size={17} />{sosyal.giriliyor ? "Tarayıcıda onay bekleniyor…" : "Discord ile giriş yap"}
      </button>
      <button class="btn ghost sm" onclick={() => nav.go("social")}>Topluluk hakkında</button>
    </div>
  {/if}
</section>

<section class="panel">
  <h3 class="h2"><Icon name="shield" />Ne paylaşılsın?</h3>
  <p class="sub small">Kapalı olanlar sunucuya hiç gönderilmez; kapatınca profilinden de kalkar. Notların, ayarların ve dosyaların hiçbir zaman paylaşılmaz.</p>
  <div class="line">
    <div><b>Oyun kütüphanem</b><span class="sub small">Oyunların ve ne kadar oynadığın (Kütüphanem'de gizlediklerin hariç); profilde "En çok oynananlar" ve "Ortak oyunlar"</span></div>
    <Switch label="Oyun kütüphanem" checked={sosyal.paylas.oyunlar} onchange={(v) => paylasimDegistir({ oyunlar: v })} />
  </div>
  <div class="line">
    <div><b>Oyun puanlarım</b><span class="sub small">Puan, tek cümlelik yorum ve durum (oynuyor, bitirdi…); yeni puanlar akışa düşer</span></div>
    <Switch label="Oyun puanlarım" checked={sosyal.paylas.oyunPuan} onchange={(v) => paylasimDegistir({ oyunPuan: v })} />
  </div>
  <div class="line">
    <div><b>Film ve dizi puanlarım</b><span class="sub small">Film & Dizi Arşivi'ndeki puanların ve yorumların; profilde film ve dizi vitrini</span></div>
    <Switch label="Film ve dizi puanlarım" checked={sosyal.paylas.filmPuan} onchange={(v) => paylasimDegistir({ filmPuan: v })} />
  </div>
  <div class="line">
    <div><b>Şu an oynadığım oyun</b><span class="sub small">Arkadaş listesinde "CS2 oynuyor"; oyuna girince akışa düşer</span></div>
    <Switch label="Şu an oynadığım oyun" checked={sosyal.paylas.simdi} onchange={(v) => paylasimDegistir({ simdi: v })} />
  </div>
  <div class="row">
    <button class="btn ghost" disabled={esitleniyor || !sosyal.anahtar} onclick={simdiEsitle}><Icon name="refresh" size={16} />{esitleniyor ? "Güncelleniyor…" : "Profilimi şimdi güncelle"}</button>
    <span class="sub small">Puan verdiğinde ve oyun listesi değiştiğinde kendiliğinden güncellenir.</span>
  </div>
</section>

<section class="panel">
  <h3 class="h2"><Icon name="bell" />Bildirimler</h3>
  <div class="line">
    <div><b>Arkadaşım oyuna girince haber ver</b><span class="sub small">Sağ altta "Ali CS2 oynamaya başladı"</span></div>
    <Switch label="Arkadaşım oyuna girince haber ver" checked={sosyal.oyunHaberi} onchange={(v) => oyunHaberiDegistir(v)} />
  </div>
</section>

<style>
  .panel { display: flex; flex-direction: column; gap: 12px; }
  .hesap { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .hesap-yazi { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .line { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 10px 0; border-top: 1px solid var(--line); }
  .line > div { display: flex; flex-direction: column; gap: 2px; }
  .row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .sm { padding: 6px 11px; font-size: 13px; }
</style>
