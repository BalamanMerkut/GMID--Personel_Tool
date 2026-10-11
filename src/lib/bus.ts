// Tauri olayları. Tarayıcıda (geliştirme) taklit katman aynı adlarla yerel olay yayar.
import { inTauri } from "./api";

export const localBus = new EventTarget();

export type AppEvent =
  | "navigate"
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
  | "dictation-state"
  | "dictation-entry"
  | "dikte"
  | "dikte-level"
  | "games-changed"
  | "reviews-changed"
  | "clips-changed"
  | "clip-fit"
  | "games-suggestions";

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

/** Ardı ardına gelen olayları tek çağrıya indirir. */
export function debounce<A extends unknown[]>(fn: (...a: A) => void, ms: number): (...a: A) => void {
  let t: ReturnType<typeof setTimeout> | undefined;
  return (...a: A) => {
    clearTimeout(t);
    t = setTimeout(() => fn(...a), ms);
  };
}
