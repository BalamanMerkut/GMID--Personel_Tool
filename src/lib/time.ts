// Türkçe zaman biçimleri.

const pad = (n: number) => String(n).padStart(2, "0");

/** 1:02:03 ya da 02:03 */
export function clock(ms: number, withCentis = false): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const base = h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
  return withCentis ? `${base},${pad(Math.floor((Math.max(0, ms) % 1000) / 10))}` : base;
}

/** "5 dk", "2 sa 15 dk", "3 gün" */
export function span(ms: number): string {
  const min = Math.round(Math.abs(ms) / 60000);
  if (min < 1) return "az";
  if (min < 60) return `${min} dk`;
  const h = Math.floor(min / 60);
  if (h < 24) return min % 60 ? `${h} sa ${min % 60} dk` : `${h} sa`;
  return `${Math.round(h / 24)} gün`;
}

const sameDay = (a: Date, b: Date) => a.toDateString() === b.toDateString();

/** "Bugün 20:30", "Yarın 09:00", "Pzt 14 Eki 09:00" */
export function when(ts: number, now = Date.now()): string {
  const d = new Date(ts);
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  const today = new Date(now);
  const tomorrow = new Date(now + 86_400_000);
  const yesterday = new Date(now - 86_400_000);
  if (sameDay(d, today)) return `Bugün ${time}`;
  if (sameDay(d, tomorrow)) return `Yarın ${time}`;
  if (sameDay(d, yesterday)) return `Dün ${time}`;
  const date = d.toLocaleDateString("tr-TR", { weekday: "short", day: "numeric", month: "short" });
  return `${date} ${time}`;
}

/** "5 dk sonra" / "3 dk önce" */
export function relative(ts: number, now = Date.now()): string {
  const diff = ts - now;
  if (Math.abs(diff) < 45_000) return "şimdi";
  return diff > 0 ? `${span(diff)} sonra` : `${span(diff)} önce`;
}

/** Notlar listesi için kısa tarih. */
export function ago(ts: number, now = Date.now()): string {
  const diff = now - ts;
  if (diff < 60_000) return "az önce";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} dk önce`;
  if (sameDay(new Date(ts), new Date(now))) return `bugün ${new Date(ts).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" })}`;
  return new Date(ts).toLocaleDateString("tr-TR", { day: "numeric", month: "short" });
}

/** <input type="date"> ve <input type="time"> değerleri */
export function toInputs(ts: number): { date: string; time: string } {
  const d = new Date(ts);
  return { date: `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`, time: `${pad(d.getHours())}:${pad(d.getMinutes())}` };
}

export function fromInputs(date: string, time: string): number | null {
  const [y, mo, d] = date.split("-").map(Number);
  const [h, mi] = time.split(":").map(Number);
  if (![y, mo, d, h, mi].every(Number.isFinite)) return null;
  return new Date(y, mo - 1, d, h, mi, 0, 0).getTime();
}
