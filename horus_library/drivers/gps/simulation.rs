//! Simulation GPS driver

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::NavSatFix;

/// Simulation GPS configuration
#[derive(Debug, Clone)]
pub struct SimulationGpsConfig {
    /// Base latitude (degrees)
    pub base_latitude: f64,
    /// Base longitude (degrees)
    pub base_longitude: f64,
    /// Base altitude (meters)
    pub base_altitude: f64,
    /// Update rate in Hz
    pub update_rate: f32,
    /// Simulate movement
    pub simulate_movement: bool,
}

impl Default for SimulationGpsConfig {
    fn default() -> Self {
        Self {
            base_latitude: 37.7749, // San Francisco
            base_longitude: -122.4194,
            base_altitude: 10.0,
            update_rate: 1.0,
            simulate_movement: true,
        }
    }
}

/// Simulation GPS driver
pub struct SimulationGpsDriver {
    config: SimulationGpsConfig,
    status: DriverStatus,
    sample_count: u64,
}

impl SimulationGpsDriver {
    pub fn new() -> Self {
        Self {
            config: SimulationGpsConfig::default(),
            status: DriverStatus::Uninitialized,
            sample_count: 0,
        }
    }

    pub fn with_config(config: SimulationGpsConfig) -> Self {
        Self {
            config,
            status: DriverStatus::Uninitialized,
            sample_count: 0,
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
        true
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Read GPS fix data
    pub fn read(&mut self) -> HorusResult<NavSatFix> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;
        Ok(self.generate_fix())
    }

    /// Check if data is available
    pub fn has_data(&self) -> bool {
        matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.update_rate)
    }

    fn generate_fix(&mut self) -> NavSatFix {
        let t = self.sample_count as f64 * 0.001;

        let (lat, lon) = if self.config.simulate_movement {
            // Simulate a slow circular path
            let radius = 0.0001; // ~11 meters
            (
                self.config.base_latitude + radius * (t * 0.1).sin(),
                self.config.base_longitude + radius * (t * 0.1).cos(),
            )
        } else {
            (self.config.base_latitude, self.config.base_longitude)
        };

        self.sample_count += 1;

        NavSatFix {
            latitude: lat,
            longitude: lon,
            altitude: self.config.base_altitude,
            position_covariance: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0],
            position_covariance_type: 2, // COVARIANCE_TYPE_DIAGONAL_KNOWN
            status: 1,                   // STATUS_FIX
            satellites_visible: 8,
            hdop: 1.0,
            vdop: 1.5,
            speed: if self.config.simulate_movement {
                0.5
            } else {
                0.0
            },
            heading: (t * 10.0 % 360.0) as f32,
            timestamp: self.now_nanos(),
        }
    }
}

impl Default for SimulationGpsDriver {
    fn default() -> Self {
        Self::new()
    }
}
