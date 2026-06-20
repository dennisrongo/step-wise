// JSON-on-disk helpers, scoped to the app data directory. We never write to
// user-chosen paths from a command — only inside `app_data_dir`.
use std::path::PathBuf;

use serde::{de::DeserializeOwned, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::ResultExt;

pub fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().into_string()?;
    std::fs::create_dir_all(&dir).into_string()?;
    Ok(dir)
}

pub fn storage_path(app: &AppHandle, file: &str) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join(file))
}

pub fn load_json<T: DeserializeOwned>(path: &PathBuf) -> Result<Option<T>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path).into_string()?;
    let value = serde_json::from_str(&raw).into_string()?;
    Ok(Some(value))
}

pub fn save_json<T: Serialize>(path: &PathBuf, value: &T) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(value).into_string()?;
    std::fs::write(path, raw).into_string()?;
    Ok(())
}
