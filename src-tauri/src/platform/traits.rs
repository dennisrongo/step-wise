/// Platform abstraction for a stable, machine-specific identifier. Only the
/// per-OS modules under `platform/` contain `cfg(target_os)` code; the rest of
/// the app goes through this trait.
pub trait MachineIdentifier {
    fn machine_id(&self) -> String;
}
