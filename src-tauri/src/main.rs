// Prevent a console window from flashing behind the app on Windows release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Carry the Info.plist inside the binary so it works when run standalone.
#[cfg(target_os = "macos")]
embed_plist::embed_info_plist!("../Info.plist");

fn main() {
    stepwise_lib::run();
}
