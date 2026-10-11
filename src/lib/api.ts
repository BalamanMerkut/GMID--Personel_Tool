// Rust çekirdeğine giden tüm çağrılar burada. Uygulama Tauri dışında (tarayıcıda `npm run dev`)
// açıldığında da çalışsın diye her çağrının küçük bir tarayıcı karşılığı var.
import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface BackupInfo {
  name: string;
  size: number;
  created: string;
}

export interface AppInfo {
  version: string;
  dataDir: string;
  dbSize: number;
  backups: BackupInfo[];
}

const BG_EXTENSIONS = ["jpg", "jpeg", "png", "webp", "gif", "mp4", "webm"];

function browserStore(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem("gmid.settings") ?? "{}");
  } catch {
    return {};
  }
}

import type {
  Alerts, AudioState, BridgeStatus, DictationStatus, FoundGame, Game, PlaySession, RunningApp, Review, ReviewInput, StoreHit, Download, Langs, Segment, Sidecar, UserPlaylist, GameInfo, Live, Parsed, ToolsState, Transcript, WhisperStatus, HotkeyStatus, JoinResult, MicStatus, Note, NoteInput, NoteSummary, OsdMessage,
  Reminder, ReminderInput, Server, ServerInput, ServerStatus, SyncResult, TimersSnapshot,
} from "./types";

/** Bir Rust komutunu çağırır; tarayıcıda taklit katmana gider. */
export async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (inTauri) return invoke<T>(cmd, args);
  const mock = await import("./mock");
  return mock.handle(cmd, args) as Promise<T>;
}

export const api = {
  getSettings: (): Promise<Record<string, string>> =>
    inTauri ? invoke("get_settings") : Promise.resolve(browserStore()),

  setSetting: async (key: string, value: string): Promise<void> => {
    if (inTauri) return invoke("set_setting", { key, value });
    try {
      localStorage.setItem("gmid.settings", JSON.stringify({ ...browserStore(), [key]: value }));
    } catch {
      /* tarayıcı depolaması kapalıysa ayar sadece bu oturumda kalır */
    }
  },

  takeSplash: (): Promise<boolean> => (inTauri ? invoke("take_splash") : Promise.resolve(true)),

  takePendingRoute: (): Promise<string | null> =>
    inTauri ? invoke("take_pending_route") : Promise.resolve(null),

  appInfo: (): Promise<AppInfo> =>
    inTauri
      ? invoke("app_info")
      : Promise.resolve({
          version: "0.1.0",
          dataDir: "C:\\Users\\Sen\\Documents\\GMID Tool",
          dbSize: 24576,
          backups: [{ name: "gmid-2026-10-11_09-00-12.db", size: 24576, created: "2026-10-11 09:00" }],
        }),

  openDataFolder: (sub?: string): Promise<void> =>
    inTauri ? invoke("open_data_folder", { sub: sub ?? null }) : Promise.resolve(),

  /** Günlük dosyasını (gmid.log) açar. */
  openLog: (): Promise<void> => (inTauri ? invoke("open_log") : Promise.resolve()),

  backupNow: (): Promise<BackupInfo[]> => (inTauri ? invoke("backup_now") : api.appInfo().then((i) => i.backups)),

  /** Dosya seçtirir ve `Arkaplanlar` klasörüne kopyalar. Vazgeçilirse `null`. */
  pickBackground: async (): Promise<string | null> => {
    if (!inTauri) return null;
    const { open } = await import("@tauri-apps/plugin-dialog");
    const picked = await open({
      multiple: false,
      directory: false,
      title: "Arka plan seç",
      filters: [{ name: "Resim, GIF veya video", extensions: BG_EXTENSIONS }],
    });
    if (typeof picked !== "string") return null;
    return invoke<string>("import_background", { source: picked });
  },

  autostart: {
    isEnabled: async (): Promise<boolean> => {
      if (!inTauri) return false;
      const m = await import("@tauri-apps/plugin-autostart");
      return m.isEnabled();
    },
    set: async (on: boolean): Promise<void> => {
      if (!inTauri) return;
      const m = await import("@tauri-apps/plugin-autostart");
      await (on ? m.enable() : m.disable());
    },
  },

  /** Diskteki bir dosyayı arayüzde gösterilebilir adrese çevirir. */
  fileSrc: (path: string): string => (inTauri ? convertFileSrc(path) : path),

  onNavigate: async (cb: (route: string) => void): Promise<() => void> => {
    if (!inTauri) return () => {};
    const { listen } = await import("@tauri-apps/api/event");
    return listen<string>("navigate", (e) => cb(e.payload));
  },

  audio: {
    state: () => call<AudioState>("audio_state"),
    setDefault: (id: string) => call<void>("audio_set_default", { id }),
    setVolume: (id: string, level: number) => call<void>("audio_set_volume", { id, level }),
    setMute: (id: string, muted: boolean) => call<void>("audio_set_mute", { id, muted }),
    /** `null` kilidi kapatır. */
    setLock: (level: number | null) => call<MicStatus>("mic_set_lock", { level }),
  },

  hotkeys: {
    list: () => call<HotkeyStatus[]>("hotkeys_list"),
    /** `null` varsayılana döndürür, `""` kapatır. */
    set: (id: string, accel: string | null) => call<HotkeyStatus[]>("hotkey_set", { id, accel }),
  },

  notes: {
    list: (trash = false) => call<NoteSummary[]>("notes_list", { trash }),
    get: (id: string) => call<Note | null>("note_get", { id }),
    save: (input: NoteInput) => call<Note>("note_save", { input }),
    trash: (id: string) => call<void>("note_trash", { id }),
    restore: (id: string) => call<void>("note_restore", { id }),
    remove: (id: string) => call<void>("note_delete", { id }),
    openQuick: () => call<void>("open_quick_note"),
    openSticky: (id: string) => call<void>("open_sticky", { id }),
    closeSticky: (id: string) => call<void>("close_sticky", { id }),
  },

  reminders: {
    list: () => call<Reminder[]>("reminders_list"),
    save: (input: ReminderInput) => call<Reminder>("reminder_save", { input }),
    remove: (id: string) => call<void>("reminder_delete", { id }),
    complete: (id: string) => call<void>("reminder_complete", { id }),
    snooze: (id: string, minutes: number) => call<void>("reminder_snooze", { id, minutes }),
  },

  timers: {
    state: () => call<TimersSnapshot>("timers_state"),
    stopwatchToggle: () => call<TimersSnapshot>("stopwatch_toggle"),
    stopwatchLap: () => call<TimersSnapshot>("stopwatch_lap"),
    stopwatchReset: () => call<TimersSnapshot>("stopwatch_reset"),
    add: (label: string, ms: number) => call<TimersSnapshot>("countdown_add", { label, ms }),
    toggle: (id: string) => call<TimersSnapshot>("countdown_toggle", { id }),
    remove: (id: string) => call<TimersSnapshot>("countdown_remove", { id }),
  },

  servers: {
    games: () => call<GameInfo[]>("servers_games"),
    list: () => call<Server[]>("servers_list"),
    /** Topluluk listesini (gerekiyorsa) indirir; `force` ile hemen. */
    sync: (force = false) => call<SyncResult>("servers_sync", { force }),
    cached: () => call<ServerStatus[]>("servers_cached"),
    /** Boş liste: hepsi. */
    query: (ids: string[] = []) => call<ServerStatus[]>("servers_query", { ids }),
    save: (input: ServerInput) => call<Server>("server_save", { input }),
    remove: (id: string) => call<void>("server_delete", { id }),
    join: (id: string) => call<JoinResult>("server_join", { id }),
    setExe: (game: string, path: string) => call<void>("server_set_exe", { game, path }),
    /** Logo seçtirir ve `Sunucular` klasörüne kopyalar. Vazgeçilirse `null`. */
    pickLogo: async (): Promise<string | null> => {
      if (!inTauri) return null;
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({
        multiple: false,
        directory: false,
        title: "Sunucu logosu seç",
        filters: [{ name: "Resim", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
      });
      if (typeof picked !== "string") return null;
      return invoke<string>("server_import_logo", { source: picked });
    },
    /** Oyunun .exe dosyasını seçtirir. */
    pickExe: async (gameName: string, exe: string): Promise<string | null> => {
      if (!inTauri) return null;
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({
        multiple: false,
        directory: false,
        title: `${gameName} · ${exe} dosyasını seç`,
        filters: [{ name: exe, extensions: ["exe"] }],
      });
      return typeof picked === "string" ? picked : null;
    },
  },

  videos: {
    tools: () => call<ToolsState>("videos_tools"),
    installTools: (force = false) => call<ToolsState>("videos_install_tools", { force }),
    update: (force = false) => call<ToolsState>("videos_update", { force }),
    /** `wholeList`: video bağlantısındaki çalma listesinin tamamını getir. */
    probe: (url: string, wholeList = false) => call<Parsed>("videos_probe", { url, wholeList }),
    /** `ids` boşsa listenin tamamı. Eklenen video sayısını döner. */
    downloadPlaylist: (url: string, option: string, ids: string[] | null) =>
      call<number>("videos_download_playlist", { url, option, ids }),
    download: (url: string, option: string, langs: Langs | null = null) =>
      call<Download>("videos_download", { url, option, langs }),
    playlists: () => call<UserPlaylist[]>("playlists_list"),
    /** `id` boşsa yeni liste. */
    savePlaylist: (id: string | null, name: string, items: string[]) =>
      call<UserPlaylist>("playlist_save", { id, name, items }),
    deletePlaylist: (id: string) => call<void>("playlist_delete", { id }),
    /** Videonun yanındaki altyazılar ve seçilenin satırları (oynatıcı için). */
    tracks: (id: string) => call<Sidecar[]>("media_tracks", { id }),
    track: (id: string, file: string) => call<Segment[]>("media_track", { id, file }),
    list: () => call<Download[]>("videos_list"),
    live: () => call<Live[]>("videos_live"),
    pause: (id: string) => call<void>("videos_pause", { id }),
    resume: (id: string) => call<void>("videos_resume", { id }),
    cancel: (id: string) => call<void>("videos_cancel", { id }),
    remove: (id: string, deleteFile: boolean) => call<void>("videos_remove", { id, deleteFile }),
    open: (id: string) => call<void>("videos_open", { id }),
    reveal: (id: string) => call<void>("videos_reveal", { id }),
    bridge: () => call<BridgeStatus>("bridge_status"),
    pair: () => call<string>("bridge_pair"),
    unpair: () => call<void>("bridge_unpair"),
    installExtension: () => call<string>("extension_install"),
  },

  games: {
    list: () => call<Game[]>("games_list"),
    /** Steam ve Epic'te kurulu oyunlar (eklemez). */
    scan: () => call<FoundGame[]>("games_scan"),
    import: (found: FoundGame[]) => call<number>("games_import", { found }),
    addManual: (name: string, exe: string) => call<void>("games_add_manual", { name, exe }),
    runningApps: () => call<RunningApp[]>("games_running_apps"),
    setHidden: (id: string, hidden: boolean) => call<void>("games_set_hidden", { id, hidden }),
    rename: (id: string, name: string) => call<void>("games_rename", { id, name }),
    remove: (id: string) => call<void>("games_remove", { id }),
    refreshMeta: (id: string) => call<void>("games_refresh_meta", { id }),
    sessions: (since: number) => call<PlaySession[]>("games_sessions", { since }),
    launch: (id: string) => call<void>("games_launch", { id }),
    reveal: (id: string) => call<void>("games_reveal", { id }),
    /** Oyunun .exe dosyasını seçtirir. */
    pickExe: async (): Promise<string | null> => {
      if (!inTauri) return "C:\\Oyunlar\\Deneme\\Deneme.exe";
      const { open } = await import("@tauri-apps/plugin-dialog");
      const picked = await open({
        multiple: false,
        directory: false,
        title: "Oyunun .exe dosyasını seç",
        filters: [{ name: "Oyun", extensions: ["exe"] }],
      });
      return typeof picked === "string" ? picked : null;
    },
    /** Steam mağazasında adıyla arama (Oyun Arşivi'ne ekleme). */
    storeSearch: (term: string) => call<StoreHit[]>("games_store_search", { term }),
    /** Arşive oyun ekler; oyunun kimliği. */
    addArchive: (appid: number | null, name: string) => call<string>("games_add_archive", { appid, name }),
  },

  reviews: {
    list: () => call<Review[]>("reviews_list"),
    save: (input: ReviewInput) => call<Review>("review_save", { input }),
    remove: (id: string) => call<void>("review_delete", { id }),
  },

  dictation: {
    status: () => call<DictationStatus>("dictation_status"),
    /** Ayarlar kaydedildikten sonra: açar/kapatır, sunucular yeni ayarla başlar. */
    apply: () => call<DictationStatus>("dictation_apply"),
    /** `part`: "whisper" | "gpu" (NVIDIA hızlandırma) | "llm" (toparlayıcı) */
    install: (part: "whisper" | "gpu" | "llm") => call<DictationStatus>("dictation_install", { part }),
    warmUp: () => call<DictationStatus>("dictation_warm_up"),
  },

  transcripts: {
    get: (id: string) => call<Transcript | null>("transcript_get", { id }),
    /** `mode`: "auto" (önce sitenin altyazısı) | "subtitles" | "whisper"; `lang`: "orig" | "tr" | "en" */
    create: (id: string, mode: string, lang: string) => call<Transcript>("transcript_create", { id, mode, lang }),
    remove: (id: string) => call<void>("transcript_delete", { id }),
    whisperStatus: () => call<WhisperStatus>("whisper_status"),
    installWhisper: (model: string) => call<WhisperStatus>("whisper_install", { model }),
  },

  overlay: {
    takeOsd: () => call<OsdMessage | null>("take_osd"),
    alerts: () => call<Alerts>("alerts_list"),
    fitAlerts: (height: number) => call<void>("alerts_fit", { height }),
    closeAlerts: () => call<void>("alerts_close"),
  },
};

/** Bu pencerenin görevi: ana pencere, gösterge, uyarılar, hızlı not ya da yapışkan not. */
export function currentView(): { kind: "main" | "osd" | "alerts" | "quicknote" | "sticky" | "dikte"; id?: string } {
  let label = "main";
  if (inTauri) {
    const meta = (window as unknown as { __TAURI_INTERNALS__: { metadata?: { currentWindow?: { label?: string } } } })
      .__TAURI_INTERNALS__.metadata;
    label = meta?.currentWindow?.label ?? "main";
  } else {
    label = new URLSearchParams(location.search).get("view") ?? "main";
  }
  if (label.startsWith("sticky-")) return { kind: "sticky", id: label.slice(7) };
  if (label === "osd" || label === "alerts" || label === "quicknote" || label === "dikte") return { kind: label };
  return { kind: "main" };
}

export function isVideo(path: string): boolean {
  return /\.(mp4|webm)$/i.test(path);
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toLocaleString("tr-TR", { maximumFractionDigits: 0 })} KB`;
  return `${(bytes / 1024 / 1024).toLocaleString("tr-TR", { maximumFractionDigits: 1 })} MB`;
}
