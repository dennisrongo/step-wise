import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauriReady } from "../tauriReady";
import { getActiveMode } from "../activeMode";
import { REFRESH_MS } from "../refreshInterval";
import { DEMO_STATUS, DEMO_WEEK } from "../mockData";
import type { DaySummary, RefreshResult, SyncStatus, WeekSummary } from "../types";

const DEMO_REFRESH: RefreshResult = { status: DEMO_STATUS, week: DEMO_WEEK };

// Thin command caller with a browser-preview fallback. Lives in the hook layer
// so components stay free of `invoke`.
async function call<T>(
  command: string,
  fallback: T,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauriReady()) {
    console.info(`[stepwise] browser preview — demo data for "${command}"`);
    return fallback;
  }
  return invoke<T>(command, args);
}

export interface HealthApi {
  status: SyncStatus | null;
  week: WeekSummary | null;
  selected: number;
  selectedDay: DaySummary | null;
  syncing: boolean;
  error: string | null;
  setSelected: (i: number) => void;
  connect: () => Promise<void>;
  disconnect: () => Promise<void>;
  refreshNow: () => Promise<void>;
}

export function useHealth(): HealthApi {
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [week, setWeek] = useState<WeekSummary | null>(null);
  const [selected, setSelected] = useState(0);
  const [syncing, setSyncing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const applyWeek = useCallback((w: WeekSummary) => {
    setWeek(w);
    setError(null);
    const todayIdx = w.days.findIndex((d) => d.isToday);
    setSelected(todayIdx >= 0 ? todayIdx : Math.max(0, w.days.length - 1));
  }, []);

  const loadWeek = useCallback(async () => {
    try {
      const w = await call<WeekSummary>("get_week_summary", DEMO_WEEK, {
        activeMode: getActiveMode(),
      });
      applyWeek(w);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [applyWeek]);

  const applyStatus = useCallback(
    async (s: SyncStatus) => {
      setStatus(s);
      if (s.state !== "reconnect") await loadWeek();
    },
    [loadWeek],
  );

  // Apply a `refresh_now` result in one shot — its `week` is already the fresh
  // fetch, so there's no follow-up `get_week_summary`.
  const applyRefresh = useCallback(
    (r: RefreshResult) => {
      setStatus(r.status);
      applyWeek(r.week);
    },
    [applyWeek],
  );

  useEffect(() => {
    call<SyncStatus>("get_sync_status", DEMO_STATUS)
      .then(applyStatus)
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, [applyStatus]);

  const run = useCallback(
    async (command: string, args?: Record<string, unknown>) => {
      setSyncing(true);
      setError(null);
      try {
        const s = isTauriReady()
          ? await invoke<SyncStatus>(command, args)
          : DEMO_STATUS;
        await applyStatus(s);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setSyncing(false);
      }
    },
    [applyStatus],
  );

  const connect = useCallback(() => run("connect_google_health"), [run]);
  const disconnect = useCallback(() => run("disconnect"), [run]);

  // Manual refresh: visible syncing state + a single `refresh_now` that returns
  // both the fresh status and the week, so we apply them without a second fetch.
  const refreshNow = useCallback(async () => {
    setSyncing(true);
    setError(null);
    try {
      const r = isTauriReady()
        ? await invoke<RefreshResult>("refresh_now", { activeMode: getActiveMode() })
        : DEMO_REFRESH;
      applyRefresh(r);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSyncing(false);
    }
  }, [applyRefresh]);

  // The same single-fetch refresh for the auto-refresh loop, but silent: it
  // advances the "Synced …" stamp and the numbers without toggling the visible
  // `syncing` state (no dim/spinner per tick). Errors are swallowed — a
  // transient failure on a background tick shouldn't blank the UI.
  const quietBusy = useRef(false);
  const refreshQuiet = useCallback(async () => {
    if (quietBusy.current) return;
    quietBusy.current = true;
    try {
      const r = isTauriReady()
        ? await invoke<RefreshResult>("refresh_now", { activeMode: getActiveMode() })
        : DEMO_REFRESH;
      applyRefresh(r);
    } catch {
      /* keep last good values */
    } finally {
      quietBusy.current = false;
    }
  }, [applyRefresh]);

  // Auto-refresh every 15s, but only while the panel is actually visible. The
  // panel stays open when it loses focus (it's dismissed only by toggling the
  // tray icon), so we gate on the window's visibility — like agent-status —
  // rather than focus: a hidden panel polls nothing, while an open one keeps
  // its numbers live even when you're working in another app. Also skips while
  // disconnected or a manual sync is already running.
  const statusRef = useRef<SyncStatus | null>(null);
  const syncingRef = useRef(false);
  useEffect(() => {
    statusRef.current = status;
  }, [status]);
  useEffect(() => {
    syncingRef.current = syncing;
  }, [syncing]);

  useEffect(() => {
    const tick = async () => {
      if (syncingRef.current) return;
      if (statusRef.current?.state === "reconnect") return;
      if (isTauriReady()) {
        const visible = await getCurrentWindow().isVisible().catch(() => true);
        if (!visible) return;
      } else if (document.visibilityState !== "visible") {
        return;
      }
      void refreshQuiet();
    };

    const timer = setInterval(() => void tick(), REFRESH_MS);
    return () => clearInterval(timer);
  }, [refreshQuiet]);

  // Refresh the instant the panel regains focus (or the tab becomes visible),
  // independent of the interval above. WebView2/Chromium throttles background
  // timers — a persistent panel left open behind another window has its 15s
  // tick clamped (down to ~1/min after a few minutes), so the numbers can sit
  // stale until you look back at it. This makes them current the moment you do.
  // No value is computed here — it just calls the same fetch sooner.
  useEffect(() => {
    if (!isTauriReady()) {
      const onVis = () => {
        if (document.visibilityState === "visible") void refreshQuiet();
      };
      document.addEventListener("visibilitychange", onVis);
      return () => document.removeEventListener("visibilitychange", onVis);
    }
    const un = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) void refreshQuiet();
    });
    return () => {
      void un.then((f) => f());
    };
  }, [refreshQuiet]);

  const selectedDay = week && week.days[selected] ? week.days[selected] : null;

  return {
    status,
    week,
    selected,
    selectedDay,
    syncing,
    error,
    setSelected,
    connect,
    disconnect,
    refreshNow,
  };
}
