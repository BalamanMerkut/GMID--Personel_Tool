// Topluluk: GMID sunucusuyla bağlantı. Discord ile giriş, kim çevrim içi / ne oynuyor, profil eşitleme (yalnız
// kullanıcının paylaşmayı seçtikleri), akış, özel mesajlar. GMID Tool bunlar olmadan da tam çalışır.
import { api, inTauri } from "./api";
import { on } from "./bus";
import { getJson, setJson } from "./settings.svelte";
import { notify } from "./toast.svelte";
import { nav } from "./player.svelte";
import type { Game, Review, Title } from "./types";

export const VARSAYILAN_SUNUCU = "https://gmid.molayazilim.com.tr";

export interface Kisi {
  id: string;
  ad: string;
  avatar: string | null;
  cevrimici?: boolean;
  oyun?: string | null;
  son?: number;
  /** Benimle ilişkisi. */
  iliski?: "arkadas" | "giden" | "gelen" | null;
}

export interface Paylasim {
  oyunlar: boolean;
  oyunPuan: boolean;
  filmPuan: boolean;
  simdi: boolean;
}

export interface Puan {
  anahtar: string;
  tur: "game" | "movie" | "series";
  ad: string;
  kapak: string | null;
  puan: number | null;
  yorum: string | null;
  durum: string | null;
  yil: number | null;
  guncelleme: number;
}

export interface ProfilOyun {
  ad: string;
  kapak: string | null;
  dakika: number;
  son: number | null;
}

export interface Profil {
  oyunlar: ProfilOyun[];
  puanlar: Puan[];
}

export interface Yorum {
  id: number;
  kisi: Kisi;
  metin: string;
  zaman: number;
}

export interface Olay {
  id: number;
  kisi: Kisi;
  tur: "puan" | "oyun" | "klip" | "gonderi" | "gece";
  veri: Partial<Puan> & {
    ad: string;
    kapak?: string | null;
    /** Klip: video adresi ("/klip/….mp4"), oyun, açıklama, süre (sn), silineceği an (ms). */
    url?: string;
    not?: string | null;
    /** Oyun gecesi: gecenin kimliği, başlangıç (ms), kişi sınırı. */
    gece?: number;
    /** Gece kartı "oyuncu arıyorum" mu. */
    hemen?: boolean;
    zaman?: number;
    kontenjan?: number | null;
    /** Gönderi: yazı ve isteğe bağlı resim ("/resim/…"). */
    metin?: string;
    resim?: string | null;
    oyun?: string | null;
    sure?: number | null;
    bitis?: number;
  };
  zaman: number;
  begeni: number;
  begendim: boolean;
  yorumlar: Yorum[];
  /** Kaç kişi şikâyet etti (yalnız yöneticiye gösterilir). */
  sikayet?: number;
}

export type GeceDurum = "geliyor" | "belki" | "gelemem";

export interface Gece {
  id: number;
  olusturan: Kisi;
  oyun: string;
  /** Başlangıç (ms). */
  zaman: number;
  kontenjan: number | null;
  not: string | null;
  etkinlik: number | null;
  /** "Oyuncu arıyorum": şimdi oynamak için, 2 saat açık kalır. */
  hemen?: boolean;
  katilanlar: { kisi: Kisi; durum: GeceDurum }[];
}

export interface Mesaj {
  id: number;
  kimden: string;
  kime: string;
  metin: string;
  zaman: number;
  okundu: boolean;
}

export interface Konusma {
  kisi: Kisi;
  son: Mesaj;
  okunmamis: number;
}

const ANAHTAR = "social.token";
const BEN = "social.me";
const SUNUCU = "social.server";
const PAYLAS = "social.share";
const OYUN_HABERI = "social.notifyGames";

export async function oyunHaberiDegistir(v: boolean) {
  sosyal.oyunHaberi = v;
  await setJson(OYUN_HABERI, v);
}

export const sosyal = $state({
  sunucu: VARSAYILAN_SUNUCU,
  anahtar: null as string | null,
  ben: null as Kisi | null,
  bagli: false,
  giriliyor: false,
  /** Çevrim içi olanlar (kimlik → kişi). */
  cevrimici: {} as Record<string, Kisi>,
  /** Okunmamış özel mesaj sayısı (kimden → sayı). */
  okunmamis: {} as Record<string, number>,
  /** Görülmemiş beğeni/yorum bildirimleri. */
  bildirim: 0,
  /** Akış her değiştiğinde artar; ekran buna bakıp yeniler. */
  akisSurum: 0,
  /** Başkasından yeni paylaşım geldikçe artar (Twitter'daki "Yeni gönderiler"). */
  yeniGonderi: 0,
  paylas: { oyunlar: true, oyunPuan: true, filmPuan: true, simdi: true } as Paylasim,
  /** Arkadaşlar ve istekler. */
  arkadaslar: [] as Kisi[],
  gelen: [] as Kisi[],
  giden: [] as Kisi[],
  /** Profil ekranında gösterilen kişi (boşsa ben). */
  profilId: null as string | null,
  /** Arkadaş listesi paneli ve sohbet penceresi (her ekranda). */
  liste: false,
  sohbetler: [] as Kisi[],
  aktifSohbet: null as string | null,
  sohbetAcik: false,
  /** Arkadaş oyuna girince sağ altta haber ver. */
  oyunHaberi: true,
  /** Bütün üyeler (@anma önerisi ve tanıma için). */
  uyeler: [] as Kisi[],
  /** Yaklaşan oyun geceleri (kimlik → gece). */
  geceler: {} as Record<number, Gece>,
  /** Yönetici: herkesin paylaşımını/yorumunu silebilir, şikâyetleri görür. */
  yonetici: false,
  /** Sosyal'de seçili konu: "#etiket" ya da gündemdeki oyun/film adı (boşsa tüm akış). */
  konu: null as string | null,
});

/** Bir #etikete ya da gündemdeki konuya tıklanınca: Sosyal'de yalnız onunla ilgili paylaşımlar. */
export function konuGoster(k: string | null) {
  sosyal.konu = k;
  nav.go("feed");
}

export const arkadasMi = (id: string) => sosyal.arkadaslar.some((k) => k.id === id);

export async function arkadaslariYukle() {
  if (!sosyal.anahtar) return;
  const r = await istek<{ arkadaslar: Kisi[]; gelen: Kisi[]; giden: Kisi[] }>("GET", "/api/arkadaslar");
  sosyal.arkadaslar = r.arkadaslar;
  sosyal.gelen = r.gelen;
  sosyal.giden = r.giden;
}

/** İstek gönderir ya da gelen isteği kabul eder. */
export async function arkadasEkle(k: Kisi) {
  const r = await istek<{ durum: string }>("POST", `/api/arkadas/${k.id}`);
  notify(r.durum === "kabul" ? `${k.ad} artık arkadaşın` : r.durum === "istek" ? `${k.ad} kişisine arkadaşlık isteği gönderildi` : `${k.ad} zaten arkadaşın`, "users");
  await arkadaslariYukle();
}

/** Arkadaşlıktan çıkarır, isteği reddeder ya da geri alır. */
export async function arkadasSil(k: Kisi) {
  await istek("DELETE", `/api/arkadas/${k.id}`);
  sosyal.sohbetler = sosyal.sohbetler.filter((x) => x.id !== k.id);
  await arkadaslariYukle();
}

export function profilGoster(id: string | null) {
  sosyal.profilId = id;
  nav.go("profile");
}

/** Sohbet penceresinde o kişinin sekmesini açar (her ekranda). */
export function sohbetAc(k: Kisi) {
  if (!sosyal.sohbetler.some((x) => x.id === k.id)) sosyal.sohbetler = [...sosyal.sohbetler, k];
  sosyal.aktifSohbet = k.id;
  sosyal.sohbetAcik = true;
}

export function sohbetKapat(id: string) {
  sosyal.sohbetler = sosyal.sohbetler.filter((x) => x.id !== id);
  if (sosyal.aktifSohbet === id) sosyal.aktifSohbet = sosyal.sohbetler.at(-1)?.id ?? null;
  if (!sosyal.sohbetler.length) sosyal.sohbetAcik = false;
}

export const toplamOkunmamis = () => Object.values(sosyal.okunmamis).reduce((a, b) => a + b, 0);

/** Sunucudaki yollar (`/resim/…`) tam adrese çevrilir. */
export function tamAdres(u: string | null | undefined): string | null {
  if (!u) return null;
  return u.startsWith("/") ? sosyal.sunucu + u : u;
}

/** Başka modüllerin dinlediği canlı iletiler (birlikte dinleme). */
export const wsEk = new Set<(v: Record<string, unknown>) => void>();

type MesajDinleyici = (m: Mesaj, kisi: Kisi) => void;
const mesajDinleyiciler = new Set<MesajDinleyici>();
export function mesajlariDinle(f: MesajDinleyici): () => void {
  mesajDinleyiciler.add(f);
  return () => mesajDinleyiciler.delete(f);
}

export async function istek<T>(yontem: string, yol: string, govde?: unknown): Promise<T> {
  if (!sosyal.anahtar) throw new Error("Giriş yapılmadı");
  const bin = govde instanceof Blob;
  const r = await fetch(sosyal.sunucu + yol, {
    method: yontem,
    headers: {
      authorization: `Bearer ${sosyal.anahtar}`,
      ...(govde !== undefined ? { "content-type": bin ? govde.type : "application/json" } : {}),
    },
    body: govde === undefined ? undefined : bin ? govde : JSON.stringify(govde),
  });
  const v = await r.json().catch(() => ({}));
  if (r.status === 401) {
    cikis(false);
    throw new Error("Oturum sona erdi, yeniden giriş yap");
  }
  if (!r.ok) throw new Error((v as { hata?: string }).hata ?? `Sunucu ${r.status} döndü`);
  return v as T;
}

/** Sunucudan gelen bildirimin yazısı ve simgesi. */
function bildirimYazisi(tur: string, ad: string, v: Record<string, unknown>): [string, "chat" | "heart" | "flag" | "bell" | "users"] {
  switch (tur) {
    case "yorum":
      return [`${ad} paylaşımına yanıt yazdı`, "chat"];
    case "begeni":
      return [`${ad} paylaşımını beğendi`, "heart"];
    case "sikayet":
      return [`${ad} bir paylaşımı şikâyet etti${v.metin ? `: ${v.metin}` : ""} · Sosyal'de bak`, "flag"];
    case "anma":
      return [`${ad} senden bahsetti`, "users"];
    case "davet":
      return [`${ad} seni oyun gecesine çağırıyor${v.metin ? `: ${v.metin}` : ""}`, "bell"];
    case "ariyor":
      return [`${ad} oyuncu arıyor${v.metin ? `: ${v.metin}` : ""} · Sosyal'de "Varım" de`, "users"];
    case "varim":
      return [`${ad} "Varım" dedi${v.metin ? `: ${v.metin}` : ""}`, "users"];
    case "sunucu":
      return [`${ad}, ${v.metin ? String(v.metin) : "bir GMID sunucusu"}'na girdi · Sunucular'dan katıl`, "users"];
    case "hatirlat":
      return [`Oyun gecesi 15 dakika sonra başlıyor${v.metin ? `: ${v.metin}` : ""}`, "bell"];
    default:
      return [`${ad} paylaşımınla ilgilendi`, "bell"];
  }
}

// ---------- canlı bağlantı ----------
let ws: WebSocket | null = null;
let yeniden: ReturnType<typeof setTimeout> | undefined;
let bekleme = 2000;
let calisiyor = false;
let sonOyun: string | null | undefined;

function gonder(v: unknown) {
  if (ws?.readyState === WebSocket.OPEN) ws.send(JSON.stringify(v));
}

function baglan() {
  clearTimeout(yeniden);
  if (!sosyal.anahtar || ws) return;
  const w = new WebSocket(sosyal.sunucu.replace(/^http/, "ws") + "/ws");
  ws = w;
  w.onopen = () => {
    w.send(JSON.stringify({ tip: "merhaba", anahtar: sosyal.anahtar }));
    sosyal.bagli = true;
    bekleme = 2000;
    sonOyun = undefined;
    void durumGonder();
  };
  w.onmessage = (e) => {
    let v: Record<string, unknown>;
    try {
      v = JSON.parse(e.data as string);
    } catch {
      return;
    }
    switch (v.tip) {
      case "liste": {
        const yeni: Record<string, Kisi> = {};
        for (const k of v.kisiler as Kisi[]) yeni[k.id] = k;
        sosyal.cevrimici = yeni;
        break;
      }
      case "geldi": {
        const k = v.kisi as Kisi;
        sosyal.cevrimici = { ...sosyal.cevrimici, [k.id]: k };
        break;
      }
      case "gitti": {
        const { [v.id as string]: _, ...kalan } = sosyal.cevrimici;
        sosyal.cevrimici = kalan;
        break;
      }
      case "durum": {
        const k = sosyal.cevrimici[v.id as string];
        const oyun = (v.oyun as string | null) ?? null;
        if (k) {
          if (oyun && oyun !== k.oyun && sosyal.oyunHaberi && arkadasMi(k.id)) notify(`${k.ad} ${oyun} oynamaya başladı`, "library");
          sosyal.cevrimici = { ...sosyal.cevrimici, [k.id]: { ...k, oyun } };
        }
        break;
      }
      case "arkadas": {
        const kisi = v.kisi as Kisi | undefined;
        if (v.olay === "istek" && kisi) notify(`${kisi.ad} sana arkadaşlık isteği gönderdi`, "users");
        if (v.olay === "kabul" && kisi) notify(`${kisi.ad} arkadaşlık isteğini kabul etti`, "users");
        void arkadaslariYukle().catch(() => {});
        break;
      }
      case "akis":
        // Kimliksiz: yeni paylaşım (Sosyal aşağıdaysa "Yeni gönderiler" düğmesi çıkar); kimlikli: beğeni/yorum.
        if (v.id == null) sosyal.yeniGonderi++;
        else sosyal.akisSurum++;
        break;
      case "gece":
        void gecelerYukle();
        break;
      case "mesaj": {
        const m = v.mesaj as Mesaj;
        const kisi = v.kisi as Kisi;
        const gorunur = sosyal.sohbetAcik && sosyal.aktifSohbet === m.kimden && document.hasFocus();
        if (m.kimden !== sosyal.ben?.id && !gorunur) {
          // Pencerede sekmesi yoksa açılmadan eklenir (sohbet listesinde görünsün).
          if (!sosyal.sohbetler.some((x) => x.id === kisi.id)) sosyal.sohbetler = [...sosyal.sohbetler, kisi];
          sosyal.okunmamis = { ...sosyal.okunmamis, [m.kimden]: (sosyal.okunmamis[m.kimden] ?? 0) + 1 };
          notify(`${kisi.ad}: ${m.metin.slice(0, 60)}`, "chat");
        }
        for (const f of mesajDinleyiciler) f(m, kisi);
        break;
      }
      case "okundu":
        if (v.kisi === sosyal.ben?.id) {
          const { [v.diger as string]: _, ...kalan } = sosyal.okunmamis;
          sosyal.okunmamis = kalan;
        }
        break;
      case "bildirim": {
        const kisi = v.kisi as Kisi;
        sosyal.bildirim++;
        const [yazi, ikon] = bildirimYazisi(String(v.tur), kisi.ad, v);
        notify(yazi, ikon);
        break;
      }
      case "dinle":
      case "oda":
        for (const f of wsEk) f(v);
        break;
      case "hata":
        cikis(false);
        break;
    }
  };
  w.onclose = () => {
    ws = null;
    sosyal.bagli = false;
    sosyal.cevrimici = {};
    if (sosyal.anahtar && calisiyor) {
      yeniden = setTimeout(baglan, bekleme);
      bekleme = Math.min(bekleme * 2, 60_000);
    }
  };
}

/** Şu an oynanan oyun (paylaşım açıksa) sunucuya. */
async function durumGonder() {
  if (!sosyal.bagli) return;
  let oyun: string | null = null;
  if (sosyal.paylas.simdi) {
    const g = (await api.games.list().catch(() => [] as Game[])).find((x) => x.runningSince);
    oyun = g?.name ?? null;
  }
  if (oyun !== sonOyun) {
    sonOyun = oyun;
    gonder({ tip: "durum", oyun });
  }
}

// ---------- profil eşitleme ----------
const RESIMLER = "gmid.social.images";
function resimOnbellek(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem(RESIMLER) ?? "{}");
  } catch {
    return {};
  }
}

/** Yerel kapak dosyasını küçültüp (en çok 300 px) sunucuya yükler; aynı dosya bir kez. */
async function resimYukle(yol: string, bellek: Record<string, string>): Promise<string | null> {
  if (bellek[yol]) return bellek[yol];
  if (!inTauri) return null;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const bayt = await invoke<ArrayBuffer>("social_image", { path: yol });
    const bmp = await createImageBitmap(new Blob([bayt]));
    const olcek = Math.min(1, 300 / bmp.width);
    const c = document.createElement("canvas");
    c.width = Math.round(bmp.width * olcek);
    c.height = Math.round(bmp.height * olcek);
    c.getContext("2d")!.drawImage(bmp, 0, 0, c.width, c.height);
    const blob = await new Promise<Blob | null>((ok) => c.toBlob(ok, "image/jpeg", 0.82));
    if (!blob) return null;
    const { url } = await istek<{ url: string }>("POST", "/api/resim", blob);
    bellek[yol] = url;
    return url;
  } catch {
    return null;
  }
}

function steamKapak(appid: number | null | undefined): string | null {
  return appid ? `https://cdn.cloudflare.steamstatic.com/steam/apps/${appid}/library_600x900.jpg` : null;
}

let esitleZamanlayici: ReturnType<typeof setTimeout> | undefined;
/** Profili kısa bir gecikmeyle eşitler (art arda değişiklikler tek istekte). */
export function esitle(gecikme = 4000) {
  clearTimeout(esitleZamanlayici);
  if (!sosyal.anahtar) return;
  esitleZamanlayici = setTimeout(() => void esitleSimdi().catch(() => {}), gecikme);
}

export async function esitleSimdi(): Promise<void> {
  if (!sosyal.anahtar) return;
  const p = sosyal.paylas;
  const [oyunlar, oyunPuan, filmPuan, filmler] = await Promise.all([
    p.oyunlar || p.oyunPuan ? api.games.list().catch(() => [] as Game[]) : Promise.resolve([] as Game[]),
    p.oyunPuan ? api.reviews.list("game").catch(() => [] as Review[]) : Promise.resolve([] as Review[]),
    p.filmPuan ? api.reviews.list("title").catch(() => [] as Review[]) : Promise.resolve([] as Review[]),
    p.filmPuan ? api.titles.list().catch(() => [] as Title[]) : Promise.resolve([] as Title[]),
  ]);
  const bellek = resimOnbellek();
  // Aynı anda en çok 3 yükleme.
  const kuyruk: (() => Promise<void>)[] = [];
  const kapak = (yerel: string | null, steam: string | null, koy: (u: string | null) => void) => {
    if (steam) return koy(steam);
    if (!yerel) return koy(null);
    if (yerel.startsWith("http")) return koy(yerel);
    kuyruk.push(async () => koy(await resimYukle(yerel, bellek)));
  };

  const profil: Profil = { oyunlar: [], puanlar: [] };
  const oyunAd = new Map(oyunlar.map((g) => [g.id, g]));
  if (p.oyunlar) {
    for (const g of oyunlar.filter((g) => g.inLibrary && !g.hidden)) {
      const o: ProfilOyun = {
        ad: g.name,
        kapak: null,
        dakika: Math.max(Math.round(g.playedSeconds / 60), g.platformMinutes),
        son: g.lastPlayed,
      };
      kapak(g.cover, steamKapak(g.steamAppid), (u) => (o.kapak = u));
      profil.oyunlar.push(o);
    }
  }
  const puan = (r: Review, tur: Puan["tur"], ad: string, yil: number | null): Puan => ({
    anahtar: `${r.subjectKind}:${r.subjectId}`,
    tur,
    ad,
    kapak: null,
    puan: r.overall,
    yorum: r.oneLiner,
    durum: r.status,
    yil,
    guncelleme: r.updatedAt,
  });
  for (const r of oyunPuan) {
    const g = oyunAd.get(r.subjectId);
    if (!g) continue;
    const x = puan(r, "game", g.name, g.year);
    kapak(g.cover, steamKapak(g.steamAppid), (u) => (x.kapak = u));
    profil.puanlar.push(x);
  }
  const film = new Map(filmler.map((t) => [t.id, t]));
  for (const r of filmPuan) {
    const t = film.get(r.subjectId);
    if (!t) continue;
    const x = puan(r, t.kind === "series" ? "series" : "movie", t.name, t.year);
    kapak(t.poster, null, (u) => (x.kapak = u));
    profil.puanlar.push(x);
  }
  const isciler = Array.from({ length: 3 }, async () => {
    for (let is = kuyruk.shift(); is; is = kuyruk.shift()) await is();
  });
  await Promise.all(isciler);
  try {
    localStorage.setItem(RESIMLER, JSON.stringify(bellek));
  } catch {
    /* yer yok */
  }
  await istek("PUT", "/api/profil", profil);
}

// ---------- giriş / başlatma ----------
export async function girisYap(): Promise<void> {
  sosyal.giriliyor = true;
  try {
    const v = await api.social.login(sosyal.sunucu);
    sosyal.anahtar = v.anahtar;
    sosyal.ben = v.kullanici;
    await setJson(ANAHTAR, v.anahtar);
    await setJson(BEN, v.kullanici);
    notify(`Hoş geldin, ${v.kullanici.ad}`, "users");
    baslat();
  } finally {
    sosyal.giriliyor = false;
  }
}

export function cikis(sunucuya = true) {
  if (sunucuya && sosyal.anahtar) void istek("POST", "/api/cikis").catch(() => {});
  calisiyor = false;
  sosyal.anahtar = null;
  sosyal.ben = null;
  sosyal.okunmamis = {};
  sosyal.bildirim = 0;
  sosyal.arkadaslar = [];
  sosyal.gelen = [];
  sosyal.giden = [];
  sosyal.sohbetler = [];
  sosyal.sohbetAcik = false;
  sosyal.liste = false;
  ws?.close();
  ws = null;
  void setJson(ANAHTAR, null);
  void setJson(BEN, null);
}

export async function paylasimDegistir(p: Partial<Paylasim>) {
  sosyal.paylas = { ...sosyal.paylas, ...p };
  await setJson(PAYLAS, sosyal.paylas);
  if (p.simdi !== undefined) {
    sonOyun = undefined;
    void durumGonder();
  }
  esitle(500);
}

async function okunmamislariYukle() {
  const { konusmalar } = await istek<{ konusmalar: Konusma[] }>("GET", "/api/konusmalar");
  const o: Record<string, number> = {};
  for (const k of konusmalar) if (k.okunmamis) o[k.kisi.id] = k.okunmamis;
  sosyal.okunmamis = o;
}

let dinliyor = false;
function baslat() {
  if (!sosyal.anahtar) return;
  calisiyor = true;
  baglan();
  void gecelerYukle();
  void uyelerYukle();
  void istek<{ yonetici?: boolean }>("GET", "/api/ben")
    .then((b) => (sosyal.yonetici = !!b.yonetici))
    .catch(() => {});
  esitle(8000);
  void okunmamislariYukle().catch(() => {});
  void arkadaslariYukle().catch(() => {});
  if (!dinliyor) {
    dinliyor = true;
    void on("games-changed", () => {
      void durumGonder();
      esitle();
    });
    void on("reviews-changed", () => esitle(1500));
  }
}

/** Uygulama açılışında bir kez: kayıtlı oturum varsa bağlanır. */
export function sosyalBaslat() {
  sosyal.sunucu = getJson<string>(SUNUCU, VARSAYILAN_SUNUCU) || VARSAYILAN_SUNUCU;
  sosyal.anahtar = getJson<string | null>(ANAHTAR, null);
  sosyal.ben = getJson<Kisi | null>(BEN, null);
  sosyal.paylas = { ...sosyal.paylas, ...getJson<Partial<Paylasim>>(PAYLAS, {}) };
  sosyal.oyunHaberi = getJson<boolean>(OYUN_HABERI, true);
  if (sosyal.anahtar) baslat();
}

/** Klibi Sosyal'de paylaş: (seçilen bölüm) küçültülür, sunucuya yüklenir; 7 gün sonra video da paylaşım da silinir. */
export async function klipPaylas(p: {
  id: string;
  ad: string | null;
  oyun: string | null;
  not: string | null;
  start: number | null;
  end: number | null;
}) {
  if (!sosyal.anahtar) throw new Error("Önce Topluluk'tan Discord ile giriş yap");
  const r = await api.clips.shareSocial({
    id: p.id,
    server: sosyal.sunucu,
    token: sosyal.anahtar,
    title: p.ad,
    game: p.oyun,
    note: p.not,
    start: p.start,
    end: p.end,
  });
  sosyal.akisSurum++;
  return r;
}

/** Seçilen resmi gönderi için küçültür (en çok 1280 px, ≤ ~390 KB JPEG). */
async function gonderiResmi(dosya: Blob): Promise<Blob> {
  const bmp = await createImageBitmap(dosya);
  for (const [en, kalite] of [[1280, 0.85], [1280, 0.72], [1024, 0.7], [800, 0.65], [640, 0.6]] as const) {
    const olcek = Math.min(1, en / Math.max(bmp.width, bmp.height));
    const c = document.createElement("canvas");
    c.width = Math.round(bmp.width * olcek);
    c.height = Math.round(bmp.height * olcek);
    c.getContext("2d")!.drawImage(bmp, 0, 0, c.width, c.height);
    const b = await new Promise<Blob | null>((ok) => c.toBlob(ok, "image/jpeg", kalite));
    if (b && b.size <= 390 * 1024) return b;
  }
  throw new Error("Resim küçültülemedi; başka bir resim dene");
}

/** Twitter'daki gibi gönderi: yazı (en çok 500 harf) ve isteğe bağlı resim. */
export async function gonderiPaylas(metin: string, resim: Blob | null) {
  let url: string | null = null;
  if (resim) url = (await istek<{ url: string }>("POST", "/api/resim", await gonderiResmi(resim))).url;
  await istek("POST", "/api/gonderi", { metin, resim: url });
  sosyal.akisSurum++;
}

// ---------- oyun gecesi ----------

/** Yaklaşan oyun gecelerini sunucudan alır (eski sunucuda sessizce boş kalır). */
export async function gecelerYukle() {
  if (!sosyal.anahtar) return;
  try {
    const r = await istek<{ geceler: Gece[] }>("GET", "/api/geceler");
    sosyal.geceler = Object.fromEntries(r.geceler.map((g) => [g.id, g]));
  } catch {
    /* sunucu henüz bilmiyor */
  }
}

export async function geceAc(g: { oyun: string; zaman: number; kontenjan: number | null; not: string | null; hemen?: boolean }) {
  await istek("POST", "/api/gece", g);
  await gecelerYukle();
  sosyal.akisSurum++;
}

export async function geceKatil(id: number, durum: GeceDurum) {
  const r = await istek<{ gece: Gece | null }>("POST", `/api/gece/${id}/katil`, { durum });
  if (r.gece) sosyal.geceler = { ...sosyal.geceler, [id]: r.gece };
}

export async function geceIptal(id: number) {
  await istek("DELETE", `/api/gece/${id}`);
  const { [id]: _, ...kalan } = sosyal.geceler;
  sosyal.geceler = kalan;
  sosyal.akisSurum++;
}

/** Bir GMID sunucusuna Oyna denince arkadaşlara "sunucuya girdi" bildirimi (sunucu 10 dakikada bir gönderir). */
export function sunucuyaGirdi(sunucu: string, adres: string) {
  if (!sosyal.anahtar) return;
  void istek("POST", "/api/sunucuya-girdi", { sunucu, adres }).catch(() => {});
}

/** Üye listesi (@anma için); Topluluk ve Sosyal de bunu kullanır. */
export async function uyelerYukle() {
  if (!sosyal.anahtar) return;
  try {
    sosyal.uyeler = (await istek<{ kisiler: Kisi[] }>("GET", "/api/kisiler")).kisiler;
  } catch {
    /* sessiz */
  }
}
