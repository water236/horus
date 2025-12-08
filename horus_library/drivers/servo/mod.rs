//! Servo drivers
//!
//! This module provides drivers for servo actuators.
//!
//! # Available Drivers
//!
//! - `SimulationServoDriver` - Always available, simulates servo behavior
//! - `Pca9685ServoDriver` - PCA9685 PWM controller (requires `i2c-hardware` feature)

mod simulation;

#[cfg(feature = "i2c-hardware")]
mod pca9685;

pub use simulation::SimulationServoDriver;

#[cfg(feature = "i2c-hardware")]
pub use pca9685::Pca9685ServoDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::ServoCommand;

/// Servo driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ServoDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "i2c-hardware")]
    Pca9685,
}

/// Type-erased servo driver
pub enum ServoDriver {
    Simulation(SimulationServoDriver),
    #[cfg(feature = "i2c-hardware")]
    Pca9685(Pca9685ServoDriver),
}

impl ServoDriver {
    pub fn new(backend: ServoDriverBackend) -> HorusResult<Self> {
        match backend {
            ServoDriverBackend::Simulation => Ok(Self::Simulation(SimulationServoDriver::new())),
            #[cfg(feature = "i2c-hardware")]
            ServoDriverBackend::Pca9685 => Ok(Self::Pca9685(Pca9685ServoDriver::new()?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationServoDriver::new())
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "i2c-hardware")]
            Self::Pca9685(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "i2c-hardware")]
            Self::Pca9685(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "i2c-hardware")]
            Self::Pca9685(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "i2c-hardware")]
            Self::Pca9685(d) => d.status(),
        }
    }

    // ========================================================================
    // Actuator methods
    // ========================================================================

    pub fn write(&mut self, cmd: ServoCommand) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write(cmd),
            #[cfg(feature = "i2c-hardware")]
            Self::Pca9685(d) => d.write(cmd),
        }
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.stop(),
            #[cfg(feature = "i2c-hardware")]
            Self::Pca9685(d) => d.stop(),
        }
    }
}
