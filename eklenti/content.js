// Her sitede: fare bir videonun üstüne gelince sağ üst köşesinde küçük "GMID ile indir" düğmesi çıkar
// (IDM'deki gibi). Düğme ya da sağ tık → "Kaliteyi seç…" kalite panelini açar. Sayfanın görünümünü
// bozmasın diye her şey kendi gölge DOM'unda durur; oynatıcının üstündeki katmanlar fareyi
// yakalasa bile çalışsın diye fare konumu videoların alanıyla karşılaştırılır.
(() => {
  if (window.__gmidLoaded) return;
  window.__gmidLoaded = true;

  const MIN_W = 200;
  const MIN_H = 110;

  const host = document.createElement("div");
  host.style.cssText = "position:fixed;left:0;top:0;width:0;height:0;z-index:2147483646;";
  const root = host.attachShadow({ mode: "closed" });
  root.innerHTML = `
    <style>
      * { box-sizing: border-box; font-family: "Segoe UI", system-ui, sans-serif; }
      .pill { position: fixed; display: none; align-items: center; gap: 6px; padding: 5px 10px 5px 6px; border-radius: 999px;
        cursor: pointer; background: rgba(18,20,23,.9); color: #e3c270; border: 1px solid rgba(227,194,112,.6);
        font: 700 12px/1 "Segoe UI",system-ui,sans-serif; box-shadow: 0 4px 16px rgba(0,0,0,.45); }
      .pill:hover { background: rgba(18,20,23,1); border-color: #f0d878; }
      .pill img { width: 18px; height: 18px; }
      .panel { position: fixed; width: 310px; max-height: 70vh; overflow: auto; padding: 12px; border-radius: 14px;
        background: #121417; color: #ece7da; border: 1px solid #363a40; box-shadow: 0 12px 34px rgba(0,0,0,.55);
        font-size: 13px; display: flex; flex-direction: column; gap: 6px; }
      .panel[hidden] { display: none; }
      .head { display: flex; align-items: center; gap: 8px; padding-right: 18px; }
      .head img { width: 20px; height: 20px; }
      .head b { color: #e3c270; font-size: 12px; letter-spacing: .06em; }
      .t { font-weight: 600; overflow: hidden; text-overflow: ellipsis; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
      .m { color: #a4a8ac; font-size: 12px; }
      .e { color: #e06a6a; font-size: 12px; }
      .ok { color: #5cc08a; font-weight: 600; }
      button.o { display: flex; justify-content: space-between; gap: 8px; width: 100%; padding: 8px 10px; border-radius: 9px; cursor: pointer;
        background: #22252a; color: inherit; border: 1px solid #363a40; font: inherit; text-align: left; }
      button.o:hover { border-color: #e3c270; }
      button.o.r { border-color: rgba(227,194,112,.6); }
      button.o small { color: #a4a8ac; }
      button.link { background: none; border: 0; color: #35a0a2; font: inherit; font-weight: 600; cursor: pointer; padding: 2px 0; text-align: left; }
      button.link:hover { color: #60d8d8; }
      .x { position: absolute; right: 8px; top: 6px; background: none; border: 0; color: #a4a8ac; cursor: pointer; font-size: 18px; line-height: 1; }
    </style>
    <div class="pill" title="GMID Tool ile indir"><img alt=""><span>GMID ile indir</span></div>
    <div class="panel" hidden><button class="x" title="Kapat">×</button>
      <div class="head"><img alt=""><b>GMID TOOL</b></div><div class="body"></div></div>`;
  for (const img of root.querySelectorAll("img")) img.src = chrome.runtime.getURL("icons/32.png");
  const pill = root.querySelector(".pill");
  const panel = root.querySelector(".panel");
  const body = root.querySelector(".body");
  root.querySelector(".x").addEventListener("click", () => (panel.hidden = true));
  // Panelin dışında bir yere tıklayınca ya da Esc ile kapanır (× düğmesine gerek kalmadan).
  document.addEventListener("pointerdown", (e) => {
    if (!panel.hidden && !e.composedPath().includes(host)) panel.hidden = true;
  }, true);
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && !panel.hidden) panel.hidden = true;
  }, true);

  const send = (msg) => new Promise((resolve, reject) =>
    chrome.runtime.sendMessage(msg, (r) => {
      if (chrome.runtime.lastError) return reject(new Error("Eklenti yeniden yüklendi; sayfayı yenile."));
      r && r.ok ? resolve(r.data) : reject(new Error(r ? r.error : "Bağlantı yok"));
    }));
  const explain = (e) => {
    const m = String(e.message || e);
    if (m === "APP_CLOSED") return "GMID Tool kapalı. Uygulamayı aç (tepside çalışıyor olmalı).";
    if (m === "NOT_PAIRED") return "Önce eklenti simgesine tıklayıp GMID Tool ile eşleştir.";
    return m;
  };
  const size = (b) => (!b ? "" : b < 1024 ** 3 ? `~${Math.round(b / 1024 / 1024)} MB` : `~${(b / 1024 ** 3).toFixed(1)} GB`);
  const el = (tag, cls, text) => Object.assign(document.createElement(tag), cls ? { className: cls } : {}, text != null ? { textContent: text } : {});

  // ---- videonun kendi bağlantısı ----
  // Akış sayfalarında (X ana sayfa, Instagram, YouTube ana sayfa) sayfanın adresi videonun değil akışın
  // adresidir; videonun gönderisinin bağlantısı bulunur.
  function linkHint(v) {
    if (!v) return null;
    const host = location.hostname;
    const abs = (a) => (a && a.href && /^https?:/.test(a.href) ? a.href : null);
    if (/(^|\.)(x|twitter)\.com$/.test(host)) {
      const art = v.closest("article");
      return abs(art?.querySelector('a[href*="/status/"] time')?.closest("a")) || abs(art?.querySelector('a[href*="/status/"]'));
    }
    if (/(^|\.)instagram\.com$/.test(host)) {
      const art = v.closest("article") || v.closest("div[role=dialog]");
      return abs(art?.querySelector('a[href*="/reel/"], a[href*="/p/"]'));
    }
    if (/(^|\.)youtube\.com$/.test(host) && !/\/(watch|shorts)/.test(location.pathname)) {
      const item = v.closest("ytd-rich-item-renderer, ytd-video-renderer, ytd-compact-video-renderer, ytd-reel-item-renderer");
      return abs(item?.querySelector('a#thumbnail, a[href*="/watch"], a[href*="/shorts/"]'))
        || abs(document.querySelector('ytd-video-preview a[href*="/watch"], #video-preview a[href*="/watch"]'));
    }
    if (/(^|\.)tiktok\.com$/.test(host)) return tiktokLink(v);
    if (/(^|\.)facebook\.com$/.test(host)) {
      const art = v.closest('[role="article"]');
      return abs(art?.querySelector('a[href*="/videos/"], a[href*="/reel/"], a[href*="/watch"]'));
    }
    return null;
  }

  // TikTok: akışta (Sana Özel, Takip, Keşfet) adres çubuğunda akışın adresi durur. Videonun kimliği
  // oynatıcının kutusunda ("xgwrapper-0-7412…"), bazen de karttaki bağlantıda yazar; kullanıcı adı
  // karttaki profil bağlantısından. yt-dlp'nin tanıdığı biçim: tiktok.com/@kullanici/video/KİMLİK.
  function tiktokLink(v) {
    const page = location.pathname.match(/^\/@([^/]+)\/(video|photo)\/(\d{8,})/);
    let id = null;
    let user = null;
    let node = v;
    for (let i = 0; node && i < 14 && !(id && user); i++, node = node.parentElement) {
      if (!id) {
        const own = (node.id || "").match(/(\d{15,22})/);
        const a = node.querySelector?.('a[href*="/video/"]');
        const fromLink = a?.getAttribute("href")?.match(/\/@([^/?#]+)\/video\/(\d{8,})/);
        if (own) id = own[1];
        else if (fromLink) {
          id = fromLink[2];
          user = user || fromLink[1];
        }
      }
      if (!user && id) {
        const prof = node.querySelector?.('a[href^="/@"]:not([href*="/video/"])');
        const m = prof?.getAttribute("href")?.match(/^\/@([^/?#]+)/);
        if (m) user = m[1];
      }
      // Bir sonraki videonun kartına taşmayalım.
      if (node.matches?.('article, [data-e2e="recommend-list-item-container"]')) break;
    }
    if (id) return `https://www.tiktok.com/@${user || "_"}/video/${id}`;
    // Video sayfası ya da açılan pencere: adres zaten videonun.
    if (page) return location.href.split(/[?#]/)[0];
    return null;
  }

  // ---- videoları izle ----
  let videos = [];
  function scan() {
    videos = [...document.querySelectorAll("video")].filter((v) => {
      const r = v.getBoundingClientRect();
      return r.width >= MIN_W && r.height >= MIN_H && getComputedStyle(v).visibility !== "hidden";
    });
    if (videos.length && !host.isConnected) (document.body || document.documentElement).append(host);
  }
  let scanTimer = 0;
  new MutationObserver(() => {
    clearTimeout(scanTimer);
    scanTimer = setTimeout(scan, 400);
  }).observe(document.documentElement, { childList: true, subtree: true });
  setInterval(scan, 2000);
  scan();

  let hovered = null;
  let hideTimer = 0;
  function placePill(v) {
    const r = v.getBoundingClientRect();
    pill.style.display = "flex";
    pill.style.top = `${Math.max(4, r.top + 10)}px`;
    pill.style.left = `${Math.max(4, r.right - 150)}px`;
  }
  let pending = false;
  document.addEventListener("mousemove", (e) => {
    if (pending) return;
    pending = true;
    requestAnimationFrame(() => {
      pending = false;
      const v = videos.find((x) => {
        const r = x.getBoundingClientRect();
        return e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
      });
      if (v) {
        hovered = v;
        clearTimeout(hideTimer);
        placePill(v);
      } else if (pill.style.display !== "none" && !pill.matches(":hover")) {
        clearTimeout(hideTimer);
        hideTimer = setTimeout(() => (pill.style.display = "none"), 700);
      }
    });
  }, { passive: true });
  window.addEventListener("scroll", () => hovered && pill.style.display !== "none" && placePill(hovered), { passive: true });

  let lastClick = { x: 0, y: 0 };
  let lastTarget = null;
  document.addEventListener("contextmenu", (e) => {
    lastClick = { x: e.clientX, y: e.clientY };
    // Sağ tıklanan yerin altındaki video (oynatıcı katmanları üstte olsa da).
    lastTarget = videos.find((v) => {
      const r = v.getBoundingClientRect();
      return e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom;
    }) || null;
  }, true);

  function placePanel(x, y) {
    if (!host.isConnected) (document.body || document.documentElement).append(host);
    panel.hidden = false;
    const w = 310;
    panel.style.left = `${Math.min(Math.max(8, x), innerWidth - w - 8)}px`;
    panel.style.top = `${Math.min(Math.max(8, y), innerHeight - 200)}px`;
  }

  // ---- kalite paneli ----
  function render(p) {
    body.replaceChildren(el("div", "t", p.title));
    const site = p.site && p.site.toLowerCase() !== "generic" ? p.site : "";
    const meta = [p.uploader, p.from === "media" ? "Bu video" : site].filter(Boolean).join(" · ");
    body.append(el("div", "m", meta));
    if (p.kind === "playlist") {
      body.append(el("div", "m", `Çalma listesi · ${p.entries.length} video. Hepsini indir:`));
      for (const [id, label] of [["1080", "1080p"], ["720", "720p"], ["mp3", "Sadece ses (MP3)"]]) {
        const b = el("button", "o");
        b.append(el("span", "", label), el("small", "", `${p.entries.length} dosya`));
        b.onclick = () => go(b, send({ type: "playlist", url: p.url, option: id }), (r) => `${r.added} video sıraya eklendi ✓`);
        body.append(b);
      }
      return;
    }
    for (const o of p.options) {
      const b = el("button", "o" + (o.recommended ? " r" : ""));
      b.append(el("span", "", o.label + (o.detail ? " · " + o.detail : "")), el("small", "", size(o.size)));
      b.onclick = () => go(b, send({ type: "download", url: p.url, option: o.id, context: p.context }), () => "Sıraya eklendi ✓");
      body.append(b);
    }
    if (p.pageLink) {
      const l = el("button", "link", "Sayfadaki tüm videolar");
      l.onclick = () => load(false, null);
      body.append(l);
    }
    if (p.from === "page" && /[?&]list=/.test(location.href)) {
      const l = el("button", "link", "Bu çalma listesinin tamamını göster");
      l.onclick = () => load(true);
      body.append(l);
    }
  }

  async function go(button, promise, done) {
    button.disabled = true;
    try {
      const r = await promise;
      body.replaceChildren(el("div", "ok", done(r)), el("div", "m", "İlerlemeyi eklenti simgesinden ya da GMID Tool'dan izleyebilirsin."));
      setTimeout(() => (panel.hidden = true), 2600);
    } catch (e) {
      body.append(el("div", "e", explain(e)));
      button.disabled = false;
    }
  }

  async function load(wholeList = false, src = null, hint = null) {
    body.replaceChildren(el("div", "m", "Video aranıyor…"));
    try {
      render(await send({ type: "smart-probe", wholeList, src, hint }));
    } catch (e) {
      body.replaceChildren(el("div", "e", explain(e)));
    }
  }

  pill.addEventListener("click", () => {
    const r = pill.getBoundingClientRect();
    placePanel(r.right - 310, r.bottom + 6);
    const src = hovered?.currentSrc || hovered?.src || "";
    void load(false, /^https?:/.test(src) ? src : null, linkHint(hovered));
  });

  chrome.runtime.onMessage.addListener((msg, _sender, reply) => {
    if (msg.type === "open-panel") {
      placePanel(lastClick.x, lastClick.y);
      void load(false, null, linkHint(lastTarget));
    }
    if (msg.type === "hint") reply(linkHint(lastTarget));
    if (msg.type === "hint-visible") {
      // Ekranda en çok görünen video; oynayan önce.
      const seen = (v) => {
        const r = v.getBoundingClientRect();
        const w = Math.max(0, Math.min(r.right, innerWidth) - Math.max(r.left, 0));
        const h = Math.max(0, Math.min(r.bottom, innerHeight) - Math.max(r.top, 0));
        return w * h * (v.paused ? 1 : 4);
      };
      const best = [...videos].sort((a, b) => seen(b) - seen(a))[0];
      reply(best && seen(best) > 0 ? linkHint(best) : null);
    }
  });
})();
