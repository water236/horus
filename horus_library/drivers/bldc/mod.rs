//! BLDC (Brushless DC) Motor drivers
//!
//! This module provides drivers for BLDC motors via ESC (Electronic Speed Controller).
//! Supports multiple ESC protocols including PWM, DShot, and CAN.
//!
//! # Available Drivers
//!
//! - `SimulationBldcDriver` - Always available, simulates BLDC motor behavior
//! - `PwmBldcDriver` - Hardware PWM-based ESC control (requires `gpio-hardware` feature)

mod simulation;

#[cfg(feature = "gpio-hardware")]
mod pwm;

pub use simulation::SimulationBldcDriver;

#[cfg(feature = "gpio-hardware")]
pub use pwm::PwmBldcDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

/// BLDC motor command
#[derive(Debug, Clone, Copy, Default)]
pub struct BldcCommand {
    /// Motor ID (0-7)
    pub motor_id: u8,
    /// Throttle value (0.0 to 1.0)
    pub throttle: f64,
    /// Direction (true = forward, false = reverse)
    pub direction: bool,
    /// Armed state
    pub armed: bool,
}

impl BldcCommand {
    pub fn new(motor_id: u8, throttle: f64, direction: bool, armed: bool) -> Self {
        Self {
            motor_id,
            throttle: throttle.clamp(0.0, 1.0),
            direction,
            armed,
        }
    }

    pub fn stop(motor_id: u8) -> Self {
        Self {
            motor_id,
            throttle: 0.0,
            direction: true,
            armed: false,
        }
    }
}

/// ESC Communication Protocol
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BldcProtocol {
    /// Standard PWM (1000-2000μs, 50Hz update rate)
    #[default]
    StandardPwm,
    /// OneShot125 (125-250μs, up to 4kHz)
    OneShot125,
    /// OneShot42 (42-84μs, up to 12kHz)
    OneShot42,
    /// DShot150 (150kbit/s digital)
    DShot150,
    /// DShot300 (300kbit/s digital)
    DShot300,
    /// DShot600 (600kbit/s digital)
    DShot600,
    /// DShot1200 (1200kbit/s digital)
    DShot1200,
}

/// BLDC driver configuration
#[derive(Debug, Clone)]
pub struct BldcConfig {
    /// Number of motors (1-8)
    pub num_motors: u8,
    /// ESC protocol
    pub protocol: BldcProtocol,
    /// PWM frequency in Hz (for PWM protocols)
    pub pwm_frequency_hz: f64,
    /// Minimum PWM pulse width in microseconds
    pub pwm_min_us: u16,
    /// Maximum PWM pulse width in microseconds
    pub pwm_max_us: u16,
    /// GPIO pins for each motor (for hardware drivers)
    pub gpio_pins: [u8; 8],
}

impl Default for BldcConfig {
    fn default() -> Self {
        Self {
            num_motors: 1,
            protocol: BldcProtocol::StandardPwm,
            pwm_frequency_hz: 50.0,
            pwm_min_us: 1000,
            pwm_max_us: 2000,
            gpio_pins: [0; 8],
        }
    }
}

/// BLDC driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BldcDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "gpio-hardware")]
    Pwm,
}

/// Type-erased BLDC driver
pub enum BldcDriver {
    Simulation(SimulationBldcDriver),
    #[cfg(feature = "gpio-hardware")]
    Pwm(PwmBldcDriver),
}

impl BldcDriver {
    pub fn new(backend: BldcDriverBackend, config: BldcConfig) -> HorusResult<Self> {
        match backend {
            BldcDriverBackend::Simulation => {
                Ok(Self::Simulation(SimulationBldcDriver::new(config)))
            }
            #[cfg(feature = "gpio-hardware")]
            BldcDriverBackend::Pwm => Ok(Self::Pwm(PwmBldcDriver::new(config)?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationBldcDriver::new(BldcConfig::default()))
    }

    pub fn with_config(config: BldcConfig) -> Self {
        Self::Simulation(SimulationBldcDriver::new(config))
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "gpio-hardware")]
            Self::Pwm(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "gpio-hardware")]
            Self::Pwm(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "gpio-hardware")]
            Self::Pwm(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "gpio-hardware")]
            Self::Pwm(d) => d.status(),
        }
    }

    // ========================================================================
    // Actuator methods
    // ========================================================================

    pub fn write(&mut self, cmd: BldcCommand) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write(cmd),
            #[cfg(feature = "gpio-hardware")]
            Self::Pwm(d) => d.write(cmd),
        }
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.stop(),
            #[cfg(feature = "gpio-hardware")]
            Self::Pwm(d) => d.stop(),
        }
    }
}
