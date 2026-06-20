//! Menu-bar / taskbar tray. Left-click toggles the full panel; hovering the icon
//! shows a compact glance popover. Both anchor to the tray icon and flip by
//! platform — macOS menu bar at the top, Windows taskbar at the bottom. The
//! hover/placement behavior is modeled on the agent-status widget.
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, LogicalPosition, Manager, PhysicalPosition, PhysicalSize, Rect, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

const TRAY_ID: &str = "stepwise-tray";

/// Hover popover window label. It loads the same bundle as "main"; the frontend
/// renders the compact popover when it sees this label.
const HOVER_LABEL: &str = "hover";

/// Logical width of the hover window (card is 300 + 10px gutter each side for
/// the popover's shadow). Height is fit to content by the frontend; the width
/// stays fixed so the right-edge anchor under the icon holds.
const HOVER_WIDTH: f64 = 320.0;

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let refresh = MenuItem::with_id(app, "refresh", "Refresh now", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Stepwise", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&refresh, &PredefinedMenuItem::separator(app)?, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Stepwise")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "refresh" => {
                let _ = app.emit("hover-show", ());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    rect,
                    ..
                } => {
                    hide_hover(app);
                    toggle_main(app, Some(rect));
                }
                // Hovering previews today's glance without opening the full panel.
                TrayIconEvent::Enter { rect, .. } => show_hover(app, rect),
                TrayIconEvent::Leave { .. } => hide_hover(app),
                _ => {}
            }
        });

    // Monochrome footprint template glyph — macOS tints it to the menu bar
    // (light/dark) and keeps the transparent background. Falls back to the app
    // icon if PNG decoding ever fails.
    let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))
        .ok()
        .or_else(|| app.default_window_icon().cloned());
    if let Some(icon) = tray_icon {
        builder = builder.icon(icon).icon_as_template(true);
    }
    builder.build(app)?;

    build_hover_window(app)?;
    Ok(())
}

/// Pre-create the hover popover (hidden) so it's loaded and listening before the
/// first hover. Borderless, transparent, non-focusing, always-on-top — a passive
/// preview that never steals focus from whatever the user is doing.
fn build_hover_window(app: &AppHandle) -> tauri::Result<()> {
    if app.get_webview_window(HOVER_LABEL).is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, HOVER_LABEL, WebviewUrl::App("index.html".into()))
        .title("Stepwise")
        .inner_size(HOVER_WIDTH, 200.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .focused(false)
        .visible(false)
        .build()?;
    Ok(())
}

fn toggle_main(app: &AppHandle, rect: Option<Rect>) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
        return;
    }
    position(&win, rect);
    let _ = win.show();
    // macOS can re-place a window onto another monitor when it becomes visible;
    // re-assert so the panel lands on the display whose menu bar was clicked.
    position(&win, rect);
    let _ = win.set_focus();
}

fn show_hover(app: &AppHandle, rect: Rect) {
    // Don't compete with the full panel when it's already open.
    if app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
    {
        return;
    }
    let Some(win) = app.get_webview_window(HOVER_LABEL) else {
        return;
    };
    position(&win, Some(rect));
    let _ = win.show();
    position(&win, Some(rect));
    // Tell the popover to refresh its numbers as it appears.
    let _ = app.emit("hover-show", ());
}

fn hide_hover(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(HOVER_LABEL) {
        let _ = win.hide();
    }
}

/// Place a tray window relative to the icon. Anchor flips by platform; falls
/// back to a monitor corner when there's no icon geometry.
fn position(window: &WebviewWindow, rect: Option<Rect>) {
    let Some(rect) = rect else {
        place_fallback(window);
        return;
    };
    let pos = rect.position.to_physical::<f64>(1.0);
    let size = rect.size.to_physical::<f64>(1.0);
    place_window(window, pos.x, pos.y, size.width, size.height);
}

/// macOS: hang the window off the menu-bar icon — top edge just below the menu
/// bar, right edge aligned with the icon's right edge so it extends down-left.
/// Math in logical points (uniform across mixed-DPI displays).
#[cfg(not(target_os = "windows"))]
fn place_window(window: &WebviewWindow, icon_x: f64, icon_y: f64, icon_w: f64, icon_h: f64) {
    let Some(scale) = icon_display_scale(window, icon_x, icon_y) else {
        place_fallback(window);
        return;
    };
    let icon_right = (icon_x + icon_w) / scale;
    let menubar_bottom = (icon_y + icon_h) / scale;
    let win_scale = window.scale_factor().unwrap_or(1.0).max(0.01);
    let win_w = window.outer_size().map(|s| s.width as f64).unwrap_or(0.0) / win_scale;
    let _ = window.set_position(LogicalPosition::new(icon_right - win_w, menubar_bottom));
}

/// Windows: pin to the bottom-right corner of the monitor's work area (flush
/// above the taskbar). The tray icon often lives in the hidden-icons flyout, so
/// the work-area corner is a more stable anchor than the icon rect.
#[cfg(target_os = "windows")]
fn place_window(window: &WebviewWindow, icon_x: f64, icon_y: f64, _icon_w: f64, _icon_h: f64) {
    let Some(monitor) = icon_monitor(window, icon_x, icon_y) else {
        place_fallback(window);
        return;
    };
    let wa = monitor.work_area();
    let outer = window.outer_size().unwrap_or_default();
    let x = wa.position.x + wa.size.width as i32 - outer.width as i32;
    let y = wa.position.y + wa.size.height as i32 - outer.height as i32;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

#[cfg(target_os = "windows")]
fn icon_monitor(window: &WebviewWindow, phys_x: f64, phys_y: f64) -> Option<tauri::Monitor> {
    let (x, y) = (phys_x as i32, phys_y as i32);
    window.available_monitors().ok()?.into_iter().find(|m| {
        let p = m.position();
        let s = m.size();
        x >= p.x && x < p.x + s.width as i32 && y >= p.y && y < p.y + s.height as i32
    })
}

/// Scale factor of the display the tray icon sits on, resolved by reinterpreting
/// the physical icon coordinate in each monitor's scale and picking the monitor
/// where the icon sits closest to the right edge (where menu-bar icons live).
#[cfg(not(target_os = "windows"))]
fn icon_display_scale(window: &WebviewWindow, phys_x: f64, phys_y: f64) -> Option<f64> {
    let mut best: Option<(f64, f64)> = None;
    for m in window.available_monitors().ok()? {
        let s = m.scale_factor();
        if s <= 0.0 {
            continue;
        }
        let (left, top) = (m.position().x as f64 / s, m.position().y as f64 / s);
        let (w, h) = (m.size().width as f64 / s, m.size().height as f64 / s);
        let (cx, cy) = (phys_x / s, phys_y / s);
        if cx >= left && cx <= left + w && cy >= top && cy <= top + h {
            let dist_right = (left + w) - cx;
            if best.map_or(true, |(_, d)| dist_right < d) {
                best = Some((s, dist_right));
            }
        }
    }
    best.map(|(s, _)| s)
}

/// Corner fallback when there's no icon geometry (menu-driven open, or a failed
/// scale lookup): top-right on macOS, bottom-right above the taskbar on Windows.
fn place_fallback(window: &WebviewWindow) {
    if let Ok(Some(monitor)) = window.primary_monitor() {
        let scale = monitor.scale_factor();
        let msize = monitor.size();
        let wsize = window.outer_size().unwrap_or(PhysicalSize {
            width: 320,
            height: 200,
        });
        let margin = (10.0 * scale) as i32;
        let x = (msize.width as i32 - wsize.width as i32 - margin).max(0);
        #[cfg(target_os = "windows")]
        let y = (msize.height as i32 - wsize.height as i32 - (48.0 * scale) as i32).max(0);
        #[cfg(not(target_os = "windows"))]
        let y = (28.0 * scale) as i32;
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
}
