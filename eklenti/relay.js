// GMID Tool eklentisi — sniff.js'in (sayfanın dünyası) bulduğu yayın listelerini ve altyazıları arka plana iletir.
window.addEventListener("message", (e) => {
  if (e.source !== window || !e.data || typeof e.data.url !== "string") return;
  const type = e.data.__gmid === "hls" ? "found-media" : e.data.__gmid === "sub" ? "found-sub" : null;
  if (!type) return;
  try {
    chrome.runtime.sendMessage({ type, url: e.data.url, master: !!e.data.master }, () => void chrome.runtime.lastError);
  } catch {
    /* eklenti yeniden yüklendiyse sessizce geç */
  }
});
