// Appearance: persisted UI preference (not a secret), applied to <html data-theme>.
// theme.css resolves :root[data-theme="light"|"dark"]. "system" follows the OS via
// matchMedia. Persisted in localStorage so it applies before first paint and is
// shared across windows (main panel + hover popover).
export type ThemeMode = "system" | "light" | "dark";

const KEY = "stepwise:theme";
const mq =
  typeof window !== "undefined" && window.matchMedia
    ? window.matchMedia("(prefers-color-scheme: dark)")
    : null;

export function getThemeMode(): ThemeMode {
  let v: string | null = null;
  try {
    v = localStorage.getItem(KEY);
  } catch {
    /* ignore */
  }
  return v === "light" || v === "dark" ? v : "system";
}

function resolve(mode: ThemeMode): "light" | "dark" {
  if (mode === "light" || mode === "dark") return mode;
  return mq && mq.matches ? "dark" : "light";
}

export function applyTheme(mode: ThemeMode = getThemeMode()): void {
  document.documentElement.dataset.theme = resolve(mode);
}

export function setThemeMode(mode: ThemeMode): void {
  try {
    if (mode === "system") localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, mode);
  } catch {
    /* ignore */
  }
  applyTheme(mode);
}

/** Apply on load and keep all windows + OS appearance in sync. */
export function initTheme(): void {
  applyTheme();
  mq?.addEventListener("change", () => {
    if (getThemeMode() === "system") applyTheme();
  });
  // Fires in *other* windows of the same origin when the choice changes.
  window.addEventListener("storage", (e) => {
    if (e.key === KEY || e.key === null) applyTheme();
  });
}
