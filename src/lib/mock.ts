// Tarayıcıda (Tauri olmadan) geliştirirken kullanılan taklit çekirdek. Uygulamanın kendisinde yüklenmez.
import { localBus } from "./bus";
import type {
  Countdown, Device, GameInfo, HotkeyStatus, Note, NoteInput, Reminder, ReminderInput, Server, ServerInput, ServerStatus,
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
  ["audio.cycleOutput", "Ses çıkışını değiştir (kulaklık ↔ hoparlör)", "Ctrl+Alt+A"],
  ["mic.toggleMute", "Mikrofonu sessize al / aç", "Ctrl+Alt+M"],
  ["mic.toggleLock", "Mikrofon seviyesi kilidini aç / kapat", "Ctrl+Alt+L"],
  ["mic.up", "Mikrofon +5", "Ctrl+Alt+PageUp"],
  ["mic.down", "Mikrofon −5", "Ctrl+Alt+PageDown"],
  ["mic.max", "Mikrofonu %100'e çek", "Ctrl+Alt+Home"],
  ["note.quick", "Hızlı not", "Ctrl+Alt+N"],
].map(([i, label, a]) => ({ id: i, label, accel: a, defaultAccel: a, error: null }));

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
    default:
      throw new Error(`Taklit katmanda tanımsız komut: ${cmd}`);
  }
}
