// Shared app state, managed behind a tokio Mutex. Commands read/clone what they
// need and drop the guard before any network `.await`.
use crate::settings::Settings;

pub struct AppState {
    pub settings: Settings,
    /// Demo mode (STEPWISE_DEMO=1) returns realistic placeholder data.
    pub demo: bool,
    pub syncing: bool,
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
            http: reqwest::Client::new(),
            client_id: std::env::var("GOOGLE_CLIENT_ID").ok().filter(|s| !s.is_empty()),
            client_secret: std::env::var("GOOGLE_CLIENT_SECRET").ok().filter(|s| !s.is_empty()),
        }
    }

    /// Connected when demo mode is on, or a refresh token is stored.
    pub fn connected(&self) -> bool {
        self.demo || self.settings.google_refresh_token.is_some()
    }
}
