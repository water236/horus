//! Simulation IMU driver
//!
//! Always-available simulation driver that generates synthetic IMU data.
//! Useful for testing and development without hardware.

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::Imu;

/// Simulation IMU driver configuration
#[derive(Debug, Clone)]
pub struct SimulationImuConfig {
    /// Sample rate in Hz
    pub sample_rate: f32,
    /// Add noise to simulated data
    pub add_noise: bool,
    /// Noise standard deviation for accelerometer (m/s²)
    pub accel_noise_std: f64,
    /// Noise standard deviation for gyroscope (rad/s)
    pub gyro_noise_std: f64,
}

impl Default for SimulationImuConfig {
    fn default() -> Self {
        Self {
            sample_rate: 100.0, // 100 Hz
            add_noise: true,
            accel_noise_std: 0.01,
            gyro_noise_std: 0.001,
        }
    }
}

/// Simulation IMU driver
///
/// Generates synthetic IMU data without requiring hardware.
/// Always available for testing and development.
///
/// # Example
///
/// ```rust,ignore
/// use horus_library::drivers::SimulationImuDriver;
/// use horus_core::driver::{Driver, Sensor};
///
/// let mut driver = SimulationImuDriver::new();
/// driver.init()?;
///
/// loop {
///     let imu_data = driver.read()?;
///     println!("Accel: {:?}", imu_data.linear_acceleration);
/// }
/// ```
pub struct SimulationImuDriver {
    config: SimulationImuConfig,
    status: DriverStatus,
    start_time: Option<u64>,
    sample_count: u64,
}

impl SimulationImuDriver {
    /// Create a new simulation IMU driver with default configuration
    pub fn new() -> Self {
        Self {
            config: SimulationImuConfig::default(),
            status: DriverStatus::Uninitialized,
            start_time: None,
            sample_count: 0,
        }
    }

    /// Create a new simulation IMU driver with custom configuration
    pub fn with_config(config: SimulationImuConfig) -> Self {
        Self {
            config,
            status: DriverStatus::Uninitialized,
            start_time: None,
            sample_count: 0,
        }
    }

    /// Get the current timestamp in nanoseconds
    fn now_nanos(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        self.start_time = Some(self.now_nanos());
        self.sample_count = 0;
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
        true // Simulation is always available
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Read IMU data
    pub fn read(&mut self) -> HorusResult<Imu> {
        if self.status != DriverStatus::Ready && self.status != DriverStatus::Running {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;
        Ok(self.generate_data())
    }

    /// Check if data is available
    pub fn has_data(&self) -> bool {
        matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.sample_rate)
    }

    /// Generate synthetic IMU data
    fn generate_data(&mut self) -> Imu {
        let timestamp = self.now_nanos();
        let elapsed = self.start_time.map(|s| timestamp - s).unwrap_or(0) as f64 / 1e9;

        // Generate a gentle swaying motion for realism
        let t = elapsed;

        // Simulate gravity on z-axis with slight oscillation
        let mut accel = [
            0.01 * (t * 0.5).sin(), // Slight x sway
            0.01 * (t * 0.7).cos(), // Slight y sway
            9.81,                   // Gravity
        ];

        // Simulate small angular velocities
        let mut gyro = [
            0.01 * (t * 0.3).sin(),
            0.01 * (t * 0.4).cos(),
            0.005 * (t * 0.2).sin(),
        ];

        // Add noise if configured
        if self.config.add_noise {
            // Simple pseudo-random noise based on sample count
            let noise_seed = self.sample_count as f64;
            let noise = |seed: f64, std: f64| -> f64 {
                // Simple noise approximation (not truly random, but deterministic)
                std * ((seed * 12.9898 + 78.233).sin() * 43758.5453).fract()
            };

            for (i, a) in accel.iter_mut().enumerate() {
                *a += noise(noise_seed + i as f64 * 100.0, self.config.accel_noise_std);
            }
            for (i, g) in gyro.iter_mut().enumerate() {
                *g += noise(
                    noise_seed + i as f64 * 200.0 + 300.0,
                    self.config.gyro_noise_std,
                );
            }
        }

        self.sample_count += 1;

        Imu {
            orientation: [0.0, 0.0, 0.0, 1.0], // Identity quaternion
            orientation_covariance: [-1.0; 9], // No orientation estimation
            angular_velocity: gyro,
            angular_velocity_covariance: [0.0001, 0.0, 0.0, 0.0, 0.0001, 0.0, 0.0, 0.0, 0.0001],
            linear_acceleration: accel,
            linear_acceleration_covariance: [0.0001, 0.0, 0.0, 0.0, 0.0001, 0.0, 0.0, 0.0, 0.0001],
            timestamp,
        }
    }
}

impl Default for SimulationImuDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_driver_lifecycle() {
        let mut driver = SimulationImuDriver::new();

        assert_eq!(driver.status(), DriverStatus::Uninitialized);
        assert!(driver.is_available());

        driver.init().unwrap();
        assert_eq!(driver.status(), DriverStatus::Ready);

        let data = driver.read().unwrap();
        assert_eq!(driver.status(), DriverStatus::Running);

        // Check gravity is approximately correct
        assert!((data.linear_acceleration[2] - 9.81).abs() < 0.1);

        driver.shutdown().unwrap();
        assert_eq!(driver.status(), DriverStatus::Shutdown);
    }

    #[test]
    fn test_simulation_driver_with_config() {
        let config = SimulationImuConfig {
            sample_rate: 200.0,
            add_noise: false,
            ..Default::default()
        };

        let driver = SimulationImuDriver::with_config(config);
        assert_eq!(driver.sample_rate(), Some(200.0));
    }
}
