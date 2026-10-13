// Medya kitaplığının kişisel bilgileri: favori şarkılar ve kaç kez dinlendikleri (ayarlarda saklanır).
import { getJson, setJson } from "./settings.svelte";

export const stats = $state<{ favs: string[]; plays: Record<string, number> }>({ favs: [], plays: {} });

let loaded = false;
/** Ayarlar yüklendikten sonra ilk kullanımda okunur. */
export function loadStats() {
  if (loaded) return;
  loaded = true;
  stats.favs = getJson<string[]>("media.favs", []);
  stats.plays = getJson<Record<string, number>>("media.plays", {});
}

export function toggleFav(id: string) {
  loadStats();
  stats.favs = stats.favs.includes(id) ? stats.favs.filter((x) => x !== id) : [...stats.favs, id];
  void setJson("media.favs", stats.favs);
}

export function countPlay(id: string) {
  loadStats();
  stats.plays = { ...stats.plays, [id]: (stats.plays[id] ?? 0) + 1 };
  void setJson("media.plays", stats.plays);
}
