// Kullanıcı ayarları. Değerler veritabanındaki `settings` tablosunda JSON metin olarak durur.
import { api } from "./api";

export type BgKind = "default" | "color" | "file";
export type BgFocus = "center" | "top" | "bottom" | "left" | "right";
export type Quality = "auto" | "high" | "light";

export interface Settings {
  bgKind: BgKind;
  bgPath: string;
  bgColor: string;
  bgBlur: number;
  bgDim: number;
  bgFocus: BgFocus;
  splash: boolean;
  quality: Quality;
  closeToTray: boolean;
}

const DEFAULTS: Settings = {
  bgKind: "default",
  bgPath: "",
  bgColor: "#16181c",
  bgBlur: 0,
  bgDim: 35,
  bgFocus: "center",
  splash: true,
  quality: "auto",
  closeToTray: true,
};

/** Rust tarafı `app.closeToTray` anahtarını okur; bu isimler değişirse orası da değişmeli. */
const KEYS: Record<keyof Settings, string> = {
  bgKind: "theme.bg.kind",
  bgPath: "theme.bg.path",
  bgColor: "theme.bg.color",
  bgBlur: "theme.bg.blur",
  bgDim: "theme.bg.dim",
  bgFocus: "theme.bg.focus",
  splash: "ui.splash",
  quality: "ui.quality",
  closeToTray: "app.closeToTray",
};

export const settings: Settings = $state({ ...DEFAULTS });

/** Ayar ekranı dışındaki modüllerin JSON ayarları (ör. `audio.devices`, `reminders.sound`). */
let raw: Record<string, string> = {};

export function getJson<T>(key: string, fallback: T): T {
  try {
    return key in raw ? (JSON.parse(raw[key]) as T) : fallback;
  } catch {
    return fallback;
  }
}

export async function setJson(key: string, value: unknown): Promise<void> {
  raw[key] = JSON.stringify(value);
  await api.setSetting(key, raw[key]);
}

export async function loadSettings(): Promise<void> {
  raw = await api.getSettings();
  for (const key of Object.keys(KEYS) as (keyof Settings)[]) {
    const stored = raw[KEYS[key]];
    if (stored === undefined) continue;
    try {
      const value = JSON.parse(stored);
      if (typeof value === typeof DEFAULTS[key]) (settings as unknown as Record<string, unknown>)[key] = value;
    } catch {
      /* bozuk değer: varsayılan kalır */
    }
  }
}

export async function save<K extends keyof Settings>(key: K, value: Settings[K]): Promise<void> {
  settings[key] = value;
  await api.setSetting(KEYS[key], JSON.stringify(value));
}

/** "Otomatik" kalite: hareket azaltma açıksa hafif, değilse yüksek. */
export function effectiveQuality(q: Quality): "high" | "light" {
  if (q !== "auto") return q;
  return matchMedia("(prefers-reduced-motion: reduce)").matches ? "light" : "high";
}
