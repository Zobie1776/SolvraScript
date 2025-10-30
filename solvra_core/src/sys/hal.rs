//! Hardware Abstraction Layer definitions for SolvraCore.
//!
//! The HAL acts as a thin shim between architecture-specific backends,
//! device drivers, and the SolvraRuntime.  It provides a trait-based
//! interface for enumerating peripherals, reading and writing registers,
//! and emitting interrupts.  Concrete implementations (such as
//! [`SoftwareHal`]) can back the interface with virtual devices, host
//! integrations, or real hardware adapters.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::RwLock;
use thiserror::Error;

use crate::backend::TargetArch;
use crate::integration::RuntimeHooks;
use crate::sys::drivers::{DriverRegistry, Interrupt};

use super::drivers::DriverError;

/// Unique handle used to reference a registered device.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceHandle(String);

impl DeviceHandle {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        DeviceHandle(format!("dev-{id:016x}"))
    }

    /// Returns the string representation of the handle.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Describes the class of device handled by the HAL.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    Keyboard,
    Mouse,
    GameController,
    Speaker { external: bool },
    Microphone,
    Storage(StorageBus),
    Sensor(SensorKind),
    Display,
    Network,
    Custom(String),
}

/// Storage bus variants supported by SolvraCore.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageBus {
    Sata,
    Nvme,
    Usb,
    SdCard,
    MemoryMapped,
    Custom(String),
}

/// Sensor types supported by SolvraCore.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SensorKind {
    Temperature,
    Motion,
    Proximity,
    Light,
    Humidity,
    Pressure,
    Custom(String),
}

/// Capabilities advertised by a device.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeviceCapability {
    Input,
    Output,
    Audio,
    Haptic,
    Storage,
    SensorReading,
    Network,
    Power,
    Custom(String),
}

/// Descriptor used to register devices with the HAL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDescriptor {
    pub name: String,
    pub kind: DeviceKind,
    pub register_count: usize,
    pub capabilities: HashSet<DeviceCapability>,
}

impl DeviceDescriptor {
    pub fn new(name: impl Into<String>, kind: DeviceKind, register_count: usize) -> Self {
        Self {
            name: name.into(),
            kind,
            register_count,
            capabilities: HashSet::new(),
        }
    }

    pub fn with_capabilities(
        mut self,
        capabilities: impl IntoIterator<Item = DeviceCapability>,
    ) -> Self {
        self.capabilities.extend(capabilities);
        self
    }
}

/// Runtime view of a registered device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub handle: DeviceHandle,
    pub descriptor: DeviceDescriptor,
}

/// Security policy invoked before HAL operations are executed.
pub trait HalSecurityPolicy: Send + Sync {
    fn authorize_registration(&self, descriptor: &DeviceDescriptor) -> Result<(), HalError>;
    fn notify_registration(&self, handle: &DeviceHandle, descriptor: &DeviceDescriptor);
    fn authorize_read(
        &self,
        handle: &DeviceHandle,
        descriptor: &DeviceDescriptor,
        register: usize,
    ) -> Result<(), HalError>;
    fn authorize_write(
        &self,
        handle: &DeviceHandle,
        descriptor: &DeviceDescriptor,
        register: usize,
        value: u32,
    ) -> Result<(), HalError>;
    fn authorize_interrupt(
        &self,
        handle: &DeviceHandle,
        descriptor: &DeviceDescriptor,
        irq: u32,
    ) -> Result<(), HalError>;
}

/// Default sandbox policy used by the software HAL when running in host mode.
pub struct SandboxSecurityPolicy {
    allowed: RwLock<HashSet<String>>,
    max_registers: usize,
}

impl SandboxSecurityPolicy {
    pub fn with_limit(max_registers: usize) -> Self {
        Self {
            allowed: RwLock::new(HashSet::new()),
            max_registers,
        }
    }

    fn ensure_allowed(&self, handle: &DeviceHandle) -> Result<(), HalError> {
        if self.allowed.read().contains(handle.as_str()) {
            Ok(())
        } else {
            Err(HalError::Driver(format!(
                "operation denied for handle {}",
                handle.as_str()
            )))
        }
    }
}

impl Default for SandboxSecurityPolicy {
    fn default() -> Self {
        Self::with_limit(256)
    }
}

impl HalSecurityPolicy for SandboxSecurityPolicy {
    fn authorize_registration(&self, descriptor: &DeviceDescriptor) -> Result<(), HalError> {
        if descriptor.register_count > self.max_registers {
            return Err(HalError::Driver(format!(
                "device '{}' exceeds sandbox register limit ({})",
                descriptor.name, self.max_registers
            )));
        }
        Ok(())
    }

    fn notify_registration(&self, handle: &DeviceHandle, _descriptor: &DeviceDescriptor) {
        self.allowed.write().insert(handle.as_str().to_string());
    }

    fn authorize_read(
        &self,
        handle: &DeviceHandle,
        _descriptor: &DeviceDescriptor,
        _register: usize,
    ) -> Result<(), HalError> {
        self.ensure_allowed(handle)
    }

    fn authorize_write(
        &self,
        handle: &DeviceHandle,
        descriptor: &DeviceDescriptor,
        _register: usize,
        _value: u32,
    ) -> Result<(), HalError> {
        self.ensure_allowed(handle)?;
        if matches!(descriptor.kind, DeviceKind::Sensor(_)) {
            return Err(HalError::Driver(format!(
                "sensor device '{}' is read-only in sandbox mode",
                descriptor.name
            )));
        }
        Ok(())
    }

    fn authorize_interrupt(
        &self,
        handle: &DeviceHandle,
        _descriptor: &DeviceDescriptor,
        _irq: u32,
    ) -> Result<(), HalError> {
        self.ensure_allowed(handle)
    }
}

/// Errors surfaced by the HAL.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HalError {
    #[error("device '{0}' already registered")]
    DuplicateDevice(String),
    #[error("device '{0}' not found")]
    UnknownDevice(String),
    #[error("register {register} out of range for device '{name}'")]
    RegisterOutOfRange { name: String, register: usize },
    #[error("{0}")]
    Driver(String),
}

impl From<DriverError> for HalError {
    fn from(value: DriverError) -> Self {
        match value {
            DriverError::AlreadyRegistered(name) => HalError::DuplicateDevice(name),
            DriverError::NotFound(name) => HalError::UnknownDevice(name),
            DriverError::RegisterOutOfRange { name, register } => {
                HalError::RegisterOutOfRange { name, register }
            }
        }
    }
}

/// Core interface implemented by all HAL backends.
pub trait HardwareAbstractionLayer: Send + Sync {
    /// Returns the architecture handled by the HAL.
    fn target_arch(&self) -> TargetArch;
    /// Provides access to runtime hooks for telemetry and logging.
    fn hooks(&self) -> Arc<RuntimeHooks>;
    /// Registers a device descriptor and returns the handle.
    fn register_device(&self, descriptor: DeviceDescriptor) -> Result<DeviceHandle, HalError>;
    /// Lists devices registered with the HAL, optionally filtered by kind.
    fn list_devices(&self, filter: Option<DeviceKind>) -> Vec<DeviceInfo>;
    /// Retrieves device metadata for the specified handle.
    fn device_info(&self, handle: &DeviceHandle) -> Option<DeviceInfo>;
    /// Reads a device register.
    fn read_register(&self, handle: &DeviceHandle, register: usize) -> Result<u32, HalError>;
    /// Writes a device register.
    fn write_register(
        &self,
        handle: &DeviceHandle,
        register: usize,
        value: u32,
    ) -> Result<(), HalError>;
    /// Raises an interrupt for the specified device.
    fn raise_interrupt(
        &self,
        handle: &DeviceHandle,
        irq: u32,
        payload: Option<u32>,
    ) -> Result<(), HalError>;
    /// Returns the driver registry used by the HAL.
    fn driver_registry(&self) -> DriverRegistry;
}

#[derive(Clone)]
struct DeviceRecord {
    descriptor: DeviceDescriptor,
}

/// Software-backed HAL that stores devices in memory and proxies register access
/// to the existing [`DriverRegistry`].
pub struct SoftwareHal {
    arch: TargetArch,
    hooks: Arc<RuntimeHooks>,
    drivers: DriverRegistry,
    devices: RwLock<HashMap<DeviceHandle, DeviceRecord>>,
    names: RwLock<HashMap<String, DeviceHandle>>,
    policy: Arc<dyn HalSecurityPolicy>,
}

impl SoftwareHal {
    /// Creates a new HAL instance backed by the software driver registry.
    pub fn new(arch: TargetArch, hooks: Arc<RuntimeHooks>) -> Self {
        Self::with_policy(
            arch,
            hooks,
            Arc::new(SandboxSecurityPolicy::with_limit(256)),
        )
    }

    /// Creates a new HAL with a custom security policy.
    pub fn with_policy(
        arch: TargetArch,
        hooks: Arc<RuntimeHooks>,
        policy: Arc<dyn HalSecurityPolicy>,
    ) -> Self {
        let drivers = DriverRegistry::new(hooks.clone());
        Self {
            arch,
            hooks,
            drivers,
            devices: RwLock::new(HashMap::new()),
            names: RwLock::new(HashMap::new()),
            policy,
        }
    }

    fn lookup_handle(&self, handle: &DeviceHandle) -> Result<(String, DeviceRecord), HalError> {
        let devices = self.devices.read();
        let Some(record) = devices.get(handle) else {
            return Err(HalError::UnknownDevice(handle.as_str().into()));
        };
        Ok((record.descriptor.name.clone(), record.clone()))
    }

    /// Registers the default virtual devices used by SolvraRuntime in sandboxed mode.
    pub fn register_builtin_devices(&self) -> Result<(), HalError> {
        for descriptor in devices::default_stub_descriptors() {
            match self.register_device(descriptor) {
                Ok(_) | Err(HalError::DuplicateDevice(_)) => continue,
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }
}

impl HardwareAbstractionLayer for SoftwareHal {
    fn target_arch(&self) -> TargetArch {
        self.arch
    }

    fn hooks(&self) -> Arc<RuntimeHooks> {
        self.hooks.clone()
    }

    fn register_device(&self, descriptor: DeviceDescriptor) -> Result<DeviceHandle, HalError> {
        let mut names = self.names.write();
        if names.contains_key(&descriptor.name) {
            return Err(HalError::DuplicateDevice(descriptor.name.clone()));
        }

        self.policy.authorize_registration(&descriptor)?;
        self.drivers
            .register_virtual_device(&descriptor.name, descriptor.register_count)
            .map_err(HalError::from)?;

        let handle = DeviceHandle::new();
        let mut devices = self.devices.write();
        devices.insert(
            handle.clone(),
            DeviceRecord {
                descriptor: descriptor.clone(),
            },
        );
        names.insert(descriptor.name.clone(), handle.clone());
        self.policy.notify_registration(&handle, &descriptor);
        Ok(handle)
    }

    fn list_devices(&self, filter: Option<DeviceKind>) -> Vec<DeviceInfo> {
        let devices = self.devices.read();
        devices
            .iter()
            .filter(|(_, record)| match &filter {
                Some(kind) => record.descriptor.kind == *kind,
                None => true,
            })
            .map(|(handle, record)| DeviceInfo {
                handle: handle.clone(),
                descriptor: record.descriptor.clone(),
            })
            .collect()
    }

    fn device_info(&self, handle: &DeviceHandle) -> Option<DeviceInfo> {
        let devices = self.devices.read();
        devices.get(handle).map(|record| DeviceInfo {
            handle: handle.clone(),
            descriptor: record.descriptor.clone(),
        })
    }

    fn read_register(&self, handle: &DeviceHandle, register: usize) -> Result<u32, HalError> {
        let (name, record) = self.lookup_handle(handle)?;
        self.policy
            .authorize_read(handle, &record.descriptor, register)?;
        self.drivers
            .read_register(&name, register)
            .map_err(HalError::from)
    }

    fn write_register(
        &self,
        handle: &DeviceHandle,
        register: usize,
        value: u32,
    ) -> Result<(), HalError> {
        let (name, record) = self.lookup_handle(handle)?;
        self.policy
            .authorize_write(handle, &record.descriptor, register, value)?;
        self.drivers
            .write_register(&name, register, value)
            .map_err(HalError::from)
    }

    fn raise_interrupt(
        &self,
        handle: &DeviceHandle,
        irq: u32,
        payload: Option<u32>,
    ) -> Result<(), HalError> {
        let (name, record) = self.lookup_handle(handle)?;
        self.policy
            .authorize_interrupt(handle, &record.descriptor, irq)?;
        self.drivers
            .trigger_interrupt(&name, Interrupt::new(irq, payload))
            .map_err(HalError::from)
    }

    fn driver_registry(&self) -> DriverRegistry {
        self.drivers.clone()
    }
}

/// Helpers to construct descriptors for builtin virtual devices.
pub mod devices {
    use super::*;

    pub fn default_stub_descriptors() -> Vec<DeviceDescriptor> {
        vec![
            keyboard_stub(),
            speaker_stub(),
            storage_stub(),
            temperature_sensor_stub(),
        ]
    }

    pub fn keyboard_stub() -> DeviceDescriptor {
        DeviceDescriptor::new("keyboard", DeviceKind::Keyboard, 8)
            .with_capabilities([DeviceCapability::Input])
    }

    pub fn speaker_stub() -> DeviceDescriptor {
        DeviceDescriptor::new("audio-out", DeviceKind::Speaker { external: true }, 16)
            .with_capabilities([DeviceCapability::Audio, DeviceCapability::Output])
    }

    pub fn storage_stub() -> DeviceDescriptor {
        DeviceDescriptor::new("storage0", DeviceKind::Storage(StorageBus::SdCard), 32)
            .with_capabilities([DeviceCapability::Storage])
    }

    pub fn temperature_sensor_stub() -> DeviceDescriptor {
        DeviceDescriptor::new(
            "sensor-temp",
            DeviceKind::Sensor(SensorKind::Temperature),
            4,
        )
        .with_capabilities([DeviceCapability::SensorReading])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::TargetArch;

    #[test]
    fn registers_and_queries_devices() {
        let hal = SoftwareHal::new(TargetArch::X86_64, Arc::new(RuntimeHooks::default()));
        hal.register_builtin_devices().expect("register stubs");
        let devices = hal.list_devices(Some(DeviceKind::Keyboard));
        assert_eq!(devices.len(), 1);
        let info = &devices[0];
        assert_eq!(info.descriptor.name, "keyboard");
        assert_eq!(info.descriptor.kind, DeviceKind::Keyboard);
        assert!(info
            .descriptor
            .capabilities
            .contains(&DeviceCapability::Input));
        let handle = info.handle.clone();

        hal.write_register(&handle, 0, 0xdead_beef)
            .expect("write register");
        let value = hal.read_register(&handle, 0).expect("read register");
        assert_eq!(value, 0xdead_beef);

        hal.raise_interrupt(&handle, 1, Some(0xFF))
            .expect("raise interrupt");

        let driver = hal.driver_registry();
        let interrupt = driver
            .next_interrupt("keyboard")
            .expect("driver not found")
            .expect("interrupt missing");
        assert_eq!(interrupt.irq, 1);
        assert_eq!(interrupt.payload, Some(0xFF));
    }

    #[test]
    fn prevents_duplicate_registration() {
        let hal = SoftwareHal::new(TargetArch::X86_64, Arc::new(RuntimeHooks::default()));
        let descriptor = DeviceDescriptor::new("gpu0", DeviceKind::Display, 8);
        hal.register_device(descriptor.clone()).expect("first ok");
        let err = hal.register_device(descriptor).expect_err("duplicate");
        assert_eq!(err, HalError::DuplicateDevice("gpu0".to_string()));
    }

    #[test]
    fn sandbox_blocks_sensor_writes() {
        let hal = SoftwareHal::new(TargetArch::X86_64, Arc::new(RuntimeHooks::default()));
        hal.register_builtin_devices().expect("register stubs");
        let sensor = hal
            .list_devices(Some(DeviceKind::Sensor(SensorKind::Temperature)))
            .pop()
            .expect("temperature sensor present");
        let err = hal
            .write_register(&sensor.handle, 0, 1)
            .expect_err("sensor write blocked");
        assert!(matches!(err, HalError::Driver(_)));
    }
}
