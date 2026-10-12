// Rust çekirdeğiyle paylaşılan veri tipleri (alan adları serde camelCase ile birebir).

export interface Device {
  id: string;
  name: string;
  isDefault: boolean;
  level: number;
  muted: boolean;
}

export interface AudioState {
  supported: boolean;
  outputs: Device[];
  inputs: Device[];
  lock: number | null;
}

export interface MicStatus {
  level: number;
  muted: boolean;
  lock: number | null;
}

export interface DevicePrefs {
  name?: string | null;
  icon?: "headphones" | "speaker" | "monitor" | null;
  hidden?: boolean;
}

export interface HotkeyStatus {
  id: string;
  group: string;
  label: string;
  accel: string;
  defaultAccel: string;
  error: string | null;
}

export interface Note {
  id: string;
  title: string;
  body: string;
  color: string;
  font: string;
  pinned: boolean;
  createdAt: number;
  updatedAt: number;
  deletedAt: number | null;
}

export interface NoteSummary {
  id: string;
  title: string;
  preview: string;
  color: string;
  pinned: boolean;
  updatedAt: number;
  deletedAt: number | null;
}

export interface NoteInput {
  id: string | null;
  title: string;
  body: string;
  color: string;
  font: string;
  pinned: boolean;
}

export type Repeat = "none" | "daily" | "weekdays" | "weekly" | "monthly";

export interface Reminder {
  id: string;
  title: string;
  note: string;
  dueAt: number;
  repeat: Repeat;
  notified: boolean;
  done: boolean;
  createdAt: number;
}

export interface ReminderInput {
  id: string | null;
  title: string;
  note: string;
  dueAt: number;
  repeat: Repeat;
}

export interface Countdown {
  id: string;
  label: string;
  totalMs: number;
  endsAt: number | null;
  remainingMs: number;
  finished: boolean;
}

export interface TimersSnapshot {
  now: number;
  stopwatch: { running: boolean; elapsedMs: number; laps: number[] };
  countdowns: Countdown[];
}

export interface Alerts {
  reminders: Reminder[];
  timers: Countdown[];
  /** "Kütüphaneye ekleyeyim mi?" kartları. */
  games: GameSuggestion[];
}

/** Kütüphanede olmayan, açıldığı görülen oyun. */
export interface GameSuggestion {
  path: string;
  exe: string;
  name: string;
  /** discord: bilinen oyun listesinde; fullscreen: tam ekran çalışan program. */
  reason: "discord" | "fullscreen";
  seenAt: number;
}

export interface OsdMessage {
  id: number;
  icon: string;
  text: string;
}

export type JoinKind = "steam" | "exe" | "uri" | "copy";

export interface GameInfo {
  id: string;
  name: string;
  short: string;
  defaultPort: number;
  join: JoinKind;
  exe: string | null;
  queryable: boolean;
  howTo: string;
}

export interface Server {
  id: string;
  name: string;
  game: string;
  host: string;
  port: number;
  queryPort: number | null;
  logo: string | null;
  description: string;
  connectNotes: string;
  /** Topluluk listesinden geldi (düzenlenemez). */
  remote: boolean;
  createdAt: number;
}

export interface ServerInput {
  id: string | null;
  name: string;
  game: string;
  host: string;
  port: number | null;
  queryPort: number | null;
  logo: string | null;
  description: string;
  connectNotes: string;
}

export interface Player {
  name: string;
  score: number | null;
  ping: number | null;
}

export interface ServerInfo {
  name: string;
  map: string;
  mode: string;
  version: string;
  players: number;
  maxPlayers: number;
  bots: number;
  password: boolean;
  playerList: Player[];
  favicon: string | null;
  pingMs: number;
}

export interface ServerStatus {
  id: string;
  state: "online" | "offline" | "unknown";
  info: ServerInfo | null;
  error: string | null;
  checkedAt: number;
  stale: boolean;
}

export interface SyncResult {
  servers: Server[];
  error: string | null;
  url: string;
  defaultUrl: string;
}

export interface JoinResult {
  kind: "launched" | "copy";
  address: string;
}

export interface ToolStatus {
  id: string;
  name: string;
  purpose: string;
  installed: boolean;
  sizeMb: number;
}

export interface ToolsState {
  tools: ToolStatus[];
  ready: boolean;
  installing: boolean;
  version: string | null;
}

export interface ToolProgress {
  tool: string;
  name: string;
  stage: "download" | "extract" | "done" | "error";
  done: number;
  total: number;
  error: string | null;
}

export interface QualityOption {
  id: string;
  label: string;
  detail: string;
  size: number | null;
  audioOnly: boolean;
  recommended: boolean;
}

export interface Probe {
  url: string;
  videoId: string;
  title: string;
  uploader: string;
  site: string;
  duration: number | null;
  thumbnail: string | null;
  options: QualityOption[];
  /** Birden çok ses dili (dublaj) varsa. */
  audioLangs: Track[];
  /** Sitenin altyazıları. */
  subtitles: Track[];
}

/** Ses dili ya da altyazı. `code` sitenin kullandığı ad ("tr", "tur", "en-US"…). */
export interface Track {
  code: string;
  label: string;
}

/** İndirmede seçilen dublaj ve altyazılar. */
export interface Langs {
  audio: string | null;
  subs: string[];
}

/** Videonun yanındaki altyazı dosyası. */
export interface Sidecar {
  file: string;
  /** "tr", "en"…; uygulamanın yazıya döktüğünde boş. */
  lang: string;
  label: string;
}

export interface PlaylistEntry {
  id: string;
  url: string;
  title: string;
  duration: number | null;
  uploader: string;
  thumbnail: string | null;
}

export interface PlaylistProbe {
  url: string;
  title: string;
  uploader: string;
  site: string;
  entries: PlaylistEntry[];
}

/** Bağlantı tek video mu, çalma listesi mi? */
export type Parsed = ({ kind: "video" } & Probe) | ({ kind: "playlist" } & PlaylistProbe);

export type DownloadStatus = "queued" | "retrying" | "downloading" | "processing" | "paused" | "done" | "error";

export interface Download {
  id: string;
  url: string;
  videoId: string;
  title: string;
  uploader: string;
  site: string;
  thumbnail: string | null;
  duration: number | null;
  quality: string;
  audioOnly: boolean;
  status: DownloadStatus;
  progress: number;
  size: number | null;
  filePath: string | null;
  error: string | null;
  source: "app" | "extension";
  createdAt: number;
  finishedAt: number | null;
  /** Çalma listesinden geldiyse listenin adı. */
  playlist: string | null;
}

export interface Live {
  id: string;
  progress: number;
  speed: number | null;
  eta: number | null;
  downloaded: number;
  total: number | null;
  phase: "download" | "processing";
}

export interface BridgeStatus {
  port: number;
  paired: boolean;
  lastSeen: number | null;
  browser: string | null;
}

export interface Word {
  start: number;
  end: number;
  text: string;
}

export interface Segment {
  start: number;
  end: number;
  text: string;
  words?: Word[];
}

export interface Transcript {
  downloadId: string;
  source: "lyrics" | "lyrics-timed" | "subtitles" | "whisper";
  language: string;
  segments: Segment[];
  createdAt: number;
}

export interface WhisperModel {
  id: string;
  label: string;
  detail: string;
  sizeMb: number;
  installed: boolean;
}

export interface WhisperStatus {
  program: boolean;
  models: WhisperModel[];
  selected: string;
  ready: boolean;
}

export interface TranscriptProgress {
  id: string;
  stage: "lyrics" | "subtitles" | "audio" | "whisper";
  progress: number;
}

/** Sesle yazmadaki bir dikte. */
export interface DictationEntry {
  text: string;
  /** Whisper'ın ham çıktısı (toparlanmadan önce). */
  raw: string;
  at: number;
  /** Bırakıştan yapıştırmaya kadar geçen süre (ms). */
  ms: number;
  tidied: boolean;
}

export interface DictationStatus {
  supported: boolean;
  enabled: boolean;
  whisperReady: boolean;
  gpuInstalled: boolean;
  /** true: şu an ekran kartında çalışıyor; null: sunucu kapalı. */
  gpuActive: boolean | null;
  gpuError: string | null;
  llmInstalled: boolean;
  phase: "idle" | "listening" | "transcribing" | "tidying";
  lastError: string | null;
  history: DictationEntry[];
}

/** Kullanıcının kendi çalma listesi. */
export interface UserPlaylist {
  id: string;
  name: string;
  createdAt: number;
  /** Sıralı indirme kimlikleri. */
  items: string[];
}

/** Kütüphanedeki oyun. */
export interface Game {
  id: string;
  name: string;
  source: "steam" | "epic" | "riot" | "xbox" | "other" | "manual" | "archive";
  sourceId: string;
  installDir: string | null;
  exes: string[];
  launch: string | null;
  steamAppid: number | null;
  cover: string | null;
  header: string | null;
  developer: string | null;
  publisher: string | null;
  year: number | null;
  description: string | null;
  genres: string[];
  categories: string[];
  /** Türkçe etiketler (Roguelike, Co-op…). */
  tags: string[];
  /** Kütüphanede mi; değilse yalnızca Oyun Arşivi'nde. */
  inLibrary: boolean;
  /** Platformun (Steam) tuttuğu süre, dakika. */
  platformMinutes: number;
  platformLastPlayed: number | null;
  hidden: boolean;
  addedAt: number;
  /** GMID'in ölçtüğü gerçek oynama (saniye). */
  playedSeconds: number;
  sessions: number;
  lastPlayed: number | null;
  /** Şu an açıksa başlangıcı (ms). */
  runningSince: number | null;
}

/** Taramada bulunan oyun. */
export interface FoundGame {
  source: "steam" | "epic" | "riot" | "xbox" | "other" | "manual";
  sourceId: string;
  name: string;
  installDir: string | null;
  exes: string[];
  launch: string | null;
  steamAppid: number | null;
  platformMinutes: number;
  lastPlayed: number | null;
  already: boolean;
}

export interface PlaySession {
  id: string;
  gameId: string;
  startedAt: number;
  endedAt: number | null;
  seconds: number;
}

export interface RunningApp {
  name: string;
  path: string;
  title: string;
}

/** Puanlama kartının 5 sorusu; `null` = "Yok". */
/** Soru → puan (1–10) ya da null ("Yok"). Oyunda: gameplay, story, enjoyment, visuals, performance;
 *  filmde: story, acting, direction, pace, enjoyment. */
export type Scores = Record<string, number | null>;

/** Oyun: playing · finished · completed · dropped · wishlist — Film/dizi: watching · watched · dropped · watchlist */
export type ReviewStatus = "playing" | "finished" | "completed" | "dropped" | "wishlist" | "watching" | "watched" | "watchlist";

/** Film/dizi için ek alanlar. */
export interface ReviewExtra {
  season?: number | null;
  episode?: number | null;
  where?: string | null;
  with?: string | null;
}
export type Recommend = "yes" | "maybe" | "no";

export interface Review {
  id: string;
  author: string;
  subjectKind: "game" | "title";
  subjectId: string;
  scores: Scores;
  extra: ReviewExtra | null;
  /** Elle düzeltme (−1 … +1). */
  adjust: number;
  /** "İstiyorum" durumunda puan yok. */
  overall: number | null;
  oneLiner: string | null;
  notes: string | null;
  status: ReviewStatus | null;
  recommend: Recommend | null;
  finishedAt: number | null;
  createdAt: number;
  updatedAt: number;
}

export interface ReviewInput {
  subjectKind: "game" | "title";
  subjectId: string;
  scores: Scores;
  adjust: number;
  oneLiner: string | null;
  notes: string | null;
  status: ReviewStatus | null;
  recommend: Recommend | null;
  extra?: ReviewExtra | null;
}

/** Steam mağazası arama sonucu. */
export interface StoreHit {
  appid: number;
  name: string;
  image: string | null;
}

/** Kliplerim: kayıt programlarının klip ve ekran görüntüleri (dosyalar yerinde). */
export interface Clip {
  id: string;
  path: string;
  /** Dosya adı (uzantısız). */
  name: string;
  kind: "clip" | "shot";
  source: "nvidia" | "xbox" | "amd" | "obs" | "steam" | "medal" | "folder";
  gameId: string | null;
  gameHint: string | null;
  title: string | null;
  size: number;
  duration: number | null;
  width: number | null;
  height: number | null;
  thumb: string | null;
  favorite: boolean;
  createdAt: number;
}

export interface ClipRoot {
  path: string;
  label: string;
  custom: boolean;
  exists: boolean;
}

/** Oyun Modu ayarları (`gamemode.profile`). */
export interface GameModeProfile {
  /** Kütüphanedeki bir oyun açılınca kendiliğinden. */
  auto: boolean;
  power: boolean;
  /** Geçilecek ses çıkışı; null değiştirme. */
  output: string | null;
  micLock: boolean;
  quiet: boolean;
  stopBackground: boolean;
  closeApps: string[];
}

export interface GameModeStatus {
  active: boolean;
  reason: "manual" | "auto" | null;
  since: number | null;
  quiet: boolean;
  stopBackground: boolean;
}

/** Film & Dizi Arşivi kaydı (bilgiler Wikipedia + Wikidata). */
export interface Title {
  id: string;
  kind: "movie" | "series";
  name: string;
  original: string | null;
  year: number | null;
  poster: string | null;
  summary: string | null;
  directors: string[];
  cast: string[];
  genres: string[];
  /** Dakika (film). */
  runtime: number | null;
  seasons: number | null;
  episodes: number | null;
  wikiTitle: string | null;
  addedAt: number;
}

/** Film/dizi araması sonucu. */
export interface TitleHit {
  wikiTitle: string;
  name: string;
  description: string;
  image: string | null;
  wikidata: string | null;
  kind: "movie" | "series";
  year: number | null;
}

/** Format Sonrası Kurulum: katalogdaki ya da listedeki bir program. */
export interface InstallItem {
  /** winget / Mağaza kimliği; web için kısa ad. */
  id: string;
  name: string;
  /** winget · msstore (Microsoft Store) · web (indirme sayfası açılır) */
  source: "winget" | "msstore" | "web";
  category?: string | null;
  url?: string | null;
  note?: string | null;
}

export type InstallStepState = "waiting" | "running" | "done" | "already" | "opened" | "failed" | "skipped";

export interface InstallStep {
  id: string;
  name: string;
  state: InstallStepState;
  /** 0..1, indirilirken. */
  progress: number | null;
  message: string | null;
}

export interface InstallRun {
  running: boolean;
  stopping: boolean;
  steps: InstallStep[];
  started: number | null;
  finished: number | null;
}

/** winget araması sonucu. */
export interface InstallHit {
  id: string;
  name: string;
  version: string;
}

export interface KitRequest {
  dir: string;
  items: InstallItem[];
  setup: string | null;
  backup: boolean;
  backupOptions: { videos: boolean; tools: boolean };
}

export interface KitResult {
  path: string;
  programs: number;
  setup: boolean;
  backupSize: number | null;
}

export interface KitProgress {
  stage: "script" | "setup" | "backup" | "done";
  done: number;
  total: number;
}

/** GMID-Yedek.zip içindeki bilgi. */
export interface BackupManifest {
  app: string;
  version: string;
  created: number;
  root: string;
  home: string;
  folders: string[];
}

/** Klip kırpma / Discord'a sığdırma seçenekleri. */
export interface FitOptions {
  start?: number | null;
  end?: number | null;
  /** Hedef boyut (MB); `null`: sınırsız, yalnız kırp. */
  limitMb: number | null;
  /** av1: aynı boyutta daha net · h264: her yerde oynar */
  codec: "av1" | "h264";
}

export interface FitPlan {
  video: number;
  audio: number;
  height: number;
  fps: number;
  quality: "great" | "good" | "ok" | "poor";
}

export interface UpdateRelease {
  build: number;
  title: string;
  notes: string;
  published: string;
  size: number;
}

export interface UpdateStatus {
  /** Bu kopyanın derleme numarası (geliştirme derlemesinde yok). */
  current: number | null;
  latest: UpdateRelease | null;
  available: boolean;
  checkedAt: number | null;
  error: string | null;
  progress: number | null;
}

/** Ücretsiz oyun dağıtımı (GamerPower ya da Epic). */
export interface Giveaway {
  id: string;
  title: string;
  /** epic | steam | gog | playstation | xbox | ubisoft | ea | prime | itch | battlenet | mobile | other */
  store: string;
  platforms: string;
  worth: string | null;
  image: string | null;
  url: string;
  starts: number | null;
  ends: number | null;
  upcoming: boolean;
}

/** İstek listesindeki bir oyunun en iyi indirimi. */
export interface WishDeal {
  gameId: string;
  name: string;
  cover: string | null;
  store: string;
  price: string;
  regular: string | null;
  discount: number;
  url: string;
}

export interface DealsSnapshot {
  free: Giveaway[];
  upcoming: Giveaway[];
  wishlist: WishDeal[];
  wishCount: number;
  fetchedAt: number | null;
  error: string | null;
}

export interface DealsPrefs {
  notifyFree: boolean;
  /** Bildirim gelecek mağazalar (boşsa hepsi). */
  stores: string[];
  notifyWish: boolean;
  minDiscount: number;
}

/** GMID Wrapped: dönem özeti. */
export interface Wrapped {
  from: number;
  to: number;
  totalSeconds: number;
  sessions: number;
  daysPlayed: number;
  distinctGames: number;
  top: { name: string; seconds: number; sessions: number; cover: string | null }[];
  longest: { game: string; seconds: number; at: number } | null;
  /** 0 = Pazartesi */
  bestWeekday: number | null;
  hours: number[];
  nightShare: number;
  weekendShare: number;
  newGames: number;
  ratedGames: number;
  bestGame: [string, number] | null;
  titlesWatched: number;
  bestTitle: [string, number] | null;
  clips: number;
  shots: number;
  downloads: number;
  notes: number;
  badges: { id: string; title: string; desc: string }[];
}

/** Genel arama sonucu. */
export interface SearchHit {
  kind: "note" | "game" | "title" | "video" | "clip" | "reminder";
  id: string;
  title: string;
  sub: string;
  image: string | null;
}

/** Oyun İçi Panel performans ölçümü. */
export interface PerfSample {
  cpu: number | null;
  ramUsed: number | null;
  ramTotal: number | null;
  gpu: number | null;
  gpuTemp: number | null;
  vramUsed: number | null;
  vramTotal: number | null;
  gpuName: string | null;
  fps: number | null;
  fpsApp: string | null;
  /** RivaTuner (RTSS) çalışıyor mu. */
  rtss: boolean;
  fpsSource: "rtss" | "presentmon" | null;
  /** PresentMon: missing (kurulmamış) · permission (izin yok) · ready · running · error */
  fpsState: "missing" | "permission" | "ready" | "running" | "error";
}

export interface FpsStatus {
  state: "missing" | "permission" | "ready" | "running" | "error";
  /** İzin istendi ama Windows oturumu henüz yenilenmedi. */
  relogin: boolean;
  message: string | null;
}

/** Ses karıştırıcısındaki bir uygulama. */
export interface AppVolume {
  /** 0: sistem sesleri */
  pid: number;
  name: string;
  exe: string;
  level: number;
  muted: boolean;
  /** Şu an ses çıkarıyor mu. */
  active: boolean;
}

/** Sunucu bildirimi: açılınca ve/veya en az N kişi olunca. */
export interface ServerWatch {
  online: boolean;
  minPlayers: number | null;
}
