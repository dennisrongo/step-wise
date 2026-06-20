// Mirrors the serde DTOs in src-tauri/src/health/mod.rs (camelCase on the wire).

export type SyncState = "connected" | "syncing" | "reconnect" | "nodata";

export interface SyncStatus {
  state: SyncState;
  connected: boolean;
  syncing: boolean;
  demo: boolean;
  /** e.g. "3 min ago" — a sync stamp, never a live ticking counter. */
  lastSyncedLabel: string | null;
  /** e.g. "yesterday at 9:41 PM" — shown in the reconnect state. */
  lastSyncedDetail: string | null;
}

export interface HourBucket {
  hour: number;
  steps: number;
}

export interface DaySummary {
  date: string; // YYYY-MM-DD
  label: string; // "Mo", "Th", …
  isToday: boolean;
  steps: number;
  goal: number;
  /** Today: midnight → current hour. Past days: full day. */
  hourly: HourBucket[];
  restingHr: number | null;
  sleepMinutes: number | null;
  distanceMi: number | null;
  activeMinutes: number | null;
  // Deltas vs. the previous day (null when unknown / first day).
  restingHrDelta: number | null;
  sleepMinutesDelta: number | null;
  distanceMiDelta: number | null;
  activeMinutesDelta: number | null;
}

export interface WeekSummary {
  days: DaySummary[]; // 7 entries, oldest → today (today last)
}
