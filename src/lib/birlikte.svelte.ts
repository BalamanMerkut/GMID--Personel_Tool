// Birlikte dinleme: oynatıcıda çalan şarkıyı arkadaşlarla aynı anda dinlemek.
//   Yayın (sahip): Oynatıcı'da "Arkadaşlarla dinle" açılınca her oynat/duraklat/sarma/parça değişikliği
//     sunucuya bildirilir (dinleme odası).
//   Bağlantıyla oda (indirmeden): Sosyal'de YouTube bağlantısı ya da şarkı adı yazılır; sahibin GMID'i de şarkıyı
//     kaynağından çalar (oynatıcıya ve indirmeye gerek yok), duraklat/sar/değiştir çubuktan yapılır.
//   Dinleyen: odaya katılınca şarkının kaynağından (YouTube vb.) kendi bilgisayarında ses alınır ve sahibin
//     konumuna eşitlenir (saat farkı sunucu saatiyle düzeltilir; 1,2 sn'den fazla kayınca sarılır).
import { api, inTauri } from "./api";
import { controls, player } from "./player.svelte";
import { istek, sosyal, wsEk, type Kisi } from "./social.svelte";
import { notify } from "./toast.svelte";
import type { Download } from "./types";

export interface Sarki {
  baslik: string;
  url: string;
  kapak: string | null;
  sure: number | null;
}

export interface Oda {
  sahip: Kisi;
  sarki: Sarki;
  /** `guncelleme` anındaki konum (sn). */
  konum: number;
  caliyor: boolean;
  /** Sunucu saatiyle (ms). */
  guncelleme: number;
  dinleyiciler: Kisi[];
}

export const birlikte = $state({
  /** Açık odalar (sahip → oda). */
  odalar: {} as Record<string, Oda>,
  /** Ben yayın yapıyorum. */
  yayin: false,
  /** Yayının kaynağı: oynatıcıdaki parça ya da bağlantıyla açılan oda (indirmeden). */
  kaynak: null as "oynatici" | "baglanti" | null,
  /** Bağlantılı odada çalan şarkı, konumu ve süresi (çubuk için). */
  simdiki: null as Sarki | null,
  hostCaliyor: false,
  hostKonum: 0,
  hostSure: 0,
  /** Bağlantı / şarkı adı çözülüyor. */
  aciliyor: false,
  /** Oda açılırken hangi aşamada (düğmede yazar). */
  asama: "",
  /** Dinlediğim odanın sahibi. */
  dinlenen: null as string | null,
  /** Ses kaynağı hazırlanıyor (yt-dlp). */
  hazirlaniyor: false,
  ses: 0.8,
});

/** Sunucu saati − yerel saat (ms). */
let fark = 0;
let ses: HTMLAudioElement | null = null;
/** Şu an yüklü olan şarkının kaynak sayfası. */
let yukluKaynak = "";

function saat(simdi: unknown) {
  if (typeof simdi === "number") fark = simdi - Date.now();
}

export function odaListesi(): Oda[] {
  return Object.values(birlikte.odalar).sort((a, b) => b.dinleyiciler.length - a.dinleyiciler.length);
}

export async function odalariYukle() {
  if (!sosyal.anahtar) return;
  try {
    const r = await istek<{ odalar: Oda[]; simdi: number }>("GET", "/api/dinle");
    saat(r.simdi);
    birlikte.odalar = Object.fromEntries(r.odalar.map((o) => [o.sahip.id, o]));
  } catch {
    /* sunucu henüz bilmiyor */
  }
}

// Sunucudan gelen oda değişiklikleri.
wsEk.add((v) => {
  if (v.tip !== "dinle") return;
  saat(v.simdi);
  if (typeof v.kapandi === "string") {
    const id = v.kapandi;
    const { [id]: _, ...kalan } = birlikte.odalar;
    birlikte.odalar = kalan;
    if (id === sosyal.ben?.id) {
      birlikte.yayin = false;
      if (birlikte.kaynak === "baglanti") hostDurdur();
      birlikte.kaynak = null;
    }
    if (birlikte.dinlenen === id) {
      durdur();
      birlikte.dinlenen = null;
      notify("Dinleme odası kapandı", "headphones");
    }
    return;
  }
  const oda = v.oda as Oda | undefined;
  if (!oda) return;
  birlikte.odalar = { ...birlikte.odalar, [oda.sahip.id]: oda };
  if (birlikte.dinlenen === oda.sahip.id) void eslestir();
});

// ---------- ses kaynağı ----------

/** Şimdiki blob adresleri (eskisi serbest bırakılsın). */
const bloblar: Record<"host" | "dinle", string | null> = { host: null, dinle: null };

/**
 * Şarkının sesi: GMID ses dosyasını geçici klasöre alır (yt-dlp), baytlar bir blob adresiyle çalınır. YouTube'un
 * doğrudan adresini uygulama içinde çalmak bazı bilgisayarlarda "desteklenen kaynak yok" hatası veriyordu.
 */
async function sesAdresi(url: string, kim: "host" | "dinle"): Promise<string> {
  const { invoke } = await import("@tauri-apps/api/core");
  const bayt = await invoke<ArrayBuffer>("social_stream_audio", { url });
  const tur = sesTuru(new Uint8Array(bayt.slice(0, 12)));
  const adres = URL.createObjectURL(new Blob([bayt], { type: tur }));
  const eski = bloblar[kim];
  bloblar[kim] = adres;
  // Eskisi çalan öğeden ayrıldıktan sonra bırakılır.
  if (eski) setTimeout(() => URL.revokeObjectURL(eski), 2000);
  return adres;
}

/** Dosyanın ilk baytlarından ses türü (m4a / webm / mp3 / ogg). */
function sesTuru(b: Uint8Array): string {
  if (b[4] === 0x66 && b[5] === 0x74 && b[6] === 0x79 && b[7] === 0x70) return "audio/mp4";
  if (b[0] === 0x1a && b[1] === 0x45 && b[2] === 0xdf && b[3] === 0xa3) return "audio/webm";
  if (b[0] === 0x4f && b[1] === 0x67 && b[2] === 0x67 && b[3] === 0x53) return "audio/ogg";
  return "audio/mpeg";
}

// ---------- yayın (sahip) ----------

let bekleyen: ReturnType<typeof setTimeout> | undefined;
let sonDurum: { item: Download; konum: number; caliyor: boolean } | null = null;

/** Oynatıcı her değişiklikte çağırır; yayın açıksa (kısa bir beklemeyle) sunucuya bildirir. */
export function yayinDurumu(item: Download | null, konum: number, caliyor: boolean) {
  // Bağlantıyla açılmış oda oynatıcıyı dinlemez.
  if (birlikte.kaynak === "baglanti") return;
  if (!item) {
    if (birlikte.yayin) void yayinBitir();
    return;
  }
  sonDurum = { item, konum, caliyor };
  if (!birlikte.yayin) return;
  clearTimeout(bekleyen);
  bekleyen = setTimeout(() => void gonder(), 250);
}

async function gonder() {
  if (!birlikte.yayin || !sonDurum) return;
  const { item, konum, caliyor } = sonDurum;
  try {
    await istek("POST", "/api/dinle", {
      baslik: item.title,
      url: item.url,
      kapak: item.thumbnail?.startsWith("https://") ? item.thumbnail : null,
      sure: item.duration,
      konum,
      caliyor,
    });
  } catch (e) {
    birlikte.yayin = false;
    notify(String(e instanceof Error ? e.message : e), "info");
  }
}

/** Parça paylaşılabilir mi (internetten indirilmiş, kaynak sayfası var)? */
export const paylasilabilir = (item: Download | null) => !!item && /^https?:\/\//.test(item.url);

export async function yayinBaslat() {
  if (!sosyal.anahtar) return notify("Önce Topluluk'tan Discord ile giriş yap", "info");
  if (!paylasilabilir(sonDurum?.item ?? player.item)) return notify("Bu parçanın internet kaynağı yok", "info");
  if (birlikte.dinlenen) await ayril();
  if (birlikte.kaynak === "baglanti") hostDurdur();
  birlikte.kaynak = "oynatici";
  birlikte.yayin = true;
  if (!sonDurum && player.item) sonDurum = { item: player.item, konum: 0, caliyor: player.playing };
  await gonder();
  if (birlikte.yayin) notify("Arkadaşların artık seninle dinleyebilir (Sosyal → Birlikte dinle)", "headphones");
}

export async function yayinBitir() {
  birlikte.yayin = false;
  if (birlikte.kaynak === "baglanti") hostDurdur();
  birlikte.kaynak = null;
  clearTimeout(bekleyen);
  await istek("DELETE", "/api/dinle").catch(() => {});
}

// ---------- dinleyen ----------

function durdur() {
  if (ses) {
    ses.pause();
    ses.removeAttribute("src");
    ses.load();
  }
  yukluKaynak = "";
}

export async function katil(sahip: string) {
  if (!inTauri) return notify("Birlikte dinleme yalnız GMID Tool'da çalışır", "info");
  if (birlikte.yayin) await yayinBitir();
  // Kendi oynatıcın çalıyorsa duraklasın (iki ses üst üste binmesin).
  if (player.playing) controls.toggle();
  try {
    const r = await istek<{ oda: Oda | null; simdi: number }>("POST", `/api/dinle/${sahip}/katil`);
    saat(r.simdi);
    if (r.oda) birlikte.odalar = { ...birlikte.odalar, [sahip]: r.oda };
    birlikte.dinlenen = sahip;
    await eslestir();
  } catch (e) {
    notify(String(e instanceof Error ? e.message : e), "info");
  }
}

export async function ayril() {
  const sahip = birlikte.dinlenen;
  birlikte.dinlenen = null;
  durdur();
  if (sahip) await istek("DELETE", `/api/dinle/${sahip}/katil`).catch(() => {});
}

export function sesAyarla(v: number) {
  birlikte.ses = Math.max(0, Math.min(1, v));
  if (ses) ses.volume = birlikte.ses;
  if (hostSes) hostSes.volume = birlikte.ses;
}

// ---------- bağlantıyla oda (indirmeden) ----------

let hostSes: HTMLAudioElement | null = null;
let hostBekleyen: ReturnType<typeof setTimeout> | undefined;
let nabiz: ReturnType<typeof setInterval> | undefined;

function hostDurdur() {
  clearInterval(nabiz);
  if (hostSes) {
    hostSes.pause();
    hostSes.removeAttribute("src");
    hostSes.load();
  }
  birlikte.simdiki = null;
  birlikte.hostCaliyor = false;
  birlikte.hostKonum = 0;
  birlikte.hostSure = 0;
}

function hostBildir() {
  clearTimeout(hostBekleyen);
  hostBekleyen = setTimeout(() => {
    const s = birlikte.simdiki;
    if (!birlikte.yayin || birlikte.kaynak !== "baglanti" || !s || !hostSes) return;
    void istek("POST", "/api/dinle", { ...s, konum: hostSes.currentTime, caliyor: !hostSes.paused }).catch((e) => {
      notify(String(e instanceof Error ? e.message : e), "info");
    });
  }, 200);
}

function hostOlustur(): HTMLAudioElement {
  if (hostSes) return hostSes;
  const a = new Audio();
  a.preload = "auto";
  a.addEventListener("play", () => ((birlikte.hostCaliyor = true), hostBildir()));
  a.addEventListener("pause", () => ((birlikte.hostCaliyor = false), hostBildir()));
  a.addEventListener("seeked", hostBildir);
  a.addEventListener("loadedmetadata", () => {
    if (isFinite(a.duration)) birlikte.hostSure = a.duration;
  });
  a.addEventListener("timeupdate", () => (birlikte.hostKonum = a.currentTime));
  a.addEventListener("ended", () => ((birlikte.hostCaliyor = false), hostBildir()));
  hostSes = a;
  return a;
}

/**
 * İndirmeden dinleme odası: YouTube bağlantısı ya da şarkı adı (ilk sonuç). Oda açıksa şarkı değişir; dinleyenler
 * yeni şarkıya kendiliğinden geçer.
 */
export async function baglantiOdasi(giris: string) {
  if (!inTauri) return notify("Birlikte dinleme yalnız GMID Tool'da çalışır", "info");
  if (!sosyal.anahtar) return notify("Önce Topluluk'tan Discord ile giriş yap", "info");
  if (!giris.trim() || birlikte.aciliyor) return;
  birlikte.aciliyor = true;
  try {
    // Sunucu birlikte dinlemeyi biliyor mu (0.12+)?
    birlikte.asama = "Sunucu…";
    try {
      await istek("GET", "/api/dinle");
    } catch (e) {
      const m = String(e instanceof Error ? e.message : e);
      throw new Error(
        m.includes("404") ? "GMID sunucusu henüz güncellenmemiş. Mola sohbetinde sunucu güncelleme komutunu çalıştır." : m,
      );
    }
    const { invoke } = await import("@tauri-apps/api/core");
    // yt-dlp yoksa bir kez kendiliğinden kurulur.
    const ytdlp = async <T,>(f: () => Promise<T>): Promise<T> => {
      try {
        return await f();
      } catch (e) {
        if (!String(e).includes("yt-dlp bulunamadı")) throw e;
        birlikte.asama = "Araçlar kuruluyor…";
        await api.videos.installTools();
        return await f();
      }
    };
    birlikte.asama = "Şarkı aranıyor…";
    const bilgi = await ytdlp(() =>
      invoke<{ url: string; title: string; duration: number | null; thumbnail: string | null }>("social_resolve", { query: giris }),
    );
    birlikte.asama = "Şarkı yükleniyor…";
    const kaynak = await ytdlp(() => sesAdresi(bilgi.url, "host"));
    if (birlikte.dinlenen) await ayril();
    // Oynatıcıdan yayın yapılıyorsa bağlantılı odaya geçilir; kendi oynatıcın da duraklar.
    if (player.playing) controls.toggle();
    const a = hostOlustur();
    a.volume = birlikte.ses;
    a.src = kaynak;
    birlikte.simdiki = { baslik: bilgi.title, url: bilgi.url, kapak: bilgi.thumbnail, sure: bilgi.duration };
    birlikte.hostSure = bilgi.duration ?? 0;
    birlikte.kaynak = "baglanti";
    birlikte.yayin = true;
    birlikte.asama = "Başlıyor…";
    try {
      await a.play();
    } catch (e) {
      hostDurdur();
      birlikte.yayin = false;
      birlikte.kaynak = null;
      throw new Error(`ses çalınamadı (${String(e instanceof Error ? e.message : e)})`);
    }
    // Oda sunucuda açılsın (hata varsa hemen söylensin).
    await istek("POST", "/api/dinle", { ...birlikte.simdiki, konum: a.currentTime, caliyor: !a.paused });
    // Yarım dakikada bir konum: geç katılan ya da bağlantısı kopan dinleyen kaymasın.
    clearInterval(nabiz);
    nabiz = setInterval(hostBildir, 30_000);
    notify(`Oda açıldı: ${bilgi.title}`, "headphones");
  } catch (e) {
    notify(`Oda açılamadı: ${String(e instanceof Error ? e.message : e)}`, "info");
  } finally {
    birlikte.aciliyor = false;
    birlikte.asama = "";
  }
}

export function hostOynat() {
  if (!hostSes) return;
  if (hostSes.paused) void hostSes.play().catch(() => {});
  else hostSes.pause();
}

export function hostSar(sn: number) {
  if (hostSes && isFinite(sn)) hostSes.currentTime = Math.max(0, sn);
}

/** Dinlediğim odanın şarkısını ve konumunu yakalar. */
async function eslestir() {
  const o = birlikte.dinlenen ? birlikte.odalar[birlikte.dinlenen] : null;
  if (!o) return;
  if (!ses) {
    ses = new Audio();
    ses.preload = "auto";
  }
  ses.volume = birlikte.ses;
  if (o.sarki.url !== yukluKaynak) {
    birlikte.hazirlaniyor = true;
    try {
      // Dinleyende de yt-dlp yoksa bir kez kendiliğinden kurulur.
      const kaynak = await sesAdresi(o.sarki.url, "dinle").catch(async (e) => {
        if (!String(e).includes("yt-dlp bulunamadı")) throw e;
        notify("Birlikte dinleme için araçlar kuruluyor (bir kez)…", "download");
        await api.videos.installTools();
        return sesAdresi(o.sarki.url, "dinle");
      });
      // Hazırlanırken oda değiştiyse ya da ayrıldıysa bırak.
      if (birlikte.dinlenen !== o.sahip.id) return;
      ses.src = kaynak;
      yukluKaynak = o.sarki.url;
      await new Promise<void>((ok) => {
        const bitti = () => ok();
        ses!.addEventListener("loadedmetadata", bitti, { once: true });
        ses!.addEventListener("error", bitti, { once: true });
        setTimeout(bitti, 8000);
      });
    } catch (e) {
      notify(`Şarkı açılamadı: ${String(e instanceof Error ? e.message : e)}`, "info");
      return;
    } finally {
      birlikte.hazirlaniyor = false;
    }
  }
  // Hazırlanırken gelen son durum geçerli.
  const son = birlikte.dinlenen ? birlikte.odalar[birlikte.dinlenen] : null;
  if (!son || !ses) return;
  const gecen = son.caliyor ? (Date.now() + fark - son.guncelleme) / 1000 : 0;
  const hedef = son.konum + Math.max(0, gecen);
  if (Math.abs(ses.currentTime - hedef) > 1.2) ses.currentTime = hedef;
  if (son.caliyor) await ses.play().catch(() => {});
  else ses.pause();
}
