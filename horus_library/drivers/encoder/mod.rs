//! Encoder drivers
//!
//! This module provides drivers for rotary encoders.

mod simulation;

#[cfg(feature = "gpio-hardware")]
mod gpio;

pub use simulation::SimulationEncoderDriver;

#[cfg(feature = "gpio-hardware")]
pub use gpio::GpioEncoderDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::Odometry;

/// Encoder driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum EncoderDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "gpio-hardware")]
    Gpio,
}

/// Type-erased encoder driver
pub enum EncoderDriver {
    Simulation(SimulationEncoderDriver),
    #[cfg(feature = "gpio-hardware")]
    Gpio(GpioEncoderDriver),
}

impl EncoderDriver {
    pub fn new(backend: EncoderDriverBackend) -> HorusResult<Self> {
        match backend {
            EncoderDriverBackend::Simulation => {
                Ok(Self::Simulation(SimulationEncoderDriver::new()))
            }
            #[cfg(feature = "gpio-hardware")]
            EncoderDriverBackend::Gpio => Ok(Self::Gpio(GpioEncoderDriver::new()?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationEncoderDriver::new())
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

    pub fn read(&mut self) -> HorusResult<Odometry> {
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
