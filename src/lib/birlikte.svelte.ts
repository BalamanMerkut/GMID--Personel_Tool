// Birlikte dinleme: oynatıcıda çalan şarkıyı arkadaşlarla aynı anda dinlemek.
//   Yayın (sahip): Oynatıcı'da "Arkadaşlarla dinle" açılınca her oynat/duraklat/sarma/parça değişikliği
//     sunucuya bildirilir (dinleme odası).
//   Dinleyen: odaya katılınca şarkının kaynağından (YouTube vb.) kendi bilgisayarında ses alınır ve sahibin
//     konumuna eşitlenir (saat farkı sunucu saatiyle düzeltilir; 1,2 sn'den fazla kayınca sarılır).
import { inTauri } from "./api";
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
  /** Ben yayın yapıyorum (oynatıcıdaki şarkı paylaşılıyor). */
  yayin: false,
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
    if (id === sosyal.ben?.id) birlikte.yayin = false;
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

// ---------- yayın (sahip) ----------

let bekleyen: ReturnType<typeof setTimeout> | undefined;
let sonDurum: { item: Download; konum: number; caliyor: boolean } | null = null;

/** Oynatıcı her değişiklikte çağırır; yayın açıksa (kısa bir beklemeyle) sunucuya bildirir. */
export function yayinDurumu(item: Download | null, konum: number, caliyor: boolean) {
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
  birlikte.yayin = true;
  if (!sonDurum && player.item) sonDurum = { item: player.item, konum: 0, caliyor: player.playing };
  await gonder();
  if (birlikte.yayin) notify("Arkadaşların artık seninle dinleyebilir (Sosyal → Birlikte dinle)", "headphones");
}

export async function yayinBitir() {
  birlikte.yayin = false;
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
      const { invoke } = await import("@tauri-apps/api/core");
      const kaynak = await invoke<string>("social_stream", { url: o.sarki.url });
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
