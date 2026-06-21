use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

use super::traits::MachineIdentifier;

pub struct Platform;

impl MachineIdentifier for Platform {
    fn machine_id(&self) -> String {
        // Read MachineGuid from the registry. This avoids spawning any child
        // process (so no console window flashes on startup, which `wmic.exe`
        // was doing) and works on all supported Windows versions — including
        // Windows 11 builds where `wmic.exe` has been removed entirely.
        //
        // NOTE: this value differs from the old `wmic csproduct uuid`, so any
        // refresh token encrypted against the old id will fail to decrypt and
        // the user will be asked to sign in again once.
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\Microsoft\\Cryptography") {
            if let Ok(guid) = key.get_value::<String, _>("MachineGuid") {
                let trimmed = guid.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
        "stepwise-windows-fallback".to_string()
    }
}
