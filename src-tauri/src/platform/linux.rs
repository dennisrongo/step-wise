use super::traits::MachineIdentifier;

pub struct Platform;

impl MachineIdentifier for Platform {
    fn machine_id(&self) -> String {
        for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
            if let Ok(contents) = std::fs::read_to_string(path) {
                let trimmed = contents.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
        "stepwise-linux-fallback".to_string()
    }
}
