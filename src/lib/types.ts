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
