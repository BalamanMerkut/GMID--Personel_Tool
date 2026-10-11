// Tarayıcıda (Tauri olmadan) geliştirirken kullanılan taklit çekirdek. Uygulamanın kendisinde yüklenmez.
import { localBus } from "./bus";
import type {
  Countdown, Device, Download, GameInfo, Live, Probe, Segment, Transcript, HotkeyStatus, Note, NoteInput, Reminder, ReminderInput, Server, ServerInput, ServerStatus,
} from "./types";

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
    case "whisper_status":
      return { program: false, ready: false, selected: "turbo", models: [
        { id: "turbo", label: "Dengeli (önerilen)", detail: "large-v3-turbo · en iyi doğruluğa çok yakın, hızlı", sizeMb: 550, installed: false },
        { id: "small", label: "Hızlı", detail: "small · eski bilgisayarlar için, daha az doğru", sizeMb: 190, installed: false },
      ] };
    case "whisper_install":
      return { program: true, ready: true, selected: a.model, models: [] };
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
    default:
      throw new Error(`Taklit katmanda tanımsız komut: ${cmd}`);
  }
}
