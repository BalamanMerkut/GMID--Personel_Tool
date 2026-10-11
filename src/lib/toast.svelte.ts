import type { IconName } from "./icons";

export const toast = $state({ text: "", icon: "check" as IconName, visible: false });

let timer: ReturnType<typeof setTimeout> | undefined;

/** Sağ üstte kısa bir bilgi gösterir. */
export function notify(text: string, icon: IconName = "check"): void {
  toast.text = text;
  toast.icon = icon;
  toast.visible = true;
  clearTimeout(timer);
  timer = setTimeout(() => (toast.visible = false), 2400);
}
