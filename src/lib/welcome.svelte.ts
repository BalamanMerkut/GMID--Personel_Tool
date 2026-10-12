// İlk açılış rehberi: bir kez kendiliğinden açılır (`ui.welcomed`), Hakkında'dan yeniden açılabilir.
import { getJson, setJson } from "./settings.svelte";

const KEY = "ui.welcomed";

export const welcome = $state({ open: false });

/** Ayarlar yüklendikten sonra çağrılır. */
export function maybeShowWelcome(): void {
  if (!getJson(KEY, false)) welcome.open = true;
}

export function finishWelcome(): void {
  welcome.open = false;
  void setJson(KEY, true).catch(() => {});
}
