//! Simulation Ultrasonic driver
//!
//! Always-available simulation driver that generates synthetic range data.
//! Useful for testing and development without hardware.

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::Range;

/// Simulation ultrasonic driver configuration
#[derive(Debug, Clone)]
pub struct SimulationUltrasonicConfig {
    /// Sample rate in Hz
    pub sample_rate: f32,
    /// Minimum range in meters
    pub min_range: f32,
    /// Maximum range in meters
    pub max_range: f32,
    /// Field of view in radians (~15 degrees for HC-SR04)
    pub field_of_view: f32,
    /// Add noise to simulated data
    pub add_noise: bool,
    /// Noise standard deviation (meters)
    pub noise_std: f32,
    /// Base distance for simulation (meters)
    pub base_distance: f32,
}

impl Default for SimulationUltrasonicConfig {
    fn default() -> Self {
        Self {
            sample_rate: 10.0,   // 10 Hz (typical for HC-SR04)
            min_range: 0.02,     // 2cm
            max_range: 4.0,      // 4m
            field_of_view: 0.26, // ~15 degrees
            add_noise: true,
            noise_std: 0.01,    // 1cm noise
            base_distance: 1.0, // 1 meter
        }
    }
}

/// Simulation Ultrasonic driver
///
/// Generates synthetic range data without requiring hardware.
/// Always available for testing and development.
///
/// # Example
///
/// ```rust,ignore
/// use horus_library::drivers::SimulationUltrasonicDriver;
/// use horus_core::driver::{Driver, Sensor};
///
/// let mut driver = SimulationUltrasonicDriver::new();
/// driver.init()?;
///
/// loop {
///     let range = driver.read()?;
///     println!("Distance: {:.3}m", range.range);
/// }
/// ```
pub struct SimulationUltrasonicDriver {
    config: SimulationUltrasonicConfig,
    status: DriverStatus,
    start_time: Option<u64>,
    sample_count: u64,
}

impl SimulationUltrasonicDriver {
    /// Create a new simulation ultrasonic driver with default configuration
    pub fn new() -> Self {
        Self {
            config: SimulationUltrasonicConfig::default(),
            status: DriverStatus::Uninitialized,
            start_time: None,
            sample_count: 0,
        }
    }

    /// Create a new simulation ultrasonic driver with custom configuration
    pub fn with_config(config: SimulationUltrasonicConfig) -> Self {
        Self {
            config,
            status: DriverStatus::Uninitialized,
            start_time: None,
            sample_count: 0,
        }
    }

    /// Set the base distance for simulation
    pub fn set_base_distance(&mut self, distance: f32) {
        self.config.base_distance = distance;
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

    /// Read range data
    pub fn read(&mut self) -> HorusResult<Range> {
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

    /// Generate synthetic range data
    fn generate_data(&mut self) -> Range {
        let timestamp = self.now_nanos();
        let elapsed = self.start_time.map(|s| timestamp - s).unwrap_or(0) as f64 / 1e9;

        // Simulate a slowly varying distance with some periodic motion
        let t = elapsed;
        let mut distance = self.config.base_distance
            + 0.3 * (t * 0.2).sin() as f32  // Slow oscillation
            + 0.1 * (t * 1.5).cos() as f32; // Faster variation

        // Add noise if configured
        if self.config.add_noise {
            // Simple pseudo-random noise based on sample count
            let noise_seed = self.sample_count as f64;
            let noise = self.config.noise_std
                * ((noise_seed * 12.9898 + 78.233).sin() * 43758.5453).fract() as f32;
            distance += noise;
        }

        // Clamp to valid range
        distance = distance.clamp(self.config.min_range, self.config.max_range);

        self.sample_count += 1;

        Range {
            sensor_type: Range::ULTRASONIC,
            field_of_view: self.config.field_of_view,
            min_range: self.config.min_range,
            max_range: self.config.max_range,
            range: distance,
            timestamp,
        }
    }
}

impl Default for SimulationUltrasonicDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_driver_lifecycle() {
        let mut driver = SimulationUltrasonicDriver::new();

        assert_eq!(driver.status(), DriverStatus::Uninitialized);
        assert!(driver.is_available());

        driver.init().unwrap();
        assert_eq!(driver.status(), DriverStatus::Ready);

        let data = driver.read().unwrap();
        assert_eq!(driver.status(), DriverStatus::Running);

        // Check range is within bounds
        assert!(data.range >= data.min_range);
        assert!(data.range <= data.max_range);
        assert_eq!(data.sensor_type, Range::ULTRASONIC);

        driver.shutdown().unwrap();
        assert_eq!(driver.status(), DriverStatus::Shutdown);
    }

    #[test]
    fn test_simulation_driver_with_config() {
        let config = SimulationUltrasonicConfig {
            sample_rate: 20.0,
            base_distance: 2.0,
            add_noise: false,
            ..Default::default()
        };

        let driver = SimulationUltrasonicDriver::with_config(config);
        assert_eq!(driver.sample_rate(), Some(20.0));
    }
}
