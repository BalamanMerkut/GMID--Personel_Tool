// GMID Tool eklentisi — arka plan. Uygulamayla sadece bu bilgisayardaki köprü üzerinden konuşur.
const PORTS = [47913, 47914, 47915, 47916, 47917];
// Giriş isteyen sitelerde çerezler (sadece o sitenin) indirme için uygulamaya geçici olarak verilir.
const COOKIE_SITES = ["instagram.com", "x.com", "twitter.com", "facebook.com"];

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

function cookieSite(url) {
  try {
    const host = new URL(url).hostname;
    return COOKIE_SITES.find((d) => host === d || host.endsWith("." + d)) || null;
  } catch {
    return null;
  }
}

/** Sitenin çerezlerini yt-dlp'nin okuduğu Netscape biçimine çevirir. */
async function cookiesFor(url) {
  const site = cookieSite(url);
  if (!site) return null;
  const list = await chrome.cookies.getAll({ domain: site });
  if (!list.length) return null;
  const lines = ["# Netscape HTTP Cookie File"];
  for (const c of list) {
    const domain = c.hostOnly ? c.domain : c.domain.startsWith(".") ? c.domain : "." + c.domain;
    lines.push([
      domain,
      c.hostOnly ? "FALSE" : "TRUE",
      c.path,
      c.secure ? "TRUE" : "FALSE",
      Math.floor(c.expirationDate || 0),
      c.name,
      c.value,
    ].join("\t"));
  }
  return lines.join("\n") + "\n";
}

async function status() {
  const b = await findBase();
  if (!b) return { app: false, paired: false };
  const t = await token();
  const r = await fetch(b + "/v1/ping", { headers: t ? { Authorization: "Bearer " + t } : {} });
  const j = await r.json();
  return { app: true, paired: !!j.paired, version: j.version };
}

async function download(url, option) {
  const cookies = await cookiesFor(url);
  return api("/v1/download", { url, option, cookies });
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
    case "probe":
      return api("/v1/probe", { url: msg.url });
    case "download":
      return download(msg.url, msg.option);
    case "jobs":
      return api("/v1/jobs");
    case "open":
      return api("/v1/open", {});
    case "video-page":
      if (sender.tab?.id !== undefined) {
        chrome.action.setBadgeBackgroundColor({ color: "#E3C270", tabId: sender.tab.id });
        chrome.action.setBadgeText({ text: msg.on ? "▼" : "", tabId: sender.tab.id });
      }
      return { ok: true };
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

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "gmid-download",
    title: "GMID Tool ile indir",
    contexts: ["page", "link", "video"],
  });
});

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const candidates = [info.linkUrl, info.srcUrl, info.pageUrl].filter((u) => u && /^https?:/.test(u));
  const url = candidates[0];
  if (!url) return;
  const tabId = tab?.id;
  try {
    // "auto": önerilen kalite (1080p ya da altındaki en iyisi).
    await download(url, "auto");
    if (tabId !== undefined) {
      chrome.action.setBadgeBackgroundColor({ color: "#5CC08A", tabId });
      chrome.action.setBadgeText({ text: "✓", tabId });
    }
  } catch (e) {
    await chrome.storage.local.set({ lastError: String(e.message || e) });
    if (tabId !== undefined) {
      chrome.action.setBadgeBackgroundColor({ color: "#E06A6A", tabId });
      chrome.action.setBadgeText({ text: "!", tabId });
    }
  }
});
