import { useCallback, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauriReady } from "../tauriReady";
import { fitWindowHeight } from "../platform";
import { REFRESH_MS } from "../refreshInterval";
import { DEMO_STATUS, DEMO_WEEK } from "../mockData";
import { getGoal } from "../goal";
import { nf, toGo } from "../format";
import type { DaySummary, SyncStatus, WeekSummary } from "../types";

// Must match HOVER_WIDTH in tray.rs — Rust anchors the window's right edge to
// the tray icon, so the width stays fixed while we fit the height here.
const WIDTH = 320;

async function call<T>(command: string, fallback: T, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriReady()) return fallback;
  return invoke<T>(command, args);
}

/**
 * Compact glance of today's activity, shown in its own borderless window when
 * the cursor hovers the tray icon. Refreshes on mount, on each `hover-show`
 * event, and then on the user's auto-refresh cadence while it's on screen —
 * polling stops on `hover-hide` so a hidden glance never hits Google. Fits its
 * window to the rendered card height.
 */
export function HoverPopover() {
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [today, setToday] = useState<DaySummary | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  const load = useCallback(async () => {
    try {
      const s = await call<SyncStatus>("get_sync_status", DEMO_STATUS);
      setStatus(s);
      if (s.state === "reconnect") {
        setToday(null);
        return;
      }
      const w = await call<WeekSummary>("get_week_summary", DEMO_WEEK, { goal: getGoal() });
      setToday(w.days.find((d) => d.isToday) ?? w.days[w.days.length - 1] ?? null);
    } catch {
      /* keep last good values */
    }
  }, []);

  useEffect(() => {
    load();
    if (!isTauriReady()) return;

    let timer: ReturnType<typeof setInterval> | undefined;
    const stop = () => {
      if (timer) {
        clearInterval(timer);
        timer = undefined;
      }
    };
    // Refresh every 30s, but only while the glance is actually shown — polling
    // starts on `hover-show` and stops on `hover-hide`, so a hidden glance uses
    // no bandwidth.
    const start = () => {
      stop();
      timer = setInterval(() => void load(), REFRESH_MS);
    };

    const unShow = listen("hover-show", () => {
      void load();
      start();
    });
    const unHide = listen("hover-hide", stop);

    return () => {
      stop();
      unShow.then((f) => f());
      unHide.then((f) => f());
    };
  }, [load]);

  // Fit the window to the card's natural height; macOS grows down, Windows up.
  useLayoutEffect(() => {
    if (!isTauriReady() || !rootRef.current) return;
    const height = Math.max(80, Math.ceil(rootRef.current.offsetHeight));
    fitWindowHeight(getCurrentWindow(), WIDTH, height).catch(() => {});
  }, [status, today]);

  const reconnect = status?.state === "reconnect";

  return (
    <div className="hover-root" ref={rootRef}>
      <div className="pop">
        <div className="pop-head">
          <span className="pop-name">Stepwise</span>
          <span className="pop-sync">
            {reconnect ? (
              <>
                <span className="sync-dot warn" /> Not connected
              </>
            ) : (
              <>
                <span className="sync-dot" /> Synced {status?.lastSyncedLabel ?? "—"}
              </>
            )}
          </span>
        </div>
        <PopBody reconnect={reconnect} today={today} />
      </div>
    </div>
  );
}

function PopBody({ reconnect, today }: { reconnect: boolean; today: DaySummary | null }) {
  if (reconnect) return <div className="pop-note">Open Stepwise to reconnect Google Health.</div>;
  if (!today) return <div className="pop-note">Reading today's activity…</div>;

  const zero = today.steps === 0;
  const { lead, tail } = toGo(today);
  const pct = Math.round(Math.min(today.steps / today.goal, 1) * 100);

  return (
    <>
      <div className="pop-hero">
        <MiniRing pct={today.steps / today.goal} show={!zero}>
          <div className="ring-steps">{nf(today.steps)}</div>
          <div className="ring-label">STEPS</div>
        </MiniRing>
        <div className="pop-togo">
          {zero ? (
            "No steps yet today"
          ) : (
            <>
              {lead && <b>{lead}</b>}
              {tail} · {pct}%
            </>
          )}
        </div>
      </div>
      <div className="pop-metrics">
        <Metric label="HR" value={today.restingHr != null ? String(today.restingHr) : "—"} />
        <Metric
          label="Sleep"
          value={
            today.sleepMinutes != null
              ? `${Math.floor(today.sleepMinutes / 60)}h${String(today.sleepMinutes % 60).padStart(2, "0")}`
              : "—"
          }
        />
        <Metric
          label="Dist"
          value={today.distanceMi != null ? today.distanceMi.toFixed(1) : "—"}
          unit={today.distanceMi != null ? "mi" : undefined}
        />
        <Metric label="Active" value={today.activeMinutes != null ? String(today.activeMinutes) : "—"} />
      </div>
    </>
  );
}

function Metric({ label, value, unit }: { label: string; value: string; unit?: string }) {
  return (
    <div className="m">
      <div className="mv">
        {value}
        {unit && <span className="u">{unit}</span>}
      </div>
      <div className="ml">{label}</div>
    </div>
  );
}

function MiniRing({ pct, show, children }: { pct: number; show: boolean; children: ReactNode }) {
  const R = 40;
  const C = 2 * Math.PI * R;
  const offset = C * (1 - Math.min(Math.max(pct, 0), 1));
  return (
    <div className="ring-wrap">
      <svg width="96" height="96" viewBox="0 0 96 96">
        <circle cx="48" cy="48" r={R} fill="none" stroke="var(--p-track)" strokeWidth="6" />
        {show && (
          <circle
            cx="48"
            cy="48"
            r={R}
            fill="none"
            stroke="var(--accent)"
            strokeWidth="6"
            strokeLinecap="round"
            transform="rotate(-90 48 48)"
            style={{ strokeDasharray: C, strokeDashoffset: offset }}
          />
        )}
      </svg>
      <div className="ring-num">{children}</div>
    </div>
  );
}
