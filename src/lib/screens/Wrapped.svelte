<script lang="ts">
  // GMID Wrapped: seçilen ayın ya da yılın özeti, paylaşılabilir bir görsel (1080×1350) olarak çizilir.
  import { onMount } from "svelte";
  import logoUrl from "../../assets/gmid-logo.webp";
  import { api } from "../api";
  import Icon from "../Icon.svelte";
  import { notify } from "../toast.svelte";
  import type { Wrapped } from "../types";

  const W = 1080;
  const H = 1350;
  const MONTHS = ["Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim", "Kasım", "Aralık"];
  const DAYS = ["Pazartesi", "Salı", "Çarşamba", "Perşembe", "Cuma", "Cumartesi", "Pazar"];

  type Period = { kind: "month" | "year"; year: number; month: number };
  const today = new Date();
  let period = $state<Period>({ kind: "month", year: today.getFullYear(), month: today.getMonth() });
  let data = $state<Wrapped | null>(null);
  let canvas = $state<HTMLCanvasElement | null>(null);
  let busy = $state(false);

  const range = (p: Period): [number, number] =>
    p.kind === "year"
      ? [new Date(p.year, 0, 1).getTime(), new Date(p.year + 1, 0, 1).getTime()]
      : [new Date(p.year, p.month, 1).getTime(), new Date(p.year, p.month + 1, 1).getTime()];
  const label = (p: Period) => (p.kind === "year" ? `${p.year}` : `${MONTHS[p.month]} ${p.year}`);

  function quick(kind: "this" | "last" | "year" | "lastYear") {
    const y = today.getFullYear();
    const m = today.getMonth();
    period =
      kind === "this" ? { kind: "month", year: y, month: m }
      : kind === "last" ? { kind: "month", year: m === 0 ? y - 1 : y, month: (m + 11) % 12 }
      : kind === "year" ? { kind: "year", year: y, month: 0 }
      : { kind: "year", year: y - 1, month: 0 };
  }
  const isQuick = (k: "this" | "last" | "year" | "lastYear") => {
    const y = today.getFullYear();
    const m = today.getMonth();
    const p = period;
    return k === "this" ? p.kind === "month" && p.year === y && p.month === m
      : k === "last" ? p.kind === "month" && p.month === (m + 11) % 12 && p.year === (m === 0 ? y - 1 : y)
      : k === "year" ? p.kind === "year" && p.year === y
      : p.kind === "year" && p.year === y - 1;
  };

  $effect(() => {
    const [from, to] = range(period);
    void api.wrapped.get(from, to).then((w) => (data = w)).catch((e) => notify(String(e), "info"));
  });

  $effect(() => {
    if (canvas && data) void draw(canvas, data, label(period));
  });

  onMount(() => {
    // Yazı tipleri yüklenmeden çizilirse sistem yazı tipine düşer.
    void document.fonts?.ready.then(() => canvas && data && draw(canvas, data, label(period)));
  });

  // ---- çizim ----

  const hrs = (s: number) => {
    const h = s / 3600;
    return h >= 10 ? `${Math.round(h)}` : h >= 1 ? h.toFixed(1).replace(".", ",") : `${Math.round(s / 60)} dk`;
  };
  const unit = (s: number) => (s / 3600 >= 1 ? " saat" : "");

  function load(src: string): Promise<HTMLImageElement | null> {
    return new Promise((res) => {
      const img = new Image();
      img.onload = () => res(img);
      img.onerror = () => res(null);
      img.src = src;
    });
  }

  function rr(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, r);
  }

  function cover(ctx: CanvasRenderingContext2D, img: HTMLImageElement, x: number, y: number, w: number, h: number) {
    // object-fit: cover
    const s = Math.max(w / img.width, h / img.height);
    const sw = w / s;
    const sh = h / s;
    ctx.drawImage(img, (img.width - sw) / 2, (img.height - sh) / 2, sw, sh, x, y, w, h);
  }

  function fit(ctx: CanvasRenderingContext2D, text: string, max: number) {
    if (ctx.measureText(text).width <= max) return text;
    let t = text;
    while (t.length > 1 && ctx.measureText(`${t}…`).width > max) t = t.slice(0, -1);
    return `${t}…`;
  }

  const GOLD = "#e3c270";
  const TEXT = "#f1ede4";
  const MUTED = "#a39e93";
  const DISPLAY = "Montserrat, 'Segoe UI', sans-serif";
  const BODY = "Inter, 'Segoe UI', sans-serif";

  async function draw(c: HTMLCanvasElement, w: Wrapped, title: string) {
    const ctx = c.getContext("2d");
    if (!ctx) return;
    const [logo, ...covers] = await Promise.all([load(logoUrl), ...w.top.map((t) => (t.cover ? load(t.cover) : Promise.resolve(null)))]);
    c.width = W;
    c.height = H;

    // Zemin: gece rengi, sağ üstte altın parıltı.
    const bg = ctx.createLinearGradient(0, 0, 0, H);
    bg.addColorStop(0, "#101215");
    bg.addColorStop(1, "#18160f");
    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, W, H);
    const glow = ctx.createRadialGradient(W - 120, 60, 10, W - 120, 60, 620);
    glow.addColorStop(0, "rgba(227,194,112,0.22)");
    glow.addColorStop(1, "rgba(227,194,112,0)");
    ctx.fillStyle = glow;
    ctx.fillRect(0, 0, W, H);

    const X = 72;
    ctx.textBaseline = "alphabetic";
    ctx.fillStyle = GOLD;
    ctx.font = `800 26px ${DISPLAY}`;
    ctx.letterSpacing = "6px";
    ctx.fillText("GMID WRAPPED", X, 100);
    ctx.letterSpacing = "0px";
    ctx.fillStyle = TEXT;
    ctx.font = `800 68px ${DISPLAY}`;
    ctx.fillText(title, X, 170);
    if (logo) ctx.drawImage(logo, W - X - 110, 56, 110, 110);

    // Ana sayı
    ctx.fillStyle = GOLD;
    ctx.font = `900 150px ${DISPLAY}`;
    const big = w.totalSeconds ? hrs(w.totalSeconds) : "0";
    ctx.fillText(big, X - 6, 330);
    const bw = ctx.measureText(big).width;
    ctx.fillStyle = TEXT;
    ctx.font = `700 44px ${DISPLAY}`;
    ctx.fillText(w.totalSeconds >= 3600 ? "saat oyun" : w.totalSeconds ? "oyun" : "saat oyun", X + bw + 18, 330);
    ctx.fillStyle = MUTED;
    ctx.font = `500 30px ${BODY}`;
    ctx.fillText(
      w.totalSeconds ? `${w.sessions} oturum · ${w.daysPlayed} gün · ${w.distinctGames} farklı oyun` : "Bu dönemde oyun kaydı yok",
      X,
      380,
    );

    // En çok oynananlar
    let y = 448;
    ctx.fillStyle = GOLD;
    ctx.font = `800 22px ${DISPLAY}`;
    ctx.letterSpacing = "4px";
    ctx.fillText("EN ÇOK OYNADIKLARIN", X, y);
    ctx.letterSpacing = "0px";
    y += 22;
    const max = w.top[0]?.seconds || 1;
    const rowH = 84;
    w.top.forEach((t, i) => {
      const top = y + i * rowH;
      const img = covers[i];
      ctx.save();
      rr(ctx, X, top, 58, 74, 9);
      ctx.clip();
      if (img) cover(ctx, img, X, top, 58, 74);
      else {
        ctx.fillStyle = "#2a2d33";
        ctx.fillRect(X, top, 58, 74);
        ctx.fillStyle = MUTED;
        ctx.font = `800 28px ${DISPLAY}`;
        ctx.textAlign = "center";
        ctx.fillText(t.name.slice(0, 1).toUpperCase(), X + 29, top + 48);
        ctx.textAlign = "left";
      }
      ctx.restore();
      const tx = X + 82;
      ctx.fillStyle = MUTED;
      ctx.font = `700 24px ${DISPLAY}`;
      ctx.fillText(`${i + 1}`, tx, top + 32);
      ctx.fillStyle = TEXT;
      ctx.font = `700 31px ${DISPLAY}`;
      ctx.fillText(fit(ctx, t.name, 600), tx + 34, top + 32);
      ctx.fillStyle = TEXT;
      ctx.font = `700 28px ${BODY}`;
      ctx.textAlign = "right";
      ctx.fillText(`${hrs(t.seconds)}${unit(t.seconds)}`, W - X, top + 32);
      ctx.textAlign = "left";
      // Çubuk: tek renk (altın), uzunluk = süre.
      const bx = tx + 34;
      const full = W - X - bx;
      ctx.fillStyle = "rgba(255,255,255,0.07)";
      rr(ctx, bx, top + 50, full, 9, 4.5);
      ctx.fill();
      ctx.fillStyle = GOLD;
      rr(ctx, bx, top + 50, Math.max(9, (full * t.seconds) / max), 9, 4.5);
      ctx.fill();
    });
    y += Math.max(1, w.top.length) * rowH + 6;

    // Rozetler
    if (w.badges.length) {
      let bx = X;
      w.badges.forEach((b) => {
        ctx.font = `800 26px ${DISPLAY}`;
        const tw = ctx.measureText(`★ ${b.title}`).width + 40;
        if (bx + tw > W - X) return;
        ctx.fillStyle = "rgba(227,194,112,0.14)";
        ctx.strokeStyle = "rgba(227,194,112,0.6)";
        ctx.lineWidth = 2;
        rr(ctx, bx, y, tw, 52, 26);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = GOLD;
        ctx.fillText(`★ ${b.title}`, bx + 20, y + 35);
        bx += tw + 14;
      });
      ctx.fillStyle = MUTED;
      ctx.font = `500 22px ${BODY}`;
      ctx.fillText(fit(ctx, w.badges.map((b) => b.desc).join(" · "), W - 2 * X), X, y + 86);
      y += 112;
    }

    // Kutular (3 × 2)
    const peak = w.hours.reduce((best, v, i, a) => (v > a[best] ? i : best), 0);
    const tiles: [string, string, string][] = [
      ["EN UZUN OTURUM", w.longest ? `${hrs(w.longest.seconds)}${unit(w.longest.seconds)}` : "—", w.longest?.game ?? ""],
      ["EN SEVDİĞİN GÜN", w.bestWeekday != null ? DAYS[w.bestWeekday] : "—", w.weekendShare >= 0.5 ? "hafta sonu oyuncusu" : ""],
      ["EN SEVDİĞİN SAAT", w.totalSeconds ? `${String(peak).padStart(2, "0")}:00` : "—", !w.totalSeconds ? "" : peak < 5 ? "gece kuşu" : peak < 12 ? "sabahçı" : peak < 18 ? "gündüz oyuncusu" : "akşam oyuncusu"],
      ["PUANLADIĞIN OYUN", `${w.ratedGames}`, w.bestGame ? `en iyisi ${w.bestGame[0]} · ${w.bestGame[1].toFixed(1).replace(".", ",")}` : ""],
      ["İZLEDİĞİN FİLM / DİZİ", `${w.titlesWatched}`, w.bestTitle ? `en iyisi ${w.bestTitle[0]}` : ""],
      ["KLİP · VİDEO", `${w.clips} · ${w.downloads}`, w.newGames ? `${w.newGames} yeni oyun` : ""],
    ];
    const gap = 14;
    const tw = (W - 2 * X - 2 * gap) / 3;
    const th = 118;
    tiles.forEach(([k, v, s], i) => {
      const tx = X + (i % 3) * (tw + gap);
      const ty = y + Math.floor(i / 3) * (th + gap);
      ctx.fillStyle = "rgba(255,255,255,0.045)";
      rr(ctx, tx, ty, tw, th, 18);
      ctx.fill();
      ctx.fillStyle = MUTED;
      ctx.font = `700 15px ${DISPLAY}`;
      ctx.letterSpacing = "1.5px";
      ctx.fillText(fit(ctx, k, tw - 36), tx + 20, ty + 32);
      ctx.letterSpacing = "0px";
      ctx.fillStyle = TEXT;
      ctx.font = `800 36px ${DISPLAY}`;
      ctx.fillText(fit(ctx, v, tw - 36), tx + 20, ty + 76);
      ctx.fillStyle = MUTED;
      ctx.font = `500 19px ${BODY}`;
      ctx.fillText(fit(ctx, s, tw - 36), tx + 20, ty + 104);
    });

    ctx.fillStyle = "rgba(241,237,228,0.45)";
    ctx.font = `600 20px ${BODY}`;
    ctx.fillText("GMID Tool · Merkut Studios", X, H - 40);
    ctx.textAlign = "right";
    ctx.fillText("Game Mansion In Discord", W - X, H - 40);
    ctx.textAlign = "left";
  }

  async function save() {
    if (!canvas) return;
    busy = true;
    try {
      await api.wrapped.save(label(period), canvas.toDataURL("image/png"));
      notify("Kaydedildi · Ozetler klasörü", "check");
    } catch (e) {
      notify(String(e), "info");
    } finally {
      busy = false;
    }
  }

  async function copy() {
    if (!canvas) return;
    try {
      const blob = await new Promise<Blob | null>((r) => canvas!.toBlob(r, "image/png"));
      if (!blob) throw new Error("Görsel oluşturulamadı");
      await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
      notify("Panoya kopyalandı · Discord'da Ctrl+V", "check");
    } catch {
      notify("Panoya kopyalanamadı; Kaydet'i kullan", "info");
    }
  }

  const years = $derived(Array.from({ length: 4 }, (_, i) => today.getFullYear() - i));
</script>

<div class="wr">
  <section class="panel bar">
    <div class="seg">
      <button aria-pressed={isQuick("this")} onclick={() => quick("this")}>Bu ay</button>
      <button aria-pressed={isQuick("last")} onclick={() => quick("last")}>Geçen ay</button>
      <button aria-pressed={isQuick("year")} onclick={() => quick("year")}>Bu yıl</button>
      <button aria-pressed={isQuick("lastYear")} onclick={() => quick("lastYear")}>Geçen yıl</button>
    </div>
    <div class="pick">
      <select value={period.kind === "year" ? "year" : String(period.month)} onchange={(e) => {
        const v = e.currentTarget.value;
        period = v === "year" ? { ...period, kind: "year" } : { ...period, kind: "month", month: Number(v) };
      }} aria-label="Ay">
        <option value="year">Tüm yıl</option>
        {#each MONTHS as m, i (m)}<option value={String(i)}>{m}</option>{/each}
      </select>
      <select value={period.year} onchange={(e) => (period = { ...period, year: Number(e.currentTarget.value) })} aria-label="Yıl">
        {#each years as y (y)}<option value={y}>{y}</option>{/each}
      </select>
    </div>
    <div class="acts">
      <button class="btn ghost sm" onclick={copy} disabled={!data}><Icon name="copy" size={14} />Panoya kopyala</button>
      <button class="btn sm" onclick={save} disabled={!data || busy}><Icon name="download" size={14} />PNG olarak kaydet</button>
    </div>
  </section>

  <div class="stage">
    <canvas bind:this={canvas} width={W} height={H} aria-label="GMID Wrapped {label(period)}"></canvas>
  </div>
</div>

<style>
  .wr { display: flex; flex-direction: column; gap: 16px; max-width: 1100px; margin: 0 auto; }
  .bar { flex-direction: row; align-items: center; gap: 12px; flex-wrap: wrap; display: flex; }
  .seg { display: flex; gap: 6px; flex-wrap: wrap; }
  .seg button { padding: 7px 12px; border-radius: 10px; border: 1px solid var(--line-2); background: var(--coal-2); color: var(--text); font-weight: 600; }
  .seg button[aria-pressed="true"] { border-color: var(--gold); color: var(--gold-glow); background: rgba(227, 194, 112, 0.12); }
  .pick { display: flex; gap: 6px; }
  select { background: var(--coal-2); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; color: var(--text); }
  .acts { display: flex; gap: 8px; margin-left: auto; }
  .stage { display: grid; place-items: center; }
  canvas { width: min(100%, calc((100vh - 230px) * 0.8)); height: auto; aspect-ratio: 1080 / 1350; border-radius: 18px; box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5); background: #101215; }
</style>
