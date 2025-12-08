//! Camera drivers
//!
//! This module provides drivers for camera sensors.
//!
//! # Available Drivers
//!
//! - `SimulationCameraDriver` - Always available, generates synthetic images
//! - `OpenCvCameraDriver` - OpenCV-based camera (requires `opencv-backend` feature)
//! - `V4l2CameraDriver` - Video4Linux2 camera (requires `v4l2-backend` feature)

mod simulation;

#[cfg(feature = "opencv-backend")]
mod opencv;

#[cfg(feature = "v4l2-backend")]
mod v4l2;

// Re-exports
pub use simulation::SimulationCameraDriver;

#[cfg(feature = "opencv-backend")]
pub use opencv::OpenCvCameraDriver;

#[cfg(feature = "v4l2-backend")]
pub use v4l2::V4l2CameraDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::Image;

/// Camera driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CameraDriverBackend {
    /// Simulation driver (always available)
    #[default]
    Simulation,
    /// OpenCV camera backend
    #[cfg(feature = "opencv-backend")]
    OpenCv,
    /// Video4Linux2 backend
    #[cfg(feature = "v4l2-backend")]
    V4l2,
}

/// Type-erased camera driver for runtime backend selection
pub enum CameraDriver {
    Simulation(SimulationCameraDriver),
    #[cfg(feature = "opencv-backend")]
    OpenCv(OpenCvCameraDriver),
    #[cfg(feature = "v4l2-backend")]
    V4l2(V4l2CameraDriver),
}

impl CameraDriver {
    /// Create a new camera driver with the specified backend
    pub fn new(backend: CameraDriverBackend) -> HorusResult<Self> {
        match backend {
            CameraDriverBackend::Simulation => Ok(Self::Simulation(SimulationCameraDriver::new())),
            #[cfg(feature = "opencv-backend")]
            CameraDriverBackend::OpenCv => Ok(Self::OpenCv(OpenCvCameraDriver::new()?)),
            #[cfg(feature = "v4l2-backend")]
            CameraDriverBackend::V4l2 => Ok(Self::V4l2(V4l2CameraDriver::new()?)),
        }
    }

    /// Create a simulation driver (always available)
    pub fn simulation() -> Self {
        Self::Simulation(SimulationCameraDriver::new())
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "opencv-backend")]
            Self::OpenCv(d) => d.init(),
            #[cfg(feature = "v4l2-backend")]
            Self::V4l2(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "opencv-backend")]
            Self::OpenCv(d) => d.shutdown(),
            #[cfg(feature = "v4l2-backend")]
            Self::V4l2(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "opencv-backend")]
            Self::OpenCv(d) => d.is_available(),
            #[cfg(feature = "v4l2-backend")]
            Self::V4l2(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "opencv-backend")]
            Self::OpenCv(d) => d.status(),
            #[cfg(feature = "v4l2-backend")]
            Self::V4l2(d) => d.status(),
        }
    }

    // ========================================================================
    // Sensor methods
    // ========================================================================

    pub fn read(&mut self) -> HorusResult<Image> {
        match self {
            Self::Simulation(d) => d.read(),
            #[cfg(feature = "opencv-backend")]
            Self::OpenCv(d) => d.read(),
            #[cfg(feature = "v4l2-backend")]
            Self::V4l2(d) => d.read(),
        }
    }

    pub fn has_data(&self) -> bool {
        match self {
            Self::Simulation(d) => d.has_data(),
            #[cfg(feature = "opencv-backend")]
            Self::OpenCv(d) => d.has_data(),
            #[cfg(feature = "v4l2-backend")]
            Self::V4l2(d) => d.has_data(),
        }
    }

    pub fn sample_rate(&self) -> Option<f32> {
        match self {
            Self::Simulation(d) => d.sample_rate(),
            #[cfg(feature = "opencv-backend")]
            Self::OpenCv(d) => d.sample_rate(),
            #[cfg(feature = "v4l2-backend")]
            Self::V4l2(d) => d.sample_rate(),
        }
    }
}
