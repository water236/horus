//! ICM20948 9-axis IMU driver
//!
//! Hardware driver for the TDK/InvenSense ICM-20948 9-axis IMU
//! (accelerometer + gyroscope + magnetometer).
//! Requires the `icm20948-imu` feature.

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use crate::Imu;

use icm20948::{AccelRangeOptions, GyroRangeOptions, ICMI2C};
use linux_embedded_hal::{Delay, I2cdev};

/// ICM20948 I2C address options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Icm20948Address {
    /// Default address (AD0 pin low): 0x68
    Primary,
    /// Alternative address (AD0 pin high): 0x69
    Secondary,
}

impl Icm20948Address {
    /// Get the I2C address byte
    pub fn address(&self) -> u8 {
        match self {
            Self::Primary => 0x68,
            Self::Secondary => 0x69,
        }
    }
}

impl Default for Icm20948Address {
    fn default() -> Self {
        Self::Primary
    }
}

/// ICM20948 accelerometer range
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AccelRange {
    /// ±2g
    #[default]
    G2,
    /// ±4g
    G4,
    /// ±8g
    G8,
    /// ±16g
    G16,
}

impl From<AccelRange> for AccelRangeOptions {
    fn from(range: AccelRange) -> Self {
        match range {
            AccelRange::G2 => AccelRangeOptions::G2,
            AccelRange::G4 => AccelRangeOptions::G4,
            AccelRange::G8 => AccelRangeOptions::G8,
            AccelRange::G16 => AccelRangeOptions::G16,
        }
    }
}

/// ICM20948 gyroscope range
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum GyroRange {
    /// ±250°/s
    #[default]
    Dps250,
    /// ±500°/s
    Dps500,
    /// ±1000°/s
    Dps1000,
    /// ±2000°/s
    Dps2000,
}

impl From<GyroRange> for GyroRangeOptions {
    fn from(range: GyroRange) -> Self {
        match range {
            GyroRange::Dps250 => GyroRangeOptions::DPS250,
            GyroRange::Dps500 => GyroRangeOptions::DPS500,
            GyroRange::Dps1000 => GyroRangeOptions::DPS1000,
            GyroRange::Dps2000 => GyroRangeOptions::DPS2000,
        }
    }
}

/// ICM20948 driver configuration
#[derive(Debug, Clone)]
pub struct Icm20948Config {
    /// I2C bus path (e.g., "/dev/i2c-1")
    pub i2c_bus: String,
    /// I2C address selection
    pub address: Icm20948Address,
    /// Accelerometer range
    pub accel_range: AccelRange,
    /// Gyroscope range
    pub gyro_range: GyroRange,
    /// Sample rate in Hz
    pub sample_rate: f32,
}

impl Default for Icm20948Config {
    fn default() -> Self {
        Self {
            i2c_bus: "/dev/i2c-1".to_string(),
            address: Icm20948Address::default(),
            accel_range: AccelRange::default(),
            gyro_range: GyroRange::default(),
            sample_rate: 100.0,
        }
    }
}

/// ICM20948 9-axis IMU driver
///
/// Provides access to the ICM-20948 accelerometer, gyroscope, and magnetometer
/// data over I2C.
///
/// # Features
///
/// - 3-axis accelerometer (±2g to ±16g)
/// - 3-axis gyroscope (±250°/s to ±2000°/s)
/// - 3-axis magnetometer (AK09916)
/// - Configurable sample rate
///
/// # Example
///
/// ```rust,ignore
/// use horus_library::drivers::Icm20948Driver;
/// use horus_core::driver::{Driver, Sensor};
///
/// let mut driver = Icm20948Driver::new()?;
/// driver.init()?;
///
/// loop {
///     let imu_data = driver.read()?;
///     println!("Accel: {:?}", imu_data.linear_acceleration);
///     println!("Gyro: {:?}", imu_data.angular_velocity);
/// }
/// ```
pub struct Icm20948Driver {
    config: Icm20948Config,
    status: DriverStatus,
    device: Option<ICMI2C<I2cdev>>,
}

impl Icm20948Driver {
    /// Create a new ICM20948 driver with default configuration
    pub fn new() -> HorusResult<Self> {
        Self::with_config(Icm20948Config::default())
    }

    /// Create a new ICM20948 driver with custom configuration
    pub fn with_config(config: Icm20948Config) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            device: None,
        })
    }

    /// Get the current timestamp in nanoseconds
    fn now_nanos(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

// ========================================================================
// Lifecycle methods
// ========================================================================

impl Icm20948Driver {
    pub fn init(&mut self) -> HorusResult<()> {
        // Open I2C bus
        let i2c = I2cdev::new(&self.config.i2c_bus).map_err(|e| {
            HorusError::driver(format!(
                "Failed to open I2C bus '{}': {}",
                self.config.i2c_bus, e
            ))
        })?;

        // Create ICM20948 device
        let mut device = ICMI2C::new(i2c);

        // Initialize the device with delay
        let mut delay = Delay;
        device
            .init(&mut delay)
            .map_err(|e| HorusError::driver(format!("Failed to initialize ICM20948: {:?}", e)))?;

        self.device = Some(device);
        self.status = DriverStatus::Ready;

        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.device = None;
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        // Check if I2C bus exists
        std::path::Path::new(&self.config.i2c_bus).exists()
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    // ========================================================================
    // Sensor methods
    // ========================================================================

    pub fn read(&mut self) -> HorusResult<Imu> {
        let device = self
            .device
            .as_mut()
            .ok_or_else(|| HorusError::driver("ICM20948 not initialized"))?;

        self.status = DriverStatus::Running;

        // Read raw accelerometer and gyroscope data
        let raw_data = device
            .get_values_accel_gyro()
            .map_err(|e| HorusError::driver(format!("Failed to read sensor data: {:?}", e)))?;

        // Scale the raw values to physical units
        let scaled = device.scale_raw_accel_gyro(raw_data);

        // Extract accelerometer data (in g) and convert to m/s²
        let linear_acceleration = [
            scaled.accel.x as f64 * 9.81,
            scaled.accel.y as f64 * 9.81,
            scaled.accel.z as f64 * 9.81,
        ];

        // Extract gyroscope data (in deg/s) and convert to rad/s
        let angular_velocity = [
            (scaled.gyro.x as f64).to_radians(),
            (scaled.gyro.y as f64).to_radians(),
            (scaled.gyro.z as f64).to_radians(),
        ];

        // Note: Magnetometer reading requires separate calls and is not
        // included in orientation calculation here. For full 9-axis fusion,
        // use an external sensor fusion library (e.g., ahrs, madgwick).
        Ok(Imu {
            orientation: [0.0, 0.0, 0.0, 1.0], // Quaternion identity (no fusion implemented)
            orientation_covariance: [-1.0; 9], // -1 indicates orientation not available
            angular_velocity,
            angular_velocity_covariance: [0.0001, 0.0, 0.0, 0.0, 0.0001, 0.0, 0.0, 0.0, 0.0001],
            linear_acceleration,
            linear_acceleration_covariance: [0.0001, 0.0, 0.0, 0.0, 0.0001, 0.0, 0.0, 0.0, 0.0001],
            timestamp: self.now_nanos(),
        })
    }

    pub fn has_data(&self) -> bool {
        self.device.is_some() && matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.sample_rate)
    }
}
