# Stepwise

A calm, glanceable macOS menu-bar health dashboard. It reads activity data
synced from a Pixel phone through Google's Health API and shows today's steps,
hourly shape, a few key metrics, and the last 7 days in a ~360px dropdown panel.

> **Honest about freshness.** The numbers reflect your phone's *last cloud
> sync*, not a live pedometer. The UI says "Synced 3 min ago" — it never fakes
> a ticking counter. This mirrors the Google Health API reality (a daily/
> intraday roll-up of what the device has uploaded).

## Architecture

- **Thin React frontend** (`src/`) — renders the four panel states and calls
  Rust commands through typed hooks. No `invoke()` inline in components.
- **Rich Rust backend** (`src-tauri/src/`) — all OAuth, HTTP, persistence, and
  OS work:
  - `oauth/` — desktop loopback + PKCE flow (ported from the reference
    `gh-auth.mjs`).
  - `health/` — daily roll-up + intraday interval calls (ported from
    `health-steps.mjs` / `live-steps.mjs`), behind a `HealthProvider` enum
    with a real Google source and a Demo source.
  - `encryption/` — the OAuth refresh token is stored AES-256-GCM encrypted
    with an Argon2id key bound to a machine id. No plaintext `.gh-tokens.json`.
  - `commands/`, `state/`, `storage/`, `settings/`, `platform/`, `error/` —
    standard modular Tauri layout.
- **Menu-bar form factor** — a tray icon toggles a borderless, transparent,
  always-on-top ~360px window positioned under the icon.

## Develop

```bash
npm install
cp .env.example .env        # fill in GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET

# Preview the Connected design with placeholder data (no Google needed):
STEPWISE_DEMO=1 npm run tauri dev

# Real data (after filling .env and connecting in-app):
npm run tauri dev
```

## TODO before shipping

- [ ] Replace the placeholder icons: `npx @tauri-apps/cli icon icons/icon-source.png`.
- [ ] Confirm the bundle identifier in `src-tauri/tauri.conf.json`
      (`com.dennisrongo.stepwise`) before any distribution — it can't change after.
- [ ] If you want auto-updates, generate keys with `npx tauri signer generate`
      and wire `plugins.updater` (off by default).
