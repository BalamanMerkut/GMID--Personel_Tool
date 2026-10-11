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
