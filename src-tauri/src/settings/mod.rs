// Persisted settings. The OAuth refresh token is stored as an encrypted blob,
// never plaintext. JSON keys are camelCase to match the frontend.
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::encryption::EncryptedSecret;
use crate::storage;

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Google Health OAuth refresh token, AES-256-GCM encrypted at rest.
    pub google_refresh_token: Option<EncryptedSecret>,
    /// Last successful sync, RFC 3339. Drives the "Synced N min ago" stamp.
    pub last_synced_at: Option<String>,
    /// Manual theme override: "light" | "dark" | null (follow system).
    pub theme: Option<String>,
}

impl Settings {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let path = storage::storage_path(app, SETTINGS_FILE)?;
        Ok(storage::load_json::<Settings>(&path)?.unwrap_or_default())
    }

    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let path = storage::storage_path(app, SETTINGS_FILE)?;
        storage::save_json(&path, self)
    }
}
