use crate::sys::hal::{
    self, DeviceHandle, DeviceKind, HalError, HardwareAbstractionLayer, StorageBus,
};

/// Registers a default SD-card storage device for sandbox testing.
pub fn ensure_storage_registered(
    hal: &dyn HardwareAbstractionLayer,
) -> Result<DeviceHandle, HalError> {
    if let Some(info) = hal
        .list_devices(Some(DeviceKind::Storage(StorageBus::SdCard)))
        .into_iter()
        .next()
    {
        return Ok(info.handle);
    }
    hal.register_device(hal::devices::storage_stub())
}
