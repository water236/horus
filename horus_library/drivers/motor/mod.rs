//! Motor/Actuator drivers
//!
//! This module provides drivers for motor actuators.
//!
//! # Available Drivers
//!
//! - `SimulationMotorDriver` - Always available, simulates motor behavior
//! - `GpioMotorDriver` - GPIO-based DC motor control (requires `gpio-hardware` feature)

mod simulation;

#[cfg(feature = "gpio-hardware")]
mod gpio;

pub use simulation::SimulationMotorDriver;

#[cfg(feature = "gpio-hardware")]
pub use gpio::GpioMotorDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::MotorCommand;

/// Motor driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum MotorDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "gpio-hardware")]
    Gpio,
}

/// Type-erased motor driver
pub enum MotorDriver {
    Simulation(SimulationMotorDriver),
    #[cfg(feature = "gpio-hardware")]
    Gpio(GpioMotorDriver),
}

impl MotorDriver {
    pub fn new(backend: MotorDriverBackend) -> HorusResult<Self> {
        match backend {
            MotorDriverBackend::Simulation => Ok(Self::Simulation(SimulationMotorDriver::new())),
            #[cfg(feature = "gpio-hardware")]
            MotorDriverBackend::Gpio => Ok(Self::Gpio(GpioMotorDriver::new()?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationMotorDriver::new())
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.status(),
        }
    }

    // ========================================================================
    // Actuator methods
    // ========================================================================

    pub fn write(&mut self, cmd: MotorCommand) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write(cmd),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.write(cmd),
        }
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.stop(),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.stop(),
        }
    }
}
