pub mod commands;
pub mod encryption;
pub mod error;
pub mod health;
pub mod oauth;
pub mod platform;
pub mod settings;
pub mod state;
pub mod storage;
pub mod tray;

use tauri::{Manager, WindowEvent};
use tokio::sync::Mutex;

use state::AppState;

pub fn run() {
    // Load .env (dev convenience) before reading GOOGLE_CLIENT_ID / SECRET.
    let _ = dotenvy::dotenv();
    init_tracing();

    tauri::Builder::default()
        // Single-instance must be registered first: a second launch focuses
        // the existing panel instead of starting a rival process.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_sync_status,
            commands::connect_google_health,
            commands::disconnect,
            commands::refresh_now,
            commands::get_week_summary,
            commands::get_day_summary,
            commands::app_version,
            commands::fit_tray_window,
        ])
        .setup(|app| {
            let demo = std::env::var("STEPWISE_DEMO")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            let settings = settings::Settings::load(&app.handle().clone()).unwrap_or_default();
            app.manage(Mutex::new(AppState::new(settings, demo)));

            tray::create_tray(&app.handle().clone())?;

            // Menu-bar agent: no Dock icon / app-switcher entry on macOS.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Hide the panel when it loses focus, like a native menu-bar popover.
            if let Some(win) = app.get_webview_window("main") {
                let w = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = w.hide();
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Stepwise");
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = fmt().with_env_filter(filter).try_init();
}
