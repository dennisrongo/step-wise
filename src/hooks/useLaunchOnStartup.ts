import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { isTauriReady } from "../tauriReady";

// Launch-at-login lives in Rust (it's an OS-level registration), so unlike the
// localStorage-backed prefs this reads/writes through the backend. The stored
// setting is the source of truth; the backend syncs the OS registration to it.
// Defaults on — a menu-bar app is expected to persist across logins.
export function useLaunchOnStartup() {
  const [enabled, setEnabled] = useState(true);

  useEffect(() => {
    if (!isTauriReady()) return;
    invoke<boolean>("get_launch_on_startup")
      .then(setEnabled)
      .catch(() => {});
  }, []);

  const toggle = useCallback(async (next: boolean) => {
    // Optimistic: reflect the choice immediately, roll back if the OS
    // registration fails so the toggle never lies about the real state.
    setEnabled(next);
    if (!isTauriReady()) return;
    try {
      const applied = await invoke<boolean>("set_launch_on_startup", { enabled: next });
      setEnabled(applied);
    } catch {
      setEnabled(!next);
    }
  }, []);

  return { enabled, toggle };
}
