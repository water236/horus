//! Bus drivers (I2C, SPI, CAN)
//!
//! This module provides drivers for communication buses.
//!
//! # Available Drivers
//!
//! ## I2C
//! - `SimulationI2cDriver` - Always available, simulates I2C bus
//! - `LinuxI2cDriver` - Linux i2cdev interface (requires `i2c-hardware` feature)
//!
//! ## SPI
//! - `SimulationSpiDriver` - Always available, simulates SPI bus
//! - `LinuxSpiDriver` - Linux spidev interface (requires `spi-hardware` feature)
//!
//! ## CAN
//! - `SimulationCanDriver` - Always available, simulates CAN bus
//! - `SocketCanDriver` - Linux SocketCAN interface (requires `can-hardware` feature)

mod simulation;

#[cfg(feature = "i2c-hardware")]
mod linux_i2c;

#[cfg(feature = "spi-hardware")]
mod linux_spi;

#[cfg(feature = "can-hardware")]
mod socketcan;

pub use simulation::{SimulationCanDriver, SimulationI2cDriver, SimulationSpiDriver};

#[cfg(feature = "i2c-hardware")]
pub use linux_i2c::LinuxI2cDriver;

#[cfg(feature = "spi-hardware")]
pub use linux_spi::LinuxSpiDriver;

#[cfg(feature = "can-hardware")]
pub use socketcan::SocketCanDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

// ============================================================================
// I2C Driver
// ============================================================================

/// I2C driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum I2cDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "i2c-hardware")]
    Linux,
}

/// Type-erased I2C driver
pub enum I2cDriver {
    Simulation(SimulationI2cDriver),
    #[cfg(feature = "i2c-hardware")]
    Linux(LinuxI2cDriver),
}

impl I2cDriver {
    pub fn new(backend: I2cDriverBackend) -> HorusResult<Self> {
        match backend {
            I2cDriverBackend::Simulation => Ok(Self::Simulation(SimulationI2cDriver::new())),
            #[cfg(feature = "i2c-hardware")]
            I2cDriverBackend::Linux => Ok(Self::Linux(LinuxI2cDriver::new()?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationI2cDriver::new())
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "i2c-hardware")]
            Self::Linux(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "i2c-hardware")]
            Self::Linux(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "i2c-hardware")]
            Self::Linux(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "i2c-hardware")]
            Self::Linux(d) => d.status(),
        }
    }

    // ========================================================================
    // Bus methods
    // ========================================================================

    pub fn read_bytes(&mut self, addr: u16, len: usize) -> HorusResult<Vec<u8>> {
        match self {
            Self::Simulation(d) => d.read_bytes(addr, len),
            #[cfg(feature = "i2c-hardware")]
            Self::Linux(d) => d.read_bytes(addr, len),
        }
    }

    pub fn write_bytes(&mut self, addr: u16, data: &[u8]) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write_bytes(addr, data),
            #[cfg(feature = "i2c-hardware")]
            Self::Linux(d) => d.write_bytes(addr, data),
        }
    }
}

// ============================================================================
// SPI Driver
// ============================================================================

/// SPI driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SpiDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "spi-hardware")]
    Linux,
}

/// Type-erased SPI driver
pub enum SpiDriver {
    Simulation(SimulationSpiDriver),
    #[cfg(feature = "spi-hardware")]
    Linux(LinuxSpiDriver),
}

impl SpiDriver {
    pub fn new(backend: SpiDriverBackend) -> HorusResult<Self> {
        match backend {
            SpiDriverBackend::Simulation => Ok(Self::Simulation(SimulationSpiDriver::new())),
            #[cfg(feature = "spi-hardware")]
            SpiDriverBackend::Linux => Ok(Self::Linux(LinuxSpiDriver::new()?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationSpiDriver::new())
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "spi-hardware")]
            Self::Linux(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "spi-hardware")]
            Self::Linux(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "spi-hardware")]
            Self::Linux(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "spi-hardware")]
            Self::Linux(d) => d.status(),
        }
    }

    // ========================================================================
    // Bus methods
    // ========================================================================

    pub fn read_bytes(&mut self, addr: u16, len: usize) -> HorusResult<Vec<u8>> {
        match self {
            Self::Simulation(d) => d.read_bytes(addr, len),
            #[cfg(feature = "spi-hardware")]
            Self::Linux(d) => d.read_bytes(addr, len),
        }
    }

    pub fn write_bytes(&mut self, addr: u16, data: &[u8]) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write_bytes(addr, data),
            #[cfg(feature = "spi-hardware")]
            Self::Linux(d) => d.write_bytes(addr, data),
        }
    }
}

// ============================================================================
// CAN Driver
// ============================================================================

/// CAN driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CanDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "can-hardware")]
    SocketCan,
}

/// Type-erased CAN driver
pub enum CanDriver {
    Simulation(SimulationCanDriver),
    #[cfg(feature = "can-hardware")]
    SocketCan(SocketCanDriver),
}

impl CanDriver {
    pub fn new(backend: CanDriverBackend) -> HorusResult<Self> {
        match backend {
            CanDriverBackend::Simulation => Ok(Self::Simulation(SimulationCanDriver::new())),
            #[cfg(feature = "can-hardware")]
            CanDriverBackend::SocketCan => Ok(Self::SocketCan(SocketCanDriver::new()?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationCanDriver::new())
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "can-hardware")]
            Self::SocketCan(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "can-hardware")]
            Self::SocketCan(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "can-hardware")]
            Self::SocketCan(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "can-hardware")]
            Self::SocketCan(d) => d.status(),
        }
    }

    // ========================================================================
    // Bus methods
    // ========================================================================

    pub fn read_bytes(&mut self, addr: u16, len: usize) -> HorusResult<Vec<u8>> {
        match self {
            Self::Simulation(d) => d.read_bytes(addr, len),
            #[cfg(feature = "can-hardware")]
            Self::SocketCan(d) => d.read_bytes(addr, len),
        }
    }

    pub fn write_bytes(&mut self, addr: u16, data: &[u8]) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write_bytes(addr, data),
            #[cfg(feature = "can-hardware")]
            Self::SocketCan(d) => d.write_bytes(addr, data),
        }
    }
}
