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
