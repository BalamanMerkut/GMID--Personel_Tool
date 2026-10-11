// Video sayfalarında sol altta küçük "GMID ile indir" düğmesi. Sayfanın görünümünü bozmasın diye
// kendi gölge DOM'unda durur. YouTube gibi sayfa yenilemeden adres değiştiren sitelerde de çalışır.
(() => {
  const VIDEO_PAGE = [
    /youtube\.com\/watch\?/, /youtube\.com\/shorts\//,
    /(x|twitter)\.com\/[^/]+\/status\/\d+/,
    /instagram\.com\/(p|reel|reels)\//,
    /tiktok\.com\/@[^/]+\/video\//,
  ];
  const isVideoPage = () => VIDEO_PAGE.some((r) => r.test(location.href));

  const host = document.createElement("div");
  host.style.cssText = "position:fixed;left:16px;bottom:16px;z-index:2147483646;";
  const root = host.attachShadow({ mode: "closed" });
  root.innerHTML = `
    <style>
      * { box-sizing: border-box; font-family: "Segoe UI", system-ui, sans-serif; }
      .fab { display:flex; align-items:center; gap:8px; padding:8px 12px 8px 8px; border-radius:999px; cursor:pointer;
        background:rgba(18,20,23,.92); color:#e3c270; border:1px solid rgba(227,194,112,.55);
        font:700 12.5px/1 "Segoe UI",system-ui,sans-serif; box-shadow:0 6px 20px rgba(0,0,0,.4); opacity:.82; transition:opacity .15s; }
      .fab:hover { opacity:1; }
      .fab img { width:20px; height:20px; }
      .panel { position:absolute; left:0; bottom:46px; width:300px; padding:10px; border-radius:14px;
        background:#121417; color:#ece7da; border:1px solid #363a40; box-shadow:0 10px 30px rgba(0,0,0,.5);
        font-size:13px; display:flex; flex-direction:column; gap:6px; }
      .panel[hidden] { display:none; }
      .t { font-weight:600; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
      .m { color:#a4a8ac; font-size:12px; }
      .e { color:#e06a6a; font-size:12px; }
      button.o { display:flex; justify-content:space-between; gap:8px; width:100%; padding:7px 10px; border-radius:9px; cursor:pointer;
        background:#22252a; color:inherit; border:1px solid #363a40; font:inherit; text-align:left; }
      button.o:hover { border-color:#e3c270; }
      button.o.r { border-color:rgba(227,194,112,.6); }
      small { color:#a4a8ac; }
      .x { position:absolute; right:8px; top:6px; background:none; border:0; color:#a4a8ac; cursor:pointer; font-size:16px; }
    </style>
    <div class="panel" hidden><button class="x" title="Kapat">×</button><div class="body"></div></div>
    <div class="fab" title="GMID Tool ile indir"><img alt=""><span>GMID ile indir</span></div>`;
  root.querySelector("img").src = chrome.runtime.getURL("icons/32.png");
  const panel = root.querySelector(".panel");
  const body = root.querySelector(".body");

  const send = (msg) => new Promise((resolve, reject) =>
    chrome.runtime.sendMessage(msg, (r) => (r && r.ok ? resolve(r.data) : reject(new Error(r ? r.error : "Bağlantı yok")))));
  const explain = (e) => {
    const m = String(e.message || e);
    if (m === "APP_CLOSED") return "GMID Tool kapalı. Uygulamayı aç.";
    if (m === "NOT_PAIRED") return "Önce eklenti simgesine tıklayıp GMID Tool ile eşleştir.";
    return m;
  };
  const size = (b) => (!b ? "" : b < 1024 ** 3 ? `~${Math.round(b / 1024 / 1024)} MB` : `~${(b / 1024 ** 3).toFixed(1)} GB`);
  const line = (cls, text) => Object.assign(document.createElement("div"), { className: cls, textContent: text });

  async function open() {
    panel.hidden = false;
    body.replaceChildren(line("m", "Kaliteler getiriliyor…"));
    const url = location.href;
    try {
      const p = await send({ type: "probe", url });
      body.replaceChildren(line("t", p.title), line("m", p.uploader || p.site));
      for (const o of p.options) {
        const b = document.createElement("button");
        b.className = "o" + (o.recommended ? " r" : "");
        b.append(Object.assign(document.createElement("span"), { textContent: o.label + (o.detail ? " · " + o.detail : "") }),
          Object.assign(document.createElement("small"), { textContent: size(o.size) }));
        b.onclick = async () => {
          b.disabled = true;
          try {
            await send({ type: "download", url, option: o.id });
            body.replaceChildren(line("t", "Sıraya eklendi ✓"), line("m", "İlerlemeyi eklenti simgesinden ya da GMID Tool'dan izleyebilirsin."));
            setTimeout(() => (panel.hidden = true), 2500);
          } catch (e) {
            body.append(line("e", explain(e)));
            b.disabled = false;
          }
        };
        body.append(b);
      }
    } catch (e) {
      body.replaceChildren(line("e", explain(e)));
    }
  }

  root.querySelector(".fab").addEventListener("click", () => (panel.hidden ? open() : (panel.hidden = true)));
  root.querySelector(".x").addEventListener("click", () => (panel.hidden = true));

  let last = "";
  function sync() {
    if (location.href === last) return;
    last = location.href;
    const on = isVideoPage();
    panel.hidden = true;
    if (on && !host.isConnected) document.documentElement.append(host);
    if (!on && host.isConnected) host.remove();
    chrome.runtime.sendMessage({ type: "video-page", on }, () => void chrome.runtime.lastError);
  }
  sync();
  setInterval(sync, 1000);
})();
