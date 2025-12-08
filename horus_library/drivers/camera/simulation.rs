//! Simulation Camera driver
//!
//! Always-available simulation driver that generates synthetic images.

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::{Image, ImageEncoding};

/// Simulation camera configuration
#[derive(Debug, Clone)]
pub struct SimulationCameraConfig {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Frame rate in Hz
    pub fps: f32,
    /// Generate color (RGB) or grayscale
    pub color: bool,
}

impl Default for SimulationCameraConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            fps: 30.0,
            color: true,
        }
    }
}

/// Simulation camera driver
///
/// Generates synthetic images for testing without hardware.
pub struct SimulationCameraDriver {
    config: SimulationCameraConfig,
    status: DriverStatus,
    frame_count: u64,
}

impl SimulationCameraDriver {
    /// Create a new simulation camera driver
    pub fn new() -> Self {
        Self {
            config: SimulationCameraConfig::default(),
            status: DriverStatus::Uninitialized,
            frame_count: 0,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: SimulationCameraConfig) -> Self {
        Self {
            config,
            status: DriverStatus::Uninitialized,
            frame_count: 0,
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
        self.frame_count = 0;
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

    /// Read image data
    pub fn read(&mut self) -> HorusResult<Image> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;
        Ok(self.generate_image())
    }

    /// Check if data is available
    pub fn has_data(&self) -> bool {
        matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.fps)
    }

    fn generate_image(&mut self) -> Image {
        let channels = if self.config.color { 3 } else { 1 };
        let size = (self.config.width * self.config.height * channels) as usize;

        // Generate a simple gradient pattern that changes over time
        let mut data = vec![0u8; size];
        let t = (self.frame_count % 256) as u8;

        for y in 0..self.config.height {
            for x in 0..self.config.width {
                let idx = ((y * self.config.width + x) * channels) as usize;
                if self.config.color {
                    // RGB gradient
                    data[idx] = (x as u8).wrapping_add(t); // R
                    data[idx + 1] = (y as u8).wrapping_add(t); // G
                    data[idx + 2] = t; // B
                } else {
                    data[idx] = ((x + y) as u8).wrapping_add(t);
                }
            }
        }

        self.frame_count += 1;

        let mut frame_id = [0u8; 32];
        let id_bytes = b"simulation_camera";
        frame_id[..id_bytes.len()].copy_from_slice(id_bytes);

        Image {
            width: self.config.width,
            height: self.config.height,
            encoding: if self.config.color {
                ImageEncoding::Rgb8
            } else {
                ImageEncoding::Mono8
            },
            step: self.config.width * channels,
            data,
            frame_id,
            timestamp: self.now_nanos(),
        }
    }
}

impl Default for SimulationCameraDriver {
    fn default() -> Self {
        Self::new()
    }
}
