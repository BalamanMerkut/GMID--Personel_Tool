import type { IconName } from "./icons";

export interface Screen {
  id: string;
  label: string;
  icon: IconName;
  /** Hazırsa `null`; değilse hangi fazda geleceği. */
  phase: number | null;
  description: string;
}

export interface Room {
  /** Malikâne odası; tek ekranlı odalarda `null`. */
  name: string | null;
  screens: Screen[];
}

export const ROOMS: Room[] = [
  {
    name: null,
    screens: [{ id: "home", label: "Giriş Salonu", icon: "castle", phase: null, description: "" }],
  },
  {
    name: "Oyun Salonu",
    screens: [
      { id: "servers", label: "Sunucular", icon: "server", phase: null,
        description: "GMID topluluğunun oyun sunucuları: kim açık, kaç kişi var, hangi harita. Bağlan'a bas, oyun açılıp sunucuya girsin." },
      { id: "library", label: "Kütüphanem", icon: "library", phase: null,
        description: "Steam, Epic ve elle eklediğin oyunlar; kapakları, türleri ve gerçek oynama süreleriyle. Açtığın oyunlar Discord'daki gibi kendiliğinden yakalanır." },
      { id: "sessions", label: "Oturum Geçmişi", icon: "clock", phase: null,
        description: "Hangi gün hangi oyunu ne kadar oynadığın: günlük ve haftalık grafikler, en uzun oturumlar." },
      { id: "clips", label: "Kliplerim", icon: "clip", phase: null,
        description: "NVIDIA Instant Replay, Xbox Game Bar ve Steam kayıtların oyuna göre tek yerde. Dosyalar yerinde kalır, çift tıklayınca açılır." },
      { id: "gamemode", label: "Oyun Modu", icon: "bolt", phase: null,
        description: "Tek tıkla oyuna hazırlan: güç planı, kulaklığa geçiş, mikrofon kilidi, bildirimleri susturma. Oyun kapanınca hepsi geri alınır." },
    ],
  },
  {
    name: "Eğlence Salonu",
    screens: [
      { id: "archive", label: "Oyun Arşivi", icon: "star", phase: null,
        description: "Oyunlarını 5 kısa soruyla puanla, yorum ve not yaz. Sağdaki filtre kutusuyla türe, etikete ve puana göre bul." },
      { id: "films", label: "Film & Dizi Arşivi", icon: "film", phase: null,
        description: "Film ve dizileri aynı sade kartla puanla; afiş, özet, yönetmen ve oyuncular Vikipedi'den kendiliğinden gelir." },
      { id: "media", label: "Medya Oynatıcı", icon: "headphones", phase: null,
        description: "İndirdiğin müzikler ve videolar: senkron şarkı sözleriyle çal, liste görünümü, filmlerde altyazı. Başka ekrana geçince altta çalmaya devam eder." },
      { id: "videos", label: "Video İndirici", icon: "download", phase: null,
        description: "Bağlantı yapıştır ya da tarayıcıdaki GMID eklentisine tıkla, kaliteyi, dublajı ve altyazıyı seç, indir. Bitenler Medya Oynatıcı'da." },
    ],
  },
  {
    name: "Çalışma Odası",
    screens: [
      { id: "notes", label: "Notlar", icon: "note", phase: null,
        description: "Her yerden Ctrl+Alt+N ile açılan hızlı not, yapışkan notlar ve yapılacaklar listeleri. Kaybolmaz, kendiliğinden kaydedilir." },
      { id: "reminders", label: "Hatırlatıcılar", icon: "bell", phase: null,
        description: "\"30 dk sonra\", \"yarın 09:00\" gibi hızlı hatırlatıcılar; Windows bildirimiyle, oyundayken köşede küçük gösterge." },
      { id: "timer", label: "Sayaç", icon: "timer", phase: null,
        description: "Kronometre ve aynı anda birden çok geri sayım. Tepsiden ve Oyun İçi Panel'den başlatılabilir." },
      { id: "audio", label: "Ses & Mikrofon", icon: "speaker", phase: null,
        description: "Kulaklık ve hoparlör arasında tek tık geçiş, mikrofon seviyesi kilidi ve oyundan çıkmadan çalışan kısayollar." },
      { id: "install", label: "Format Sonrası Kurulum", icon: "install", phase: null,
        description: "Formattan sonra kurulacak programların listesi: tek tıkla hepsini kur ya da USB'ye bir Format Kiti çıkar: formattan sonra programlar ve GMID'in kendisi, ayarları ve geçmişiyle geri gelir." },
    ],
  },
  {
    name: null,
    screens: [{ id: "settings", label: "Kontrol Odası", icon: "gear", phase: null, description: "" }],
  },
];

export const SCREENS: Screen[] = ROOMS.flatMap((r) => r.screens);

export function findScreen(id: string): Screen | undefined {
  return SCREENS.find((s) => s.id === id);
}

export function roomOf(id: string): string | null {
  return ROOMS.find((r) => r.screens.some((s) => s.id === id))?.name ?? null;
}
