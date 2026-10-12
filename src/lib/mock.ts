// Tarayıcıda (Tauri olmadan) geliştirirken kullanılan taklit çekirdek. Uygulamanın kendisinde yüklenmez.
import { localBus } from "./bus";
import type {
  Clip, Countdown, InstallItem, InstallRun, Device, Title, TitleHit, DictationStatus, Download, Game, PlaySession, Review, ReviewExtra, ReviewInput, ReviewStatus, Recommend, Scores, UserPlaylist, GameInfo, Live, Probe, Segment, Transcript, HotkeyStatus, Note, NoteInput, Reminder, ReminderInput, Server, ServerInput, ServerStatus,
} from "./types";
import { overallOf } from "./review";

const emit = (name: string, detail?: unknown) => localBus.dispatchEvent(new CustomEvent(name, { detail }));
const now = () => Date.now();
let seq = 0;
const id = () => `${now().toString(16)}${(seq++).toString(16)}`;

const outputs: Device[] = [
  { id: "out-1", name: "Kulaklık (HyperX Cloud II)", isDefault: true, level: 0.62, muted: false },
  { id: "out-2", name: "Hoparlör (Realtek(R) Audio)", isDefault: false, level: 0.4, muted: false },
  { id: "out-3", name: "LG ULTRAGEAR (NVIDIA High Definition Audio)", isDefault: false, level: 1, muted: false },
];
const inputs: Device[] = [
  { id: "in-1", name: "Mikrofon (HyperX Cloud II)", isDefault: true, level: 1, muted: false },
  { id: "in-2", name: "Mikrofon Dizisi (Realtek(R) Audio)", isDefault: false, level: 0.8, muted: false },
];
let lock: number | null = 1;

const hotkeys: HotkeyStatus[] = [
  ["audio.cycleOutput", "Ses", "Ses çıkışını değiştir (kulaklık ↔ hoparlör)", "Ctrl+Alt+A"],
  ["mic.toggleMute", "Mikrofon", "Mikrofonu sessize al / aç", "Ctrl+Alt+M"],
  ["mic.toggleLock", "Mikrofon", "Mikrofon seviyesi kilidini aç / kapat", "Ctrl+Alt+L"],
  ["mic.up", "Mikrofon", "Mikrofon +5", "Ctrl+Alt+PageUp"],
  ["mic.down", "Mikrofon", "Mikrofon −5", "Ctrl+Alt+PageDown"],
  ["mic.max", "Mikrofon", "Mikrofonu %100'e çek", "Ctrl+Alt+Home"],
  ["note.quick", "Uygulama", "Hızlı not", "Ctrl+Alt+N"],
  ["app.show", "Uygulama", "GMID Tool penceresini aç", "Ctrl+Alt+G"],
  ["videos.clipboard", "Video İndirici", "Panodaki bağlantıyı indir (önerilen kalite)", "Ctrl+Alt+D"],
].map(([i, group, label, a]) => ({ id: i, group, label, accel: a, defaultAccel: a, error: null }));

const notes: Note[] = [
  {
    id: "n1", title: "Hades II · Nyx build denemesi", color: "plum", font: "inter", pinned: true,
    body: "Selene aspect + Artemis çağrısı birlikte **çok güçlü**.\n\n- [x] Asfodel'i 15 dakikanın altında bitir\n- [ ] Kılıçla korku 16'yı dene\n- [ ] Hecate'ye hediye götür",
    createdAt: now() - 86_400_000, updatedAt: now() - 3_600_000, deletedAt: null,
  },
  {
    id: "n2", title: "Hafta sonu co-op listesi", color: "teal", font: "inter", pinned: false,
    body: "Cumartesi akşamı herkes müsait.\n\n- [ ] Risk of Rain 2\n- [x] It Takes Two\n- [ ] Stardew · ortak çiftlik",
    createdAt: now() - 2 * 86_400_000, updatedAt: now() - 2 * 86_400_000, deletedAt: null,
  },
  {
    id: "n3", title: "Market", color: "gold", font: "hand", pinned: false,
    body: "- [ ] Çay\n- [ ] Kahve\n- [x] Ekmek", createdAt: now() - 4 * 86_400_000, updatedAt: now() - 4 * 86_400_000, deletedAt: null,
  },
];

const reminders: Reminder[] = [
  { id: "r1", title: "Çay demle", note: "", dueAt: now() + 25 * 60_000, repeat: "none", notified: false, done: false, createdAt: now() },
  { id: "r2", title: "Epic ücretsiz oyun yenilendi", note: "Kütüphaneye eklemeyi unutma", dueAt: now() + 3 * 86_400_000, repeat: "weekly", notified: false, done: false, createdAt: now() },
  { id: "r3", title: "Su iç", note: "", dueAt: now() - 4 * 60_000, repeat: "daily", notified: true, done: false, createdAt: now() },
];

const sw = { started: null as number | null, accum: 0, laps: [] as number[] };
const countdowns: Countdown[] = [
  { id: "c1", label: "Fırın", totalMs: 20 * 60_000, endsAt: now() + 13 * 60_000, remainingMs: 0, finished: false },
  { id: "c2", label: "Mola", totalMs: 5 * 60_000, endsAt: null, remainingMs: 3 * 60_000 + 12_000, finished: false },
];

const COD_HOWTO = "Oyunu aç → Multiplayer → Join Game. Ya da konsolu (~) açıp yaz: /connect {host}:{port}";
const STEAM_HOWTO = "Bağlan'a basınca Steam oyunu açıp sunucuya sokar. Elle: konsola (~) connect {host}:{port}";
const games: GameInfo[] = (
  [
    ["cod2", "Call of Duty 2", "CoD2", 28960, "exe", "CoD2MP_s.exe", COD_HOWTO],
    ["cod4", "Call of Duty 4", "CoD4", 28960, "exe", "iw3mp.exe", COD_HOWTO],
    ["coduo", "Call of Duty: United Offensive", "UO", 28960, "exe", "CoDUOMP.exe", COD_HOWTO],
    ["cod1", "Call of Duty", "CoD", 28960, "exe", "CoDMP.exe", COD_HOWTO],
    ["minecraft", "Minecraft (Java)", "MC", 25565, "copy", null, "Minecraft'ı aç → Çok Oyunculu → Sunucu Ekle. Adres: {host}:{port}"],
    ["minecraft-bedrock", "Minecraft (Bedrock)", "MCB", 19132, "uri", null, "Bağlan'a basınca sunucu Minecraft'taki Sunucular listesine eklenir."],
    ["cs16", "Counter-Strike 1.6", "CS", 27015, "steam", null, STEAM_HOWTO],
    ["cs2", "Counter-Strike 2", "CS2", 27015, "steam", null, STEAM_HOWTO],
    ["source", "Diğer Steam oyunları (TF2, GMod…)", "SRC", 27015, "steam", null, STEAM_HOWTO],
    ["valheim", "Valheim", "VH", 2456, "copy", null, "Valheim → Oyuna Başla → karakter → Sunucuya Katıl → IP ekle: {host}:{port}"],
    ["samp", "GTA San Andreas (SA-MP)", "SA", 7777, "uri", null, "Bağlan'a basınca SA-MP açılır. Elle: SA-MP → Add Server → {host}:{port}"],
    ["mta", "MTA: San Andreas", "MTA", 22003, "uri", null, "Bağlan'a basınca MTA açılır. Elle: Quick Connect → {host}:{port}"],
    ["other", "Diğer", "?", 0, "copy", null, "Adres: {host}:{port}"],
  ] as const
).map(([gid, name, short, defaultPort, join, exe, howTo]) => ({
  id: gid, name, short, defaultPort, join, exe, queryable: !["mta", "other"].includes(gid), howTo,
}));

const servers: Server[] = [
  { id: "r-cod2", name: "GMID · Call of Duty 2", game: "cod2", host: "cod2.gmid.gg", port: 28960, queryPort: null, logo: null,
    description: "Her akşam 21:00'de TDM. Toujane, Carentan, Dawnville dönüyor.", connectNotes: "Şifre Discord'da #sunucular kanalında.", remote: true, createdAt: 1 },
  { id: "r-mc", name: "GMID Survival", game: "minecraft", host: "mc.gmid.gg", port: 25565, queryPort: null, logo: null,
    description: "1.21 hayatta kalma, beyaz liste var. Discord'dan isim yaz, eklensin.", connectNotes: "", remote: true, createdAt: 2 },
  { id: "r-cs", name: "GMID · CS 1.6 Public", game: "cs16", host: "cs.gmid.gg", port: 27015, queryPort: null, logo: null,
    description: "Klasik haritalar, 5v5 hafta sonu.", connectNotes: "", remote: true, createdAt: 3 },
  { id: "l-1", name: "Kuzenlerle Valheim", game: "valheim", host: "88.231.14.7", port: 2456, queryPort: null, logo: null,
    description: "", connectNotes: "Şifre: kahve", remote: false, createdAt: 4 },
];

const P = (name: string, score: number, ping: number) => ({ name, score, ping });
const statuses: Record<string, ServerStatus> = {
  "r-cod2": { id: "r-cod2", state: "online", error: null, checkedAt: now(), stale: false, info: {
    name: "GMID CoD2 | TDM 7/24", map: "mp_toujane", mode: "tdm", version: "1.3", players: 14, maxPlayers: 24, bots: 0, password: true,
    playerList: [P("Merkut", 42, 38), P("Balaman", 37, 41), P("Kaptan", 25, 55), P("Ağaç", 19, 62), P("Fırtına", 12, 47), P("Yıldırım", 9, 33),
      P("Kartal", 7, 71), P("Şahin", 6, 49), P("Gölge", 4, 58), P("Sis", 3, 44), P("Pars", 2, 39), P("Kurt", 1, 66), P("Tilki", 0, 52), P("Baykuş", 0, 80)],
    favicon: null, pingMs: 41 } },
  "r-mc": { id: "r-mc", state: "online", error: null, checkedAt: now(), stale: false, info: {
    name: "GMID Survival · 1.21", map: "", mode: "", version: "Paper 1.21.1", players: 3, maxPlayers: 20, bots: 0, password: false,
    playerList: [P("Merkut", 0, 0), P("Balaman", 0, 0), P("Kaptan", 0, 0)].map((p) => ({ ...p, score: null, ping: null })), favicon: null, pingMs: 28 } },
  "r-cs": { id: "r-cs", state: "offline", error: "Yanıt vermedi", checkedAt: now(), stale: false, info: null },
  "l-1": { id: "l-1", state: "online", error: null, checkedAt: now(), stale: false, info: {
    name: "Kuzenler", map: "", mode: "valheim", version: "0.219.14", players: 2, maxPlayers: 10, bots: 0, password: true,
    playerList: [], favicon: null, pingMs: 63 } },
};

// ---- Video İndirici ----
const thumb = (a: string, b: string, t: string) =>
  "data:image/svg+xml," + encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs><rect width="320" height="180" fill="url(#g)"/><text x="20" y="160" font-family="Segoe UI,sans-serif" font-size="22" font-weight="700" fill="rgba(255,255,255,.85)">${t}</text></svg>`);
const toolsReady = !new URLSearchParams(location.search).has("tools");
const dl = (o: Partial<Download>): Download => ({
  id: id(), url: "https://www.youtube.com/watch?v=x", videoId: "x", title: "", uploader: "", site: "Youtube", thumbnail: null,
  duration: 300, quality: "1080p", audioOnly: false, status: "done", progress: 1, size: 120_000_000, filePath: "C:\\Videolar\\a.mp4",
  error: null, source: "app", createdAt: now() - 3_600_000, finishedAt: now() - 3_000_000, playlist: null, ...o,
});
const downloadsList: Download[] = [
  dl({ title: "Hades II — Tüm silahlar ve en iyi build'ler (Rehber)", uploader: "Supergiant Rehber", status: "downloading", progress: 0.47, size: 412_000_000, finishedAt: null, thumbnail: thumb("#7848a8", "#121417", "HADES II") }),
  dl({ title: "Elden Ring Nightreign ilk bakış", uploader: "Oyun Günlüğü", status: "queued", progress: 0, quality: "720p", size: 180_000_000, finishedAt: null, thumbnail: thumb("#8a7646", "#121417", "NIGHTREIGN") }),
  dl({ title: "Bu clip efsane 😂", uploader: "@merkut", site: "Twitter", status: "error", progress: 0.1, quality: "720p", error: "Bu site giriş istiyor. Tarayıcı eklentisinden indirirsen giriş bilgin kullanılır.", finishedAt: null, thumbnail: thumb("#0b203e", "#35a0a2", "X") }),
  dl({ title: "CoD2 Toujane 1v5 clutch", uploader: "GMID Klipler", duration: 74, size: 48_200_000, thumbnail: thumb("#c9a35a", "#2f3235", "CoD2"), finishedAt: now() - 600_000 }),
  dl({ title: "Lofi oyun müzikleri · 1 saat", uploader: "Chill Malikâne", quality: "MP3", audioOnly: true, duration: 3600, size: 86_000_000, thumbnail: thumb("#35a0a2", "#121417", "LOFI"), finishedAt: now() - 86_400_000, playlist: "GMID Lofi" }),
  dl({ title: "Gece Sürüşü", uploader: "Chill Malikâne", quality: "MP3", audioOnly: true, duration: 186, size: 7_400_000, thumbnail: thumb("#7848a8", "#121417", "2"), finishedAt: now() - 86_400_000, playlist: "GMID Lofi" }),
  dl({ title: "Minecraft sunucumuzun turu", uploader: "@gmid", site: "Instagram", quality: "1080p", duration: 45, size: 22_000_000, thumbnail: thumb("#e06a6a", "#7848a8", "REELS"), finishedAt: now() - 2 * 86_400_000 }),
  dl({ title: "Yeryüzü Şarkısı (2026)", uploader: "", site: "Generic", quality: "1080p", duration: 7116, filePath: "ornek/film.mp4", size: 2_100_000_000, thumbnail: thumb("#0b203e", "#e3c270", "FİLM"), finishedAt: now() - 7_200_000 }),
  dl({ title: "Stardew Valley ortak çiftlik planı", uploader: "Çiftçi Abi", quality: "1440p", duration: 1260, size: 640_000_000, thumbnail: thumb("#5cc08a", "#0b203e", "STARDEW"), finishedAt: now() - 3 * 86_400_000 }),
];
const liveMap: Record<string, Live> = {};
setInterval(() => {
  const d = downloadsList.find((x) => x.status === "downloading");
  if (!d) return;
  const l = (liveMap[d.id] ??= { id: d.id, progress: d.progress, speed: 6_400_000, eta: 0, downloaded: 0, total: d.size, phase: "download" });
  l.progress = Math.min(0.97, l.progress + 0.004);
  l.speed = 5_800_000 + Math.random() * 1_400_000;
  l.eta = Math.round(((1 - l.progress) * (d.size ?? 0)) / l.speed);
  emit("downloads-progress", Object.values(liveMap));
}, 1000);
const probeResult: Probe = {
  url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ", videoId: "dQw4w9WgXcQ", title: "GMID Malikâne turu — 4K", uploader: "Merkut Studios", site: "Youtube",
  duration: 212, thumbnail: thumb("#e3c270", "#0b203e", "GMID TOOL"),
  options: [
    { id: "2160", label: "2160p · 4K", detail: "MKV", size: 912_000_000, audioOnly: false, recommended: false },
    { id: "1440", label: "1440p · 2K", detail: "MKV", size: 402_000_000, audioOnly: false, recommended: false },
    { id: "1080", label: "1080p", detail: "60 fps · MP4", size: 168_000_000, audioOnly: false, recommended: true },
    { id: "720", label: "720p", detail: "MP4", size: 74_000_000, audioOnly: false, recommended: false },
    { id: "480", label: "480p", detail: "MP4", size: 31_000_000, audioOnly: false, recommended: false },
    { id: "mp3", label: "Sadece ses", detail: "MP3", size: 3_400_000, audioOnly: true, recommended: false },
  ],
  audioLangs: [{ code: "tur", label: "Türkçe" }, { code: "eng", label: "İngilizce" }],
  subtitles: [{ code: "tr", label: "Türkçe" }, { code: "en", label: "İngilizce" }, { code: "de", label: "Almanca" }],
};

// ---- Oyunlar ----
const gameTags: string[][] = [
  ["Roguelike", "Hack and Slash", "Zor", "Tek oyunculu", "Fantastik"],
  ["FPS", "Rekabetçi", "Çok oyunculu", "Taktik"],
  ["Soulslike", "Açık dünya", "Zor", "Fantastik", "Atmosferik", "Co-op"],
  ["Rahatlatıcı", "Piksel grafik", "Co-op", "Üretim"],
  ["Battle Royale", "Çok oyunculu", "İnşa"],
  ["FPS", "Taktik", "Rekabetçi", "Çok oyunculu"],
  ["Sandbox", "Hayatta kalma", "Co-op", "Üretim", "Açık dünya"],
  ["Metroidvania", "Zor", "Atmosferik", "Piksel grafik"],
  ["Açık dünya", "Hikâye odaklı", "Atmosferik"],
];
const gameNames: [string, string, string, number, string[], number][] = [
  // ad, kaynak, renk, Steam dakikası, türler, yıl
  ["Hades II", "steam", "#7848a8", 2950, ["Aksiyon", "Bağımsız", "RPG"], 2025],
  ["Counter-Strike 2", "steam", "#c9a35a", 41230, ["Aksiyon", "Ücretsiz Oyna"], 2023],
  ["Elden Ring", "steam", "#8a7646", 9380, ["Aksiyon", "RPG"], 2022],
  ["Stardew Valley", "steam", "#5cc08a", 4410, ["Bağımsız", "RPG", "Simülasyon"], 2016],
  ["Fortnite", "epic", "#35a0a2", 0, ["Aksiyon"], 2017],
  ["Valorant", "manual", "#e06a6a", 0, ["Aksiyon"], 2020],
  ["Minecraft", "manual", "#5c8a3a", 0, ["Macera"], 2011],
  ["Hollow Knight", "steam", "#4a6a9a", 0, ["Aksiyon", "Bağımsız", "Macera"], 2017],
  ["Red Dead Redemption 2", "archive", "#9a4a3a", 0, ["Aksiyon", "Macera"], 2019],
];
const libGames: Game[] = gameNames.map(([name, source, color, minutes, genres, year], i) => ({
  id: `g${i}`, name, source: source as Game["source"], sourceId: String(1000 + i), installDir: null, exes: [], launch: null,
  steamAppid: source === "steam" ? 1000 + i : null, cover: thumb(color, "#121417", name.toUpperCase()), header: null,
  developer: ["Supergiant Games", "Valve", "FromSoftware", "ConcernedApe", "Epic Games", "Riot Games", "Mojang", "Team Cherry", "Rockstar Games"][i],
  publisher: null, year, description: "Kısa oyun açıklaması Steam mağazasından Türkçe gelir.", genres, categories: ["Tek oyunculu"],
  tags: gameTags[i], inLibrary: i < 7,
  platformMinutes: minutes, platformLastPlayed: null, hidden: false, addedAt: now() - 9 * 86_400_000,
  playedSeconds: 0, sessions: 0, lastPlayed: null, runningSince: null,
}));
const playSessions: PlaySession[] = [];
{
  const weights = [5, 4, 2, 2, 1, 2, 1, 0, 0];
  let k = 0;
  for (let day = 20; day >= 0; day--) {
    const n = (day * 7 + 3) % 4;
    for (let j = 0; j < n; j++) {
      const gi = (day * 3 + j * 5) % 7;
      const secs = Math.round((25 + ((day * 37 + j * 53) % 140)) * 60 * (weights[gi] / 3 + 0.4));
      const start = now() - day * 86_400_000 - (j + 1) * 4 * 3_600_000;
      playSessions.push({ id: `s${k++}`, gameId: libGames[gi].id, startedAt: start, endedAt: start + secs * 1000, seconds: secs });
    }
  }
  for (const g of libGames) {
    const mine = playSessions.filter((s) => s.gameId === g.id);
    g.playedSeconds = mine.reduce((t, s) => t + s.seconds, 0);
    g.sessions = mine.length;
    g.lastPlayed = mine.length ? Math.max(...mine.map((s) => s.endedAt ?? s.startedAt)) : null;
  }
  libGames[0].runningSince = now() - 47 * 60_000;
}

// ---- Oyun Arşivi ----
const sc = (g: number, st: number | null, e: number, v: number, p: number | null) => ({ gameplay: g, story: st, enjoyment: e, visuals: v, performance: p });
const reviews: Review[] = (
  [
    ["g0", sc(9, 8, 10, 9, 9), 0, "Bir tur daha derken sabah oldu.", "finished", "yes", "Zagreus'tan bile iyi. ||Son boss'ta Chronos ikinci fazda saati durduruyor||, kalkanı aç."],
    ["g1", sc(8, null, 8, 7, 6), 0, "Arkadaşlarla akşamları vazgeçilmez.", "playing", "yes", null],
    ["g2", sc(10, 9, 10, 10, 6), 0.5, "Zor ama her ölüm bir ders.", "completed", "yes", "Malenia için bleed build."],
    ["g3", sc(8, 6, 9, 8, 10), 0, "Kafa dağıtmak için birebir.", "playing", "maybe", null],
    ["g4", sc(6, null, 5, 7, 7), 0, "Eskisi gibi değil.", "dropped", "no", null],
    ["g7", sc(9, 8, 9, 9, 9), 0, "Metroidvania'nın zirvesi.", "finished", "yes", null],
    ["g8", sc(8, 10, 9, 10, 7), 0, "Arthur'u unutmak mümkün değil.", "finished", "yes", "PS4'te oynadım."],
  ] as [string, Scores, number, string, ReviewStatus, Recommend, string | null][]
).map(([gid, scores, adjust, oneLiner, status, recommend, notes], i) => ({
  id: `r${i}`, author: "me", subjectKind: "game" as const, subjectId: gid, scores, adjust, extra: null,
  overall: overallOf(scores, adjust) ?? 7, oneLiner, notes, status, recommend,
  finishedAt: status === "finished" || status === "completed" ? now() - i * 20 * 86_400_000 : null,
  createdAt: now() - i * 86_400_000, updatedAt: now() - i * 86_400_000,
}));

// ---- Algılanan, kütüphanede olmayan oyunlar ----
const suggestions = [
  { path: "d:\\games\\gtav\\gta5.exe", exe: "gta5.exe", name: "Grand Theft Auto V", reason: "discord" as const, seenAt: now() - 5 * 60_000 },
  { path: "c:\\oyunlar\\hollow knight\\hollow_knight.exe", exe: "hollow_knight.exe", name: "Hollow Knight", reason: "fullscreen" as const, seenAt: now() - 3 * 86_400_000 },
];

// ---- Oyun Modu ----
let gameMode = { active: false, reason: null as "manual" | "auto" | null, since: null as number | null, quiet: true, stopBackground: true };

// ---- Kliplerim ----
const clipList: Clip[] = (
  [
    ["g1", "nvidia", "clip", "Counter-strike 2 2026.10.11 - 21.33.12.02.DVR", "ACE · Mirage", 30, 48_000_000, "#c9a35a", 0.2, true],
    ["g1", "nvidia", "clip", "Counter-strike 2 2026.10.11 - 20.10.44.01.DVR", null, 30, 46_000_000, "#a8843a", 1.5, false],
    ["g1", "nvidia", "clip", "Counter-strike 2 2026.10.10 - 23.02.10.03.DVR", "Clutch 1v3", 60, 95_000_000, "#8a6a2a", 20, false],
    ["g0", "xbox", "clip", "Hades II 2026-10-11 18-05-00", null, 45, 31_000_000, "#7848a8", 3, false],
    ["g0", "steam", "shot", "20261011181233_1", null, null, 2_400_000, "#9a58c8", 3.2, true],
    ["g0", "steam", "shot", "20261011181530_1", null, null, 2_100_000, "#5a3888", 3.3, false],
    ["g2", "nvidia", "clip", "Elden Ring 2026.10.09 - 22.40.11.04.DVR", "Malenia ikinci faz", 120, 230_000_000, "#8a7646", 50, true],
    ["g5", "nvidia", "clip", "Valorant 2026.10.11 - 12.40.00.01.DVR", null, 20, 28_000_000, "#e06a6a", 8, false],
    ["g5", "nvidia", "shot", "Valorant Screenshot 2026.10.11 - 12.41.00.01", null, null, 3_100_000, "#c04a4a", 8.1, false],
    [null, "obs", "clip", "2026-10-08 19-00-12", null, 300, 410_000_000, "#3a5a7a", 70, false],
  ] as [string | null, Clip["source"], Clip["kind"], string, string | null, number | null, number, string, number, boolean][]
).map(([gameId, source, kind, name, title, duration, size, color, hoursAgo, favorite], i) => ({
  id: `c${i}`, path: "/ornek/klip.mp4", name, kind, source, gameId, gameHint: gameId ? null : "Desktop", title, size, duration,
  width: 1920, height: 1080, thumb: thumb(color, "#121417", (title ?? name).slice(0, 18).toUpperCase()), favorite,
  createdAt: now() - hoursAgo * 3_600_000,
}));

// ---- Film & Dizi Arşivi ----
const poster = (c: string, t: string) =>
  "data:image/svg+xml," + encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="200" height="300"><defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${c}"/><stop offset="1" stop-color="#0c0d10"/></linearGradient></defs><rect width="200" height="300" fill="url(#g)"/><text x="14" y="270" font-family="Segoe UI,sans-serif" font-size="20" font-weight="800" fill="rgba(255,255,255,.9)">${t}</text></svg>`);
const titles: Title[] = (
  [
    ["t0", "movie", "Başlangıç", "Inception", 2010, "#2a4a6a", ["Christopher Nolan"], ["Leonardo DiCaprio", "Joseph Gordon-Levitt", "Elliot Page"], ["Bilimkurgu", "Aksiyon", "Gerilim"], 148, null],
    ["t1", "series", "Breaking Bad", null, 2008, "#4a6a2a", ["Vince Gilligan"], ["Bryan Cranston", "Aaron Paul"], ["Suç", "Dram", "Gerilim"], null, 5],
    ["t2", "movie", "Esaretin Bedeli", "The Shawshank Redemption", 1994, "#6a4a2a", ["Frank Darabont"], ["Tim Robbins", "Morgan Freeman"], ["Dram"], 142, null],
    ["t3", "series", "Dark", null, 2017, "#2a2a4a", ["Baran bo Odar", "Jantje Friese"], ["Louis Hofmann", "Lisa Vicari"], ["Bilimkurgu", "Gizem"], null, 3],
    ["t4", "movie", "Yıldızlararası", "Interstellar", 2014, "#3a3a5a", ["Christopher Nolan"], ["Matthew McConaughey", "Anne Hathaway"], ["Bilimkurgu", "Dram"], 169, null],
    ["t5", "movie", "Dune: İkinci Bölüm", "Dune: Part Two", 2024, "#7a5a2a", ["Denis Villeneuve"], ["Timothée Chalamet", "Zendaya"], ["Bilimkurgu", "Macera"], 166, null],
  ] as [string, "movie" | "series", string, string | null, number, string, string[], string[], string[], number | null, number | null][]
).map(([id, kind, name, original, year, color, directors, cast, genres, runtime, seasons], i) => ({
  id, kind, name, original, year, poster: poster(color, name.toUpperCase().slice(0, 14)),
  summary: "Kısa özet Türkçe Vikipedi'den gelir. Filmin konusu birkaç cümleyle anlatılır; spoiler içermez.",
  directors, cast, genres, runtime, seasons, episodes: seasons ? seasons * 10 : null, wikiTitle: original ?? name, addedAt: now() - i * 86_400_000,
}));
const tsc = (st: number, ac: number, di: number, pa: number, en: number) => ({ story: st, acting: ac, direction: di, pace: pa, enjoyment: en });
const titleReviews: Review[] = (
  [
    ["t0", tsc(9, 9, 10, 8, 10), "Rüya içinde rüya, sonu hâlâ tartışılıyor.", "watched", "yes", { where: "Sinema", with: "Arkadaşlarla" }],
    ["t1", tsc(10, 10, 9, 8, 10), "Say my name.", "watching", "yes", { season: 4, episode: 6, where: "Netflix" }],
    ["t2", tsc(10, 10, 9, 8, 10), "Umut asla ölmez.", "watched", "yes", { where: "Televizyon" }],
    ["t3", tsc(9, 8, 9, 6, 8), "Kafa karıştırıcı ama değer.", "dropped", "maybe", { season: 2, episode: 3, where: "Netflix" }],
    ["t5", null, "Herkes ikincisinin daha iyi olduğunu söylüyor.", "watchlist", null, null],
  ] as [string, Scores | null, string, ReviewStatus, Recommend | null, ReviewExtra | null][]
).map(([tid, scores, oneLiner, status, recommend, extra], i) => ({
  id: `tr${i}`, author: "me", subjectKind: "title" as const, subjectId: tid, scores: scores ?? {}, adjust: 0, extra,
  overall: scores ? overallOf(scores, 0, "title") : null, oneLiner, notes: null, status, recommend,
  finishedAt: status === "watched" ? now() - i * 30 * 86_400_000 : null, createdAt: now() - i * 86_400_000, updatedAt: now() - i * 86_400_000,
}));

// ---- Kendi çalma listeleri ----
const userPlaylists: UserPlaylist[] = [];

// ---- Sesle yazma ----
const dictation: DictationStatus = {
  supported: true, enabled: false, whisperReady: true, gpuInstalled: false, gpuActive: null, gpuError: null,
  llmInstalled: false, phase: "idle", lastError: null,
  history: [
    { text: "Yarın akşam 9'da Valorant turnuvası var, herkes Discord'da olsun.\n\nKazanan takıma Steam kartı veriyoruz.", raw: "yarın akşam dokuzda valorant turnuvası var herkes discordda olsun ee kazanan takıma steam kartı veriyoruz", at: now() - 120_000, ms: 1840, tidied: true },
    { text: "Toplantı cuma günü saat 3'te.", raw: "toplantı perşembe yok yok cuma günü saat üçte", at: now() - 600_000, ms: 2210, tidied: true },
  ],
};

// ---- Yazılar ----
const lyric = (start: number, text: string): Segment => {
  const words = text.split(" ");
  const step = 2.6 / words.length;
  return { start, end: start + 3, text, words: words.map((w, i) => ({ start: start + i * step, end: start + (i + 1) * step, text: w })) };
};
const transcripts: Record<string, Transcript> = {};
const demoLyrics = [
  "Gece yine uzun, yol yine sessiz", "Farlarda kayan şehir ışıkları", "Radyoda eski bir şarkı çalıyor",
  "Ve ben seni düşünüyorum hâlâ", "Malikânenin kapısı aralık", "Şöminede çıtırdayan odunlar",
  "Bu gece de oyun bitmeyecek", "Sabaha kadar biz buradayız", "Gece sürüşü, gece sürüşü", "Yıldızlar yolu gösteriyor",
];

function preview(body: string) {
  return body.split("\n").map((l) => l.replace(/^[#>\-*+\s]*(\[[ x]\])?\s*/, "").replace(/[*_`]/g, "").trim()).filter(Boolean).join(" · ").slice(0, 140);
}

function snapshot() {
  return {
    now: now(),
    stopwatch: { running: sw.started !== null, elapsedMs: sw.accum + (sw.started ? now() - sw.started : 0), laps: [...sw.laps] },
    countdowns: countdowns.map((c) => ({ ...c })),
  };
}

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
const cat = (category: string, list: [string, string, string?][], source: InstallItem["source"] = "winget"): InstallItem[] =>
  list.map(([id, name, note]) => ({ id, name, category, source, note: note ?? null, url: null }));
const installCatalog: InstallItem[] = [
  ...cat("Tarayıcılar", [["Google.Chrome", "Google Chrome"], ["Mozilla.Firefox", "Firefox"], ["Brave.Brave", "Brave"], ["Opera.OperaGX", "Opera GX"]]),
  ...cat("Oyun platformları", [["Valve.Steam", "Steam"], ["EpicGames.EpicGamesLauncher", "Epic Games"], ["ElectronicArts.EADesktop", "EA app"], ["Ubisoft.Connect", "Ubisoft Connect"], ["Blizzard.BattleNet", "Battle.net"], ["RiotGames.Valorant.EU", "VALORANT", "Riot Client ile gelir"]]),
  ...cat("İletişim", [["Discord.Discord", "Discord"], ["Telegram.TelegramDesktop", "Telegram"]]),
  { id: "9NKSQGP7F2NH", name: "WhatsApp", category: "İletişim", source: "msstore", note: "Microsoft Store'dan", url: null },
  ...cat("Araçlar", [["Anthropic.Claude", "Claude"], ["RARLab.WinRAR", "WinRAR"], ["7zip.7zip", "7-Zip"], ["Notepad++.Notepad++", "Notepad++"], ["Microsoft.PowerToys", "PowerToys"], ["voidtools.Everything", "Everything", "Anında dosya arama"]]),
  ...cat("Medya", [["VideoLAN.VLC", "VLC"], ["Spotify.Spotify", "Spotify"], ["OBSProject.OBSStudio", "OBS Studio"]]),
  { id: "nvidia-app", name: "NVIDIA App", category: "Donanım", source: "web", url: "https://www.nvidia.com/tr-tr/software/nvidia-app/", note: "winget'te yok: indirme sayfası açılır" },
  ...cat("Donanım", [["Guru3D.Afterburner", "MSI Afterburner"], ["Logitech.GHUB", "Logitech G HUB"], ["CPUID.CPU-Z", "CPU-Z"]]),
  ...cat("Oyun bileşenleri", [["Microsoft.VCRedist.2015+.x64", "Visual C++ (2015-2022)", "Çoğu oyun ister"], ["Microsoft.DirectX", "DirectX (eski oyunlar)"]]),
];
let installList: InstallItem[] = ["Google.Chrome", "Valve.Steam", "Discord.Discord", "Anthropic.Claude", "RARLab.WinRAR", "EpicGames.EpicGamesLauncher", "nvidia-app", "VideoLAN.VLC", "7zip.7zip"]
  .map((i) => installCatalog.find((c) => c.id === i)!)
  .filter(Boolean);
let installRun: InstallRun = { running: false, stopping: false, steps: [], started: null, finished: null };
let installStop = false;
const day = 86_400_000;
const gw = (id: string, title: string, store: string, platforms: string, worth: string | null, ends: number | null, color: string, upcoming = false) => ({
  id, title, store, platforms, worth, ends, upcoming, starts: upcoming ? now() + 4 * day : now() - day,
  image: thumb(color, "#121417", title.toUpperCase()), url: "https://store.epicgames.com/tr/free-games",
});
let dealsSnap = {
  free: [
    gw("epic-1", "Hogwarts Legacy", "epic", "PC, Epic Games Store", "₺1.799,00", now() + 2 * day + 3_600_000, "#6b4aa8"),
    gw("gp-2", "Rogue Lords", "steam", "PC, Steam", "$19.99", now() + 5 * day, "#3a5f8a"),
    gw("gp-3", "Alan Wake", "gog", "PC, GOG", "$14.99", now() + 9 * 3_600_000, "#7a3a3a"),
    gw("gp-4", "Ghostrunner", "prime", "PC, DRM-Free", "$29.99", now() + 20 * day, "#3a7a5a"),
    gw("gp-5", "Sackboy Weekend", "playstation", "PS4, PS5", null, now() + 2 * day, "#2a4a9a"),
    gw("gp-6", "Halo Free Weekend", "xbox", "Xbox Series X|S, Xbox One", null, null, "#2a7a2a"),
  ],
  upcoming: [gw("epic-9", "Mystery Game", "epic", "PC, Epic Games Store", null, now() + 11 * day, "#4a4a4a", true)],
  wishlist: [
    { gameId: "g2", name: "Baldur's Gate 3", cover: thumb("#8a5a2a", "#121417", "BG3"), store: "Steam", price: "$41.99", regular: "$59.99", discount: 30, url: "https://store.steampowered.com/app/1086940" },
    { gameId: "g5", name: "Hades II", cover: thumb("#5a2a6a", "#121417", "HADES"), store: "GOG", price: "$22.49", regular: "$29.99", discount: 25, url: "https://www.gog.com" },
  ],
  wishCount: 4,
  fetchedAt: now() - 40 * 60_000,
  error: null as string | null,
};
let updateStatus = { current: 29 as number | null, latest: null as unknown, available: false, checkedAt: null as number | null, error: null as string | null, progress: null as number | null };

export async function handle(cmd: string, a: Record<string, unknown>): Promise<unknown> {
  switch (cmd) {
    case "audio_state":
      return { supported: true, outputs: outputs.map((d) => ({ ...d })), inputs: inputs.map((d) => ({ ...d })), lock };
    case "audio_set_default": {
      const list = outputs.some((d) => d.id === a.id) ? outputs : inputs;
      list.forEach((d) => (d.isDefault = d.id === a.id));
      emit("audio-changed");
      return;
    }
    case "audio_set_volume": {
      const d = [...outputs, ...inputs].find((x) => x.id === a.id);
      if (d) d.level = a.level as number;
      if (lock !== null && d?.id === "in-1") lock = d.level;
      emit("audio-changed");
      return;
    }
    case "audio_set_mute": {
      const d = [...outputs, ...inputs].find((x) => x.id === a.id);
      if (d) d.muted = a.muted as boolean;
      emit("audio-changed");
      return;
    }
    case "mic_set_lock":
      lock = (a.level as number | null) ?? null;
      if (lock !== null) inputs[0].level = lock;
      emit("audio-changed");
      return { level: inputs[0].level, muted: inputs[0].muted, lock };
    case "hotkeys_list":
      return hotkeys.map((h) => ({ ...h }));
    case "hotkey_set": {
      const h = hotkeys.find((x) => x.id === a.id)!;
      const accel = a.accel === null ? h.defaultAccel : (a.accel as string);
      const other = hotkeys.find((x) => x.id !== h.id && accel && x.accel.toLowerCase() === accel.toLowerCase());
      if (other) throw `Bu kısayol zaten "${other.label}" için kullanılıyor.`;
      h.accel = accel;
      return hotkeys.map((x) => ({ ...x }));
    }
    case "notes_list":
      return notes
        .filter((n) => (a.trash ? n.deletedAt !== null : n.deletedAt === null))
        .sort((x, y) => Number(y.pinned) - Number(x.pinned) || y.updatedAt - x.updatedAt)
        .map((n) => ({ id: n.id, title: n.title, preview: preview(n.body), color: n.color, pinned: n.pinned, updatedAt: n.updatedAt, deletedAt: n.deletedAt }));
    case "note_get":
      return notes.find((n) => n.id === a.id) ?? null;
    case "note_save": {
      const input = a.input as NoteInput;
      let n = notes.find((x) => x.id === input.id);
      if (!n) {
        n = { ...input, id: input.id ?? id(), createdAt: now(), updatedAt: now(), deletedAt: null };
        notes.push(n);
      } else Object.assign(n, input, { updatedAt: now() });
      emit("notes-changed", n.id);
      return { ...n };
    }
    case "note_trash":
    case "note_restore":
    case "note_delete": {
      const i = notes.findIndex((n) => n.id === a.id);
      if (i >= 0) {
        if (cmd === "note_delete") notes.splice(i, 1);
        else notes[i].deletedAt = cmd === "note_trash" ? now() : null;
      }
      emit("notes-changed", a.id);
      return;
    }
    case "open_quick_note":
    case "open_sticky":
    case "close_sticky":
      return;
    case "reminders_list":
      return [...reminders].sort((x, y) => Number(x.done) - Number(y.done) || x.dueAt - y.dueAt);
    case "reminder_save": {
      const input = a.input as ReminderInput;
      if (!input.title.trim()) throw "Hatırlatıcıya bir başlık yaz.";
      let r = reminders.find((x) => x.id === input.id);
      if (!r) {
        r = { ...input, id: id(), notified: false, done: false, createdAt: now() };
        reminders.push(r);
      } else Object.assign(r, input, { notified: false, done: false });
      emit("reminders-changed");
      return r;
    }
    case "reminder_delete": {
      const i = reminders.findIndex((x) => x.id === a.id);
      if (i >= 0) reminders.splice(i, 1);
      emit("reminders-changed");
      return;
    }
    case "reminder_complete": {
      const r = reminders.find((x) => x.id === a.id);
      if (r) {
        if (r.repeat === "none") r.done = true;
        else r.dueAt += 86_400_000;
        r.notified = false;
      }
      emit("reminders-changed");
      emit("alerts-changed");
      return;
    }
    case "reminder_snooze": {
      const r = reminders.find((x) => x.id === a.id);
      if (r) {
        r.dueAt = now() + (a.minutes as number) * 60_000;
        r.notified = false;
      }
      emit("reminders-changed");
      emit("alerts-changed");
      return;
    }
    case "timers_state":
      return snapshot();
    case "stopwatch_toggle":
      if (sw.started) {
        sw.accum += now() - sw.started;
        sw.started = null;
      } else sw.started = now();
      return snapshot();
    case "stopwatch_lap":
      if (sw.started) sw.laps.push(sw.accum + now() - sw.started);
      return snapshot();
    case "stopwatch_reset":
      Object.assign(sw, { started: null, accum: 0, laps: [] });
      return snapshot();
    case "countdown_add":
      countdowns.push({ id: id(), label: (a.label as string) || "Geri sayım", totalMs: a.ms as number, endsAt: now() + (a.ms as number), remainingMs: a.ms as number, finished: false });
      return snapshot();
    case "countdown_toggle": {
      const c = countdowns.find((x) => x.id === a.id);
      if (c) {
        if (c.finished) Object.assign(c, { finished: false, endsAt: now() + c.totalMs, remainingMs: c.totalMs });
        else if (c.endsAt) Object.assign(c, { remainingMs: Math.max(0, c.endsAt - now()), endsAt: null });
        else c.endsAt = now() + c.remainingMs;
      }
      return snapshot();
    }
    case "countdown_remove": {
      const i = countdowns.findIndex((x) => x.id === a.id);
      if (i >= 0) countdowns.splice(i, 1);
      emit("alerts-changed");
      return snapshot();
    }
    case "take_osd":
      return { id: 1, icon: "lock", text: "Mikrofon %100 seviyesine geri çekildi" };
    case "alerts_list":
      return {
        reminders: reminders.filter((r) => r.notified && !r.done),
        timers: [{ id: "c9", label: "Fırın", totalMs: 1_200_000, endsAt: null, remainingMs: 0, finished: true }],
        games: suggestions.slice(0, 1),
      };
    case "alerts_fit":
    case "alerts_close":
      return;
    case "servers_games":
      return games;
    case "servers_list":
      return servers.map((x) => ({ ...x }));
    case "servers_sync":
      await new Promise((r) => setTimeout(r, 300));
      return {
        servers: servers.map((x) => ({ ...x })), error: null,
        url: "https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/topluluk/sunucular.json",
        defaultUrl: "https://raw.githubusercontent.com/BalamanMerkut/GMID--Personel_Tool/HEAD/topluluk/sunucular.json",
      };
    case "servers_cached":
      return [];
    case "servers_query":
      await new Promise((r) => setTimeout(r, 700));
      return servers.map((x) => statuses[x.id] ?? { id: x.id, state: games.find((g) => g.id === x.game)?.queryable ? "offline" : "unknown",
        info: null, error: "Yanıt vermedi", checkedAt: now(), stale: false });
    case "server_save": {
      const input = a.input as ServerInput;
      if (!/^[a-z0-9.-]+(:\d+)?$/i.test(input.host.trim())) throw "Adres geçersiz. Örnek: 185.12.34.56 ya da cod2.gmid.gg";
      const game = games.find((g) => g.id === input.game)!;
      let [host, port] = input.host.trim().split(":");
      const srv: Server = {
        id: input.id ?? id(), name: input.name.trim() || `${game.name} sunucusu`, game: input.game, host: host.toLowerCase(),
        port: Number(port) || input.port || game.defaultPort, queryPort: input.queryPort, logo: input.logo,
        description: input.description, connectNotes: input.connectNotes, remote: false, createdAt: now(),
      };
      const i = servers.findIndex((x) => x.id === srv.id);
      if (i >= 0) servers[i] = srv;
      else servers.push(srv);
      return srv;
    }
    case "server_delete": {
      const i = servers.findIndex((x) => x.id === a.id);
      if (i >= 0) servers.splice(i, 1);
      return;
    }
    case "server_join": {
      const srv = servers.find((x) => x.id === a.id)!;
      const game = games.find((g) => g.id === srv.game)!;
      const address = `${srv.host}:${srv.port}`;
      if (game.join === "exe") throw `EXE_NEEDED:${game.id}`;
      return { kind: game.join === "copy" ? "copy" : "launched", address };
    }
    case "server_set_exe":
      return;
    case "videos_tools":
      return {
        ready: toolsReady, installing: false, version: toolsReady ? "2026.08.19" : null,
        tools: [
          { id: "yt-dlp", name: "yt-dlp", purpose: "Videoyu bulur ve indirir", installed: toolsReady, sizeMb: 18 },
          { id: "ffmpeg", name: "FFmpeg", purpose: "Görüntü ile sesi birleştirir, MP3'e çevirir", installed: toolsReady, sizeMb: 89 },
          { id: "deno", name: "Deno", purpose: "YouTube'un doğrulamasını çözer", installed: toolsReady, sizeMb: 43 },
        ],
      };
    case "videos_install_tools":
    case "videos_update":
      return handle("videos_tools", {});
    case "videos_probe": {
      await new Promise((r) => setTimeout(r, 900));
      const u = String(a.url);
      if (!/^https?:\/\//.test(u)) throw "Geçerli bir bağlantı yapıştır (https://… ile başlamalı).";
      if (u.includes("/playlist") || (a.wholeList && u.includes("list="))) {
        const songs = ["Lo-fi Malikâne", "Gece Sürüşü", "Yağmurlu Kütüphane", "Şömine Başında", "Kale Bahçesi", "Ay Işığı Sonatı (lofi)", "Sabah Kahvesi", "Kuzey Rüzgârı"];
        return {
          kind: "playlist", url: u, title: "GMID Lofi · Çalışma ve Oyun", uploader: "Chill Malikâne", site: "YoutubeTab",
          entries: songs.map((t, i) => ({ id: `s${i}`, url: `https://www.youtube.com/watch?v=s${i}`, title: t, duration: 140 + i * 23, uploader: "Chill Malikâne",
            thumbnail: thumb(["#35a0a2", "#7848a8", "#e3c270", "#0b203e"][i % 4], "#121417", String(i + 1)) })),
        };
      }
      return { kind: "video", ...probeResult };
    }
    case "videos_download_playlist":
      return (a.ids as string[] | null)?.length ?? 8;
    case "videos_download": {
      const o = probeResult.options.find((x) => x.id === a.option)!;
      const d = dl({ title: probeResult.title, uploader: probeResult.uploader, thumbnail: probeResult.thumbnail, status: "queued", progress: 0,
        quality: o.audioOnly ? "MP3" : `${o.id}p`, audioOnly: o.audioOnly, size: o.size, finishedAt: null, createdAt: now() });
      downloadsList.unshift(d);
      emit("downloads-changed");
      return d;
    }
    case "videos_list":
      return downloadsList.map((x) => ({ ...x, progress: liveMap[x.id]?.progress ?? x.progress }));
    case "videos_live":
      return Object.values(liveMap);
    case "videos_pause":
    case "videos_resume":
    case "videos_cancel":
    case "videos_remove": {
      const d = downloadsList.find((x) => x.id === a.id);
      if (d) {
        if (cmd === "videos_pause") { d.status = "paused"; delete liveMap[d.id]; }
        if (cmd === "videos_resume") d.status = "queued";
        if (cmd === "videos_cancel" || cmd === "videos_remove") downloadsList.splice(downloadsList.indexOf(d), 1);
      }
      emit("downloads-changed");
      emit("downloads-progress", Object.values(liveMap));
      return;
    }
    case "transcript_get": {
      const d = downloadsList.find((x) => x.id === a.id);
      if (d && d.title === "Gece Sürüşü" && !transcripts[d.id])
        transcripts[d.id] = { downloadId: d.id, source: "lyrics", language: "tr", createdAt: now(), segments: demoLyrics.map((t, i) => lyric(1 + i * 3.2, t)) };
      return transcripts[a.id as string] ?? null;
    }
    case "transcript_create": {
      for (const p of [0.1, 0.35, 0.6, 0.85, 1]) {
        await new Promise((r) => setTimeout(r, 300));
        emit("transcript-progress", { id: a.id, stage: "whisper", progress: p });
      }
      transcripts[a.id as string] = { downloadId: a.id as string, source: a.mode === "whisper" ? "whisper" : a.mode === "lyrics" ? "lyrics" : "subtitles", language: "tr", createdAt: now(),
        segments: ["Merhaba, malikâneye hoş geldin.", "Bugün yeni bir oyuna bakıyoruz.", "Önce ayarları açalım.", "Grafikler gerçekten çok güzel."].map((t, i) => lyric(0.5 + i * 3.4, t)) };
      return transcripts[a.id as string];
    }
    case "transcript_delete":
      delete transcripts[a.id as string];
      return;
    case "media_tracks": {
      const d = downloadsList.find((x) => x.id === a.id);
      return d && !d.audioOnly
        ? [{ file: "a.tr.vtt", lang: "tr", label: "Türkçe" }, { file: "a.en.vtt", lang: "en", label: "İngilizce" }]
        : [];
    }
    case "media_track": {
      const tr = String(a.file).includes(".tr.");
      const lines = tr
        ? ["Bu dünya hepimizin.", "Onu korumak zorundayız.", "Bak, denizler ağlıyor.", "Peki ya biz ne yapıyoruz?"]
        : ["This world belongs to all of us.", "We have to protect it.", "Look, the seas are crying.", "And what are we doing?"];
      return Array.from({ length: 40 }, (_, i) => ({ start: 1 + i * 3, end: 3.6 + i * 3, text: lines[i % 4] }));
    }
    case "games_list":
      return libGames;
    case "games_scan":
      await new Promise((r) => setTimeout(r, 900));
      return [
        ...libGames.filter((g) => g.source !== "manual").map((g) => ({ source: g.source, sourceId: g.sourceId, name: g.name, installDir: null, exes: [], launch: null, steamAppid: g.steamAppid, platformMinutes: g.platformMinutes, lastPlayed: null, already: true })),
        { source: "steam", sourceId: "413150", name: "Baldur's Gate 3", installDir: "D:\\SteamLibrary\\steamapps\\common\\Baldurs Gate 3", exes: ["bg3.exe"], launch: null, steamAppid: 1086940, platformMinutes: 6120, lastPlayed: now() - 20 * 86_400_000, already: false },
        { source: "epic", sourceId: "Kinglet", name: "Rocket League", installDir: null, exes: [], launch: null, steamAppid: null, platformMinutes: 0, lastPlayed: null, already: false },
      ];
    case "games_import":
      return (a.found as unknown[]).length;
    case "games_running_apps":
      return [{ name: "riotclientux.exe", path: "c:\\riot games\\riot client\\riotclientux.exe", title: "Riot Client" }];
    case "games_add_manual":
    case "games_set_hidden":
    case "games_rename":
    case "games_remove":
    case "games_refresh_meta":
    case "games_launch":
    case "games_reveal":
      if (cmd === "games_set_hidden") libGames.find((g) => g.id === a.id)!.hidden = a.hidden as boolean;
      return;
    case "games_suggestions":
      return suggestions;
    case "games_suggestion_answer": {
      const i = suggestions.findIndex((x) => x.path === a.path);
      if (i >= 0 && a.action !== "later") {
        if (a.action === "add") libGames.push({ ...libGames[0], id: `g${Date.now()}`, name: String(a.name ?? suggestions[i].name), source: "manual", runningSince: null, cover: thumb("#4a6a4a", "#121417", String(a.name ?? suggestions[i].name).toUpperCase()) });
        suggestions.splice(i, 1);
      }
      emit("games-suggestions");
      emit("games-changed");
      return;
    }
    case "games_set_cover":
      return;
    case "gamemode_status":
      return gameMode;
    case "gamemode_set":
      gameMode = a.on ? { ...gameMode, active: true, reason: "manual", since: now() } : { ...gameMode, active: false, reason: null, since: null };
      emit("gamemode", gameMode);
      return gameMode;
    case "community_discord":
      return "https://discord.gg/gmid";
    case "fps_status":
      return { state: "missing", relogin: false, message: null };
    case "fps_setup":
      await wait(1200);
      return { state: "permission", relogin: true, message: "Windows oturumunu bir kez kapatıp aç (ya da bilgisayarı yeniden başlat), sonra FPS görünür." };
    case "panel_perf":
      return {
        cpu: 30 + Math.random() * 25, ramUsed: 14.2e9, ramTotal: 32e9, gpu: 85 + Math.random() * 12, gpuTemp: 66 + Math.random() * 4,
        vramUsed: 7.1e9, vramTotal: 12e9, gpuName: "NVIDIA GeForce RTX 4070", fps: 140 + Math.random() * 20, fpsApp: "VALORANT-Win64-Shipping.exe", rtss: false, fpsSource: "presentmon", fpsState: "running",
      };
    case "panel_hide":
    case "panel_focus":
    case "panel_interact":
      return;
    case "clips_list":
      return clipList;
    case "clips_roots":
      return [
        { path: "C:\\Users\\Merkut\\Videos", label: "Videolar (NVIDIA, Xbox Game Bar, AMD, OBS)", custom: false, exists: true },
        { path: "C:\\Program Files (x86)\\Steam\\userdata", label: "Steam ekran görüntüleri", custom: false, exists: true },
      ];
    case "clips_rescan":
    case "clips_open":
    case "clips_reveal":
      return;
    case "clips_set_favorite":
    case "clips_rename":
    case "clips_set_game":
    case "clips_delete": {
      const c = clipList.find((x) => x.id === a.id);
      if (!c) return;
      if (cmd === "clips_set_favorite") c.favorite = a.on as boolean;
      if (cmd === "clips_rename") c.title = String(a.title).trim() || null;
      if (cmd === "clips_set_game") c.gameId = (a.game as string) ?? null;
      if (cmd === "clips_delete") clipList.splice(clipList.indexOf(c), 1);
      emit("clips-changed");
      return;
    }
    case "clips_plan": {
      const o = a.options as { start?: number; end?: number; limitMb: number | null; codec: string };
      const c = clipList.find((x) => x.id === a.id);
      const d = Math.max(0.5, (o.end ?? c?.duration ?? 30) - (o.start ?? 0));
      if (o.limitMb == null) return { video: 0, audio: 192, height: c?.height ?? 1080, fps: 60, quality: "great" };
      const kbps = (o.limitMb * 1e6 * 0.94 * 8) / 1000 / d;
      const bpp = o.codec === "av1" ? 0.022 : 0.065;
      const h = [1440, 1080, 720, 540, 480, 360].filter((x) => x <= (c?.height ?? 1080)).find((x) => (kbps * 1000) / (x * x * 1.777 * 30) >= bpp);
      const fps = h && (kbps * 1000) / (h * h * 1.777 * 60) >= bpp ? 60 : 30;
      return { video: Math.round(kbps - 96), audio: 96, height: h ?? 360, fps, quality: !h ? "poor" : h >= 1080 ? "great" : h >= 720 ? "good" : h >= 480 ? "ok" : "poor" };
    }
    case "clips_fit_discord": {
      for (let k = 1; k <= 10; k++) {
        await new Promise((r) => setTimeout(r, 150));
        emit("clip-fit", { id: a.id, progress: k / 10 });
      }
      return "C:\\Users\\Merkut\\Documents\\GMID Tool\\Discord'a Hazır\\klip (Discord).mp4";
    }
    case "reviews_list":
      return a.kind === "title" ? titleReviews : reviews;
    case "review_save": {
      const input = a.input as ReviewInput;
      const list = input.subjectKind === "title" ? titleReviews : reviews;
      const wish = input.status === "wishlist" || input.status === "watchlist";
      const overall = wish ? null : overallOf(input.scores, input.adjust, input.subjectKind);
      if (!wish && overall == null) throw "En az bir soruya puan ver";
      if (wish) input.recommend = null;
      const old = list.find((r) => r.subjectId === input.subjectId);
      const r: Review = { id: old?.id ?? `r${Date.now()}`, author: "me", ...input, extra: input.extra ?? null, adjust: Math.max(-1, Math.min(1, input.adjust)), overall,
        finishedAt: ["finished", "completed", "watched"].includes(input.status ?? "") ? (old?.finishedAt ?? now()) : null,
        createdAt: old?.createdAt ?? now(), updatedAt: now() };
      if (old) Object.assign(old, r);
      else list.unshift(r);
      emit(input.subjectKind === "title" ? "titles-changed" : "reviews-changed");
      return r;
    }
    case "review_delete": {
      const i = reviews.findIndex((r) => r.id === a.id);
      if (i >= 0) {
        const gid = reviews[i].subjectId;
        reviews.splice(i, 1);
        const gi = libGames.findIndex((g) => g.id === gid && !g.inLibrary);
        if (gi >= 0) libGames.splice(gi, 1);
      }
      const j = titleReviews.findIndex((r) => r.id === a.id);
      if (j >= 0) {
        const tid = titleReviews[j].subjectId;
        titleReviews.splice(j, 1);
        titles.splice(titles.findIndex((t) => t.id === tid), 1);
        emit("titles-changed");
      }
      return;
    }
    case "titles_list":
      return titles;
    case "titles_search": {
      const q = String(a.term).toLocaleLowerCase("tr-TR");
      const hits: TitleHit[] = [
        { wikiTitle: "The Matrix", name: "The Matrix", description: "1999 film by the Wachowskis", image: poster("#1a4a2a", "MATRIX"), wikidata: "Q83495", kind: "movie", year: 1999 },
        { wikiTitle: "The Matrix Reloaded", name: "The Matrix Reloaded", description: "2003 film by the Wachowskis", image: poster("#1a3a2a", "RELOADED"), wikidata: "Q189600", kind: "movie", year: 2003 },
        { wikiTitle: "Dark (TV series)", name: "Dark", description: "German science fiction television series", image: poster("#2a2a4a", "DARK"), wikidata: "Q27877", kind: "series", year: null },
      ];
      return hits.filter((h) => h.name.toLocaleLowerCase("tr-TR").includes(q) || q.length < 3);
    }
    case "titles_add": {
      const h = a.hit as TitleHit | null;
      const t: Title = { id: `t${Date.now()}`, kind: h?.kind ?? ((a.kind as "movie" | "series") || "movie"), name: h?.name ?? String(a.name), original: null,
        year: h?.year ?? null, poster: h?.image ?? null, summary: null, directors: [], cast: [], genres: [], runtime: null, seasons: null, episodes: null,
        wikiTitle: h?.wikiTitle ?? null, addedAt: now() };
      titles.unshift(t);
      return t;
    }
    case "titles_discard": {
      if (!titleReviews.some((r) => r.subjectId === a.id)) {
        const i = titles.findIndex((t) => t.id === a.id);
        if (i >= 0) titles.splice(i, 1);
      }
      return;
    }
    case "titles_refresh":
      return;
    case "games_store_search": {
      const t = String(a.term).toLocaleLowerCase("tr-TR");
      return [
        { appid: 1086940, name: "Baldur's Gate 3", image: thumb("#6a3a2a", "#121417", "BG3") },
        { appid: 367520, name: "Hollow Knight", image: thumb("#4a6a9a", "#121417", "HK") },
        { appid: 1145360, name: "Hades", image: thumb("#a84838", "#121417", "HADES") },
        { appid: 504230, name: "Celeste", image: thumb("#8a4a9a", "#121417", "CELESTE") },
      ].filter((h) => h.name.toLocaleLowerCase("tr-TR").includes(t));
    }
    case "games_add_archive": {
      const id = `g${Date.now()}`;
      libGames.push({ ...libGames[0], id, name: String(a.name), source: a.appid ? "steam" : "archive", steamAppid: (a.appid as number) ?? null,
        cover: thumb("#3a5a7a", "#121417", String(a.name).toUpperCase()), tags: [], genres: [], inLibrary: false, developer: null, year: null,
        playedSeconds: 0, sessions: 0, lastPlayed: null, runningSince: null, platformMinutes: 0 });
      return id;
    }
    case "games_sessions":
      return playSessions.filter((s) => (s.endedAt ?? s.startedAt) >= (a.since as number));
    case "playlists_list":
      return userPlaylists;
    case "playlist_save": {
      const items = (a.items as string[]).filter((x, i, all) => all.indexOf(x) === i);
      const existing = userPlaylists.find((p) => p.id === a.id);
      if (existing) {
        existing.name = String(a.name);
        existing.items = items;
        return existing;
      }
      const p = { id: id(), name: String(a.name), createdAt: now(), items };
      userPlaylists.push(p);
      return p;
    }
    case "playlist_delete":
      userPlaylists.splice(userPlaylists.findIndex((p) => p.id === a.id), 1);
      return;
    case "dictation_view_state": {
      // Tarayıcı önizlemesi: ?view=dikte&demo=listening|processing|error
      const demo = new URLSearchParams(location.search).get("demo") ?? "listening";
      if (demo === "listening") setInterval(() => emit("dikte-level", Math.random() * 0.8), 80);
      return { state: demo, secs: 7 };
    }
    case "dictation_status":
    case "dictation_apply":
    case "dictation_warm_up":
    case "dictation_install": {
      if (cmd === "dictation_install") await new Promise((r) => setTimeout(r, 800));
      if (cmd === "dictation_install" && a.part === "gpu") dictation.gpuInstalled = true;
      if (cmd === "dictation_install" && a.part === "llm") dictation.llmInstalled = true;
      if (cmd === "dictation_install" && a.part === "whisper") dictation.whisperReady = true;
      if (cmd === "dictation_warm_up") dictation.gpuActive = dictation.gpuInstalled;
      try {
        dictation.enabled = JSON.parse(localStorage.getItem("gmid.settings") ?? "{}")["dictation.enabled"] === "true";
      } catch {
        /* tarayıcı depolaması kapalı */
      }
      return { ...dictation };
    }
    case "whisper_status":
      return { program: false, ready: false, selected: "turbo", models: [
        { id: "turbo", label: "Dengeli (önerilen)", detail: "large-v3-turbo · en iyi doğruluğa çok yakın, hızlı", sizeMb: 550, installed: false },
        { id: "small", label: "Hızlı", detail: "small · eski bilgisayarlar için, daha az doğru", sizeMb: 190, installed: false },
      ] };
    case "whisper_install":
      return { program: true, ready: true, selected: a.model, models: [] };
    case "videos_repair_thumb":
      return false;
    case "videos_open":
    case "videos_reveal":
      return;
    case "bridge_status":
      return { port: 47913, paired: true, lastSeen: now() - 120_000, browser: "Chrome" };
    case "bridge_pair":
      return "482915";
    case "bridge_unpair":
      return;
    case "extension_install":
      return "C:\\Users\\Sen\\Documents\\GMID Tool\\Eklenti";
    case "search_all": {
      const q = String(a.query).toLocaleLowerCase("tr");
      if (q.trim().length < 2) return [];
      const has = (t: string) => t.toLocaleLowerCase("tr").includes(q);
      return [
        ...notes.filter((n) => has(n.title) || has(n.body)).slice(0, 6).map((n) => ({ kind: "note", id: n.id, title: n.title || "Adsız not", sub: n.body.replace(/[#*\n]/g, " ").slice(0, 80), image: null })),
        ...libGames.filter((g) => has(g.name)).slice(0, 6).map((g) => ({ kind: "game", id: g.id, title: g.name, sub: "Kütüphanem", image: g.cover })),
        ...titles.filter((t) => has(t.name)).slice(0, 6).map((t) => ({ kind: "title", id: t.id, title: t.name, sub: `${t.kind === "series" ? "Dizi" : "Film"} · ${t.year ?? ""}`, image: t.poster })),
        ...clipList.filter((c) => has(c.title ?? c.name)).slice(0, 6).map((c) => ({ kind: "clip", id: c.id, title: c.title ?? c.name, sub: c.kind === "clip" ? "Klip" : "Ekran görüntüsü", image: c.thumb })),
      ];
    }
    case "wrapped_get": {
      const h = (n: number) => n * 3600;
      const hours = Array.from({ length: 24 }, (_, i) => (i < 3 ? h(6) : i >= 20 ? h(9) : i >= 14 ? h(2) : 0));
      return {
        from: a.from, to: a.to, totalSeconds: h(87), sessions: 41, daysPlayed: 22, distinctGames: 9,
        top: [
          { name: "VALORANT", seconds: h(38), sessions: 17, cover: thumb("#c94a4a", "#121417", "VALORANT") },
          { name: "Hades II", seconds: h(21), sessions: 9, cover: thumb("#5a2a6a", "#121417", "HADES II") },
          { name: "Counter-Strike 2", seconds: h(14), sessions: 8, cover: thumb("#c9a35a", "#121417", "CS2") },
          { name: "Minecraft", seconds: h(9), sessions: 4, cover: null },
          { name: "Baldur's Gate 3", seconds: h(5), sessions: 3, cover: thumb("#8a5a2a", "#121417", "BG3") },
        ],
        longest: { game: "VALORANT", seconds: h(5) + 1200, at: now() - 6 * 86_400_000 },
        bestWeekday: 5, hours, nightShare: 0.34, weekendShare: 0.48, newGames: 3, ratedGames: 4,
        bestGame: ["Hades II", 9.4], titlesWatched: 6, bestTitle: ["Dark", 9.1], clips: 14, shots: 6, downloads: 11, notes: 7,
        badges: [
          { id: "night", title: "Gece Kuşu", desc: "Oyunlarının %34'i gece 00:00–05:00 arası" },
          { id: "marathon", title: "Maratoncu", desc: "VALORANT ile 5 saatlik tek oturum" },
          { id: "explorer", title: "Kâşif", desc: "9 farklı oyun" },
        ],
      };
    }
    case "wrapped_save":
      return "C:\\Users\\Sen\\Documents\\GMID Tool\\Ozetler\\GMID Wrapped.png";
    case "deals_get":
      return dealsSnap;
    case "deals_refresh":
      await wait(800);
      dealsSnap = { ...dealsSnap, fetchedAt: now() };
      emit("deals", dealsSnap);
      return dealsSnap;
    case "update_status":
      return updateStatus;
    case "update_check":
      await wait(700);
      updateStatus = {
        ...updateStatus,
        checkedAt: now(),
        available: true,
        latest: { build: 31, title: "Klip kırpma ve daha net 'Discord'a sığdır'", notes: "- Kırp ve hazırla penceresi\n- Net (AV1) ya da Uyumlu (H.264)", published: new Date().toISOString(), size: 9_400_000 },
      };
      emit("update", updateStatus);
      return updateStatus;
    case "update_install":
      for (let k = 0; k <= 10; k++) {
        updateStatus = { ...updateStatus, progress: k / 10 };
        emit("update", updateStatus);
        await wait(200);
      }
      return;
    case "install_catalog":
      return installCatalog;
    case "install_list":
      return installList;
    case "install_save_list":
      installList = a.items as InstallItem[];
      return;
    case "install_state":
      return installRun;
    case "install_installed":
      await wait(900);
      return ["google.chrome", "valve.steam", "discord.discord"];
    case "install_search": {
      await wait(700);
      const q = String(a.query).toLowerCase();
      return [
        { id: "Ookla.Speedtest.Desktop", name: "Speedtest by Ookla", version: "1.17.188.1" },
        { id: "WinSCP.WinSCP", name: "WinSCP", version: "6.5.3" },
        { id: "Rufus.Rufus", name: "Rufus", version: "4.15" },
      ].filter((h) => q.length < 2 || h.name.toLowerCase().includes(q) || q === "test");
    }
    case "install_run": {
      const items = a.items as InstallItem[];
      installStop = false;
      installRun = {
        running: true, stopping: false, started: now(), finished: null,
        steps: items.map((i) => ({ id: i.id, name: i.name, state: "waiting", progress: null, message: null })),
      };
      emit("install", installRun);
      void (async () => {
        for (const [k, item] of items.entries()) {
          const st = installRun.steps[k];
          if (installStop) { st.state = "skipped"; st.message = "Durduruldu"; emit("install", { ...installRun }); continue; }
          st.state = "running";
          if (item.source === "web") { st.state = "opened"; st.message = "İndirme sayfası tarayıcıda açıldı"; emit("install", { ...installRun }); continue; }
          for (let p = 0; p <= 1; p += 0.2) { st.progress = p; emit("install", { ...installRun }); await wait(250); }
          st.progress = null; st.message = "Paket yüklemesi başlatılıyor..."; emit("install", { ...installRun }); await wait(500);
          if (item.id === "Discord.Discord") { st.state = "already"; st.message = "Zaten kurulu"; }
          else if (item.id === "Anthropic.Claude") { st.state = "failed"; st.message = "kod 0x80070005: Erişim engellendi"; }
          else { st.state = "done"; st.message = null; }
          emit("install", { ...installRun });
        }
        installRun = { ...installRun, running: false, stopping: false, finished: now() };
        emit("install", installRun);
      })();
      return installRun;
    }
    case "install_stop":
      installStop = true;
      installRun = { ...installRun, stopping: true };
      emit("install", installRun);
      return;
    case "install_find_setup":
      return "C:\\Users\\Sen\\Downloads\\GMID Tool_0.1.0_x64-setup.exe";
    case "install_backup_size": {
      const o = a.options as { videos: boolean; tools: boolean };
      return 38_400_000 + (o.videos ? 4_200_000_000 : 0) + (o.tools ? 2_100_000_000 : 0);
    }
    case "install_export": {
      const r = a.request as { dir: string; items: InstallItem[]; setup: string | null; backup: boolean };
      emit("install-kit", { stage: "script", done: 0, total: 1 });
      await wait(300);
      if (r.backup) for (let d = 0; d <= 10; d++) { emit("install-kit", { stage: "backup", done: d * 3_840_000, total: 38_400_000 }); await wait(150); }
      emit("install-kit", { stage: "done", done: 1, total: 1 });
      return { path: `${r.dir.replace(/\\$/, "")}\\GMID Format Kiti`, programs: r.items.length, setup: !!r.setup, backupSize: r.backup ? 31_200_000 : null };
    }
    case "install_open_kit":
      return;
    case "backup_restore_stage":
      await wait(600);
      return { app: "GMID Tool", version: "0.1.0", created: now() - 3 * 86_400_000, root: "C:\\Users\\Sen\\Documents\\GMID Tool", home: "C:\\Users\\Sen", folders: ["Notlar", "Arkaplanlar", "Kapaklar", "Sunucular", "Eklenti"] };
    case "backup_restore_cancel":
    case "backup_restore_apply":
      return;
    default:
      throw new Error(`Taklit katmanda tanımsız komut: ${cmd}`);
  }
}
