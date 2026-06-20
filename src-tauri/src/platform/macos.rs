use std::process::Command;

use super::traits::MachineIdentifier;

pub struct Platform;

impl MachineIdentifier for Platform {
    fn machine_id(&self) -> String {
        if let Ok(out) = Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if line.contains("IOPlatformUUID") {
                    if let Some(start) = line.find("= \"") {
                        let rest = &line[start + 3..];
                        if let Some(end) = rest.find('"') {
                            return rest[..end].to_string();
                        }
                    }
                }
            }
        }
        "stepwise-macos-fallback".to_string()
    }
}
