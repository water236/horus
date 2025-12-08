//! Serial port drivers
//!
//! This module provides drivers for serial port (UART) communication.
//!
//! # Available Drivers
//!
//! - `SimulationSerialDriver` - Always available, simulates serial port
//! - `SystemSerialDriver` - System serial port (requires `serial-hardware` feature)

mod simulation;

#[cfg(feature = "serial-hardware")]
mod system;

pub use simulation::SimulationSerialDriver;

#[cfg(feature = "serial-hardware")]
pub use system::SystemSerialDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

/// Serial port configuration
#[derive(Debug, Clone)]
pub struct SerialConfig {
    /// Serial port path (e.g., "/dev/ttyUSB0")
    pub port: String,
    /// Baud rate
    pub baud_rate: u32,
    /// Data bits (5, 6, 7, 8)
    pub data_bits: u8,
    /// Stop bits (1, 2)
    pub stop_bits: u8,
    /// Parity (None, Even, Odd)
    pub parity: SerialParity,
    /// Flow control
    pub flow_control: SerialFlowControl,
    /// Read timeout in milliseconds (0 = blocking)
    pub timeout_ms: u64,
}

/// Serial parity
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SerialParity {
    #[default]
    None,
    Even,
    Odd,
}

/// Serial flow control
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SerialFlowControl {
    #[default]
    None,
    Hardware,
    Software,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            data_bits: 8,
            stop_bits: 1,
            parity: SerialParity::None,
            flow_control: SerialFlowControl::None,
            timeout_ms: 1000,
        }
    }
}

/// Serial driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SerialDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "serial-hardware")]
    System,
}

/// Type-erased serial driver
pub enum SerialDriver {
    Simulation(SimulationSerialDriver),
    #[cfg(feature = "serial-hardware")]
    System(SystemSerialDriver),
}

impl SerialDriver {
    pub fn new(backend: SerialDriverBackend, config: SerialConfig) -> HorusResult<Self> {
        match backend {
            SerialDriverBackend::Simulation => {
                Ok(Self::Simulation(SimulationSerialDriver::new(config)))
            }
            #[cfg(feature = "serial-hardware")]
            SerialDriverBackend::System => Ok(Self::System(SystemSerialDriver::new(config)?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationSerialDriver::new(SerialConfig::default()))
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "serial-hardware")]
            Self::System(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "serial-hardware")]
            Self::System(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "serial-hardware")]
            Self::System(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "serial-hardware")]
            Self::System(d) => d.status(),
        }
    }

    // ========================================================================
    // Bus methods
    // ========================================================================

    pub fn read_bytes(&mut self, len: usize) -> HorusResult<Vec<u8>> {
        match self {
            Self::Simulation(d) => d.read_bytes(len),
            #[cfg(feature = "serial-hardware")]
            Self::System(d) => d.read_bytes(len),
        }
    }

    pub fn write_bytes(&mut self, data: &[u8]) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write_bytes(data),
            #[cfg(feature = "serial-hardware")]
            Self::System(d) => d.write_bytes(data),
        }
    }
}
