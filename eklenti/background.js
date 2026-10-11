// GMID Tool eklentisi — arka plan. Uygulamayla sadece bu bilgisayardaki köprü üzerinden konuşur.
//
// Videoyu iki yoldan bulur:
//  1) Sayfanın adresi: yt-dlp 1800'den fazla siteyi tanır (YouTube, X, Instagram, TikTok, Twitch…).
//  2) Yakalanan kaynak: sayfa tanınmıyorsa, oynatıcının indirdiği video dosyası ya da yayın listesi
//     (mp4, webm, m3u8, mpd) ağ trafiğinden yakalanır ve sayfa bilgisiyle (Referer) indirilir.
const PORTS = [47913, 47914, 47915, 47916, 47917];
// Giriş isteyen bu sitelerde sayfa bağlantısıyla indirirken o sitenin çerezleri geçici olarak verilir.
const COOKIE_SITES = ["instagram.com", "x.com", "twitter.com", "facebook.com"];
// Bu sitelerin kendi oynatıcı parçalarını yakalamaya gerek yok: yt-dlp sayfa adresinden indirir.
const SKIP_MEDIA_HOSTS = ["googlevideo.com", "youtube.com", "ytimg.com", "twimg.com", "cdninstagram.com", "fbcdn.net", "tiktokcdn.com"];
const QUALITIES = [
  ["auto", "En iyi (1080p'ye kadar)"],
  ["720", "720p"],
  ["480", "480p"],
  ["mp3", "Sadece ses (MP3)"],
];

let base = null;

async function token() {
  return (await chrome.storage.local.get("token")).token || null;
}

async function ping(url) {
  try {
    const r = await fetch(url + "/v1/ping", { signal: AbortSignal.timeout(800) });
    const j = await r.json();
    return j.app === "GMID Tool" ? j : null;
  } catch {
    return null;
  }
}

async function findBase() {
  if (base && (await ping(base))) return base;
  base = null;
  for (const p of PORTS) {
    const url = `http://127.0.0.1:${p}`;
    if (await ping(url)) return (base = url);
  }
  return null;
}

async function api(path, body) {
  const b = await findBase();
  if (!b) throw new Error("APP_CLOSED");
  const t = await token();
  const r = await fetch(b + path, {
    method: body ? "POST" : "GET",
    headers: { "Content-Type": "application/json", ...(t ? { Authorization: "Bearer " + t } : {}) },
    body: body ? JSON.stringify(body) : undefined,
  });
  const j = await r.json().catch(() => ({}));
  if (r.status === 401) {
    await chrome.storage.local.remove("token");
    throw new Error("NOT_PAIRED");
  }
  if (!r.ok) throw new Error(j.error || "HTTP " + r.status);
  return j;
}

// ---------------------------------------------------------------- yakalanan kaynaklar

/** Sekme kimliği → [{ url, kind, size, frameUrl, time }] (bellekte; tarayıcı kapanınca silinir) */
const media = new Map();

function hostOf(url) {
  try {
    return new URL(url).hostname;
  } catch {
    return "";
  }
}

function kindOf(url, type) {
  const path = url.split(/[?#]/)[0].toLowerCase();
  type = (type || "").toLowerCase();
  if (path.endsWith(".m3u8") || type.includes("mpegurl")) return "hls";
  if (path.endsWith(".mpd") || type.includes("dash+xml")) return "dash";
  // Parça dosyaları (yayının küçük parçaları) değil, tam dosyalar.
  if (/\.(ts|m4s|aac|vtt|webvtt)$/.test(path) || type.includes("mp2t")) return null;
  if (/\.(mp4|webm|mov|m4v|mkv|flv)$/.test(path) || type.startsWith("video/")) return "file";
  return null;
}

chrome.webRequest.onHeadersReceived.addListener(
  (d) => {
    if (d.tabId < 0 || d.statusCode >= 400) return;
    const host = hostOf(d.url);
    if (!host || SKIP_MEDIA_HOSTS.some((h) => host === h || host.endsWith("." + h))) return;
    const header = (n) => d.responseHeaders?.find((h) => h.name.toLowerCase() === n)?.value || "";
    const kind = kindOf(d.url, header("content-type"));
    if (!kind) return;
    const size = Number(header("content-range").split("/")[1] || header("content-length")) || 0;
    // Reklam ve önizleme gibi çok küçük dosyalar sayılmaz.
    if (kind === "file" && size && size < 400 * 1024) return;
    const list = media.get(d.tabId) || [];
    if (!list.some((m) => m.url === d.url)) {
      list.push({ url: d.url, kind, size, frameUrl: d.initiator || "", time: Date.now() });
      media.set(d.tabId, list.slice(-30));
      updateBadge(d.tabId);
    }
  },
  { urls: ["<all_urls>"], types: ["media", "xmlhttprequest", "other"] },
  ["responseHeaders"],
);

chrome.tabs.onUpdated.addListener((tabId, info) => {
  // Yeni sayfa açılınca o sekmenin listesi sıfırlanır.
  if (info.status === "loading" && info.url) {
    media.delete(tabId);
    updateBadge(tabId);
  }
});
chrome.tabs.onRemoved.addListener((tabId) => media.delete(tabId));

function updateBadge(tabId) {
  const n = (media.get(tabId) || []).length;
  chrome.action.setBadgeBackgroundColor({ color: "#E3C270", tabId });
  chrome.action.setBadgeText({ text: n ? String(n) : "", tabId }).catch(() => {});
}

/** En umut verici kaynak önce: ana yayın listesi (HLS/DASH), sonra en büyük dosya. */
function candidates(tabId) {
  const rank = { hls: 3, dash: 2, file: 1 };
  return [...(media.get(tabId) || [])].sort((a, b) => rank[b.kind] - rank[a.kind] || b.size - a.size || b.time - a.time);
}

// ---------------------------------------------------------------- çözümleme ve indirme

function cookieSite(url) {
  const host = hostOf(url);
  return COOKIE_SITES.find((d) => host === d || host.endsWith("." + d)) || null;
}

function netscape(list) {
  const lines = ["# Netscape HTTP Cookie File"];
  for (const c of list) {
    const domain = c.hostOnly ? c.domain : c.domain.startsWith(".") ? c.domain : "." + c.domain;
    lines.push([domain, c.hostOnly ? "FALSE" : "TRUE", c.path, c.secure ? "TRUE" : "FALSE",
      Math.floor(c.expirationDate || 0), c.name, c.value].join("\t"));
  }
  return lines.join("\n") + "\n";
}

/**
 * İndirme için çerezler: giriş isteyen sitelerde sayfanın, yakalanan kaynaklarda ise kaynağın
 * kendi alan adının çerezleri. Başka hiçbir sitenin çerezi gönderilmez.
 */
async function cookiesFor(url, fromMedia) {
  let list = [];
  if (fromMedia) list = await chrome.cookies.getAll({ url });
  else {
    const site = cookieSite(url);
    if (site) list = await chrome.cookies.getAll({ domain: site });
  }
  return list.length ? netscape(list) : null;
}

/**
 * Önce sayfa adresini dener; tanınmazsa yakalanan kaynakları sırayla dener.
 * Dönen: { kind, …probe, url, context, from: "page"|"media", mediaCount }
 */
async function smartProbe({ tabId, pageUrl, frameUrl, title, wholeList, src }) {
  const tries = [];
  // Üstünde durulan videonun kendi dosyası görünüyorsa (blob değilse) önce o.
  if (src && /^https?:/.test(src) && !wholeList) {
    const context = { referer: frameUrl || pageUrl, userAgent: navigator.userAgent, title };
    try {
      const p = await api("/v1/probe", { url: src, context });
      return { ...p, url: src, context, from: "media", mediaCount: (media.get(tabId) || []).length, pageLink: true };
    } catch (e) {
      if (["APP_CLOSED", "NOT_PAIRED"].includes(e.message)) throw e;
      tries.push(e.message);
    }
  }
  const embed = frameUrl && frameUrl !== pageUrl && /^https?:/.test(frameUrl) ? frameUrl : null;
  for (const u of [pageUrl, embed]) {
    if (!u || !/^https?:/.test(u)) continue;
    try {
      const p = await api("/v1/probe", { url: u, wholeList: !!wholeList });
      return { ...p, url: p.url || u, context: null, from: "page", mediaCount: (media.get(tabId) || []).length };
    } catch (e) {
      if (["APP_CLOSED", "NOT_PAIRED"].includes(e.message) || e.message.includes("araçları kur")) throw e;
      tries.push(e.message);
    }
  }
  const context = { referer: pageUrl, userAgent: navigator.userAgent, title };
  for (const m of candidates(tabId).slice(0, 3)) {
    try {
      const p = await api("/v1/probe", { url: m.url, context });
      return { ...p, url: m.url, context, from: "media", mediaCount: (media.get(tabId) || []).length };
    } catch (e) {
      tries.push(e.message);
    }
  }
  throw new Error(tries.length && !candidates(tabId).length
    ? "Bu sayfada indirilebilir video bulunamadı. Videoyu birkaç saniye oynatıp tekrar dene."
    : tries[tries.length - 1] || "Video bulunamadı.");
}

async function download({ url, option, context }) {
  const fromMedia = !!context?.referer;
  const cookies = await cookiesFor(url, fromMedia);
  return api("/v1/download", { url, option, cookies, context: context || undefined });
}

async function status() {
  const b = await findBase();
  if (!b) return { app: false, paired: false };
  const t = await token();
  const r = await fetch(b + "/v1/ping", { headers: t ? { Authorization: "Bearer " + t } : {} });
  const j = await r.json();
  return { app: true, paired: !!j.paired, version: j.version };
}

/** Mesajı gönderen (içerik betiği ya da eklenti penceresi) için sekme bilgisi. */
async function tabInfo(msg, sender) {
  if (sender.tab) return { tabId: sender.tab.id, pageUrl: sender.tab.url, title: sender.tab.title, frameUrl: sender.url };
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return { tabId: tab?.id, pageUrl: tab?.url, title: tab?.title, frameUrl: null };
}

async function handle(msg, sender) {
  switch (msg.type) {
    case "status":
      return status();
    case "pair": {
      const j = await api("/v1/pair", { code: msg.code });
      await chrome.storage.local.set({ token: j.token });
      return { ok: true };
    }
    case "smart-probe":
      return smartProbe({ ...(await tabInfo(msg, sender)), wholeList: msg.wholeList, src: msg.src });
    case "media": {
      const { tabId } = await tabInfo(msg, sender);
      return candidates(tabId);
    }
    case "probe-media": {
      const { pageUrl, title } = await tabInfo(msg, sender);
      const context = { referer: pageUrl, userAgent: navigator.userAgent, title };
      const p = await api("/v1/probe", { url: msg.url, context });
      return { ...p, url: msg.url, context, from: "media" };
    }
    case "download":
      return download(msg);
    case "playlist":
      return api("/v1/playlist", { url: msg.url, option: msg.option, ids: null });
    case "jobs":
      return api("/v1/jobs");
    case "open":
      return api("/v1/open", {});
    default:
      throw new Error("Bilinmeyen istek");
  }
}

chrome.runtime.onMessage.addListener((msg, sender, reply) => {
  handle(msg, sender)
    .then((data) => reply({ ok: true, data }))
    .catch((e) => reply({ ok: false, error: String(e.message || e) }));
  return true; // yanıt sonra gelecek
});

// ---------------------------------------------------------------- sağ tık menüsü

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.removeAll(() => {
    const contexts = ["video", "audio", "link", "page"];
    chrome.contextMenus.create({ id: "gmid", title: "GMID Tool ile indir", contexts });
    chrome.contextMenus.create({ id: "gmid:pick", parentId: "gmid", title: "Kaliteyi seç…", contexts });
    chrome.contextMenus.create({ id: "gmid:sep", parentId: "gmid", type: "separator", contexts });
    for (const [id, title] of QUALITIES) {
      chrome.contextMenus.create({ id: "gmid:q:" + id, parentId: "gmid", title, contexts });
    }
  });
});

function flash(tabId, ok) {
  if (tabId === undefined) return;
  chrome.action.setBadgeBackgroundColor({ color: ok ? "#5CC08A" : "#E06A6A", tabId });
  chrome.action.setBadgeText({ text: ok ? "✓" : "!", tabId });
  setTimeout(() => updateBadge(tabId), 4000);
}

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const tabId = tab?.id;
  if (info.menuItemId === "gmid:pick") {
    chrome.tabs.sendMessage(tabId, { type: "open-panel" }, { frameId: info.frameId ?? 0 }).catch(() => {});
    return;
  }
  const option = String(info.menuItemId).replace("gmid:q:", "");
  if (!QUALITIES.some(([q]) => q === option)) return;
  try {
    const direct = [info.srcUrl].find((u) => u && /^https?:/.test(u) && kindOf(u, ""));
    if (info.linkUrl && /^https?:/.test(info.linkUrl)) {
      // Bağlantıya sağ tık: bağlantının gösterdiği video.
      await download({ url: info.linkUrl, option });
    } else if (direct) {
      // Videonun kendi dosyası görünüyorsa (blob değilse) doğrudan o.
      await download({ url: direct, option, context: { referer: info.pageUrl, userAgent: navigator.userAgent, title: tab?.title } });
    } else {
      const p = await smartProbe({ tabId, pageUrl: info.pageUrl, frameUrl: info.frameUrl, title: tab?.title });
      if (p.kind === "playlist") await api("/v1/playlist", { url: p.url, option, ids: null });
      else await download({ url: p.url, option, context: p.context });
    }
    flash(tabId, true);
  } catch (e) {
    await chrome.storage.local.set({ lastError: String(e.message || e) });
    flash(tabId, false);
  }
});
