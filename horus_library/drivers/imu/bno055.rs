//! BNO055 9-axis IMU driver with sensor fusion
//!
//! Hardware driver for the Bosch BNO055 9-axis absolute orientation sensor.
//! The BNO055 includes on-chip sensor fusion, providing quaternion orientation
//! directly from the hardware.
//!
//! Requires the `bno055-imu` feature.

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use crate::Imu;

use bno055::{BNO055OperationMode, Bno055 as Bno055Device};
use linux_embedded_hal::I2cdev;

/// BNO055 I2C address options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bno055Address {
    /// Default address (COM3 pin low): 0x28
    Primary,
    /// Alternative address (COM3 pin high): 0x29
    Secondary,
}

impl Bno055Address {
    /// Get the I2C address byte
    pub fn address(&self) -> u8 {
        match self {
            Self::Primary => 0x28,
            Self::Secondary => 0x29,
        }
    }
}

impl Default for Bno055Address {
    fn default() -> Self {
        Self::Primary
    }
}

/// BNO055 operation mode
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Bno055Mode {
    /// NDOF mode: 9-axis sensor fusion (recommended)
    #[default]
    Ndof,
    /// IMU mode: 6-axis (accel + gyro only)
    Imu,
    /// Compass mode: magnetometer + accelerometer
    Compass,
    /// M4G mode: similar to IMU but with magnetometer for heading
    M4G,
}

impl From<Bno055Mode> for BNO055OperationMode {
    fn from(mode: Bno055Mode) -> Self {
        match mode {
            Bno055Mode::Ndof => BNO055OperationMode::NDOF,
            Bno055Mode::Imu => BNO055OperationMode::IMU,
            Bno055Mode::Compass => BNO055OperationMode::COMPASS,
            Bno055Mode::M4G => BNO055OperationMode::M4G,
        }
    }
}

/// BNO055 driver configuration
#[derive(Debug, Clone)]
pub struct Bno055Config {
    /// I2C bus path (e.g., "/dev/i2c-1")
    pub i2c_bus: String,
    /// I2C address selection
    pub address: Bno055Address,
    /// Operation mode
    pub mode: Bno055Mode,
    /// Sample rate in Hz
    pub sample_rate: f32,
}

impl Default for Bno055Config {
    fn default() -> Self {
        Self {
            i2c_bus: "/dev/i2c-1".to_string(),
            address: Bno055Address::default(),
            mode: Bno055Mode::default(),
            sample_rate: 100.0,
        }
    }
}

/// BNO055 9-axis IMU driver with on-chip sensor fusion
///
/// The BNO055 is a 9-axis IMU (accelerometer, gyroscope, magnetometer) with
/// built-in sensor fusion that provides absolute orientation as a quaternion.
///
/// # Features
///
/// - 3-axis accelerometer
/// - 3-axis gyroscope
/// - 3-axis magnetometer
/// - On-chip sensor fusion (no external processing needed)
/// - Absolute orientation output (quaternion)
/// - Temperature sensor
///
/// # Example
///
/// ```rust,ignore
/// use horus_library::drivers::Bno055Driver;
/// use horus_core::driver::{Driver, Sensor};
///
/// let mut driver = Bno055Driver::new()?;
/// driver.init()?;
///
/// loop {
///     let imu_data = driver.read()?;
///     // BNO055 provides orientation directly!
///     println!("Orientation (quaternion): {:?}", imu_data.orientation);
///     println!("Angular velocity: {:?}", imu_data.angular_velocity);
/// }
/// ```
pub struct Bno055Driver {
    config: Bno055Config,
    status: DriverStatus,
    device: Option<Bno055Device<I2cdev>>,
}

impl Bno055Driver {
    /// Create a new BNO055 driver with default configuration
    pub fn new() -> HorusResult<Self> {
        Self::with_config(Bno055Config::default())
    }

    /// Create a new BNO055 driver with custom configuration
    pub fn with_config(config: Bno055Config) -> HorusResult<Self> {
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

impl Bno055Driver {
    pub fn init(&mut self) -> HorusResult<()> {
        // Open I2C bus
        let i2c = I2cdev::new(&self.config.i2c_bus).map_err(|e| {
            HorusError::driver(format!(
                "Failed to open I2C bus '{}': {}",
                self.config.i2c_bus, e
            ))
        })?;

        // Create BNO055 device
        let mut device = Bno055Device::new(i2c);

        // Initialize the device with delay for reset
        device
            .init(&mut linux_embedded_hal::Delay)
            .map_err(|e| HorusError::driver(format!("Failed to initialize BNO055: {:?}", e)))?;

        // Set operation mode
        device
            .set_mode(self.config.mode.into(), &mut linux_embedded_hal::Delay)
            .map_err(|e| HorusError::driver(format!("Failed to set BNO055 mode: {:?}", e)))?;

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
            .ok_or_else(|| HorusError::driver("BNO055 not initialized"))?;

        self.status = DriverStatus::Running;

        // Read quaternion (BNO055's built-in sensor fusion!)
        let quat = device
            .quaternion()
            .map_err(|e| HorusError::driver(format!("Failed to read quaternion: {:?}", e)))?;

        // Read linear acceleration (gravity-compensated by sensor fusion)
        let lin_accel = device.linear_acceleration().map_err(|e| {
            HorusError::driver(format!("Failed to read linear acceleration: {:?}", e))
        })?;

        // Read gyroscope data
        let gyro = device
            .gyroscope()
            .map_err(|e| HorusError::driver(format!("Failed to read gyroscope: {:?}", e)))?;

        // Quaternion from BNO055 is [w, x, y, z], we store as [x, y, z, w]
        let orientation = [quat.x as f64, quat.y as f64, quat.z as f64, quat.w as f64];

        // Linear acceleration in m/s² (already from sensor)
        let linear_acceleration = [lin_accel.x as f64, lin_accel.y as f64, lin_accel.z as f64];

        // Gyroscope in rad/s (need to convert from deg/s)
        let angular_velocity = [
            (gyro.x as f64).to_radians(),
            (gyro.y as f64).to_radians(),
            (gyro.z as f64).to_radians(),
        ];

        // BNO055 has good sensor fusion, so covariance is lower than raw sensors
        Ok(Imu {
            orientation,
            orientation_covariance: [0.0001, 0.0, 0.0, 0.0, 0.0001, 0.0, 0.0, 0.0, 0.0001],
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
