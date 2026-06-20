// Which activity levels count toward "Active minutes". Persisted locally (like
// theme) and passed to the backend at fetch time — the backend does the summing
// and the day-over-day delta. "full" (light + moderate + vigorous) is default.
export type ActiveMode = "full" | "intense";

const KEY = "stepwise:activeMode";

export function getActiveMode(): ActiveMode {
  try {
    return localStorage.getItem(KEY) === "intense" ? "intense" : "full";
  } catch {
    return "full";
  }
}

export function setActiveMode(mode: ActiveMode): void {
  try {
    if (mode === "full") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, mode);
  } catch {
    /* ignore */
  }
}
