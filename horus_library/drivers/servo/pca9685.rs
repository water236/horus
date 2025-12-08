//! PCA9685 PWM Servo driver
//!
//! Servo driver using PCA9685 16-channel PWM controller via I2C.
//! Requires the `i2c-hardware` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use crate::ServoCommand;

/// PCA9685 configuration
#[derive(Debug, Clone)]
pub struct Pca9685Config {
    /// I2C device path
    pub i2c_device: String,
    /// I2C address (default: 0x40)
    pub address: u8,
    /// PWM frequency in Hz (typically 50 Hz for servos)
    pub frequency: u16,
    /// Minimum pulse width in microseconds
    pub min_pulse_us: u16,
    /// Maximum pulse width in microseconds
    pub max_pulse_us: u16,
}

impl Default for Pca9685Config {
    fn default() -> Self {
        Self {
            i2c_device: "/dev/i2c-1".to_string(),
            address: 0x40,
            frequency: 50,
            min_pulse_us: 500,
            max_pulse_us: 2500,
        }
    }
}

/// PCA9685 servo driver
pub struct Pca9685ServoDriver {
    config: Pca9685Config,
    status: DriverStatus,
    positions: [f64; 16], // Current positions for 16 channels
}

impl Pca9685ServoDriver {
    /// Create a new PCA9685 servo driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: Pca9685Config::default(),
            status: DriverStatus::Uninitialized,
            positions: [0.0; 16],
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: Pca9685Config) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            positions: [0.0; 16],
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        // I2C initialization would go here
        // - Open I2C device
        // - Configure PCA9685 (set prescaler for frequency, enable oscillator)

        self.positions = [0.0; 16];
        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
        // Disable all PWM outputs
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

    /// Write servo command
    pub fn write(&mut self, cmd: ServoCommand) -> HorusResult<()> {
        if !self.is_available() {
            return Err(HorusError::driver("Servo driver not initialized"));
        }

        // Set positions for each channel
        for (i, &position) in cmd.positions.iter().enumerate() {
            if i < 16 {
                self.set_channel(i as u8, position)?;
            }
        }

        self.status = DriverStatus::Running;
        Ok(())
    }

    /// Stop all servos (center position)
    pub fn stop(&mut self) -> HorusResult<()> {
        for i in 0..16 {
            self.set_channel(i, 0.0)?;
        }
        Ok(())
    }

    /// Set a single channel position (-1.0 to 1.0)
    pub fn set_channel(&mut self, channel: u8, position: f64) -> HorusResult<()> {
        if channel >= 16 {
            return Err(HorusError::driver("Invalid channel (0-15)"));
        }

        let position = position.clamp(-1.0, 1.0);
        self.positions[channel as usize] = position;

        // Calculate pulse width
        let range = self.config.max_pulse_us - self.config.min_pulse_us;
        let pulse_us = self.config.min_pulse_us + ((position + 1.0) / 2.0 * range as f64) as u16;

        // Convert to PCA9685 register values
        let period_us = 1_000_000 / self.config.frequency as u32;
        let on_ticks = 0u16;
        let off_ticks = ((pulse_us as u32 * 4096) / period_us) as u16;

        // In real implementation, write to I2C:
        // - Register LED{channel}_ON_L = on_ticks & 0xFF
        // - Register LED{channel}_ON_H = (on_ticks >> 8) & 0x0F
        // - Register LED{channel}_OFF_L = off_ticks & 0xFF
        // - Register LED{channel}_OFF_H = (off_ticks >> 8) & 0x0F
        let _ = (on_ticks, off_ticks); // Suppress unused warning

        Ok(())
    }

    /// Get current position for a channel
    pub fn get_position(&self, channel: u8) -> Option<f64> {
        if channel < 16 {
            Some(self.positions[channel as usize])
        } else {
            None
        }
    }
}

impl Default for Pca9685ServoDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create PCA9685 servo driver")
    }
}
