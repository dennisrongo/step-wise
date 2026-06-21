// The daily step goal. The Google Health API has no goal/target data type, so
// this is a local preference (like theme / active mode), persisted here and
// passed to the backend at fetch time — the backend stamps it onto each day and
// re-clamps it. 10,000 is the conventional default.
//
// NOTE: DEFAULT_GOAL / MIN_GOAL / MAX_GOAL must stay in sync with the Rust side
// (`resolve_goal` in src-tauri/src/health/mod.rs), which re-clamps to the same
// bounds — change both together.
export const DEFAULT_GOAL = 10_000;
export const MIN_GOAL = 1_000;
export const MAX_GOAL = 100_000;
export const GOAL_STEP = 500;

const KEY = "stepwise:goal";

/** Snap to the nearest step and clamp into the supported range. */
export function clampGoal(n: number): number {
  if (!Number.isFinite(n)) return DEFAULT_GOAL;
  const snapped = Math.round(n / GOAL_STEP) * GOAL_STEP;
  return Math.min(MAX_GOAL, Math.max(MIN_GOAL, snapped));
}

export function getGoal(): number {
  try {
    const raw = localStorage.getItem(KEY);
    return raw == null ? DEFAULT_GOAL : clampGoal(Number(raw));
  } catch {
    return DEFAULT_GOAL;
  }
}

export function setGoal(goal: number): void {
  try {
    const v = clampGoal(goal);
    // Mirror activeMode: storing the default just clears the key.
    if (v === DEFAULT_GOAL) localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, String(v));
  } catch {
    /* ignore */
  }
}
