//! GPIO Encoder driver
//!
//! Quadrature encoder driver using GPIO pins.
//! Requires the `gpio-hardware` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::Odometry;

/// GPIO encoder configuration
#[derive(Debug, Clone)]
pub struct GpioEncoderConfig {
    /// GPIO chip (e.g., "/dev/gpiochip0")
    pub chip: String,
    /// Channel A pin number
    pub pin_a: u32,
    /// Channel B pin number
    pub pin_b: u32,
    /// Pulses per revolution
    pub ppr: u32,
    /// Wheel radius in meters (for distance calculation)
    pub wheel_radius: f64,
}

impl Default for GpioEncoderConfig {
    fn default() -> Self {
        Self {
            chip: "/dev/gpiochip0".to_string(),
            pin_a: 17,
            pin_b: 27,
            ppr: 1024,
            wheel_radius: 0.05,
        }
    }
}

/// GPIO encoder driver
pub struct GpioEncoderDriver {
    config: GpioEncoderConfig,
    status: DriverStatus,
    count: i64,
    last_count: i64,
}

impl GpioEncoderDriver {
    /// Create a new GPIO encoder driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: GpioEncoderConfig::default(),
            status: DriverStatus::Uninitialized,
            count: 0,
            last_count: 0,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: GpioEncoderConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            count: 0,
            last_count: 0,
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        // GPIO setup would go here using gpio_cdev or similar
        self.count = 0;
        self.last_count = 0;
        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
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

    /// Read odometry
    pub fn read(&mut self) -> HorusResult<Odometry> {
        self.status = DriverStatus::Running;

        let delta = self.count - self.last_count;
        self.last_count = self.count;

        // Calculate distance traveled
        let distance = (delta as f64 / self.config.ppr as f64)
            * 2.0
            * std::f64::consts::PI
            * self.config.wheel_radius;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut frame_id = [0u8; 32];
        let id_bytes = b"encoder_odom";
        frame_id[..id_bytes.len()].copy_from_slice(id_bytes);

        Ok(Odometry {
            x: distance,
            y: 0.0,
            z: 0.0,
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            vx: 0.0,
            vy: 0.0,
            vz: 0.0,
            wx: 0.0,
            wy: 0.0,
            wz: 0.0,
            frame_id,
            timestamp,
        })
    }

    /// Check if data is available
    pub fn has_data(&self) -> bool {
        self.is_available()
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> Option<f32> {
        None // Hardware-driven
    }

    /// Get current encoder count
    pub fn get_count(&self) -> i64 {
        self.count
    }

    /// Reset encoder count
    pub fn reset(&mut self) {
        self.count = 0;
        self.last_count = 0;
    }
}

impl Default for GpioEncoderDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create GPIO encoder driver")
    }
}
