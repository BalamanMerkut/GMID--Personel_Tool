// Dinleme odaları (2.0): önce oda açılır (ad, kişi sınırı, kimler görebilir, kim şarkı ekleyebilir), sonra şarkılar
// ve videolar sıraya eklenir. Oda sunucuda durur; herkes kendi bilgisayarında aynı şarkıyı aynı saniyeden çalar.
//   Sahip ve DJ'ler: oynat/duraklat, sar, atla. Sahip: ayarlar, DJ yap, odadan çıkar, odayı kapat.
//   Ekleme yetkisi odaya göre: yalnız sahip / sahip + DJ'ler / herkes.
// Ses (ya da 720p görüntü) GMID'in kendi aracıyla (yt-dlp) geçici klasöre alınır ve oradan çalınır: kütüphaneye
// eklenmez, doğrudan YouTube adresini çalmaktaki "desteklenen kaynak yok" hatası olmaz.
import { api, inTauri } from "./api";
import { controls, nav, player } from "./player.svelte";
import { istek, sosyal, wsEk, type Kisi } from "./social.svelte";
import { notify } from "./toast.svelte";
import type { Download } from "./types";

export interface Sarki {
  baslik: string;
  url: string;
  kapak: string | null;
  sure: number | null;
  /** Birlikte izleme: görüntüsüyle. */
  video: boolean;
  /** Eklentiden gelen videoda videonun bulunduğu sayfa (site ister). */
  sayfa?: string | null;
}

export interface Oge {
  id: number;
  sarki: Sarki;
  ekleyen: Kisi;
}

export type Gizlilik = "herkes" | "arkadaslar";
export type Yetki = "sahip" | "dj" | "herkes";

export type Tekrar = "yok" | "sarki" | "liste";
/** Müzik odası (yalnız ses) ya da izleme odası (görüntülü). */
export type OdaTuru = "muzik" | "video";

export interface Oda {
  id: string;
  /** Eski sunucuda yok (müzik sayılır). */
  tur?: OdaTuru;
  ad: string;
  sahip: Kisi;
  gizlilik: Gizlilik;
  sinir: number | null;
  yetki: Yetki;
  djler: string[];
  uyeler: Kisi[];
  simdiki: Oge | null;
  sira: Oge[];
  /** `guncelleme` anındaki konum (sn). */
  konum: number;
  caliyor: boolean;
  /** Sunucu saatiyle (ms). */
  guncelleme: number;
  /** Eski sunucuda yok. */
  tekrar?: Tekrar;
  karisik?: boolean;
}

const BASLANGIC_SESI = 0.1;

export const birlikte = $state({
  /** Görebildiğim odalar (kimlik → oda). */
  odalar: {} as Record<string, Oda>,
  /** İçinde bulunduğum oda. */
  odaId: null as string | null,
  /** Oda paneli açık. */
  panel: false,
  /** Sosyal'de "Birlikte izle" sekmesi seçili. */
  izle: false,
  /** İzleme salonu ekranda (medya öğesi orada; yüzen panel ve çubuk gizlenir). */
  salonAcik: false,
  /** Şarkı hazırlanıyor (yt-dlp). */
  hazirlaniyor: false,
  /** Hazırlanan dosyanın indirilen yüzdesi (bilinmiyorsa 0). */
  yuzde: 0,
  /** Şarkı ekleniyor: hangi aşamada (düğmede yazar). */
  asama: "",
  /** Odaya girince %10 sesle başlar (yüksek başlamasın); odadayken değiştirilen ses odadan çıkana kadar kalır. */
  ses: BASLANGIC_SESI,
  /** Yerelde çalan konum ve süre (çubuk için). */
  konum: 0,
  sure: 0,
});

/** Sunucu saati − yerel saat (ms). */
let fark = 0;
let medya: HTMLVideoElement | null = null;
/** Yüklü olan sıra öğesi. */
let yukluOge: number | null = null;
let blob: string | null = null;

const ben = () => sosyal.ben?.id ?? "";
const hataYaz = (e: unknown) => String(e instanceof Error ? e.message : e);

export function benimOdam(): Oda | null {
  return birlikte.odaId ? (birlikte.odalar[birlikte.odaId] ?? null) : null;
}
export const yonetir = (o: Oda, id = ben()) => o.sahip.id === id || o.djler.includes(id);
export const ekleyebilir = (o: Oda, id = ben()) =>
  o.yetki === "sahip" ? o.sahip.id === id : o.yetki === "dj" ? yonetir(o, id) : o.uyeler.some((u) => u.id === id);

export const odaTuru = (o: Oda): OdaTuru => o.tur ?? "muzik";

export function odaListesi(tur?: OdaTuru): Oda[] {
  return Object.values(birlikte.odalar)
    .filter((o) => !tur || odaTuru(o) === tur)
    .sort((a, b) => b.uyeler.length - a.uyeler.length);
}

/** Sosyal → Birlikte izle sekmesini açar (izleme odası orada büyük oynar). */
export function izleAc() {
  birlikte.izle = true;
  birlikte.panel = false;
  nav.go("feed");
}

/** Odanın penceresini aç: izleme odası büyük salonda, müzik odası yüzen panelde. */
export function odayiGoster() {
  const o = benimOdam();
  if (o && odaTuru(o) === "video") izleAc();
  else birlikte.panel = true;
}

function saat(simdi: unknown) {
  if (typeof simdi === "number") fark = simdi - Date.now();
}

function odaYaz(o: Oda) {
  birlikte.odalar = { ...birlikte.odalar, [o.id]: o };
}

export async function odalariYukle() {
  if (!sosyal.anahtar) return;
  try {
    const r = await istek<{ odalar: Oda[]; simdi: number }>("GET", "/api/odalar");
    saat(r.simdi);
    birlikte.odalar = Object.fromEntries(r.odalar.map((o) => [o.id, o]));
    // Bağlantı koptuysa sunucu odadan çıkarmış olabilir.
    const o = benimOdam();
    if (birlikte.odaId && (!o || !o.uyeler.some((u) => u.id === ben()))) cikildi();
    else if (o) void eslestir();
  } catch {
    /* sunucu henüz bilmiyor */
  }
}

// Sunucudan gelen oda değişiklikleri.
wsEk.add((v) => {
  if (v.tip !== "oda") return;
  saat(v.simdi);
  if (typeof v.kapandi === "string") {
    const { [v.kapandi]: _, ...kalan } = birlikte.odalar;
    birlikte.odalar = kalan;
    if (birlikte.odaId === v.kapandi) {
      cikildi();
      notify("Dinleme odası kapandı", "headphones");
    }
    return;
  }
  if (typeof v.atildi === "string" && birlikte.odaId === v.atildi) {
    cikildi();
    notify(`"${String(v.ad ?? "Oda")}" odasından çıkarıldın`, "info");
    return;
  }
  const o = v.oda as Oda | undefined;
  if (!o) return;
  odaYaz(o);
  if (birlikte.odaId === o.id) void eslestir();
});

/** Şu an hazırlanan bağlantı (ilerleme olayları buna göre süzülür). */
let hazirlananUrl: string | null = null;
if (inTauri) {
  void import("@tauri-apps/api/event").then(({ listen }) =>
    listen<{ url: string; yuzde: number }>("oda-hazirlik", (e) => {
      if (e.payload.url === hazirlananUrl) birlikte.yuzde = Math.min(100, Math.max(0, e.payload.yuzde));
    }),
  );
}

// Eklentiden "GMID odasında birlikte izle": içinde olduğun odaya eklenir; oda yoksa izleme odası açılır.
if (inTauri) {
  void import("@tauri-apps/api/event").then(({ listen }) =>
    listen<{ url: string; baslik: string | null; sayfa: string | null }>("oda-ekle", async (e) => {
      if (!sosyal.anahtar) return notify("Önce Topluluk'tan Discord ile giriş yap", "info");
      if (!birlikte.odaId) await odaAc({ tur: "video" });
      const id = birlikte.odaId;
      if (!id) return;
      const o = benimOdam();
      try {
        await sirayaYaz(id, {
          baslik: e.payload.baslik?.trim() || "Tarayıcıdan video",
          url: e.payload.url,
          kapak: null,
          sure: null,
          video: !o || odaTuru(o) === "video",
          sayfa: e.payload.sayfa,
        });
        notify("Video odaya eklendi", "film");
        odayiGoster();
      } catch (err) {
        notify(hataYaz(err), "info");
      }
    }),
  );
}

// ---------------- oda ----------------

export interface OdaAyar {
  tur?: OdaTuru;
  ad?: string;
  gizlilik?: Gizlilik;
  /** 0: sınırsız. */
  sinir?: number;
  yetki?: Yetki;
}

export async function odaAc(a: OdaAyar = {}) {
  if (!inTauri) return notify("Dinleme odaları yalnız GMID Tool'da çalışır", "info");
  if (!sosyal.anahtar) return notify("Önce Topluluk'tan Discord ile giriş yap", "info");
  try {
    const r = await istek<{ oda: Oda; simdi: number }>("POST", "/api/oda", a);
    saat(r.simdi);
    odaYaz(r.oda);
    sesAyarla(BASLANGIC_SESI);
    birlikte.odaId = r.oda.id;
    odayiGoster();
  } catch (e) {
    const m = hataYaz(e);
    notify(m.includes("404") ? "GMID sunucusu henüz güncellenmemiş (Mola sohbetinde sunucu güncellemesi)" : m, "info");
  }
}

export async function ayarla(d: OdaAyar & { dj?: { kisi: string; ver: boolean }; tekrar?: Tekrar; karisik?: boolean }) {
  const id = birlikte.odaId;
  if (!id) return;
  await istek<{ oda: Oda }>("PATCH", `/api/oda/${id}`, d)
    .then((r) => odaYaz(r.oda))
    .catch((e) => notify(hataYaz(e), "info"));
}

export async function katil(id: string) {
  if (!inTauri) return notify("Dinleme odaları yalnız GMID Tool'da çalışır", "info");
  if (birlikte.odaId === id) {
    odayiGoster();
    return;
  }
  try {
    const r = await istek<{ oda: Oda; simdi: number }>("POST", `/api/oda/${id}/katil`);
    saat(r.simdi);
    durdur();
    odaYaz(r.oda);
    sesAyarla(BASLANGIC_SESI);
    birlikte.odaId = id;
    odayiGoster();
    // Kendi oynatıcın çalıyorsa duraklasın (iki ses üst üste binmesin).
    if (player.playing) controls.toggle();
    await eslestir();
  } catch (e) {
    notify(hataYaz(e), "info");
  }
}

function cikildi() {
  durdur();
  birlikte.odaId = null;
  birlikte.panel = false;
}

export async function ayril() {
  const id = birlikte.odaId;
  cikildi();
  if (id) await istek("DELETE", `/api/oda/${id}/katil`).catch(() => {});
}

export async function odaKapat() {
  const id = birlikte.odaId;
  if (!id) return;
  cikildi();
  await istek("DELETE", `/api/oda/${id}`).catch((e) => notify(hataYaz(e), "info"));
}

export const at = (kisi: string) => birlikte.odaId && komut("POST", `/api/oda/${birlikte.odaId}/at/${kisi}`);
export const djYap = (kisi: string, ver: boolean) => ayarla({ dj: { kisi, ver } });

/** Tekrar: yok → şarkı → liste → yok (sahip/DJ). */
export function tekrarDegistir() {
  const o = benimOdam();
  if (!o) return;
  const sonraki: Record<Tekrar, Tekrar> = { yok: "sarki", sarki: "liste", liste: "yok" };
  void ayarla({ tekrar: sonraki[o.tekrar ?? "yok"] });
}
export function karistirDegistir() {
  const o = benimOdam();
  if (o) void ayarla({ karisik: !o.karisik });
}

// ---------------- sıra ----------------

/** yt-dlp yoksa bir kez kendiliğinden kurulur. */
async function aracla<T>(f: () => Promise<T>): Promise<T> {
  try {
    return await f();
  } catch (e) {
    if (!String(e).includes("yt-dlp bulunamadı")) throw e;
    birlikte.asama = "Araçlar kuruluyor…";
    notify("Birlikte dinleme için araçlar kuruluyor (bir kez)…", "download");
    await api.videos.installTools();
    return await f();
  }
}

/** YouTube / SoundCloud çalma listesi bağlantısı mı? */
const listeMi = (g: string) => /^https?:\/\//.test(g) && /[?&]list=|\/playlist\b|\/sets\//.test(g);

/** Şarkı adı ya da bağlantı → sıraya. Çalma listesi bağlantısıysa listedeki her şey sıraya girer.
 *  `video`: birlikte izleme. Oda yoksa önce oda açılır. */
export async function ekle(giris: string, video = false) {
  giris = giris.trim();
  if (!giris || birlikte.asama) return;
  if (!birlikte.odaId) await odaAc();
  const id = birlikte.odaId;
  if (!id) return;
  // Sunucu da odanın türüne göre düzeltir; eski sunucu için burada da.
  const o = benimOdam();
  if (o?.tur) video = o.tur === "video";
  try {
    if (listeMi(giris)) {
      birlikte.asama = "Liste açılıyor…";
      const p = await aracla(() => api.videos.probe(giris, true));
      if (p.kind === "playlist") {
        await topluEkle(
          id,
          p.entries.map((e) => ({ baslik: e.title, url: e.url, kapak: e.thumbnail?.startsWith("https://") ? e.thumbnail : null, sure: e.duration, video })),
          p.title,
        );
        return;
      }
    }
    birlikte.asama = "Aranıyor…";
    const { invoke } = await import("@tauri-apps/api/core");
    const b = await aracla(() =>
      invoke<{ url: string; title: string; duration: number | null; thumbnail: string | null }>("social_resolve", { query: giris }),
    );
    birlikte.asama = "Ekleniyor…";
    await sirayaYaz(id, { baslik: b.title, url: b.url, kapak: b.thumbnail, sure: b.duration, video });
  } catch (e) {
    notify(`Eklenemedi: ${hataYaz(e)}`, "info");
  } finally {
    birlikte.asama = "";
  }
}

const indirilenSarki = (item: Download): Sarki => ({
  baslik: item.title,
  url: item.url,
  kapak: item.thumbnail?.startsWith("https://") ? item.thumbnail : null,
  sure: item.duration,
  video: !item.audioOnly,
});

/** Medya'daki indirilmiş parçayı odaya ekler (oda yoksa açar). */
export async function ekleIndirilen(item: Download) {
  if (!/^https?:\/\//.test(item.url)) return notify("Bu parçanın internet kaynağı yok", "info");
  if (!birlikte.odaId) await odaAc();
  const id = birlikte.odaId;
  if (!id) return;
  try {
    await sirayaYaz(id, indirilenSarki(item));
    notify(`Odaya eklendi: ${item.title}`, "headphones");
  } catch (e) {
    notify(hataYaz(e), "info");
  }
}

/** Medya'daki bir listenin tamamını (internet kaynağı olanları) odanın sırasına ekler. */
export async function ekleListe(items: Download[], ad: string) {
  const uygun = items.filter((d) => /^https?:\/\//.test(d.url));
  if (!uygun.length) return notify("Bu listedeki parçaların internet kaynağı yok", "info");
  if (birlikte.asama) return;
  if (!birlikte.odaId) await odaAc();
  const id = birlikte.odaId;
  if (!id) return;
  try {
    await topluEkle(id, uygun.map(indirilenSarki), ad);
  } finally {
    birlikte.asama = "";
  }
}

/** Sıradaki yer kadarını sırayla ekler; sıra dolunca durur. */
async function topluEkle(id: string, sarkilar: Sarki[], ad: string) {
  const bos = Math.max(0, 50 - (benimOdam()?.sira.length ?? 0) + (benimOdam()?.simdiki ? 0 : 1));
  const eklenecek = sarkilar.filter((s) => s.url).slice(0, bos);
  if (!eklenecek.length) return notify("Sıra dolu (en çok 50 şarkı)", "info");
  let n = 0;
  for (const s of eklenecek) {
    birlikte.asama = `Ekleniyor ${n + 1}/${eklenecek.length}…`;
    try {
      const r = await istek<{ oda: Oda; simdi: number }>("POST", `/api/oda/${id}/sira`, s);
      saat(r.simdi);
      odaYaz(r.oda);
      n++;
    } catch (e) {
      if (!n) throw e;
      break;
    }
  }
  void eslestir();
  const kalan = sarkilar.length - n;
  notify(`"${ad}" listesinden ${n} parça odaya eklendi${kalan > 0 ? ` (${kalan} tanesi sığmadı; sıra en çok 50)` : ""}`, "headphones");
}

async function sirayaYaz(id: string, s: Sarki) {
  const r = await istek<{ oda: Oda; simdi: number }>("POST", `/api/oda/${id}/sira`, s);
  saat(r.simdi);
  odaYaz(r.oda);
  // Hazırlık (indirme) arkada sürer; ekleme kutusu hemen boşalır.
  void eslestir();
}

export const siradanCikar = (oge: number) => birlikte.odaId && komut("DELETE", `/api/oda/${birlikte.odaId}/sira/${oge}`);

// ---------------- oynatma ----------------

/** Oda komutlarının yanıtı (güncel oda): canlı bağlantının yankısını beklemeden hemen uygulanır. */
function odaYaniti(r: unknown) {
  const v = r as { oda?: Oda; simdi?: number } | null;
  if (!v?.oda) return;
  saat(v.simdi);
  odaYaz(v.oda);
  if (birlikte.odaId === v.oda.id) void eslestir();
}

/** Oda komutu: yanıtı uygular, hatayı gösterir. */
async function komut(yontem: string, yol: string, govde?: unknown) {
  await istek(yontem, yol, govde)
    .then(odaYaniti)
    .catch((e) => notify(hataYaz(e), "info"));
}

async function oynatBildir(konum: number, caliyor: boolean) {
  const id = birlikte.odaId;
  if (!id) return;
  await komut("POST", `/api/oda/${id}/oynat`, { konum, caliyor });
}

export function oynatDuraklat() {
  const o = benimOdam();
  if (!o?.simdiki || !medya) return;
  const cal = !o.caliyor;
  // Beklemeden: düğmeye basınca hemen dursun / başlasın (sunucu yanıtı sonra herkesi eşitler).
  if (cal) void medya.play().catch(() => {});
  else medya.pause();
  odaYaz({ ...o, caliyor: cal, konum: medya.currentTime, guncelleme: Date.now() + fark });
  void oynatBildir(medya.currentTime, cal);
}

export function sar(sn: number) {
  const o = benimOdam();
  if (!o?.simdiki || !isFinite(sn)) return;
  void oynatBildir(Math.max(0, sn), o.caliyor);
}

export async function atla() {
  const id = birlikte.odaId;
  if (!id) return;
  await komut("POST", `/api/oda/${id}/sonraki`, { oge: benimOdam()?.simdiki?.id ?? null });
}

/** Sıradaki bir şarkıyı hemen çal (sahip/DJ; listede tıklanınca). */
export async function simdiCal(oge: number) {
  const id = birlikte.odaId;
  if (!id) return;
  await komut("POST", `/api/oda/${id}/sonraki`, { sec: oge });
}

export function sesAyarla(v: number) {
  birlikte.ses = Math.max(0, Math.min(1, v));
  if (medya) medya.volume = birlikte.ses;
}

/** Panel kapalıyken medya öğesinin durduğu gizli kutu. Öğe sayfadan çıkarılırsa tarayıcı onu duraklatır;
 *  bu yüzden panel kapanınca silinmez, buraya taşınır (aynı anda taşındığı için çalmaya devam eder). */
export function medyaYedekKutusu(): HTMLElement {
  let k = document.getElementById("oda-medya-yedek");
  if (!k) {
    k = document.createElement("div");
    k.id = "oda-medya-yedek";
    k.setAttribute("aria-hidden", "true");
    k.style.cssText = "position:fixed;width:1px;height:1px;left:-10px;top:-10px;overflow:hidden;opacity:0;pointer-events:none";
    document.body.appendChild(k);
  }
  return k;
}

/** Oda panelinin göstereceği öğe (videoda görüntü, şarkıda yalnız ses; panel kapalıyken de çalar). */
export function medyaElemani(): HTMLVideoElement {
  if (medya) return medya;
  const m = document.createElement("video");
  m.preload = "auto";
  m.playsInline = true;
  m.volume = birlikte.ses;
  m.addEventListener("timeupdate", () => (birlikte.konum = m.currentTime));
  m.addEventListener("loadedmetadata", () => {
    if (isFinite(m.duration)) birlikte.sure = m.duration;
  });
  // Şarkı bitti: sıradakine geçilsin (sunucu iki kez geçirmez).
  m.addEventListener("ended", () => {
    const id = birlikte.odaId;
    if (id && yukluOge != null) void istek("POST", `/api/oda/${id}/sonraki`, { oge: yukluOge, bitti: true }).catch(() => {});
  });
  medya = m;
  medyaYedekKutusu().appendChild(m);
  return m;
}

function durdur() {
  if (medya) {
    medya.pause();
    medya.removeAttribute("src");
    medya.load();
  }
  if (blob) setTimeout((u: string) => URL.revokeObjectURL(u), 2000, blob);
  blob = null;
  yukluOge = null;
  birlikte.konum = 0;
  birlikte.sure = 0;
  birlikte.hazirlaniyor = false;
}


let yukleniyor: number | null = null;

/** Odadaki şarkıyı ve konumu yakalar. */
async function eslestir() {
  const o = benimOdam();
  if (!o) return;
  const m = medyaElemani();
  m.volume = birlikte.ses;
  const simdiki = o.simdiki;
  if (!simdiki) {
    if (yukluOge != null) durdur();
    return;
  }
  if (simdiki.id !== yukluOge) {
    if (yukleniyor === simdiki.id) return;
    yukleniyor = simdiki.id;
    birlikte.hazirlaniyor = true;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      birlikte.yuzde = 0;
      hazirlananUrl = simdiki.sarki.url;
      // Dosya geçici klasöre iner, oynatıcı doğrudan dosyadan açar (büyük video belleğe okunmaz).
      const yol = await aracla(() =>
        invoke<string>("social_stream_file", { url: simdiki.sarki.url, video: simdiki.sarki.video, sayfa: simdiki.sarki.sayfa ?? null }),
      );
      // Hazırlanırken şarkı değiştiyse ya da odadan çıkıldıysa bırak.
      if (benimOdam()?.simdiki?.id !== simdiki.id) return;
      if (blob) setTimeout((u: string) => URL.revokeObjectURL(u), 2000, blob);
      blob = null;
      m.src = api.fileSrc(yol);
      yukluOge = simdiki.id;
      await new Promise<void>((ok) => {
        const bitti = () => ok();
        m.addEventListener("loadedmetadata", bitti, { once: true });
        m.addEventListener("error", bitti, { once: true });
        setTimeout(bitti, 8000);
      });
    } catch (e) {
      notify(`Açılamadı: ${hataYaz(e)}`, "info");
      return;
    } finally {
      if (yukleniyor === simdiki.id) yukleniyor = null;
      birlikte.hazirlaniyor = false;
    }
  }
  // Hazırlanırken gelen son durum geçerli.
  const son = benimOdam();
  if (!son?.simdiki || son.simdiki.id !== yukluOge) return;
  const gecen = son.caliyor ? (Date.now() + fark - son.guncelleme) / 1000 : 0;
  const hedef = son.konum + Math.max(0, gecen);
  if (Math.abs(m.currentTime - hedef) > 1.2) m.currentTime = hedef;
  // Kendi oynatıcın çalıyorsa duraklasın (iki ses üst üste binmesin).
  if (son.caliyor && player.playing) controls.toggle();
  if (son.caliyor) await m.play().catch((e) => notify(`Çalınamadı: ${hataYaz(e)}`, "info"));
  else m.pause();
}
