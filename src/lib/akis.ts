// Birlikte izle: video inerken izlenir. GMID videoyu parça parça (fragmented MP4) geçici dosyaya yazar; burası
// dosyanın inen kısmını okuyup oynatıcıya ekler (MediaSource). İnmemiş bir yere sarılırsa inen kısmın sonuna
// geri döner. İndirme bitince oynatıcı tamamlanmış dosyaya geçer (her yere sarılabilir).

export interface AkisKaynagi {
  /** Akışı başlatır: { id, hazir (tam dosya yolu), mime }. */
  baslat(): Promise<{ id: string; hazir: string | null; mime: string }>;
  durum(id: string): Promise<{ boyut: number; bitti: boolean; hata: string | null; yol: string | null }>;
  oku(id: string, konum: number): Promise<ArrayBuffer>;
  /** Dosya yolunu oynatıcının açabileceği adrese çevirir. */
  dosyaAdresi(yol: string): string;
}

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
  let b;
  try {
    b = await k.baslat();
  } catch (e) {
    if (String(e).includes("AKIS_YOK")) throw new AkisYok();
    throw e;
  }
  if (b.hazir) {
    m.src = k.dosyaAdresi(b.hazir);
    return { yuklenen: () => Infinity, bitti: () => true, iptal: () => {} };
  }
  if (!akisDesteklenir(b.mime)) throw new AkisYok();
  const id = b.id;
  const ms = new MediaSource();
  const adres = URL.createObjectURL(ms);
  m.src = adres;
  await new Promise<void>((ok) => ms.addEventListener("sourceopen", () => ok(), { once: true }));
  const sb = ms.addSourceBuffer(b.mime);

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
    } else if (t < baslangic() - 0.5) {
      yenidenBasla = true;
    }
  };
  m.addEventListener("seeking", sarildi);
  let yenidenBasla = false;

  let ilkParca: () => void = () => {};
  const hazir = new Promise<void>((ok) => (ilkParca = ok));

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
            if (yuklenen() > 0) ilkParca();
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
      if (!iptal) hata(String(e instanceof Error ? e.message : e));
      ilkParca();
    }
  })();

  await hazir;
  return {
    yuklenen: () => (bittiMi ? Infinity : yuklenen()),
    bitti: () => bittiMi,
    iptal: () => {
      iptal = true;
      m.removeEventListener("seeking", sarildi);
      URL.revokeObjectURL(adres);
    },
  };
}
