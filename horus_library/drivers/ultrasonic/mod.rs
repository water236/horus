//! Ultrasonic Range Sensor drivers
//!
//! This module provides drivers for ultrasonic distance sensors.
//!
//! # Available Drivers
//!
//! - `SimulationUltrasonicDriver` - Always available, generates synthetic data
//! - `GpioUltrasonicDriver` - GPIO echo/trigger driver (requires `gpio-hardware` feature)

mod simulation;

#[cfg(feature = "gpio-hardware")]
mod gpio;

// Re-exports
pub use simulation::SimulationUltrasonicDriver;

#[cfg(feature = "gpio-hardware")]
pub use gpio::GpioUltrasonicDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::Range;

/// Enum of all available ultrasonic driver backends
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum UltrasonicDriverBackend {
    /// Simulation driver (always available)
    #[default]
    Simulation,
    /// GPIO echo/trigger driver
    #[cfg(feature = "gpio-hardware")]
    Gpio,
}

/// Type-erased ultrasonic driver for runtime backend selection
pub enum UltrasonicDriver {
    Simulation(SimulationUltrasonicDriver),
    #[cfg(feature = "gpio-hardware")]
    Gpio(GpioUltrasonicDriver),
}

impl UltrasonicDriver {
    /// Create a new ultrasonic driver with the specified backend
    pub fn new(backend: UltrasonicDriverBackend) -> HorusResult<Self> {
        match backend {
            UltrasonicDriverBackend::Simulation => {
                Ok(Self::Simulation(SimulationUltrasonicDriver::new()))
            }
            #[cfg(feature = "gpio-hardware")]
            UltrasonicDriverBackend::Gpio => Ok(Self::Gpio(GpioUltrasonicDriver::new()?)),
        }
    }

    /// Create a simulation driver (always available)
    pub fn simulation() -> Self {
        Self::Simulation(SimulationUltrasonicDriver::new())
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
    // Sensor methods
    // ========================================================================

    pub fn read(&mut self) -> HorusResult<Range> {
        match self {
            Self::Simulation(d) => d.read(),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.read(),
        }
    }

    pub fn has_data(&self) -> bool {
        match self {
            Self::Simulation(d) => d.has_data(),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.has_data(),
        }
    }

    pub fn sample_rate(&self) -> Option<f32> {
        match self {
            Self::Simulation(d) => d.sample_rate(),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.sample_rate(),
        }
    }
}
