//! Simulation LiDAR driver
//!
//! Always-available simulation driver that generates synthetic laser scans.

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::LaserScan;

/// Simulation lidar configuration
#[derive(Debug, Clone)]
pub struct SimulationLidarConfig {
    /// Angular resolution (radians per reading)
    pub angle_increment: f32,
    /// Minimum range in meters
    pub range_min: f32,
    /// Maximum range in meters
    pub range_max: f32,
    /// Update rate in Hz
    pub update_rate: f32,
}

impl Default for SimulationLidarConfig {
    fn default() -> Self {
        Self {
            angle_increment: std::f32::consts::PI * 2.0 / 360.0,
            range_min: 0.1,
            range_max: 10.0,
            update_rate: 10.0,
        }
    }
}

/// Simulation lidar driver
///
/// Generates synthetic laser scans for testing without hardware.
pub struct SimulationLidarDriver {
    config: SimulationLidarConfig,
    status: DriverStatus,
    scan_count: u64,
}

impl SimulationLidarDriver {
    pub fn new() -> Self {
        Self {
            config: SimulationLidarConfig::default(),
            status: DriverStatus::Uninitialized,
            scan_count: 0,
        }
    }

    pub fn with_config(config: SimulationLidarConfig) -> Self {
        Self {
            config,
            status: DriverStatus::Uninitialized,
            scan_count: 0,
        }
    }

    fn now_nanos(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        self.scan_count = 0;
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
        true
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Read laser scan data
    pub fn read(&mut self) -> HorusResult<LaserScan> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;
        Ok(self.generate_scan())
    }

    /// Check if data is available
    pub fn has_data(&self) -> bool {
        matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.update_rate)
    }

    fn generate_scan(&mut self) -> LaserScan {
        // Generate a simulated room with walls
        let mut ranges = [0.0f32; 360];
        let t = self.scan_count as f32 * 0.1;

        for (i, range) in ranges.iter_mut().enumerate() {
            let angle = (i as f32) * self.config.angle_increment;

            // Simulate a rectangular room with some variation
            let dx = angle.cos().abs();
            let dy = angle.sin().abs();

            // Room dimensions: 5m x 4m, centered at origin
            let wall_x = 2.5 / dx.max(0.001);
            let wall_y = 2.0 / dy.max(0.001);

            let base_range = wall_x.min(wall_y);

            // Add some noise and time-varying obstacles
            let noise = ((i as f32 + t).sin() * 0.1).abs();
            let obstacle = if (i > 45 && i < 55) || (i > 135 && i < 145) {
                // Simulate some closer objects
                1.0 + (t * 0.5).sin().abs()
            } else {
                base_range
            };

            *range = (obstacle + noise).clamp(self.config.range_min, self.config.range_max);
        }

        self.scan_count += 1;

        LaserScan {
            ranges,
            angle_min: 0.0,
            angle_max: std::f32::consts::PI * 2.0,
            range_min: self.config.range_min,
            range_max: self.config.range_max,
            angle_increment: self.config.angle_increment,
            time_increment: 0.0,
            scan_time: 1.0 / self.config.update_rate,
            timestamp: self.now_nanos(),
        }
    }
}

impl Default for SimulationLidarDriver {
    fn default() -> Self {
        Self::new()
    }
}
