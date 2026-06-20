import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { isTauriReady } from "../tauriReady";
import { DEMO_STATUS, DEMO_WEEK } from "../mockData";
import type { DaySummary, SyncStatus, WeekSummary } from "../types";

// Thin command caller with a browser-preview fallback. Lives in the hook layer
// so components stay free of `invoke`.
async function call<T>(command: string, fallback: T): Promise<T> {
  if (!isTauriReady()) {
    console.info(`[stepwise] browser preview — demo data for "${command}"`);
    return fallback;
  }
  return invoke<T>(command);
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
  refreshNow: () => Promise<void>;
}

export function useHealth(): HealthApi {
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [week, setWeek] = useState<WeekSummary | null>(null);
  const [selected, setSelected] = useState(0);
  const [syncing, setSyncing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadWeek = useCallback(async () => {
    try {
      const w = await call<WeekSummary>("get_week_summary", DEMO_WEEK);
      setWeek(w);
      const todayIdx = w.days.findIndex((d) => d.isToday);
      setSelected(todayIdx >= 0 ? todayIdx : Math.max(0, w.days.length - 1));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);

  const applyStatus = useCallback(
    async (s: SyncStatus) => {
      setStatus(s);
      if (s.state !== "reconnect") await loadWeek();
    },
    [loadWeek],
  );

  useEffect(() => {
    call<SyncStatus>("get_sync_status", DEMO_STATUS)
      .then(applyStatus)
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, [applyStatus]);

  const run = useCallback(
    async (command: string) => {
      setSyncing(true);
      setError(null);
      try {
        const s = isTauriReady()
          ? await invoke<SyncStatus>(command)
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
  const refreshNow = useCallback(() => run("refresh_now"), [run]);

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
    refreshNow,
  };
}
