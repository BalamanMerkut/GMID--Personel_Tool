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
  Alerts, AudioState, BridgeStatus, Download, GameInfo, Live, Probe, ToolsState, HotkeyStatus, JoinResult, MicStatus, Note, NoteInput, NoteSummary, OsdMessage,
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
    probe: (url: string) => call<Probe>("videos_probe", { url }),
    download: (url: string, option: string) => call<Download>("videos_download", { url, option }),
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

  overlay: {
    takeOsd: () => call<OsdMessage | null>("take_osd"),
    alerts: () => call<Alerts>("alerts_list"),
    fitAlerts: (height: number) => call<void>("alerts_fit", { height }),
    closeAlerts: () => call<void>("alerts_close"),
  },
};

/** Bu pencerenin görevi: ana pencere, gösterge, uyarılar, hızlı not ya da yapışkan not. */
export function currentView(): { kind: "main" | "osd" | "alerts" | "quicknote" | "sticky"; id?: string } {
  let label = "main";
  if (inTauri) {
    const meta = (window as unknown as { __TAURI_INTERNALS__: { metadata?: { currentWindow?: { label?: string } } } })
      .__TAURI_INTERNALS__.metadata;
    label = meta?.currentWindow?.label ?? "main";
  } else {
    label = new URLSearchParams(location.search).get("view") ?? "main";
  }
  if (label.startsWith("sticky-")) return { kind: "sticky", id: label.slice(7) };
  if (label === "osd" || label === "alerts" || label === "quicknote") return { kind: label };
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
