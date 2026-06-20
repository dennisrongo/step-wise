pub mod traits;

use traits::MachineIdentifier;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
use macos::Platform;
#[cfg(target_os = "windows")]
use windows::Platform;
#[cfg(target_os = "linux")]
use linux::Platform;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
struct Platform;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
impl MachineIdentifier for Platform {
    fn machine_id(&self) -> String {
        "stepwise-generic-machine".to_string()
    }
}

/// A stable machine id for the current OS, used to bind encrypted secrets.
pub fn machine_id() -> String {
    Platform.machine_id()
}
