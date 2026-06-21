// Realistic placeholder data used for browser previews (no Tauri) and mirrored
// by the Rust DemoSource so the "Connected" design looks identical either way.
import type { DaySummary, HourBucket, SyncStatus, WeekSummary } from "./types";
import { getGoal } from "./goal";

const CURRENT_HOUR = 14; // 2 PM
const GOAL = getGoal();

// Shape of an active day (relative weight per hour, 0..23).
const BASE_HOURLY = [
  0.2, 0.1, 0.05, 0.05, 0.1, 0.4, 1.2, 2.6, 3.1, 2.2, 1.8, 2.0, 3.3, 2.6, 2.5,
  2.2, 2.7, 3.5, 3.1, 2.4, 1.8, 1.2, 0.8, 0.4,
];

function hourly(steps: number, isToday: boolean): HourBucket[] {
  const range = isToday ? CURRENT_HOUR + 1 : 24;
  const weights = BASE_HOURLY.slice(0, range);
  const sum = weights.reduce((a, b) => a + b, 0) || 1;
  const scale = steps / sum;
  return weights.map((w, hour) => ({ hour, steps: Math.round(w * scale) }));
}

const TWO_LETTER = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];

interface Seed {
  steps: number;
  hr: number;
  sleep: number;
  dist: number;
  active: number;
}

// Oldest → today.
const SEEDS: Seed[] = [
  { steps: 7540, hr: 60, sleep: 408, dist: 3.4, active: 35 },
  { steps: 12030, hr: 56, sleep: 475, dist: 5.4, active: 64 },
  { steps: 9980, hr: 57, sleep: 490, dist: 4.5, active: 51 },
  { steps: 9120, hr: 59, sleep: 422, dist: 4.1, active: 47 },
  { steps: 11340, hr: 57, sleep: 450, dist: 5.1, active: 58 },
  { steps: 6890, hr: 61, sleep: 380, dist: 3.1, active: 28 },
  { steps: 8427, hr: 58, sleep: 432, dist: 3.8, active: 42 }, // today
];

function buildWeek(): WeekSummary {
  const days: DaySummary[] = SEEDS.map((s, i) => {
    const isToday = i === SEEDS.length - 1;
    const d = new Date();
    d.setDate(d.getDate() - (SEEDS.length - 1 - i));
    const prev = i > 0 ? SEEDS[i - 1] : null;
    return {
      date: d.toISOString().slice(0, 10),
      label: TWO_LETTER[d.getDay()],
      isToday,
      steps: s.steps,
      goal: GOAL,
      hourly: hourly(s.steps, isToday),
      restingHr: s.hr,
      sleepMinutes: s.sleep,
      distanceMi: s.dist,
      activeMinutes: s.active,
      restingHrDelta: prev ? s.hr - prev.hr : null,
      sleepMinutesDelta: prev ? s.sleep - prev.sleep : null,
      distanceMiDelta: prev ? Number((s.dist - prev.dist).toFixed(1)) : null,
      activeMinutesDelta: prev ? s.active - prev.active : null,
    };
  });
  return { days };
}

export const DEMO_WEEK: WeekSummary = buildWeek();

export const DEMO_STATUS: SyncStatus = {
  state: "connected",
  connected: true,
  syncing: false,
  demo: true,
  lastSyncedLabel: "3 min ago",
  lastSyncedDetail: null,
};
