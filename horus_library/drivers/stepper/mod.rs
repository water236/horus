//! Stepper Motor drivers
//!
//! This module provides drivers for stepper motors using step/direction interface.
//! Compatible with common stepper drivers (A4988, DRV8825, TMC2208, TMC2209, etc.).
//!
//! # Available Drivers
//!
//! - `SimulationStepperDriver` - Always available, simulates stepper behavior
//! - `GpioStepperDriver` - GPIO step/dir interface (requires `gpio-hardware` feature)

mod simulation;

#[cfg(feature = "gpio-hardware")]
mod gpio;

pub use simulation::SimulationStepperDriver;

#[cfg(feature = "gpio-hardware")]
pub use gpio::GpioStepperDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

/// Stepper motor command
#[derive(Debug, Clone, Copy, Default)]
pub struct StepperDriverCommand {
    /// Motor ID (0-7)
    pub motor_id: u8,
    /// Number of steps to move (positive = forward, negative = reverse)
    pub steps: i64,
    /// Target velocity in steps/second
    pub velocity: f64,
    /// Enable motor
    pub enable: bool,
}

impl StepperDriverCommand {
    pub fn new(motor_id: u8, steps: i64, velocity: f64) -> Self {
        Self {
            motor_id,
            steps,
            velocity: velocity.abs(),
            enable: true,
        }
    }

    pub fn velocity_mode(motor_id: u8, velocity: f64) -> Self {
        Self {
            motor_id,
            steps: if velocity >= 0.0 { i64::MAX } else { i64::MIN },
            velocity: velocity.abs(),
            enable: true,
        }
    }

    pub fn stop(motor_id: u8) -> Self {
        Self {
            motor_id,
            steps: 0,
            velocity: 0.0,
            enable: false,
        }
    }
}

/// Stepper driver configuration
#[derive(Debug, Clone)]
pub struct StepperConfig {
    /// Number of motors (1-8)
    pub num_motors: u8,
    /// Steps per revolution (typically 200 for NEMA 17)
    pub steps_per_rev: u32,
    /// Microstepping (1, 2, 4, 8, 16, 32, etc.)
    pub microsteps: u16,
    /// Maximum velocity in steps/second
    pub max_velocity: f64,
    /// Acceleration in steps/second^2
    pub acceleration: f64,
    /// GPIO pins: (step, dir, enable) per motor
    pub gpio_pins: [(u64, u64, u64); 8],
    /// Step pulse duration in microseconds
    pub step_pulse_us: u64,
}

impl Default for StepperConfig {
    fn default() -> Self {
        Self {
            num_motors: 1,
            steps_per_rev: 200,
            microsteps: 16,
            max_velocity: 1000.0,
            acceleration: 500.0,
            gpio_pins: [(0, 0, 0); 8],
            step_pulse_us: 5,
        }
    }
}

/// Stepper driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum StepperDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "gpio-hardware")]
    Gpio,
}

/// Type-erased stepper driver
pub enum StepperDriver {
    Simulation(SimulationStepperDriver),
    #[cfg(feature = "gpio-hardware")]
    Gpio(GpioStepperDriver),
}

impl StepperDriver {
    pub fn new(backend: StepperDriverBackend, config: StepperConfig) -> HorusResult<Self> {
        match backend {
            StepperDriverBackend::Simulation => {
                Ok(Self::Simulation(SimulationStepperDriver::new(config)))
            }
            #[cfg(feature = "gpio-hardware")]
            StepperDriverBackend::Gpio => Ok(Self::Gpio(GpioStepperDriver::new(config)?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationStepperDriver::new(StepperConfig::default()))
    }

    pub fn with_config(config: StepperConfig) -> Self {
        Self::Simulation(SimulationStepperDriver::new(config))
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

    pub fn write(&mut self, cmd: StepperDriverCommand) -> HorusResult<()> {
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
