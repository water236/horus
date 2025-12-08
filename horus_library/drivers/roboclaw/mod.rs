//! RoboClaw motor controller drivers
//!
//! This module provides drivers for RoboClaw motor controllers.
//!
//! # Available Drivers
//!
//! - `SimulationRoboclawDriver` - Always available, simulates RoboClaw behavior
//! - `SerialRoboclawDriver` - Serial protocol (requires `serial-hardware` feature)

mod simulation;

#[cfg(feature = "serial-hardware")]
mod serial;

pub use simulation::SimulationRoboclawDriver;

#[cfg(feature = "serial-hardware")]
pub use serial::SerialRoboclawDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

/// RoboClaw motor command
#[derive(Debug, Clone, Copy, Default)]
pub struct RoboclawCommand {
    /// Motor channel (1 or 2)
    pub channel: u8,
    /// Control mode
    pub mode: RoboclawMode,
    /// Target value (duty -1.0 to 1.0, velocity in encoder counts/sec, position in encoder counts)
    pub target: f64,
    /// Acceleration (for velocity/position modes)
    pub acceleration: u32,
}

impl RoboclawCommand {
    pub fn duty(channel: u8, duty: f64) -> Self {
        Self {
            channel,
            mode: RoboclawMode::Duty,
            target: duty.clamp(-1.0, 1.0),
            acceleration: 0,
        }
    }

    pub fn velocity(channel: u8, counts_per_sec: f64, acceleration: u32) -> Self {
        Self {
            channel,
            mode: RoboclawMode::Velocity,
            target: counts_per_sec,
            acceleration,
        }
    }

    pub fn position(channel: u8, position: i64, _velocity: u32, acceleration: u32) -> Self {
        Self {
            channel,
            mode: RoboclawMode::Position,
            target: position as f64,
            acceleration,
        }
    }

    pub fn stop(channel: u8) -> Self {
        Self {
            channel,
            mode: RoboclawMode::Duty,
            target: 0.0,
            acceleration: 0,
        }
    }
}

/// RoboClaw control mode
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RoboclawMode {
    #[default]
    Duty,
    Velocity,
    Position,
}

/// RoboClaw configuration
#[derive(Debug, Clone)]
pub struct RoboclawConfig {
    /// Serial port path
    pub port: String,
    /// Baud rate
    pub baud_rate: u32,
    /// Device address (0x80 default)
    pub address: u8,
    /// Encoder counts per revolution (for position mode)
    pub counts_per_rev: u32,
}

impl Default for RoboclawConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyACM0".to_string(),
            baud_rate: 115200,
            address: 0x80,
            counts_per_rev: 4096,
        }
    }
}

/// RoboClaw driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RoboclawDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "serial-hardware")]
    Serial,
}

/// Type-erased RoboClaw driver
pub enum RoboclawDriver {
    Simulation(SimulationRoboclawDriver),
    #[cfg(feature = "serial-hardware")]
    Serial(SerialRoboclawDriver),
}

impl RoboclawDriver {
    pub fn new(backend: RoboclawDriverBackend, config: RoboclawConfig) -> HorusResult<Self> {
        match backend {
            RoboclawDriverBackend::Simulation => {
                Ok(Self::Simulation(SimulationRoboclawDriver::new(config)))
            }
            #[cfg(feature = "serial-hardware")]
            RoboclawDriverBackend::Serial => Ok(Self::Serial(SerialRoboclawDriver::new(config)?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationRoboclawDriver::new(RoboclawConfig::default()))
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.status(),
        }
    }

    // ========================================================================
    // Actuator methods
    // ========================================================================

    pub fn write(&mut self, cmd: RoboclawCommand) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write(cmd),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.write(cmd),
        }
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.stop(),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.stop(),
        }
    }
}
