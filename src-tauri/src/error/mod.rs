// Helpers for converting module-level `thiserror` enums into the `String`
// errors that `#[tauri::command]` returns at the IPC boundary.

/// Convert any `Result<T, E: Display>` into `Result<T, String>`.
pub trait ResultExt<T> {
    fn into_string(self) -> Result<T, String>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
    fn into_string(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}
