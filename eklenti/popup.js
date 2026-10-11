// Eklenti penceresi: açık sekmedeki videonun kalitelerini gösterir, seçileni GMID Tool'a indirtir.
const view = document.getElementById("view");

function send(msg) {
  return new Promise((resolve, reject) =>
    chrome.runtime.sendMessage(msg, (r) => (r && r.ok ? resolve(r.data) : reject(new Error(r ? r.error : "Bağlantı yok")))),
  );
}

function el(tag, attrs = {}, ...children) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k === "class") e.className = v;
    else if (k.startsWith("on")) e.addEventListener(k.slice(2), v);
    else e.setAttribute(k, v);
  }
  for (const c of children) if (c != null) e.append(c);
  return e;
}

function size(b) {
  if (!b) return "";
  if (b < 1024 * 1024) return `~${Math.round(b / 1024)} KB`;
  if (b < 1024 ** 3) return `~${(b / 1024 / 1024).toFixed(b < 100 * 1024 * 1024 ? 1 : 0)} MB`;
  return `~${(b / 1024 ** 3).toFixed(1)} GB`;
}

function show(...nodes) {
  view.replaceChildren(...nodes);
}

function explain(e) {
  const m = String(e.message || e);
  if (m === "APP_CLOSED") return "GMID Tool kapalı görünüyor. Uygulamayı aç (tepside, saatin yanında çalışıyor olmalı).";
  if (m === "NOT_PAIRED") return "Eşleştirme gerekli.";
  return m;
}

async function currentUrl() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab && /^https?:/.test(tab.url || "") ? tab.url : null;
}

function pairView() {
  const input = el("input", { maxlength: "6", inputmode: "numeric", placeholder: "000000", autofocus: "" });
  const msg = el("p", { class: "err" });
  const btn = el("button", { class: "btn", onclick: async () => {
    btn.disabled = true;
    try {
      await send({ type: "pair", code: input.value });
      start();
    } catch (e) {
      msg.textContent = explain(e);
      btn.disabled = false;
    }
  } }, "Eşleştir");
  input.addEventListener("keydown", (e) => e.key === "Enter" && btn.click());
  show(
    el("p", {}, "Eklentiyi GMID Tool'a bir kez bağlaman gerekiyor:"),
    el("ol", {},
      el("li", {}, "GMID Tool → Video İndirici → Tarayıcı eklentisi"),
      el("li", {}, "“Eşleştir”e bas, çıkan 6 haneli kodu buraya yaz")),
    input, btn, msg,
  );
  input.focus();
}

let polling = null;

async function jobsView(container) {
  const render = async () => {
    try {
      const { downloads, live } = await send({ type: "jobs" });
      const byId = Object.fromEntries(live.map((l) => [l.id, l]));
      container.replaceChildren(...downloads.slice(0, 5).map((d) => {
        const l = byId[d.id];
        const pct = Math.round(((l ? l.progress : d.progress) || 0) * 100);
        const state = d.status === "done" ? el("span", { class: "done" }, "İndi ✓")
          : d.status === "error" ? el("span", { class: "err" }, "Hata")
          : d.status === "queued" ? el("span", { class: "muted" }, "Sırada")
          : d.status === "paused" ? el("span", { class: "muted" }, "Duraklatıldı")
          : el("span", {}, d.status === "processing" ? "Birleştiriliyor" : `%${pct}`);
        return el("div", { class: "job" },
          el("div", { class: "row" }, el("span", { class: "name", title: d.title }, d.title), state),
          d.status === "done" ? null : el("div", { class: "bar" }, el("i", { style: `width:${pct}%` })));
      }));
    } catch {
      /* uygulama kapandıysa sessizce dur */
    }
  };
  await render();
  clearInterval(polling);
  polling = setInterval(render, 1000);
}

async function videoView() {
  const url = await currentUrl();
  const jobs = el("div");
  if (!url) {
    show(el("p", { class: "muted" }, "Bu sekmede indirilecek bir şey yok. Video olan bir sayfada tekrar dene."), jobs);
    return jobsView(jobs);
  }
  show(el("p", { class: "muted" }, "Video aranıyor…"), jobs);
  void jobsView(jobs);
  try {
    const p = await send({ type: "probe", url });
    const opts = el("div", { class: "opts" }, ...p.options.map((o) =>
      el("button", { class: "opt" + (o.recommended ? " rec" : ""), onclick: async (e) => {
        e.currentTarget.disabled = true;
        try {
          await send({ type: "download", url, option: o.id });
          e.currentTarget.replaceChildren(el("b", {}, "Sıraya eklendi ✓"));
        } catch (err) {
          e.currentTarget.replaceChildren(el("span", { class: "err" }, explain(err)));
        }
      } },
        el("span", {}, el("b", {}, o.label), o.recommended ? el("span", { class: "tag" }, "ÖNERİLEN") : null,
          o.detail ? el("small", {}, " · " + o.detail) : null),
        el("small", {}, size(o.size)))));
    show(
      p.thumbnail ? el("img", { class: "thumb", src: p.thumbnail, alt: "" }) : null,
      el("p", { class: "title" }, p.title),
      el("p", { class: "muted" }, [p.uploader, p.site].filter(Boolean).join(" · ")),
      opts, jobs,
    );
  } catch (e) {
    show(el("p", { class: "err" }, explain(e)), jobs);
  }
}

async function start() {
  try {
    const s = await send({ type: "status" });
    if (!s.app) return show(el("p", { class: "err" }, explain(new Error("APP_CLOSED"))));
    if (!s.paired) return pairView();
    const { lastError } = await chrome.storage.local.get("lastError");
    if (lastError) await chrome.storage.local.remove("lastError");
    await videoView();
    if (lastError) view.prepend(el("p", { class: "err" }, "Son sağ tık indirmesi: " + lastError));
  } catch (e) {
    show(el("p", { class: "err" }, explain(e)));
  }
}

document.getElementById("open").addEventListener("click", () => send({ type: "open" }).catch(() => {}));
start();
