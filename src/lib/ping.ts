// Bağlantı testi sonuçlarının ortak biçimleri (Bağlantı Testi ekranı ve Oyun İçi Panel).
import type { NetProbe } from "./types";

/** Ping'in rengi: iyi / idare eder / kötü. Kayıp, gecikmeden önce gelir. */
export function tone(p: Pick<NetProbe, "avg" | "loss" | "jitter">): "good" | "warn" | "bad" | "none" {
  if (p.avg == null) return "none";
  if (p.loss >= 10 || p.avg >= 120) return "bad";
  if (p.loss >= 3 || p.avg >= 70 || (p.jitter ?? 0) > 20) return "warn";
  return "good";
}

export const msText = (v: number | null) => (v == null ? "—" : v < 1 ? "<1" : String(Math.round(v)));

export const GROUPS: { id: NetProbe["group"]; title: string; hint: string }[] = [
  { id: "net", title: "Ev ağı", hint: "Bilgisayarından modeme kadar. Wi-Fi sorunları burada görünür." },
  { id: "internet", title: "İnternet", hint: "Modemden dışarıya: internet sağlayıcın." },
  { id: "gmid", title: "GMID sunucuları", hint: "Topluluk listesindeki oyun sunucuları." },
  { id: "game", title: "Steam (CS2 ve Valve oyunları)", hint: "Sana en yakın Steam yönlendirme noktaları." },
];

/** Panel için özet: modem, en iyi internet, en iyi oyun/GMID sunucusu. */
export function summary(probes: NetProbe[]): NetProbe[] {
  const best = (g: NetProbe["group"][]) =>
    probes
      .filter((p) => g.includes(p.group) && p.avg != null)
      .sort((a, b) => a.loss - b.loss || (a.avg ?? 0) - (b.avg ?? 0))[0];
  return [best(["net"]), best(["internet"]), ...probes.filter((p) => p.group === "gmid").slice(0, 2), best(["game"])].filter(
    (p): p is NetProbe => !!p,
  );
}
