// Turnuva şampiyon kartı: Discord'a görsel olarak yapıştırmak için (1200×675 PNG).
import logoUrl from "../assets/gmid-logo.webp";

const W = 1200;
const H = 675;
const GOLD = "#e3c270";
const TEXT = "#f1ede4";
const MUTED = "#a39e93";
const DISPLAY = "Montserrat, 'Segoe UI', sans-serif";
const BODY = "Inter, 'Segoe UI', sans-serif";

function load(src: string): Promise<HTMLImageElement | null> {
  return new Promise((res) => {
    const img = new Image();
    img.onload = () => res(img);
    img.onerror = () => res(null);
    img.src = src;
  });
}

function fit(ctx: CanvasRenderingContext2D, text: string, max: number): string {
  if (ctx.measureText(text).width <= max) return text;
  let t = text;
  while (t.length > 1 && ctx.measureText(`${t}…`).width > max) t = t.slice(0, -1);
  return `${t}…`;
}

export interface CupCard {
  title: string;
  first: string;
  second: string | null;
  thirds: string[];
  players: number;
  final: string | null;
}

export async function drawCupCard(c: HTMLCanvasElement, d: CupCard): Promise<void> {
  const ctx = c.getContext("2d");
  if (!ctx) return;
  const logo = await load(logoUrl);
  c.width = W;
  c.height = H;
  const bg = ctx.createLinearGradient(0, 0, 0, H);
  bg.addColorStop(0, "#101215");
  bg.addColorStop(1, "#1a170f");
  ctx.fillStyle = bg;
  ctx.fillRect(0, 0, W, H);
  const glow = ctx.createRadialGradient(W / 2, 250, 10, W / 2, 250, 560);
  glow.addColorStop(0, "rgba(227,194,112,0.28)");
  glow.addColorStop(1, "rgba(227,194,112,0)");
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, W, H);
  ctx.strokeStyle = "rgba(227,194,112,0.55)";
  ctx.lineWidth = 3;
  ctx.strokeRect(18, 18, W - 36, H - 36);

  ctx.textAlign = "center";
  ctx.fillStyle = GOLD;
  ctx.font = `800 24px ${DISPLAY}`;
  ctx.fillText("GMID TURNUVASI", W / 2, 84);
  ctx.fillStyle = TEXT;
  ctx.font = `800 46px ${DISPLAY}`;
  ctx.fillText(fit(ctx, d.title, W - 200), W / 2, 142);

  // Kupa
  ctx.font = `110px ${BODY}`;
  ctx.fillText("🏆", W / 2, 278);
  ctx.fillStyle = GOLD;
  ctx.font = `700 24px ${DISPLAY}`;
  ctx.fillText("ŞAMPİYON", W / 2, 330);
  ctx.fillStyle = TEXT;
  ctx.font = `900 84px ${DISPLAY}`;
  ctx.fillText(fit(ctx, d.first, W - 160), W / 2, 418);

  // İkinci ve üçüncüler
  ctx.font = `700 30px ${DISPLAY}`;
  const lines: string[] = [];
  if (d.second) lines.push(`🥈 ${d.second}`);
  if (d.thirds.length) lines.push(`🥉 ${d.thirds.join(" · ")}`);
  ctx.fillStyle = TEXT;
  lines.forEach((l, i) => ctx.fillText(fit(ctx, l, W - 200), W / 2, 486 + i * 46));

  ctx.fillStyle = MUTED;
  ctx.font = `600 20px ${BODY}`;
  const date = new Date().toLocaleDateString("tr-TR", { day: "numeric", month: "long", year: "numeric" });
  ctx.fillText(`${d.players} oyuncu${d.final ? ` · Final: ${d.final}` : ""} · ${date}`, W / 2, 582);
  if (logo) ctx.drawImage(logo, 40, H - 100, 56, 56);
  ctx.textAlign = "right";
  ctx.fillText("GMID Tool · Game Mansion In Discord", W - 44, H - 44);
  ctx.textAlign = "left";
}
