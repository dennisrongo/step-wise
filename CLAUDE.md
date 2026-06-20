# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Stepwise — a macOS (also Windows/Linux) menu-bar health dashboard built with **Tauri 2**. A React frontend renders a ~360px tray panel; a Rust backend does all OAuth, HTTP, persistence, and OS work. It reads activity data from the **Google Health API v4** (synced from a Pixel phone). The numbers reflect the phone's last cloud sync, not a live counter — the UI deliberately never fakes a ticking value.

## Commands

```bash
npm install                      # frontend deps

# Run the full app (Tauri compiles Rust + serves the Vite frontend):
STEPWISE_DEMO=1 npm run tauri dev   # placeholder data, no Google creds needed
npm run tauri dev                   # real data (needs .env + in-app connect)

npm run build                    # tsc typecheck + vite build → dist/ (Tauri bundles this)
npm run tauri build              # produce the distributable app bundle

# Rust tests (must run from src-tauri/):
cd src-tauri && cargo test --all
cd src-tauri && cargo test --all demo_today_matches_the_design   # a single test by name
```

- **Lint/typecheck:** the frontend has no eslint; `tsc` (via `npm run build`) is the type gate (`strict`, `noUnusedLocals`, `noUnusedParameters`). For Rust use `cargo clippy` / `cargo fmt` from `src-tauri/`.
- **There is no frontend test runner.** All automated tests are Rust (`src-tauri/tests/` + `#[cfg(test)]` modules). CI (`.github/workflows/unit-tests.yml`) runs `npm run build` then `cargo test --all` on macOS/Windows/Ubuntu.
- Copy `.env.example` → `.env` and fill `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` (Desktop-app OAuth client) for real data. `.env` is gitignored.

## Architecture

**Thin frontend, rich backend.** The architectural rule is that all logic lives in Rust; React only renders state and calls typed commands.

- **Frontend (`src/`)** — components NEVER call Tauri `invoke()` directly. They go through the hook layer: `hooks/useHealth.ts` (the one domain hook the app uses) and the generic `hooks/useTauriCommand.ts`. `tauriReady.ts#isTauriReady()` detects whether the Tauri IPC bridge exists; when it doesn't (plain `npm run dev` in a browser), the hooks fall back to `mockData.ts` so the UI can be iterated without the backend.
- **Backend (`src-tauri/src/`)** — modular Tauri layout: `commands/` (IPC entry points), `oauth/`, `health/`, `encryption/`, `state/`, `storage/`, `settings/`, `platform/`, `error/`, `tray.rs`. `lib.rs#run()` wires plugins, registers the command handler, and sets up the tray + windows.

### Adding a Tauri command (the end-to-end path)

1. Write `#[tauri::command] pub async fn …` in `commands/health.rs` or `commands/system.rs`.
2. Register it in the `tauri::generate_handler![…]` list in `src-tauri/src/lib.rs` — **forgetting this is the usual "command not found" cause.**
3. Call it from a hook in `src/hooks/` (never inline in a component), adding a demo fallback for browser-preview mode.
4. Keep the wire types camelCase: Rust structs use `#[serde(rename_all = "camelCase")]` to match `src/types.ts`.

### Conventions that span files

- **State locking discipline (`state/mod.rs`, `commands/health.rs`):** `AppState` lives behind a `tokio::sync::Mutex`. Commands lock, read/clone what they need (including `http` client and decrypted token), then **drop the guard before any network `.await`.** Never hold the lock across an await.
- **IPC error boundary:** internal modules use `thiserror` enums (`HealthError`, `OAuthError`, `EncryptionError`); `#[tauri::command]` functions stringify them to `Result<T, String>`. `error/mod.rs#ResultExt::into_string()` is the helper for `String`-returning storage code.
- **Secrets at rest (`encryption/mod.rs`, `settings/mod.rs`):** the OAuth refresh token is the only secret persisted, stored AES-256-GCM encrypted with an Argon2id key bound to a machine id (`platform::machine_id()`). It is never written in plaintext. JSON settings live in `app_data_dir` only — `storage/mod.rs` never writes user-chosen paths.
- **OAuth (`oauth/mod.rs`):** desktop loopback + PKCE. `start_flow` binds `127.0.0.1:0`, `wait_for_code` blocks on the redirect (runs inside `spawn_blocking`), `exchange_code`/`refresh` hit Google's token endpoint. State param is checked for CSRF.
- **Health data (`health/`):** behind a real Google source (`google.rs`) and a Demo source (`demo.rs`); demo is selected by `STEPWISE_DEMO=1`. Only **steps** are fully wired from the API (daily roll-up + intraday hourly). Resting HR / sleep / distance / active minutes are intentionally `None` (need extra scopes) — keep them honest, don't fabricate.

### Menu-bar form factor (`tray.rs`, `platform.ts`, `commands/system.rs`)

- Two windows load the **same** frontend bundle: `main` (full panel) and `hover` (compact glance). `src/main.tsx` routes by `getCurrentWindow().label === "hover"` to render `HoverPopover` vs `App`.
- On macOS the app is an `Accessory` (no Dock icon); the panel hides on focus loss like a native popover.
- Window placement flips by platform: macOS hangs the window down from the menu-bar icon; Windows pins it to the bottom-right work area above the taskbar. On **Windows**, resize-and-reposition must happen in one native call via the `fit_tray_window` command — doing setSize then setPosition as two webview calls races WebView2's IPC and drops the second op. macOS uses a plain `setSize`. This branch lives in `platform.ts#fitWindowHeight`.

## Gotchas / before shipping

- The bundle identifier `com.dennisrongo.stepwise` in `src-tauri/tauri.conf.json` is permanent after distribution — confirm before any release.
- Icons under `src-tauri/icons/` are placeholders; regenerate with `npx @tauri-apps/cli icon icons/icon-source.png`.
- Files matching `*.backup`, `*.orig`, `*.temp`, `.gh-tokens.json` are gitignored and must never be committed.
