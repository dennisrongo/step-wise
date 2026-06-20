import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "react";
import { isTauriReady } from "../tauriReady";

export interface CommandState<T> {
  data: T | null;
  isLoading: boolean;
  error: string | null;
}

/**
 * Generic typed wrapper around Tauri's `invoke`. Per-domain hooks compose this;
 * components never call `invoke` directly. When Tauri is unavailable and a
 * `fallback` is provided, it resolves to the fallback (browser preview mode).
 */
export function useTauriCommand<T>(command: string, fallback?: T) {
  const [state, setState] = useState<CommandState<T>>({
    data: null,
    isLoading: false,
    error: null,
  });

  const execute = useCallback(
    async (args?: Record<string, unknown>): Promise<T> => {
      setState((s) => ({ ...s, isLoading: true, error: null }));
      try {
        let data: T;
        if (!isTauriReady() && fallback !== undefined) {
          data = fallback;
        } else {
          data = await invoke<T>(command, args);
        }
        setState({ data, isLoading: false, error: null });
        return data;
      } catch (e) {
        const error = e instanceof Error ? e.message : String(e);
        setState((s) => ({ ...s, isLoading: false, error }));
        throw e;
      }
    },
    [command, fallback],
  );

  return { ...state, execute };
}
