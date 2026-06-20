import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri-tuned Vite config. `base: './'` so the bundled HTML loads assets
// from tauri://localhost; port is fixed; src-tauri is ignored by the watcher
// and Rust errors are never hidden by a screen clear.
export default defineConfig({
  plugins: [react()],
  base: "./",
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2021",
    sourcemap: false,
  },
});
