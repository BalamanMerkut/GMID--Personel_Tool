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
};

export function isVideo(path: string): boolean {
  return /\.(mp4|webm)$/i.test(path);
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toLocaleString("tr-TR", { maximumFractionDigits: 0 })} KB`;
  return `${(bytes / 1024 / 1024).toLocaleString("tr-TR", { maximumFractionDigits: 1 })} MB`;
}
