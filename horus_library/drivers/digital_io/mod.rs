//! Digital I/O drivers
//!
//! This module provides drivers for digital GPIO input/output.
//!
//! # Available Drivers
//!
//! - `SimulationDigitalIoDriver` - Always available, simulates GPIO
//! - `GpioDigitalIoDriver` - Linux sysfs GPIO (requires `gpio-hardware` feature)

mod simulation;

#[cfg(feature = "gpio-hardware")]
mod gpio;

pub use simulation::SimulationDigitalIoDriver;

#[cfg(feature = "gpio-hardware")]
pub use gpio::GpioDigitalIoDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

/// Digital I/O pin mode
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PinMode {
    #[default]
    Input,
    Output,
    InputPullUp,
    InputPullDown,
}

/// Digital I/O pin configuration
#[derive(Debug, Clone)]
pub struct DigitalIoPin {
    /// GPIO pin number
    pub pin: u64,
    /// Pin mode
    pub mode: PinMode,
    /// Initial value for outputs
    pub initial_value: bool,
    /// Invert logic
    pub inverted: bool,
}

impl Default for DigitalIoPin {
    fn default() -> Self {
        Self {
            pin: 0,
            mode: PinMode::Input,
            initial_value: false,
            inverted: false,
        }
    }
}

/// Digital I/O configuration
#[derive(Debug, Clone, Default)]
pub struct DigitalIoConfig {
    /// Configured pins
    pub pins: Vec<DigitalIoPin>,
}

impl DigitalIoConfig {
    pub fn new() -> Self {
        Self { pins: Vec::new() }
    }

    pub fn add_input(mut self, pin: u64) -> Self {
        self.pins.push(DigitalIoPin {
            pin,
            mode: PinMode::Input,
            ..Default::default()
        });
        self
    }

    pub fn add_output(mut self, pin: u64, initial: bool) -> Self {
        self.pins.push(DigitalIoPin {
            pin,
            mode: PinMode::Output,
            initial_value: initial,
            ..Default::default()
        });
        self
    }
}

/// Digital I/O driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DigitalIoDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "gpio-hardware")]
    Gpio,
}

/// Type-erased digital I/O driver
pub enum DigitalIoDriver {
    Simulation(SimulationDigitalIoDriver),
    #[cfg(feature = "gpio-hardware")]
    Gpio(GpioDigitalIoDriver),
}

impl DigitalIoDriver {
    pub fn new(backend: DigitalIoDriverBackend, config: DigitalIoConfig) -> HorusResult<Self> {
        match backend {
            DigitalIoDriverBackend::Simulation => {
                Ok(Self::Simulation(SimulationDigitalIoDriver::new(config)))
            }
            #[cfg(feature = "gpio-hardware")]
            DigitalIoDriverBackend::Gpio => Ok(Self::Gpio(GpioDigitalIoDriver::new(config)?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationDigitalIoDriver::new(DigitalIoConfig::default()))
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
    // Digital I/O methods
    // ========================================================================

    /// Read a digital input pin
    pub fn read_pin(&mut self, pin: u64) -> HorusResult<bool> {
        match self {
            Self::Simulation(d) => d.read_pin(pin),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.read_pin(pin),
        }
    }

    /// Write to a digital output pin
    pub fn write_pin(&mut self, pin: u64, value: bool) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write_pin(pin, value),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.write_pin(pin, value),
        }
    }

    /// Read all configured inputs
    pub fn read_all(&mut self) -> HorusResult<Vec<(u64, bool)>> {
        match self {
            Self::Simulation(d) => d.read_all(),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.read_all(),
        }
    }

    /// Set pin mode
    pub fn set_mode(&mut self, pin: u64, mode: PinMode) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.set_mode(pin, mode),
            #[cfg(feature = "gpio-hardware")]
            Self::Gpio(d) => d.set_mode(pin, mode),
        }
    }
}
