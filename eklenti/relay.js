// GMID Tool eklentisi — sniff.js'in (sayfanın dünyası) bulduğu yayın listelerini arka plana iletir.
window.addEventListener("message", (e) => {
  if (e.source !== window || !e.data || e.data.__gmid !== "hls" || typeof e.data.url !== "string") return;
  try {
    chrome.runtime.sendMessage({ type: "found-media", url: e.data.url, master: !!e.data.master }, () => void chrome.runtime.lastError);
  } catch {
    /* eklenti yeniden yüklendiyse sessizce geç */
  }
});
