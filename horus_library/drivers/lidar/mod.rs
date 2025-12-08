//! LiDAR drivers
//!
//! This module provides drivers for LiDAR sensors.
//!
//! # Available Drivers
//!
//! - `SimulationLidarDriver` - Always available, generates synthetic scans
//! - `RplidarDriver` - RPLidar A2/A3 (requires `rplidar` feature)

mod simulation;

#[cfg(feature = "rplidar")]
mod rplidar;

pub use simulation::SimulationLidarDriver;

#[cfg(feature = "rplidar")]
pub use rplidar::RplidarDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::LaserScan;

/// LiDAR driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LidarDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "rplidar")]
    Rplidar,
}

/// Type-erased LiDAR driver
pub enum LidarDriver {
    Simulation(SimulationLidarDriver),
    #[cfg(feature = "rplidar")]
    Rplidar(RplidarDriver),
}

impl LidarDriver {
    pub fn new(backend: LidarDriverBackend) -> HorusResult<Self> {
        match backend {
            LidarDriverBackend::Simulation => Ok(Self::Simulation(SimulationLidarDriver::new())),
            #[cfg(feature = "rplidar")]
            LidarDriverBackend::Rplidar => Ok(Self::Rplidar(RplidarDriver::new()?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationLidarDriver::new())
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "rplidar")]
            Self::Rplidar(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "rplidar")]
            Self::Rplidar(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "rplidar")]
            Self::Rplidar(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "rplidar")]
            Self::Rplidar(d) => d.status(),
        }
    }

    // ========================================================================
    // Sensor methods
    // ========================================================================

    pub fn read(&mut self) -> HorusResult<LaserScan> {
        match self {
            Self::Simulation(d) => d.read(),
            #[cfg(feature = "rplidar")]
            Self::Rplidar(d) => d.read(),
        }
    }

    pub fn has_data(&self) -> bool {
        match self {
            Self::Simulation(d) => d.has_data(),
            #[cfg(feature = "rplidar")]
            Self::Rplidar(d) => d.has_data(),
        }
    }

    pub fn sample_rate(&self) -> Option<f32> {
        match self {
            Self::Simulation(d) => d.sample_rate(),
            #[cfg(feature = "rplidar")]
            Self::Rplidar(d) => d.sample_rate(),
        }
    }
}
