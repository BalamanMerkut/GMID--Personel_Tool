// Birlikte izle: video inerken izlenir. GMID videoyu parça parça (fragmented MP4) geçici dosyaya yazar; burası
// dosyanın inen kısmını okuyup oynatıcıya ekler (MediaSource). İnmemiş bir yere sarılırsa inen kısmın sonuna
// geri döner. İndirme bitince oynatıcı tamamlanmış dosyaya geçer (her yere sarılabilir).

export interface AkisKaynagi {
  /** Akışı başlatır: { id, hazir (tam dosya yolu) }. */
  baslat(): Promise<{ id: string; hazir: string | null }>;
  /** `mime`: dosyanın başı inince (o zamana kadar boyut 0). */
  durum(id: string): Promise<{ boyut: number; bitti: boolean; hata: string | null; yol: string | null; mime: string | null }>;
  oku(id: string, konum: number): Promise<ArrayBuffer>;
  /** Dosya yolunu oynatıcının açabileceği adrese çevirir. */
  dosyaAdresi(yol: string): string;
  /** İzleme bırakıldı: indirme durdurulsun. */
  birak(id: string): void;
}

/** İlk görüntü bu kadar sürede gelmezse eski yola geçilir. */
const ILK_SINIR = 40_000;

export interface Akis {
  /** Oynatıcıya eklenmiş (izlenebilir) son saniye. */
  yuklenen(): number;
  bitti(): boolean;
  iptal(): void;
}

const bekle = (ms: number) => new Promise((ok) => setTimeout(ok, ms));

/** MediaSource bu türü çalabilir mi? */
export const akisDesteklenir = (mime: string) =>
  typeof MediaSource !== "undefined" && MediaSource.isTypeSupported(mime);

/** Hata: akış kurulamadı, önce tamamını indirme yoluna geçilmeli. */
export class AkisYok extends Error {}

/**
 * Akışı oynatıcıya bağlar. İlk parça eklenince (oynatılabilir olunca) döner; indirme arkada sürer.
 * `hata`: indirme sonradan kesilirse çağrılır.
 */
export async function akisBagla(m: HTMLVideoElement, k: AkisKaynagi, hata: (e: string) => void): Promise<Akis> {
  const yok = (e: unknown) => (String(e).includes("AKIS_YOK") ? new AkisYok() : e);
  let b;
  try {
    b = await k.baslat();
  } catch (e) {
    throw yok(e);
  }
  if (b.hazir) {
    m.src = k.dosyaAdresi(b.hazir);
    return { yuklenen: () => Infinity, bitti: () => true, iptal: () => {} };
  }
  const id = b.id;
  // Videonun türü dosyanın başından okunur (tahmin edilirse oynatıcı siyah ekranda kalabiliyor).
  const t0 = Date.now();
  let mime: string | null = null;
  try {
    for (;;) {
      const d = await k.durum(id);
      if (d.hata) throw new AkisYok();
      if (d.mime) {
        mime = d.mime;
        break;
      }
      if (Date.now() - t0 > ILK_SINIR) throw new AkisYok();
      await bekle(300);
    }
  } catch (e) {
    k.birak(id);
    throw yok(e);
  }
  if (!akisDesteklenir(mime)) {
    k.birak(id);
    throw new AkisYok();
  }
  const ms = new MediaSource();
  const adres = URL.createObjectURL(ms);
  m.src = adres;
  await new Promise<void>((ok) => ms.addEventListener("sourceopen", () => ok(), { once: true }));
  const sb = ms.addSourceBuffer(mime);

  let iptal = false;
  let bittiMi = false;
  let konum = 0;
  let duzeltiyor = false;

  const yuklenen = () => (sb.buffered.length ? sb.buffered.end(sb.buffered.length - 1) : 0);
  const baslangic = () => (sb.buffered.length ? sb.buffered.start(0) : 0);
  const bitir = () =>
    new Promise<void>((ok, no) => {
      sb.addEventListener("updateend", () => ok(), { once: true });
      sb.addEventListener("error", () => no(new Error("Video parçası okunamadı")), { once: true });
    });
  async function ekle(bayt: ArrayBuffer) {
    for (let deneme = 0; ; deneme++) {
      try {
        const s = bitir();
        sb.appendBuffer(bayt);
        await s;
        return;
      } catch (e) {
        // Oynatıcının belleği doldu: geride kalan kısmı boşalt, yine olmazsa biraz bekle.
        if ((e as DOMException)?.name !== "QuotaExceededError" || deneme > 30) throw e;
        const geri = m.currentTime - 20;
        if (geri > baslangic() + 1) {
          const s = bitir();
          sb.remove(0, geri);
          await s;
        } else {
          await bekle(1000);
        }
        if (iptal) return;
      }
    }
  }

  // İnmemiş yere sarılırsa inen kısmın sonuna dön; bellekten atılmış eski yere sarılırsa baştan yeniden ekle.
  const sarildi = () => {
    if (duzeltiyor || bittiMi || iptal) return;
    const t = m.currentTime;
    const son = yuklenen();
    if (t > son - 0.5) {
      duzeltiyor = true;
      m.currentTime = Math.max(0, son - 1.5);
      duzeltiyor = false;
    } else if (t < baslangic() - 0.5 && baslangic() > ilkBas + 1) {
      yenidenBasla = true;
    }
  };
  m.addEventListener("seeking", sarildi);
  let yenidenBasla = false;
  /** İlk parçanın başladığı saniye (bazı videolar tam 0'dan başlamaz). */
  let ilkBas = 0;

  let ilkParca: () => void = () => {};
  let ilkHata: (e: unknown) => void = () => {};
  let basladi = false;
  const hazir = new Promise<void>((ok, no) => {
    ilkParca = () => {
      basladi = true;
      ok();
    };
    ilkHata = no;
  });
  // Oynatıcı parçayı çözemezse (siyah ekran) ya da ilk görüntü gecikirse eski yola geçilir.
  const oynaticiHatasi = () => {
    if (!basladi) ilkHata(new AkisYok());
  };
  m.addEventListener("error", oynaticiHatasi, { once: true });
  const sinir = setTimeout(() => ilkHata(new AkisYok()), ILK_SINIR);

  void (async () => {
    try {
      while (!iptal) {
        if (yenidenBasla && !sb.updating) {
          yenidenBasla = false;
          const s = bitir();
          sb.remove(0, Infinity);
          await s;
          konum = 0;
        }
        // Çok ileride ekli kısım varsa bekle (oynatıcının belleği dolmasın).
        if (yuklenen() - m.currentTime > 240) {
          await bekle(1000);
          continue;
        }
        const d = await k.durum(id);
        if (iptal) return;
        if (d.hata) throw new Error(d.hata);
        if (d.boyut > konum) {
          const bayt = await k.oku(id, konum);
          if (iptal) return;
          if (bayt.byteLength) {
            await ekle(bayt);
            konum += bayt.byteLength;
            if (yuklenen() > 0 && !basladi) {
              ilkBas = baslangic();
              // Başta boşluk varsa oynatıcı orada takılmasın (siyah ekran).
              if (m.currentTime < ilkBas) m.currentTime = ilkBas;
              ilkParca();
            }
            continue;
          }
        }
        if (d.bitti && konum >= d.boyut && d.yol) {
          // İndirme bitti: tam dosyaya geç (her yere sarılabilir, bellek boşalır).
          bittiMi = true;
          ilkParca();
          const t = m.currentTime;
          const caliyordu = !m.paused;
          m.removeEventListener("seeking", sarildi);
          m.src = k.dosyaAdresi(d.yol);
          await new Promise<void>((ok) => {
            m.addEventListener("loadedmetadata", () => ok(), { once: true });
            m.addEventListener("error", () => ok(), { once: true });
          });
          m.currentTime = t;
          if (caliyordu) await m.play().catch(() => {});
          URL.revokeObjectURL(adres);
          return;
        }
        await bekle(400);
      }
    } catch (e) {
      if (!basladi) ilkHata(yok(e));
      else if (!iptal) hata(String(e instanceof Error ? e.message : e));
    }
  })();

  const birak = () => {
    iptal = true;
    clearTimeout(sinir);
    m.removeEventListener("seeking", sarildi);
    m.removeEventListener("error", oynaticiHatasi);
    URL.revokeObjectURL(adres);
    if (!bittiMi) k.birak(id);
  };
  try {
    await hazir;
  } catch (e) {
    birak();
    m.removeAttribute("src");
    m.load();
    throw e;
  }
  clearTimeout(sinir);
  m.removeEventListener("error", oynaticiHatasi);
  return {
    yuklenen: () => (bittiMi ? Infinity : yuklenen()),
    bitti: () => bittiMi,
    iptal: birak,
  };
}
