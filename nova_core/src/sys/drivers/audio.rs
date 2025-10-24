use crate::sys::hal::{self, DeviceHandle, DeviceKind, HalError, HardwareAbstractionLayer};

/// Registers an external speaker device if none exists.
pub fn ensure_speaker_registered(
    hal: &dyn HardwareAbstractionLayer,
) -> Result<DeviceHandle, HalError> {
    if let Some(info) = hal
        .list_devices(Some(DeviceKind::Speaker { external: true }))
        .into_iter()
        .next()
    {
        return Ok(info.handle);
    }
    hal.register_device(hal::devices::speaker_stub())
}
