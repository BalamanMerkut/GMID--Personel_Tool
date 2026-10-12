// GMID topluluğu: Discord davet bağlantısı topluluk listesiyle (GitHub) gelir; yan menü, Giriş Salonu ve
// Hakkında aynı değeri gösterir.
import { api } from "./api";
import { on } from "./bus";

export const community = $state<{ discord: string | null }>({ discord: null });

let started = false;

export function startCommunity(): void {
  if (started) return;
  started = true;
  void api.community.discord().then((d) => (community.discord = d)).catch(() => {});
  void on<string | null>("community", (d) => (community.discord = d));
}

export function joinDiscord(): void {
  if (community.discord) void api.openUrl(community.discord);
}
