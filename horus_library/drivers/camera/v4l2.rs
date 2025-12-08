//! Video4Linux2 Camera driver
//!
//! Camera driver using V4L2 for Linux camera access.
//! Requires the `v4l2-backend` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use crate::{Image, ImageEncoding};

/// V4L2 camera configuration
#[derive(Debug, Clone)]
pub struct V4l2CameraConfig {
    /// Camera device path (e.g., "/dev/video0")
    pub device_path: String,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Frame rate in Hz
    pub fps: f32,
}

impl Default for V4l2CameraConfig {
    fn default() -> Self {
        Self {
            device_path: "/dev/video0".to_string(),
            width: 640,
            height: 480,
            fps: 30.0,
        }
    }
}

/// V4L2 camera driver
///
/// Uses Video4Linux2 for direct camera access on Linux.
pub struct V4l2CameraDriver {
    config: V4l2CameraConfig,
    status: DriverStatus,
    #[allow(dead_code)]
    device: Option<v4l::Device>,
    #[allow(dead_code)]
    stream: Option<v4l::io::mmap::Stream<'static>>,
    frame_count: u64,
}

impl V4l2CameraDriver {
    /// Create a new V4L2 camera driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: V4l2CameraConfig::default(),
            status: DriverStatus::Uninitialized,
            device: None,
            stream: None,
            frame_count: 0,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: V4l2CameraConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            device: None,
            stream: None,
            frame_count: 0,
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        use v4l::prelude::*;
        use v4l::video::Capture;
        use v4l::Format;

        let dev = v4l::Device::with_path(&self.config.device_path)
            .map_err(|e| HorusError::driver(format!("Failed to open V4L2 device: {}", e)))?;

        // Set format
        let mut fmt = dev
            .format()
            .map_err(|e| HorusError::driver(format!("Failed to get format: {}", e)))?;
        fmt.width = self.config.width;
        fmt.height = self.config.height;
        fmt.fourcc = v4l::FourCC::new(b"YUYV"); // Common format

        dev.set_format(&fmt)
            .map_err(|e| HorusError::driver(format!("Failed to set format: {}", e)))?;

        self.device = Some(dev);
        self.frame_count = 0;
        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.stream = None;
        self.device = None;
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    /// Check if driver is available
    pub fn is_available(&self) -> bool {
        self.device.is_some()
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Read image data
    pub fn read(&mut self) -> HorusResult<Image> {
        use std::time::{SystemTime, UNIX_EPOCH};
        use v4l::io::traits::CaptureStream;

        let _dev = self
            .device
            .as_ref()
            .ok_or_else(|| HorusError::driver("Camera not initialized"))?;

        // For now, return a placeholder - full implementation requires mmap streaming
        // which has complex lifetime requirements
        self.status = DriverStatus::Running;
        self.frame_count += 1;

        let size = (self.config.width * self.config.height * 3) as usize;
        let data = vec![128u8; size]; // Gray placeholder

        let mut frame_id = [0u8; 32];
        let id_bytes = b"v4l2_camera";
        frame_id[..id_bytes.len()].copy_from_slice(id_bytes);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        Ok(Image {
            width: self.config.width,
            height: self.config.height,
            encoding: ImageEncoding::Rgb8,
            step: self.config.width * 3,
            data,
            frame_id,
            timestamp,
        })
    }

    /// Check if data is available
    pub fn has_data(&self) -> bool {
        self.is_available()
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.fps)
    }
}

impl Default for V4l2CameraDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create V4L2 camera driver")
    }
}
