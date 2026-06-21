// Shared app state, managed behind a tokio Mutex. Commands read/clone what they
// need and drop the guard before any network `.await`.
use crate::settings::Settings;

pub struct AppState {
    pub settings: Settings,
    /// Demo mode (STEPWISE_DEMO=1) returns realistic placeholder data.
    pub demo: bool,
    pub syncing: bool,
    /// Set when a stored refresh token failed to decrypt this session (so it was
    /// cleared). Lets every fetch — not just the one that hit the failure first —
    /// report `NeedsReconnect` instead of a generic "not connected", so concurrent
    /// callers can't race into the wrong error. Reset on (dis)connect.
    pub needs_reconnect: bool,
    pub http: reqwest::Client,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

impl AppState {
    pub fn new(settings: Settings, demo: bool) -> Self {
        Self {
            settings,
            demo,
            syncing: false,
            needs_reconnect: false,
            http: reqwest::Client::new(),
            // Runtime env wins (dev / `.env`); fall back to values baked in at
            // build time via `option_env!`, so distributed bundles — which inherit
            // neither the shell env nor the gitignored `.env` — still have creds.
            client_id: std::env::var("GOOGLE_CLIENT_ID")
                .ok()
                .or_else(|| option_env!("GOOGLE_CLIENT_ID").map(str::to_owned))
                .filter(|s| !s.is_empty()),
            client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
                .ok()
                .or_else(|| option_env!("GOOGLE_CLIENT_SECRET").map(str::to_owned))
                .filter(|s| !s.is_empty()),
        }
    }

    /// Connected when demo mode is on, or a refresh token is stored.
    pub fn connected(&self) -> bool {
        self.demo || self.settings.google_refresh_token.is_some()
    }
}
