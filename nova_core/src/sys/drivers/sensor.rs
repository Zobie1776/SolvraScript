use crate::sys::hal::{
    self, DeviceHandle, DeviceKind, HalError, HardwareAbstractionLayer, SensorKind,
};

/// Registers a temperature sensor stub used by the sandbox security policy.
pub fn ensure_temperature_sensor(
    hal: &dyn HardwareAbstractionLayer,
) -> Result<DeviceHandle, HalError> {
    if let Some(info) = hal
        .list_devices(Some(DeviceKind::Sensor(SensorKind::Temperature)))
        .into_iter()
        .next()
    {
        return Ok(info.handle);
    }
    hal.register_device(hal::devices::temperature_sensor_stub())
}
