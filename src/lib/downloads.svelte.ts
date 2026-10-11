// İndirmelerin uygulama genelindeki durumu: üst çubuktaki gösterge, menü rozeti, Giriş Salonu ve
// Video İndirici ekranı aynı listeyi kullanır.
import { api } from "./api";
import { on } from "./bus";
import type { Download, Live } from "./types";

export const downloads = $state({ list: [] as Download[], live: {} as Record<string, Live>, loaded: false });

const ACTIVE = new Set(["queued", "downloading", "processing"]);

export function isActive(d: Download): boolean {
  return ACTIVE.has(d.status);
}

/** Üst çubuk için: kaç tane iniyor, ortalama yüzde. */
export function activeSummary(): { count: number; percent: number } {
  const active = downloads.list.filter(isActive);
  if (!active.length) return { count: 0, percent: 0 };
  const sum = active.reduce((s, d) => s + (downloads.live[d.id]?.progress ?? d.progress), 0);
  return { count: active.length, percent: Math.round((sum / active.length) * 100) };
}

async function reload() {
  try {
    downloads.list = await api.videos.list();
    const live = await api.videos.live();
    downloads.live = Object.fromEntries(live.map((l) => [l.id, l]));
    downloads.loaded = true;
  } catch (e) {
    console.error(e);
  }
}

let started = false;

/** Bir kez çağrılır (ana pencere açılınca). */
export function startDownloadsSync(): () => void {
  if (started) return () => {};
  started = true;
  void reload();
  const stops = [
    on("downloads-changed", () => void reload()),
    on<Live[]>("downloads-progress", (live) => {
      downloads.live = Object.fromEntries(live.map((l) => [l.id, l]));
    }),
  ];
  return () => {
    started = false;
    stops.forEach((s) => void s.then((f) => f()));
  };
}

export function formatSpeed(bps: number | null | undefined): string {
  if (!bps) return "";
  if (bps < 1024 * 1024) return `${Math.round(bps / 1024)} KB/sn`;
  return `${(bps / 1024 / 1024).toFixed(1)} MB/sn`;
}

export function formatEta(sec: number | null | undefined): string {
  if (sec == null) return "";
  if (sec < 60) return `${sec} sn`;
  if (sec < 3600) return `${Math.floor(sec / 60)} dk ${sec % 60} sn`;
  return `${Math.floor(sec / 3600)} sa ${Math.floor((sec % 3600) / 60)} dk`;
}

export function formatDuration(sec: number | null | undefined): string {
  if (!sec) return "";
  const s = Math.round(sec);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${r}` : `${m}:${r}`;
}

/** Site adı → kısa görünen ad ve sekme. */
export function siteLabel(site: string): string {
  const s = site.toLowerCase();
  if (s.startsWith("youtube")) return "YouTube";
  if (s.startsWith("twitter")) return "X";
  if (s.startsWith("instagram")) return "Instagram";
  if (s.startsWith("tiktok")) return "TikTok";
  if (s === "generic") return "Diğer";
  return site;
}
