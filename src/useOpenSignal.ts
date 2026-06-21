import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { isTauriReady } from "./tauriReady";

/**
 * Increments once per genuine panel open — i.e. each time the panel is shown
 * from the tray (or surfaced by a second app launch). The Rust backend emits
 * `panel-opened` precisely at those moments. Used as a React `key` so the
 * dashboard remounts and replays its entrance animation + count-up on every
 * open, like the design.
 *
 * We deliberately do NOT key off window focus here: on Windows the panel stays
 * open while you click around, and almost any click re-focuses the window,
 * which fired this hook on every click and re-triggered the full dashboard
 * animation. An explicit open event fires exactly when the user actually opens
 * the panel, on every platform.
 */
export function useOpenSignal(): number {
  const [n, setN] = useState(0);
  useEffect(() => {
    if (!isTauriReady()) return;
    const un = listen("panel-opened", () => setN((x) => x + 1));
    return () => {
      void un.then((f) => f());
    };
  }, []);
  return n;
}
