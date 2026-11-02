// Futbol verisi önbelleği: ekrana her girişte aynı veriyi yeniden indirmeyelim.
// Eldeki veri hemen gösterilir; süresi dolmuşsa arka planda tazelenir ve yalnız değiştiyse ekrana yansır.
// GMID kapanıp açılınca da kalsın diye tarayıcı deposunda (bu pencereye özel) saklanır; en çok 40 kayıt.

const STORE = "gmid.football.cache";
const MAX = 40;

interface Entry {
  at: number;
  json: string;
}

const mem = new Map<string, Entry>();
try {
  const raw = localStorage.getItem(STORE);
  if (raw) for (const [k, e] of Object.entries(JSON.parse(raw) as Record<string, Entry>)) mem.set(k, e);
} catch {
  /* depo kapalı ya da bozuk: boş başla */
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;
function persist() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    try {
      // En yeni MAX kayıt.
      const keep = [...mem.entries()].sort((a, b) => b[1].at - a[1].at).slice(0, MAX);
      mem.clear();
      for (const [k, e] of keep) mem.set(k, e);
      localStorage.setItem(STORE, JSON.stringify(Object.fromEntries(keep)));
    } catch {
      /* yer yoksa yalnız bellekte kalır */
    }
  }, 400);
}

/** Saklanan veri ve ne kadar önce alındığı (ms). */
export function peek<T>(key: string): { data: T; age: number } | null {
  const e = mem.get(key);
  if (!e) return null;
  try {
    return { data: JSON.parse(e.json) as T, age: Date.now() - e.at };
  } catch {
    return null;
  }
}

/** Yeni veriyi saklar; öncekiyle aynıysa `false` (ekranı boşuna yeniden çizmeyelim). */
export function store(key: string, data: unknown): boolean {
  const json = JSON.stringify(data);
  const old = mem.get(key);
  mem.set(key, { at: Date.now(), json });
  persist();
  return old?.json !== json;
}

/** Ne kadar taze kalsın (ms): canlı maç varken 30 sn, bugün 2 dk, geçmiş hafta 6 sa, gelecek 30 dk. */
export const FRESH = {
  live: 30_000,
  today: 120_000,
  past: 6 * 3_600_000,
  future: 30 * 60_000,
  table: 20 * 60_000,
  leaders: 60 * 60_000,
  team: 20 * 60_000,
};

/**
 * Açılışta (ekran açılmadan) seçili ligin bu haftaki maçlarını ve puan durumunu arkada hazırlar: Futbol'a ilk
 * girişte veri hazır gelir. Anahtarlar Futbol ekranındakiyle aynı; taze veri varsa hiç istenmez.
 */
export async function onceHazirla(api: {
  leagues: () => Promise<[string, string][]>;
  week: (leagues: string[], from: string, to: string) => Promise<unknown>;
  standings: (league: string) => Promise<unknown>;
}, league: string) {
  const leagues = await api.leagues().catch(() => [] as [string, string][]);
  if (!leagues.length) return;
  const lg = league !== "all" && leagues.some(([s]) => s === league) ? league : "all";
  const ymd = (d: Date) => `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}`;
  const mon = new Date();
  mon.setHours(12, 0, 0, 0);
  mon.setDate(mon.getDate() - ((mon.getDay() + 6) % 7));
  const plus = (n: number) => {
    const x = new Date(mon);
    x.setDate(x.getDate() + n);
    return x;
  };
  const want = lg === "all" ? leagues.map(([s]) => s) : [lg];
  const from = ymd(plus(-1));
  const key = `w:${want.join(",")}:${from}`;
  const eski = peek<unknown>(key);
  if (!eski || eski.age > FRESH.today) {
    const list = await api.week(want, from, ymd(plus(6))).catch(() => null);
    if (list) store(key, list);
  }
  if (lg !== "all") {
    const t = peek<unknown>(`t:${lg}`);
    if (!t || t.age > FRESH.table) {
      const g = await api.standings(lg).catch(() => null);
      if (g) store(`t:${lg}`, g);
    }
  }
}
