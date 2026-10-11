// GMID Tool eklentisi — sayfanın kendi dünyasında çalışır (oynatıcıyla aynı yerde).
// Bazı film siteleri yayın listesinin (HLS, m3u8) adresini ve türünü gizler: ".txt", ".png" ya da
// "text/plain" gibi görünür, ağ trafiğinden anlaşılmaz. Burada oynatıcının indirdiği yanıtın
// İÇERİĞİNE bakılır: "#EXTM3U" ile başlıyorsa bu bir yayın listesidir ve adresi eklentiye bildirilir.
// Hiçbir yanıt değiştirilmez, hiçbir şey dışarı gönderilmez.
(() => {
  if (window.__gmidSniff) return;
  Object.defineProperty(window, "__gmidSniff", { value: true });
  const MAX = 2 * 1024 * 1024;
  const seen = new Set();

  function report(url, text) {
    if (typeof text !== "string" || !text.startsWith("#EXTM3U")) return;
    try {
      url = new URL(url, location.href).href;
    } catch {
      return;
    }
    if (!/^https?:/.test(url) || seen.has(url)) return;
    seen.add(url);
    // Ana liste (çözünürlük seçenekleri) tek kalitelik listeden önce denenir.
    window.postMessage({ __gmid: "hls", url, master: text.includes("#EXT-X-STREAM-INF") }, "*");
  }

  const native = (fn, name) => {
    try {
      Object.defineProperty(fn, "toString", { value: () => `function ${name}() { [native code] }` });
    } catch {
      /* yok say */
    }
    return fn;
  };

  // XMLHttpRequest (hls.js, video.js, JW Player…)
  const XHR = XMLHttpRequest.prototype;
  const open = XHR.open;
  const send = XHR.send;
  XHR.open = native(function (method, url) {
    try {
      this.__gmidUrl = String(url);
    } catch {
      /* yok say */
    }
    return open.apply(this, arguments);
  }, "open");
  XHR.send = native(function () {
    this.addEventListener("load", function () {
      try {
        const url = this.responseURL || this.__gmidUrl;
        if (this.responseType === "" || this.responseType === "text") {
          if (this.responseText.length < MAX) report(url, this.responseText);
        } else if (this.responseType === "arraybuffer" && this.response && this.response.byteLength < MAX) {
          const head = new TextDecoder().decode(new Uint8Array(this.response, 0, Math.min(7, this.response.byteLength)));
          if (head === "#EXTM3U") report(url, new TextDecoder().decode(this.response));
        }
      } catch {
        /* yok say */
      }
    });
    return send.apply(this, arguments);
  }, "send");

  // fetch: yanıtın yalnızca ilk parçasına bakılır; yayın listesi değilse okumayı hemen bırakır.
  const origFetch = window.fetch;
  if (typeof origFetch === "function") {
    window.fetch = native(function () {
      const p = origFetch.apply(this, arguments);
      p.then((r) => {
        try {
          const type = (r.headers.get("content-type") || "").toLowerCase();
          const len = Number(r.headers.get("content-length") || 0);
          if (!r.ok || !r.body || len > MAX || type.startsWith("video/") || type.startsWith("audio/")) return;
          const reader = r.clone().body.getReader();
          const decoder = new TextDecoder();
          let text = "";
          const pump = () =>
            reader.read().then(({ done, value }) => {
              if (value) text += decoder.decode(value, { stream: true });
              if (text.length >= 7 && !text.startsWith("#EXTM3U")) return reader.cancel();
              if (text.length > MAX) return reader.cancel();
              if (done) return report(r.url, text);
              return pump();
            });
          pump().catch(() => {});
        } catch {
          /* yok say */
        }
      }, () => {});
      return p;
    }, "fetch");
  }
})();
