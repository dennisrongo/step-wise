import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauriReady } from "./tauriReady";

/**
 * Increments every time the window gains focus — i.e. each time the panel is
 * opened from the tray (it hides on blur, shows + focuses on open). Used as a
 * React `key` so the dashboard remounts and replays its entrance animation +
 * count-up on every open, like the design.
 */
export function useOpenSignal(): number {
  const [n, setN] = useState(0);
  useEffect(() => {
    if (!isTauriReady()) return;
    let unlisten: (() => void) | undefined;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) setN((x) => x + 1);
      })
      .then((f) => {
        unlisten = f;
      });
    return () => unlisten?.();
  }, []);
  return n;
}
