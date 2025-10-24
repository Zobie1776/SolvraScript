use crate::sys::hal::{self, DeviceHandle, DeviceKind, HalError, HardwareAbstractionLayer};

/// Ensures that at least one keyboard device is registered with the HAL.
pub fn ensure_keyboard_registered(
    hal: &dyn HardwareAbstractionLayer,
) -> Result<DeviceHandle, HalError> {
    if let Some(info) = hal
        .list_devices(Some(DeviceKind::Keyboard))
        .into_iter()
        .next()
    {
        return Ok(info.handle);
    }
    hal.register_device(hal::devices::keyboard_stub())
}
