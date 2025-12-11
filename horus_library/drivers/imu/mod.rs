//! IMU (Inertial Measurement Unit) drivers
//!
//! This module provides drivers for various IMU sensors.
//!
//! # Available Drivers
//!
//! - `SimulationImuDriver` - Always available, generates synthetic data
//! - `Mpu6050Driver` - MPU6050 6-axis IMU (requires `mpu6050-imu` feature)
//! - `Bno055Driver` - BNO055 9-axis IMU with fusion (requires `bno055-imu` feature)
//! - `Icm20948Driver` - ICM-20948 9-axis IMU (requires `icm20948-imu` feature)

mod simulation;

#[cfg(feature = "mpu6050-imu")]
mod mpu6050;

#[cfg(feature = "bno055-imu")]
mod bno055;

#[cfg(feature = "icm20948-imu")]
mod icm20948;

// Re-exports
pub use simulation::SimulationImuDriver;

#[cfg(feature = "mpu6050-imu")]
pub use mpu6050::Mpu6050Driver;

#[cfg(feature = "bno055-imu")]
pub use bno055::Bno055Driver;

#[cfg(feature = "icm20948-imu")]
pub use icm20948::Icm20948Driver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::Imu;

/// Enum of all available IMU driver backends
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ImuDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "mpu6050-imu")]
    Mpu6050,
    #[cfg(feature = "bno055-imu")]
    Bno055,
    #[cfg(feature = "icm20948-imu")]
    Icm20948,
}

/// Type-erased IMU driver for runtime backend selection
pub enum ImuDriver {
    Simulation(SimulationImuDriver),
    #[cfg(feature = "mpu6050-imu")]
    Mpu6050(Mpu6050Driver),
    #[cfg(feature = "bno055-imu")]
    Bno055(Bno055Driver),
    #[cfg(feature = "icm20948-imu")]
    Icm20948(Icm20948Driver),
}

impl ImuDriver {
    /// Create a new IMU driver with the specified backend
    pub fn new(backend: ImuDriverBackend) -> HorusResult<Self> {
        match backend {
            ImuDriverBackend::Simulation => Ok(Self::Simulation(SimulationImuDriver::new())),
            #[cfg(feature = "mpu6050-imu")]
            ImuDriverBackend::Mpu6050 => Ok(Self::Mpu6050(Mpu6050Driver::new()?)),
            #[cfg(feature = "bno055-imu")]
            ImuDriverBackend::Bno055 => Ok(Self::Bno055(Bno055Driver::new()?)),
            #[cfg(feature = "icm20948-imu")]
            ImuDriverBackend::Icm20948 => Ok(Self::Icm20948(Icm20948Driver::new()?)),
        }
    }

    /// Create a simulation driver (always available)
    pub fn simulation() -> Self {
        Self::Simulation(SimulationImuDriver::new())
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "mpu6050-imu")]
            Self::Mpu6050(d) => d.init(),
            #[cfg(feature = "bno055-imu")]
            Self::Bno055(d) => d.init(),
            #[cfg(feature = "icm20948-imu")]
            Self::Icm20948(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "mpu6050-imu")]
            Self::Mpu6050(d) => d.shutdown(),
            #[cfg(feature = "bno055-imu")]
            Self::Bno055(d) => d.shutdown(),
            #[cfg(feature = "icm20948-imu")]
            Self::Icm20948(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "mpu6050-imu")]
            Self::Mpu6050(d) => d.is_available(),
            #[cfg(feature = "bno055-imu")]
            Self::Bno055(d) => d.is_available(),
            #[cfg(feature = "icm20948-imu")]
            Self::Icm20948(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "mpu6050-imu")]
            Self::Mpu6050(d) => d.status(),
            #[cfg(feature = "bno055-imu")]
            Self::Bno055(d) => d.status(),
            #[cfg(feature = "icm20948-imu")]
            Self::Icm20948(d) => d.status(),
        }
    }

    // ========================================================================
    // Sensor methods
    // ========================================================================

    pub fn read(&mut self) -> HorusResult<Imu> {
        match self {
            Self::Simulation(d) => d.read(),
            #[cfg(feature = "mpu6050-imu")]
            Self::Mpu6050(d) => d.read(),
            #[cfg(feature = "bno055-imu")]
            Self::Bno055(d) => d.read(),
            #[cfg(feature = "icm20948-imu")]
            Self::Icm20948(d) => d.read(),
        }
    }

    pub fn has_data(&self) -> bool {
        match self {
            Self::Simulation(d) => d.has_data(),
            #[cfg(feature = "mpu6050-imu")]
            Self::Mpu6050(d) => d.has_data(),
            #[cfg(feature = "bno055-imu")]
            Self::Bno055(d) => d.has_data(),
            #[cfg(feature = "icm20948-imu")]
            Self::Icm20948(d) => d.has_data(),
        }
    }

    pub fn sample_rate(&self) -> Option<f32> {
        match self {
            Self::Simulation(d) => d.sample_rate(),
            #[cfg(feature = "mpu6050-imu")]
            Self::Mpu6050(d) => d.sample_rate(),
            #[cfg(feature = "bno055-imu")]
            Self::Bno055(d) => d.sample_rate(),
            #[cfg(feature = "icm20948-imu")]
            Self::Icm20948(d) => d.sample_rate(),
        }
    }
}
