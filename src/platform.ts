import { invoke } from "@tauri-apps/api/core";
import { LogicalSize, type Window } from "@tauri-apps/api/window";

// Reliable in the webview without the os plugin. Flips the tray-window anchor:
// macOS hangs windows down from the top menu bar; Windows grows them up from the
// bottom taskbar.
export const isWindows =
  typeof navigator !== "undefined" && navigator.userAgent.includes("Windows");

/**
 * Resize a tray window to fit its content.
 *
 * macOS: the window is anchored top-under-the-menu-bar, so the OS default
 * (top-left fixed, grows downward) is exactly right — a plain setSize.
 *
 * Windows: the window is anchored bottom-on-the-taskbar (see place_window in
 * tray.rs), so the resize must keep the bottom edge pinned. The `fit_tray_window`
 * Rust command resizes AND re-pins the bottom-right corner in one native step;
 * doing it as two webview calls races WebView2's IPC and drops the second op.
 */
export async function fitWindowHeight(
  win: Window,
  width: number,
  height: number,
): Promise<void> {
  if (!isWindows) {
    await win.setSize(new LogicalSize(width, height));
    return;
  }
  await invoke("fit_tray_window", { label: win.label, width, height });
}
