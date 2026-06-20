use tauri::Manager;

/// Generic system command — returns the running app version.
#[tauri::command]
pub fn app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
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
