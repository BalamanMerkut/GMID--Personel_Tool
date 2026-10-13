// Tauri olayları. Tarayıcıda (geliştirme) taklit katman aynı adlarla yerel olay yayar.
import { inTauri } from "./api";

export const localBus = new EventTarget();

export type AppEvent =
  | "navigate"
  | "clips-thumbs"
  | "audio-changed"
  | "notes-changed"
  | "reminders-changed"
  | "timers-changed"
  | "alerts-changed"
  | "osd"
  | "downloads-changed"
  | "downloads-progress"
  | "tools-progress"
  | "bridge-changed"
  | "transcript-progress"
  | "karaoke-progress"
  | "dictation-state"
  | "dictation-entry"
  | "dikte"
  | "dikte-level"
  | "games-changed"
  | "reviews-changed"
  | "clips-changed"
  | "clip-fit"
  | "games-suggestions"
  | "gamemode"
  | "panel-open"
  | "titles-changed"
  | "install"
  | "install-kit"
  | "update"
  | "deals"
  | "panel-close"
  | "panel-grabbed"
  | "community"
  | "alarm-changed"
  | "panel-alarm"
  | "power-changed"
  | "gmid-player"
  | "gmid-player-cmd";

/** Olayı dinler; dinlemeyi bırakmak için dönen fonksiyonu çağır. */
export async function on<T = unknown>(event: AppEvent, cb: (payload: T) => void): Promise<() => void> {
  if (inTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(event, (e) => cb(e.payload));
  }
  const handler = (e: Event) => cb((e as CustomEvent<T>).detail);
  localBus.addEventListener(event, handler);
  return () => localBus.removeEventListener(event, handler);
}

/** Bütün pencerelere olay gönderir (ör. ana penceredeki oynatıcı ↔ oyun içi panel). */
export async function emitAll(event: AppEvent, payload: unknown): Promise<void> {
  if (inTauri) {
    const { emit } = await import("@tauri-apps/api/event");
    await emit(event, payload);
    return;
  }
  localBus.dispatchEvent(new CustomEvent(event, { detail: payload }));
}

/** Ardı ardına gelen olayları tek çağrıya indirir. */
export function debounce<A extends unknown[]>(fn: (...a: A) => void, ms: number): (...a: A) => void {
  let t: ReturnType<typeof setTimeout> | undefined;
  return (...a: A) => {
    clearTimeout(t);
    t = setTimeout(() => fn(...a), ms);
  };
}
