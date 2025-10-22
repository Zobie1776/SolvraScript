use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;

use parking_lot::RwLock;
use thiserror::Error;

use crate::integration::{RuntimeHooks, TelemetryEvent};

/// Descriptor used to register drivers from Rust host applications.
#[derive(Debug, Clone)]
pub struct DriverDescriptor {
    pub name: String,
    pub registers: Vec<u32>,
}

impl DriverDescriptor {
    pub fn new(name: impl Into<String>, registers: Vec<u32>) -> Self {
        Self {
            name: name.into(),
            registers,
        }
    }
}

/// Interrupt metadata surfaced to NovaCore drivers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interrupt {
    pub irq: u32,
    pub payload: Option<u32>,
}

impl Interrupt {
    pub fn new(irq: u32, payload: Option<u32>) -> Self {
        Self { irq, payload }
    }
}

#[derive(Default, Debug)]
struct Device {
    registers: Vec<u32>,
    interrupts: VecDeque<Interrupt>,
}

impl Device {
    fn with_registers(registers: Vec<u32>) -> Self {
        Self {
            registers,
            interrupts: VecDeque::new(),
        }
    }
}

/// Errors surfaced when interacting with the driver registry.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DriverError {
    #[error("driver {0} already registered")]
    AlreadyRegistered(String),
    #[error("driver {0} not found")]
    NotFound(String),
    #[error("register index {register} out of range for driver {name}")]
    RegisterOutOfRange { name: String, register: usize },
}

/// In-memory driver registry backing the NovaCore driver bindings.
#[derive(Clone)]
pub struct DriverRegistry {
    devices: Arc<RwLock<HashMap<String, Device>>>,
    hooks: Arc<RuntimeHooks>,
}

impl fmt::Debug for DriverRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let device_count = self.devices.read().len();
        f.debug_struct("DriverRegistry")
            .field("device_count", &device_count)
            .finish()
    }
}

impl DriverRegistry {
    pub fn new(hooks: Arc<RuntimeHooks>) -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            hooks,
        }
    }

    /// Registers a driver with zero-initialised registers.
    pub fn register_virtual_device(
        &self,
        name: impl Into<String>,
        registers: usize,
    ) -> Result<(), DriverError> {
        let name = name.into();
        let mut guard = self.devices.write();
        if guard.contains_key(&name) {
            return Err(DriverError::AlreadyRegistered(name));
        }
        guard.insert(name.clone(), Device::with_registers(vec![0; registers]));
        self.hooks
            .emit_telemetry(TelemetryEvent::DriverRegistered { name, registers });
        Ok(())
    }

    /// Registers a driver using a descriptor provided by host code.
    pub fn register_descriptor(&self, descriptor: DriverDescriptor) -> Result<(), DriverError> {
        let registers = descriptor.registers.len();
        let mut guard = self.devices.write();
        if guard.contains_key(&descriptor.name) {
            return Err(DriverError::AlreadyRegistered(descriptor.name));
        }
        guard.insert(
            descriptor.name.clone(),
            Device::with_registers(descriptor.registers),
        );
        self.hooks.emit_telemetry(TelemetryEvent::DriverRegistered {
            name: descriptor.name,
            registers,
        });
        Ok(())
    }

    /// Reads a register value.
    pub fn read_register(&self, name: &str, register: usize) -> Result<u32, DriverError> {
        let guard = self.devices.read();
        let Some(device) = guard.get(name) else {
            return Err(DriverError::NotFound(name.into()));
        };
        device
            .registers
            .get(register)
            .copied()
            .ok_or_else(|| DriverError::RegisterOutOfRange {
                name: name.into(),
                register,
            })
    }

    /// Writes a register value.
    pub fn write_register(
        &self,
        name: &str,
        register: usize,
        value: u32,
    ) -> Result<(), DriverError> {
        let mut guard = self.devices.write();
        let Some(device) = guard.get_mut(name) else {
            return Err(DriverError::NotFound(name.into()));
        };
        if let Some(slot) = device.registers.get_mut(register) {
            *slot = value;
            self.hooks.emit_telemetry(TelemetryEvent::RegisterWrite {
                name: name.into(),
                register,
                value,
            });
            Ok(())
        } else {
            Err(DriverError::RegisterOutOfRange {
                name: name.into(),
                register,
            })
        }
    }

    /// Queues an interrupt for the specified driver.
    pub fn trigger_interrupt(&self, name: &str, interrupt: Interrupt) -> Result<(), DriverError> {
        let mut guard = self.devices.write();
        let Some(device) = guard.get_mut(name) else {
            return Err(DriverError::NotFound(name.into()));
        };
        self.hooks.emit_telemetry(TelemetryEvent::InterruptRaised {
            name: name.into(),
            irq: interrupt.irq,
            payload: interrupt.payload,
        });
        device.interrupts.push_back(interrupt);
        Ok(())
    }

    /// Pops the next pending interrupt, if any.
    pub fn next_interrupt(&self, name: &str) -> Result<Option<Interrupt>, DriverError> {
        let mut guard = self.devices.write();
        let Some(device) = guard.get_mut(name) else {
            return Err(DriverError::NotFound(name.into()));
        };
        Ok(device.interrupts.pop_front())
    }
}
