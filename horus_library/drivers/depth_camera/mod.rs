//! Depth Camera drivers
//!
//! This module provides drivers for RGB-D cameras.
//!
//! # Available Drivers
//!
//! - `SimulationDepthCameraDriver` - Always available, generates synthetic data
//! - `RealSenseDriver` - Intel RealSense cameras (requires `realsense` feature)
//! - `ZedDriver` - Stereolabs ZED cameras (requires `zed` feature)

mod simulation;

#[cfg(feature = "realsense")]
mod realsense;

// Re-exports
pub use simulation::SimulationDepthCameraDriver;

#[cfg(feature = "realsense")]
pub use realsense::RealSenseDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

/// Composite output from a depth camera
///
/// Contains both RGB and depth data from a single capture frame.
#[derive(Debug, Clone)]
pub struct DepthCameraFrame {
    /// RGB image data (width * height * 3 bytes)
    pub rgb_data: Vec<u8>,
    /// Depth data (width * height u16 values)
    pub depth_data: Vec<u16>,
    /// RGB resolution (width, height)
    pub rgb_resolution: (u32, u32),
    /// Depth resolution (width, height)
    pub depth_resolution: (u32, u32),
    /// Timestamp in nanoseconds
    pub timestamp: u64,
}

impl Default for DepthCameraFrame {
    fn default() -> Self {
        Self {
            rgb_data: Vec::new(),
            depth_data: Vec::new(),
            rgb_resolution: (640, 480),
            depth_resolution: (640, 480),
            timestamp: 0,
        }
    }
}

/// Enum of all available depth camera driver backends
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DepthCameraDriverBackend {
    /// Simulation driver (always available)
    #[default]
    Simulation,
    /// Intel RealSense cameras
    #[cfg(feature = "realsense")]
    RealSense,
    /// Stereolabs ZED cameras
    #[cfg(feature = "zed")]
    Zed,
}

/// Type-erased depth camera driver for runtime backend selection
pub enum DepthCameraDriver {
    Simulation(SimulationDepthCameraDriver),
    #[cfg(feature = "realsense")]
    RealSense(RealSenseDriver),
}

impl DepthCameraDriver {
    /// Create a new depth camera driver with the specified backend
    pub fn new(backend: DepthCameraDriverBackend) -> HorusResult<Self> {
        match backend {
            DepthCameraDriverBackend::Simulation => {
                Ok(Self::Simulation(SimulationDepthCameraDriver::new()))
            }
            #[cfg(feature = "realsense")]
            DepthCameraDriverBackend::RealSense => Ok(Self::RealSense(RealSenseDriver::new()?)),
            #[cfg(feature = "zed")]
            DepthCameraDriverBackend::Zed => {
                // ZED support pending
                Ok(Self::Simulation(SimulationDepthCameraDriver::new()))
            }
        }
    }

    /// Create a simulation driver (always available)
    pub fn simulation() -> Self {
        Self::Simulation(SimulationDepthCameraDriver::new())
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "realsense")]
            Self::RealSense(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "realsense")]
            Self::RealSense(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "realsense")]
            Self::RealSense(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "realsense")]
            Self::RealSense(d) => d.status(),
        }
    }

    // ========================================================================
    // Sensor methods
    // ========================================================================

    pub fn read(&mut self) -> HorusResult<DepthCameraFrame> {
        match self {
            Self::Simulation(d) => d.read(),
            #[cfg(feature = "realsense")]
            Self::RealSense(d) => d.read(),
        }
    }

    pub fn has_data(&self) -> bool {
        match self {
            Self::Simulation(d) => d.has_data(),
            #[cfg(feature = "realsense")]
            Self::RealSense(d) => d.has_data(),
        }
    }

    pub fn sample_rate(&self) -> Option<f32> {
        match self {
            Self::Simulation(d) => d.sample_rate(),
            #[cfg(feature = "realsense")]
            Self::RealSense(d) => d.sample_rate(),
        }
    }
}
