//! Tanıtım ve indirme sayfası (gmid.molayazilim.com.tr): arkadaşlar GMID Tool'u buradan indirir.
//! `/indir` her zaman GitHub'daki en son kurulum dosyasına yönlendirir.

use axum::http::header;
use axum::response::{Html, IntoResponse, Redirect};

const KURULUM: &str =
    "https://github.com/BalamanMerkut/GMID--Personel_Tool/releases/latest/download/GMID-Tool-Kurulum.exe";

pub async fn indir() -> Redirect {
    Redirect::temporary(KURULUM)
}

pub async fn ana() -> impl IntoResponse {
    (
        [(header::CACHE_CONTROL, "public, max-age=300")],
        Html(SAYFA),
    )
}

const SAYFA: &str = r##"<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GMID Tool · Game Mansion In Discord</title>
<meta name="description" content="GMID topluluğunun Windows uygulaması: oyunların, arkadaşların, sohbet, film-dizi arşivi, video indirici ve daha fazlası.">
<style>
  :root { --gold: #e3c270; --gold2: #f3dc9b; --text: #ecebe6; --muted: #a9adb5; --bg: #121418; }
  * { box-sizing: border-box; }
  body { margin: 0; min-height: 100vh; font-family: "Segoe UI", system-ui, sans-serif; color: var(--text);
    background: radial-gradient(1200px 600px at 20% -10%, rgba(227,194,112,.18), transparent 60%),
                radial-gradient(900px 500px at 110% 110%, rgba(110,96,220,.16), transparent 60%), var(--bg); }
  main { max-width: 980px; margin: 0 auto; padding: 56px 22px 40px; }
  .ust { display: flex; align-items: center; gap: 14px; color: var(--gold); font-weight: 800; letter-spacing: .14em; font-size: 12px; text-transform: uppercase; }
  .ust i { width: 10px; height: 10px; border-radius: 50%; background: var(--gold); box-shadow: 0 0 14px var(--gold); }
  h1 { font-size: clamp(34px, 6vw, 58px); line-height: 1.05; margin: 18px 0 14px; }
  h1 span { color: var(--gold2); }
  p.giris { font-size: 18px; color: var(--muted); max-width: 640px; line-height: 1.55; margin: 0 0 28px; }
  .dugmeler { display: flex; gap: 12px; flex-wrap: wrap; align-items: center; }
  a.indir { display: inline-flex; align-items: center; gap: 10px; padding: 15px 26px; border-radius: 14px; font-weight: 800; font-size: 17px; color: #1a1408;
    text-decoration: none; background: linear-gradient(160deg, var(--gold2), var(--gold)); box-shadow: 0 12px 34px rgba(227,194,112,.3); }
  a.indir:hover { filter: brightness(1.06); transform: translateY(-1px); }
  .not { color: var(--muted); font-size: 13px; }
  .izgara { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 14px; margin-top: 52px; }
  .kart { padding: 18px 18px 16px; border-radius: 16px; border: 1px solid rgba(255,255,255,.08); background: rgba(255,255,255,.035); }
  .kart b { display: block; font-size: 16px; margin-bottom: 6px; }
  .kart span { color: var(--muted); font-size: 14px; line-height: 1.5; }
  .kart em { font-style: normal; font-size: 22px; display: block; margin-bottom: 8px; }
  details { margin-top: 40px; padding: 16px 18px; border-radius: 14px; border: 1px solid rgba(255,255,255,.08); background: rgba(0,0,0,.2); color: var(--muted); font-size: 14px; line-height: 1.6; }
  summary { cursor: pointer; color: var(--text); font-weight: 700; }
  footer { margin-top: 46px; color: #6f747d; font-size: 12.5px; }
</style>
</head>
<body>
<main>
  <div class="ust"><i></i>Game Mansion In Discord</div>
  <h1>GMID Tool ile <span>malikânede</span> buluş.</h1>
  <p class="giris">Arkadaşların ne oynuyor, hangi diziyi kaç puanladı, kim çevrim içi; hepsi tek uygulamada. Sohbet et, puanla, paylaş.
  Üstüne oyun içi panel, video indirici, sesle yazma ve daha fazlası.</p>
  <div class="dugmeler">
    <a class="indir" href="/indir">⬇ Windows için indir</a>
    <span class="not">Ücretsiz · Windows 10/11 · kendiliğinden güncellenir</span>
  </div>

  <section class="izgara">
    <div class="kart"><em>👥</em><b>Arkadaşlar ve sohbet</b><span>Steam gibi arkadaş listesi; kim oyunda, kim çevrim içi. Her ekrandan özel mesaj.</span></div>
    <div class="kart"><em>✨</em><b>Sosyal akış</b><span>Kim neyi kaç puanladı, kim hangi oyuna girdi. Beğen, yorum yap.</span></div>
    <div class="kart"><em>🏆</em><b>Profil ve vitrinler</b><span>Favori oyunların, en çok oynadıkların, film ve dizi vitrinin.</span></div>
    <div class="kart"><em>🎮</em><b>Oyunlarım</b><span>Oyunlar kendiliğinden algılanır, süre tutulur; puan kartı, klipler, Wrapped.</span></div>
    <div class="kart"><em>🎬</em><b>Film &amp; Dizi Arşivi</b><span>İzlediklerini puanla, afiş ve bilgiler kendiliğinden gelsin.</span></div>
    <div class="kart"><em>🎙️</em><b>Sesle yazma</b><span>Ctrl + Alt basılı tut, konuş, bırak: her programda yazar.</span></div>
  </section>

  <details>
    <summary>Kurulumda "Windows bilgisayarınızı korudu" uyarısı çıkarsa</summary>
    GMID Tool küçük bir topluluk uygulaması olduğu için Microsoft'tan ücretli imza almadı; Windows bu yüzden uyarır.
    <b>Ek bilgi</b>'ye, sonra <b>Yine de çalıştır</b>'a bas. Kurulum dosyası doğrudan GMID'in GitHub sayfasından gelir.
  </details>

  <footer>Merkut Studios · GMID topluluğu için</footer>
</main>
</body>
</html>
"##;
