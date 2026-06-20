import { useCallback, useEffect, useState } from "react";
import { check as checkForUpdate, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type Phase = "idle" | "checking" | "uptodate" | "available" | "downloading" | "ready" | "error";

interface UpdaterState {
  phase: Phase;
  version: string | null;
  error: string | null;
}

/**
 * Update checking against the configured endpoint (tauri.conf.json
 * plugins.updater). `auto: false` only checks when check() is called and
 * surfaces errors / an explicit "up to date" — suitable for the Settings
 * button. install() downloads, applies, and relaunches.
 *
 * Until an endpoint + signing pubkey are configured, check() resolves to the
 * "error" phase ("updater not configured"), which the UI shows as unavailable.
 */
export function useUpdater(opts: { auto?: boolean } = {}) {
  const { auto = true } = opts;
  const [state, setState] = useState<UpdaterState>({ phase: "idle", version: null, error: null });
  const [update, setUpdate] = useState<Update | null>(null);

  const check = useCallback(async () => {
    setState({ phase: "checking", version: null, error: null });
    try {
      const found = await checkForUpdate();
      if (found) {
        setUpdate(found);
        setState({ phase: "available", version: found.version, error: null });
      } else {
        setState({ phase: "uptodate", version: null, error: null });
      }
    } catch (e) {
      setState({ phase: "error", version: null, error: e instanceof Error ? e.message : String(e) });
    }
  }, []);

  useEffect(() => {
    if (!auto) return;
    let cancelled = false;
    checkForUpdate()
      .then((found) => {
        if (cancelled || !found) return;
        setUpdate(found);
        setState({ phase: "available", version: found.version, error: null });
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [auto]);

  const install = useCallback(async () => {
    if (!update) return;
    try {
      setState((s) => ({ ...s, phase: "downloading" }));
      await update.downloadAndInstall();
      setState((s) => ({ ...s, phase: "ready" }));
      await relaunch();
    } catch (e) {
      setState((s) => ({ ...s, phase: "error", error: e instanceof Error ? e.message : String(e) }));
    }
  }, [update]);

  return { ...state, check, install };
}
