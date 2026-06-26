use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tokio::sync::Mutex;

use crate::state::AppState;

/// Generic system command — returns the running app version.
#[tauri::command]
pub fn app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Apply (or deliberately skip) the OS launch-at-login registration.
///
/// No-op in dev builds: `tauri dev` runs the binary straight out of
/// `target/debug`, so registering autostart there would write a login item
/// pointing at that throwaway path — at the next login the OS would try to
/// launch a stale/dev binary, and on macOS every dev run would re-fire the
/// "added to Login Items" notification. Only bundled / `tauri build` builds
/// (where `is_dev()` is false) touch the OS; the stored setting is untouched
/// here, so the UI still reflects the user's choice. Called on startup to keep
/// the registration in sync with the saved setting, and by the toggle command.
pub fn apply_autostart(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if tauri::is_dev() {
        return Ok(());
    }
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable().map_err(|e| e.to_string())
    } else {
        autostart.disable().map_err(|e| e.to_string())
    }
}

/// Read the persisted launch-at-login preference (the stored setting is the
/// source of truth for the UI; the OS registration is synced to match it).
#[tauri::command]
pub async fn get_launch_on_startup(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<bool, String> {
    Ok(state.lock().await.settings.launch_on_startup)
}

/// Toggle launch-at-login: register/unregister the OS launch agent, then persist
/// the choice. The OS registration is applied before saving so a failure to
/// update the OS leaves the stored setting untouched.
#[tauri::command]
pub async fn set_launch_on_startup(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    enabled: bool,
) -> Result<bool, String> {
    apply_autostart(&app, enabled)?;
    let mut st = state.lock().await;
    st.settings.launch_on_startup = enabled;
    st.settings.save(&app).map_err(|e| e.to_string())?;
    Ok(enabled)
}

/// Resize a tray window, and on Windows re-pin it to the bottom-right work-area
/// corner in the same native call. Issuing setSize then setPosition as two
/// webview calls races WebView2's IPC and the second op is often dropped; doing
/// both here (sequential Win32 calls, no paint between) is reliable. macOS keeps
/// the plain top-anchored setSize in the frontend and never calls this.
#[tauri::command]
pub fn fit_tray_window(app: tauri::AppHandle, label: String, width: f64, height: f64) {
    let Some(win) = app.get_webview_window(&label) else {
        return;
    };
    let _ = win.set_size(tauri::LogicalSize::new(width, height));
    #[cfg(target_os = "windows")]
    if let Some(mon) = win.current_monitor().ok().flatten() {
        let wa = mon.work_area();
        let scale = mon.scale_factor();
        let x = wa.position.x + wa.size.width as i32 - (width * scale).round() as i32;
        let y = wa.position.y + wa.size.height as i32 - (height * scale).round() as i32;
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
    }
}
