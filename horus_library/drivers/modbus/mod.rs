//! Modbus protocol drivers
//!
//! This module provides drivers for Modbus RTU/TCP communication.
//!
//! # Available Drivers
//!
//! - `SimulationModbusDriver` - Always available, simulates Modbus behavior
//! - `RtuModbusDriver` - Modbus RTU over serial (requires `serial-hardware` feature)

mod simulation;

#[cfg(feature = "serial-hardware")]
mod rtu;

pub use simulation::SimulationModbusDriver;

#[cfg(feature = "serial-hardware")]
pub use rtu::RtuModbusDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

/// Modbus configuration
#[derive(Debug, Clone)]
pub struct ModbusConfig {
    /// Serial port path (for RTU)
    pub port: String,
    /// Baud rate (for RTU)
    pub baud_rate: u32,
    /// Slave address
    pub slave_id: u8,
    /// Response timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for ModbusConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 9600,
            slave_id: 1,
            timeout_ms: 1000,
        }
    }
}

/// Modbus driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ModbusDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "serial-hardware")]
    Rtu,
}

/// Type-erased Modbus driver
pub enum ModbusDriver {
    Simulation(SimulationModbusDriver),
    #[cfg(feature = "serial-hardware")]
    Rtu(RtuModbusDriver),
}

impl ModbusDriver {
    pub fn new(backend: ModbusDriverBackend, config: ModbusConfig) -> HorusResult<Self> {
        match backend {
            ModbusDriverBackend::Simulation => {
                Ok(Self::Simulation(SimulationModbusDriver::new(config)))
            }
            #[cfg(feature = "serial-hardware")]
            ModbusDriverBackend::Rtu => Ok(Self::Rtu(RtuModbusDriver::new(config)?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationModbusDriver::new(ModbusConfig::default()))
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.status(),
        }
    }

    // ========================================================================
    // Bus methods
    // ========================================================================

    pub fn read_bytes(&mut self, addr: u16, len: usize) -> HorusResult<Vec<u8>> {
        match self {
            Self::Simulation(d) => d.read_bytes(addr, len),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.read_bytes(addr, len),
        }
    }

    pub fn write_bytes(&mut self, addr: u16, data: &[u8]) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write_bytes(addr, data),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.write_bytes(addr, data),
        }
    }

    // ========================================================================
    // Modbus-specific methods
    // ========================================================================

    /// Read holding registers (function code 0x03)
    pub fn read_holding_registers(&mut self, addr: u16, count: u16) -> HorusResult<Vec<u16>> {
        match self {
            Self::Simulation(d) => d.read_holding_registers(addr, count),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.read_holding_registers(addr, count),
        }
    }

    /// Read input registers (function code 0x04)
    pub fn read_input_registers(&mut self, addr: u16, count: u16) -> HorusResult<Vec<u16>> {
        match self {
            Self::Simulation(d) => d.read_input_registers(addr, count),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.read_input_registers(addr, count),
        }
    }

    /// Write single register (function code 0x06)
    pub fn write_single_register(&mut self, addr: u16, value: u16) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write_single_register(addr, value),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.write_single_register(addr, value),
        }
    }

    /// Write multiple registers (function code 0x10)
    pub fn write_multiple_registers(&mut self, addr: u16, values: &[u16]) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write_multiple_registers(addr, values),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.write_multiple_registers(addr, values),
        }
    }

    /// Read coils (function code 0x01)
    pub fn read_coils(&mut self, addr: u16, count: u16) -> HorusResult<Vec<bool>> {
        match self {
            Self::Simulation(d) => d.read_coils(addr, count),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.read_coils(addr, count),
        }
    }

    /// Write single coil (function code 0x05)
    pub fn write_single_coil(&mut self, addr: u16, value: bool) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write_single_coil(addr, value),
            #[cfg(feature = "serial-hardware")]
            Self::Rtu(d) => d.write_single_coil(addr, value),
        }
    }
}
