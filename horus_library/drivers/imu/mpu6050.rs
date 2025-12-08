//! MPU6050 6-axis IMU driver
//!
//! Hardware driver for the InvenSense MPU6050 6-axis IMU (accelerometer + gyroscope).
//! Requires the `mpu6050-imu` feature.

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use crate::Imu;

use linux_embedded_hal::I2cdev;
use mpu6050::Mpu6050 as Mpu6050Device;

/// MPU6050 I2C address options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mpu6050Address {
    /// Default address (AD0 pin low): 0x68
    Primary,
    /// Alternative address (AD0 pin high): 0x69
    Secondary,
}

impl Mpu6050Address {
    /// Get the I2C address byte
    pub fn address(&self) -> u8 {
        match self {
            Self::Primary => 0x68,
            Self::Secondary => 0x69,
        }
    }
}

impl Default for Mpu6050Address {
    fn default() -> Self {
        Self::Primary
    }
}

/// MPU6050 accelerometer range
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

/// MPU6050 gyroscope range
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

/// MPU6050 driver configuration
#[derive(Debug, Clone)]
pub struct Mpu6050Config {
    /// I2C bus path (e.g., "/dev/i2c-1")
    pub i2c_bus: String,
    /// I2C address selection
    pub address: Mpu6050Address,
    /// Accelerometer range
    pub accel_range: AccelRange,
    /// Gyroscope range
    pub gyro_range: GyroRange,
    /// Sample rate in Hz (actual rate may vary based on DLPF settings)
    pub sample_rate: f32,
}

impl Default for Mpu6050Config {
    fn default() -> Self {
        Self {
            i2c_bus: "/dev/i2c-1".to_string(),
            address: Mpu6050Address::default(),
            accel_range: AccelRange::default(),
            gyro_range: GyroRange::default(),
            sample_rate: 100.0,
        }
    }
}

/// MPU6050 6-axis IMU driver
///
/// Provides access to the MPU6050 accelerometer and gyroscope data
/// over I2C.
///
/// # Features
///
/// - 3-axis accelerometer (±2g to ±16g)
/// - 3-axis gyroscope (±250°/s to ±2000°/s)
/// - Configurable sample rate
///
/// # Example
///
/// ```rust,ignore
/// use horus_library::drivers::Mpu6050Driver;
/// use horus_core::driver::{Driver, Sensor};
///
/// let mut driver = Mpu6050Driver::new()?;
/// driver.init()?;
///
/// loop {
///     let imu_data = driver.read()?;
///     println!("Accel: {:?}", imu_data.linear_acceleration);
///     println!("Gyro: {:?}", imu_data.angular_velocity);
/// }
/// ```
pub struct Mpu6050Driver {
    config: Mpu6050Config,
    status: DriverStatus,
    device: Option<Mpu6050Device<I2cdev>>,
}

impl Mpu6050Driver {
    /// Create a new MPU6050 driver with default configuration
    pub fn new() -> HorusResult<Self> {
        Self::with_config(Mpu6050Config::default())
    }

    /// Create a new MPU6050 driver with custom configuration
    pub fn with_config(config: Mpu6050Config) -> HorusResult<Self> {
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

impl Mpu6050Driver {
    pub fn init(&mut self) -> HorusResult<()> {
        // Open I2C bus
        let i2c = I2cdev::new(&self.config.i2c_bus).map_err(|e| {
            HorusError::driver(format!(
                "Failed to open I2C bus '{}': {}",
                self.config.i2c_bus, e
            ))
        })?;

        // Create MPU6050 device
        let mut device = Mpu6050Device::new(i2c);

        // Initialize the device
        device
            .init(&mut linux_embedded_hal::Delay)
            .map_err(|e| HorusError::driver(format!("Failed to initialize MPU6050: {:?}", e)))?;

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
            .ok_or_else(|| HorusError::driver("MPU6050 not initialized"))?;

        self.status = DriverStatus::Running;

        // Read accelerometer data (returns in g)
        let accel = device
            .get_acc()
            .map_err(|e| HorusError::driver(format!("Failed to read accelerometer: {:?}", e)))?;

        // Read gyroscope data (returns in deg/s)
        let gyro = device
            .get_gyro()
            .map_err(|e| HorusError::driver(format!("Failed to read gyroscope: {:?}", e)))?;

        // Convert accelerometer from g to m/s²
        let linear_acceleration = [
            accel.x as f64 * 9.81,
            accel.y as f64 * 9.81,
            accel.z as f64 * 9.81,
        ];

        // Convert gyroscope from deg/s to rad/s
        let angular_velocity = [
            (gyro.x as f64).to_radians(),
            (gyro.y as f64).to_radians(),
            (gyro.z as f64).to_radians(),
        ];

        Ok(Imu {
            orientation: [0.0, 0.0, 0.0, 1.0], // MPU6050 doesn't provide orientation
            orientation_covariance: [-1.0; 9], // No orientation data available
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
