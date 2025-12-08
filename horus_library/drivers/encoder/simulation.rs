//! Simulation Encoder driver

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::{Odometry, Pose2D, Twist};

/// Simulation encoder configuration
#[derive(Debug, Clone)]
pub struct SimulationEncoderConfig {
    /// Ticks per revolution
    pub ticks_per_rev: u32,
    /// Wheel radius in meters
    pub wheel_radius: f64,
    /// Update rate in Hz
    pub update_rate: f32,
}

impl Default for SimulationEncoderConfig {
    fn default() -> Self {
        Self {
            ticks_per_rev: 1024,
            wheel_radius: 0.05,
            update_rate: 100.0,
        }
    }
}

/// Simulation encoder driver
pub struct SimulationEncoderDriver {
    config: SimulationEncoderConfig,
    status: DriverStatus,
    ticks: i64,
    position: f64,
    velocity: f64,
}

impl SimulationEncoderDriver {
    pub fn new() -> Self {
        Self {
            config: SimulationEncoderConfig::default(),
            status: DriverStatus::Uninitialized,
            ticks: 0,
            position: 0.0,
            velocity: 0.0,
        }
    }

    pub fn with_config(config: SimulationEncoderConfig) -> Self {
        Self {
            config,
            status: DriverStatus::Uninitialized,
            ticks: 0,
            position: 0.0,
            velocity: 0.0,
        }
    }

    fn now_nanos(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// Simulate encoder tick (call from motor simulation)
    pub fn add_ticks(&mut self, delta: i64) {
        self.ticks += delta;
        let revolutions = self.ticks as f64 / self.config.ticks_per_rev as f64;
        self.position = revolutions * 2.0 * std::f64::consts::PI * self.config.wheel_radius;
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        self.ticks = 0;
        self.position = 0.0;
        self.velocity = 0.0;
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

    /// Read odometry data
    pub fn read(&mut self) -> HorusResult<Odometry> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;
        Ok(self.generate_reading())
    }

    /// Check if data is available
    pub fn has_data(&self) -> bool {
        matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.update_rate)
    }

    fn generate_reading(&mut self) -> Odometry {
        // Simulate slow forward motion
        self.ticks += 10;
        let revolutions = self.ticks as f64 / self.config.ticks_per_rev as f64;
        self.position = revolutions * 2.0 * std::f64::consts::PI * self.config.wheel_radius;
        self.velocity = 0.1; // 0.1 m/s

        let mut frame_id = [0u8; 32];
        let mut child_frame_id = [0u8; 32];
        let odom_bytes = b"odom";
        let base_bytes = b"base_link";
        frame_id[..odom_bytes.len()].copy_from_slice(odom_bytes);
        child_frame_id[..base_bytes.len()].copy_from_slice(base_bytes);

        let timestamp = self.now_nanos();

        Odometry {
            pose: Pose2D {
                x: self.position,
                y: 0.0,
                theta: 0.0,
                timestamp,
            },
            twist: Twist {
                linear: [self.velocity, 0.0, 0.0],
                angular: [0.0, 0.0, 0.0],
                timestamp,
            },
            pose_covariance: [0.01; 36],
            twist_covariance: [0.01; 36],
            frame_id,
            child_frame_id,
            timestamp,
        }
    }
}

impl Default for SimulationEncoderDriver {
    fn default() -> Self {
        Self::new()
    }
}
