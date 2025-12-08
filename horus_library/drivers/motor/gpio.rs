//! GPIO Motor driver
//!
//! DC motor driver using GPIO pins (H-bridge control).
//! Requires the `gpio-hardware` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use crate::MotorCommand;

/// GPIO motor configuration
#[derive(Debug, Clone)]
pub struct GpioMotorConfig {
    /// GPIO chip (e.g., "/dev/gpiochip0")
    pub chip: String,
    /// Forward direction pin
    pub pin_forward: u32,
    /// Backward direction pin
    pub pin_backward: u32,
    /// PWM pin for speed control (optional)
    pub pin_pwm: Option<u32>,
    /// Enable pin (optional)
    pub pin_enable: Option<u32>,
}

impl Default for GpioMotorConfig {
    fn default() -> Self {
        Self {
            chip: "/dev/gpiochip0".to_string(),
            pin_forward: 17,
            pin_backward: 27,
            pin_pwm: Some(18),
            pin_enable: Some(22),
        }
    }
}

/// GPIO motor driver
pub struct GpioMotorDriver {
    config: GpioMotorConfig,
    status: DriverStatus,
    current_speed: f64,
}

impl GpioMotorDriver {
    /// Create a new GPIO motor driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: GpioMotorConfig::default(),
            status: DriverStatus::Uninitialized,
            current_speed: 0.0,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: GpioMotorConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            current_speed: 0.0,
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        // GPIO setup would go here
        // Initialize pins as outputs
        self.current_speed = 0.0;
        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
        // Stop motor and release GPIO
        self.current_speed = 0.0;
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    /// Check if driver is available
    pub fn is_available(&self) -> bool {
        matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Write motor command
    pub fn write(&mut self, command: &MotorCommand) -> HorusResult<()> {
        if !self.is_available() {
            return Err(HorusError::driver("Motor not initialized"));
        }

        // Extract speed from command (assuming first motor)
        let speed = if !command.velocities.is_empty() {
            command.velocities[0]
        } else {
            0.0
        };

        self.set_speed(speed)?;
        self.status = DriverStatus::Running;
        Ok(())
    }

    /// Set motor speed (-1.0 to 1.0)
    pub fn set_speed(&mut self, speed: f64) -> HorusResult<()> {
        let speed = speed.clamp(-1.0, 1.0);
        self.current_speed = speed;

        // In real implementation:
        // - Set direction pins based on sign of speed
        // - Set PWM duty cycle based on absolute value
        // - Enable motor if enable pin is configured

        Ok(())
    }

    /// Get current speed
    pub fn get_speed(&self) -> f64 {
        self.current_speed
    }

    /// Emergency stop
    pub fn emergency_stop(&mut self) -> HorusResult<()> {
        self.current_speed = 0.0;
        // Set all pins low immediately
        Ok(())
    }

    /// Check if command can be executed
    pub fn can_execute(&self) -> bool {
        self.is_available()
    }
}

impl Default for GpioMotorDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create GPIO motor driver")
    }
}
