import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

import { isTauriReady } from "../tauriReady";
import { getThemeMode, setThemeMode, type ThemeMode } from "../theme";
import { getActiveMode, setActiveMode, type ActiveMode } from "../activeMode";
import { clampGoal, getGoal, setGoal, GOAL_STEP, MIN_GOAL, MAX_GOAL } from "../goal";
import { useUpdater } from "../hooks/useUpdater";
import { nf } from "../format";
import type { SyncStatus } from "../types";

const TILES: { mode: ThemeMode; label: string; cls: string }[] = [
  { mode: "light", label: "Light", cls: "light" },
  { mode: "dark", label: "Dark", cls: "dark" },
  { mode: "system", label: "Auto", cls: "auto" },
];

const ACTIVE_OPTS: { mode: ActiveMode; label: string }[] = [
  { mode: "full", label: "All activity" },
  { mode: "intense", label: "Moderate & vigorous" },
];
const ACTIVE_HELP: Record<ActiveMode, string> = {
  full: "Counts light, moderate, and vigorous active minutes.",
  intense: "Counts only moderate and vigorous minutes — excludes light activity.",
};

async function openExternal(url: string) {
  try {
    if (isTauriReady()) await openUrl(url);
    else window.open(url, "_blank", "noopener");
  } catch {
    window.open(url, "_blank", "noopener");
  }
}

function ExternalIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M7 17 17 7" />
      <path d="M8 7h9v9" />
    </svg>
  );
}

export function Settings({
  status,
  onBack,
  onReconnect,
  onDisconnect,
  onActiveModeChange,
  onGoalChange,
}: {
  status: SyncStatus;
  onBack: () => void;
  onReconnect: () => void;
  onDisconnect: () => void;
  /** Called after the active-minutes preference changes, to re-fetch the week. */
  onActiveModeChange?: () => void;
  /** Called after the step goal changes, with the new goal, to apply it to the
   * loaded week (no re-fetch needed — the goal is a client-side threshold). */
  onGoalChange?: (goal: number) => void;
}) {
  const [mode, setMode] = useState<ThemeMode>(getThemeMode());
  const [activeMode, setActiveModeState] = useState<ActiveMode>(getActiveMode());
  const [goal, setGoalState] = useState<number>(getGoal());
  const [version, setVersion] = useState("0.1.0");
  const upd = useUpdater({ auto: false });

  useEffect(() => {
    if (!isTauriReady()) return;
    invoke<string>("app_version")
      .then(setVersion)
      .catch(() => {});
  }, []);

  const pick = (m: ThemeMode) => {
    setThemeMode(m);
    setMode(m);
  };
  const pickActive = (m: ActiveMode) => {
    if (m === activeMode) return;
    setActiveMode(m);
    setActiveModeState(m);
    onActiveModeChange?.();
  };
  const bumpGoal = (delta: number) => {
    const next = clampGoal(goal + delta);
    if (next === goal) return;
    setGoal(next);
    setGoalState(next);
    onGoalChange?.(next);
  };
  const connected = status.connected;

  const hasUpdate =
    upd.phase === "available" || upd.phase === "downloading" || upd.phase === "ready";
  const updBusy = upd.phase === "downloading" || upd.phase === "ready";
  const updHeadline =
    upd.phase === "available"
      ? `Update available · v${upd.version}`
      : upd.phase === "downloading"
        ? "Downloading update…"
        : upd.phase === "ready"
          ? "Restarting…"
          : upd.phase === "uptodate"
            ? "Stepwise is up to date"
            : upd.phase === "checking"
              ? "Checking for updates…"
              : upd.phase === "error"
                ? "Update check unavailable"
                : "Check for updates";

  return (
    <>
      <div className="set-head">
        <button className="set-back" onClick={onBack} type="button">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
            <path d="M15 6l-6 6 6 6" />
          </svg>
          Today
        </button>
        <span className="set-title">Settings</span>
        <span className="set-spacer" />
      </div>

      <div className="set-body">
        <div className="set-section">
          <div className="set-label">Appearance</div>
          <div className="set-card ap-card">
            <div className="ap-tiles">
              {TILES.map((t) => (
                <button
                  key={t.mode}
                  type="button"
                  className={`ap-tile${mode === t.mode ? " sel" : ""}`}
                  onClick={() => pick(t.mode)}
                  aria-pressed={mode === t.mode}
                >
                  <div className={`ap-thumb ${t.cls}`}>
                    <div className="ap-bar" />
                    <div className="ap-ring" />
                  </div>
                  <div className="ap-name">{t.label}</div>
                </button>
              ))}
            </div>
            <div className="ap-help">Match your Mac's appearance, or force a mode.</div>
          </div>
        </div>

        <div className="set-section">
          <div className="set-label">Daily step goal</div>
          <div className="set-card gl-card">
            <div className="gl-stepper" role="group" aria-label="Daily step goal">
              <button
                type="button"
                className="gl-btn"
                onClick={() => bumpGoal(-GOAL_STEP)}
                disabled={goal <= MIN_GOAL}
                aria-label={`Decrease goal by ${GOAL_STEP} steps`}
              >
                &#8722;
              </button>
              {/* aria-live so screen readers announce the new value when the
                  buttons change it (the buttons themselves only say "Decrease/
                  Increase"); aria-atomic so it reads as one "N steps" unit. */}
              <div className="gl-val" aria-live="polite" aria-atomic="true">
                {nf(goal)}
                <span className="gl-unit">steps</span>
              </div>
              <button
                type="button"
                className="gl-btn"
                onClick={() => bumpGoal(GOAL_STEP)}
                disabled={goal >= MAX_GOAL}
                aria-label={`Increase goal by ${GOAL_STEP} steps`}
              >
                +
              </button>
            </div>
            <div className="ap-help">The target your step ring fills toward. Default is 10,000.</div>
          </div>
        </div>

        <div className="set-section">
          <div className="set-label">Active minutes</div>
          <div className="set-card am-card">
            <div className="am-seg" role="group" aria-label="Active minutes definition">
              {ACTIVE_OPTS.map((o) => (
                <button
                  key={o.mode}
                  type="button"
                  className={`am-opt${activeMode === o.mode ? " sel" : ""}`}
                  onClick={() => pickActive(o.mode)}
                  aria-pressed={activeMode === o.mode}
                >
                  {o.label}
                </button>
              ))}
            </div>
            <div className="ap-help">{ACTIVE_HELP[activeMode]}</div>
          </div>
        </div>

        <div className="set-section">
          <div className="set-label">Google Health</div>
          <div className="set-card">
            <div className="set-row">
              <span className={`r-dot ${connected ? "ok" : "warn"}`} />
              <div className="r-main">
                <span className="r-label">{connected ? "Connected" : "Not connected"}</span>
                <span className="r-sub">
                  {connected
                    ? `Last sync ${status.lastSyncedLabel ?? "just now"}`
                    : "Connect to sync your steps"}
                </span>
              </div>
            </div>
            {connected ? (
              <div className="gh-actions">
                <button className="gh-btn" type="button" onClick={onReconnect}>
                  Switch account
                </button>
                <button className="gh-btn danger" type="button" onClick={onDisconnect}>
                  Disconnect
                </button>
              </div>
            ) : (
              <div className="gh-actions">
                <button className="gh-btn primary" type="button" onClick={onReconnect}>
                  Connect Google Health
                </button>
              </div>
            )}
          </div>
        </div>

        <div className="set-section">
          <div className="set-label">Updates</div>
          <div className="set-card">
            <div className="set-row">
              <div className="r-main">
                <span className="r-label">{updHeadline}</span>
                <span className="r-sub">Version {version}</span>
              </div>
              <span className="r-spacer" />
              {hasUpdate ? (
                <button className="r-btn" type="button" onClick={upd.install} disabled={updBusy}>
                  {upd.phase === "downloading"
                    ? "Downloading…"
                    : upd.phase === "ready"
                      ? "Restarting…"
                      : "Update & restart"}
                </button>
              ) : (
                <button
                  className="r-btn"
                  type="button"
                  onClick={upd.check}
                  disabled={upd.phase === "checking"}
                >
                  {upd.phase === "checking" ? "Checking…" : "Check"}
                </button>
              )}
            </div>
            {upd.phase === "error" && upd.error && (
              <div className="set-err">Couldn't check for updates: {upd.error}</div>
            )}
          </div>
        </div>

        <div className="set-section">
          <div className="set-label">About</div>
          <div className="set-card">
            <div className="set-row">
              <span className="r-label">Version</span>
              <span className="r-spacer" />
              <span className="r-val">{version}</span>
            </div>
            <div className="set-row">
              <span className="r-label">Made by</span>
              <span className="r-spacer" />
              <button className="r-link" type="button" onClick={() => openExternal("https://dennisrongo.com")}>
                Dennis Rongo <ExternalIcon />
              </button>
            </div>
            <div className="set-row">
              <span className="r-label">Data source</span>
              <span className="r-spacer" />
              <span className="r-val">Local · Google Health</span>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
