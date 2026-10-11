// Sadece Latin + Türkçe karakterler (diğer alfabeler paketi büyütmesin).
import "@fontsource/montserrat/latin-600.css";
import "@fontsource/montserrat/latin-ext-600.css";
import "@fontsource/montserrat/latin-700.css";
import "@fontsource/montserrat/latin-ext-700.css";
import "@fontsource/montserrat/latin-800.css";
import "@fontsource/montserrat/latin-ext-800.css";
import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-ext-400.css";
import "@fontsource/inter/latin-500.css";
import "@fontsource/inter/latin-ext-500.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/inter/latin-ext-600.css";
import "./app.css";
import { mount, type Component } from "svelte";
import { currentView } from "./lib/api";

// Tarayıcının kendi sağ tık menüsü (videoda "Tüm kontrolleri göster", "Geri", "Yazdır"…) uygulamada
// görünmesin; yazı alanlarında kopyala/yapıştır menüsü kalsın.
document.addEventListener("contextmenu", (e) => {
  const t = e.target as HTMLElement | null;
  if (!t?.closest("input, textarea, [contenteditable=''], [contenteditable='true']")) e.preventDefault();
});

// Her pencere sadece kendi arayüzünü yükler (gösterge penceresi notları, notlar Ses ekranını yüklemez).
const view = currentView();
document.documentElement.dataset.view = view.kind;

async function root(): Promise<{ component: Component<any>; props: Record<string, unknown> }> {
  switch (view.kind) {
    case "osd":
      return { component: (await import("./lib/windows/Osd.svelte")).default, props: {} };
    case "dikte":
      return { component: (await import("./lib/windows/DictationPill.svelte")).default, props: {} };
    case "panel":
      return { component: (await import("./lib/windows/GamePanel.svelte")).default, props: {} };
    case "alerts":
      return { component: (await import("./lib/windows/Alerts.svelte")).default, props: {} };
    case "quicknote":
      return { component: (await import("./lib/windows/NoteWindow.svelte")).default, props: {} };
    case "sticky":
      return { component: (await import("./lib/windows/NoteWindow.svelte")).default, props: { id: view.id } };
    default:
      return { component: (await import("./App.svelte")).default, props: {} };
  }
}

void root().then(({ component, props }) => mount(component, { target: document.getElementById("app")!, props }));
