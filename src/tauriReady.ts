// True when the Tauri IPC bridge is available. In a plain browser (`npm run
// dev` without Tauri) this is false, and the data hooks fall back to demo data
// so the UI can be iterated on without the backend.
export function isTauriReady(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
