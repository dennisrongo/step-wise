use std::process::Command;

use super::traits::MachineIdentifier;

pub struct Platform;

impl MachineIdentifier for Platform {
    fn machine_id(&self) -> String {
        if let Ok(out) = Command::new("wmic").args(["csproduct", "get", "uuid"]).output() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines().map(|l| l.trim()) {
                if !line.is_empty() && !line.eq_ignore_ascii_case("UUID") {
                    return line.to_string();
                }
            }
        }
        "stepwise-windows-fallback".to_string()
    }
}
