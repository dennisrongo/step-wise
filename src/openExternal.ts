import { openUrl } from "@tauri-apps/plugin-opener";
import { isTauriReady } from "./tauriReady";

// Open a URL in the user's default browser — never inside the tray webview.
// Falls back to window.open in browser-preview mode (no Tauri IPC bridge).
export async function openExternal(url: string): Promise<void> {
  if (isTauriReady()) {
    await openUrl(url);
  } else {
    window.open(url, "_blank", "noopener");
  }
}
