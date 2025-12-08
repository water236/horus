//! OpenCV Camera driver
//!
//! Camera driver using OpenCV for hardware camera access.
//! Requires the `opencv-backend` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use crate::{Image, ImageEncoding};

/// OpenCV camera configuration
#[derive(Debug, Clone)]
pub struct OpenCvCameraConfig {
    /// Camera device index (usually 0 for default camera)
    pub device_index: i32,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Frame rate in Hz
    pub fps: f32,
}

impl Default for OpenCvCameraConfig {
    fn default() -> Self {
        Self {
            device_index: 0,
            width: 640,
            height: 480,
            fps: 30.0,
        }
    }
}

/// OpenCV camera driver
///
/// Uses OpenCV's VideoCapture for hardware camera access.
pub struct OpenCvCameraDriver {
    config: OpenCvCameraConfig,
    status: DriverStatus,
    #[allow(dead_code)]
    capture: Option<opencv::videoio::VideoCapture>,
    frame_count: u64,
}

impl OpenCvCameraDriver {
    /// Create a new OpenCV camera driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: OpenCvCameraConfig::default(),
            status: DriverStatus::Uninitialized,
            capture: None,
            frame_count: 0,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: OpenCvCameraConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            capture: None,
            frame_count: 0,
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        use opencv::prelude::*;
        use opencv::videoio;

        let mut cap = videoio::VideoCapture::new(self.config.device_index, videoio::CAP_ANY)
            .map_err(|e| HorusError::driver(format!("Failed to open camera: {}", e)))?;

        if !cap.is_opened().unwrap_or(false) {
            return Err(HorusError::driver("Camera device not available"));
        }

        // Set resolution
        cap.set(videoio::CAP_PROP_FRAME_WIDTH, self.config.width as f64)
            .ok();
        cap.set(videoio::CAP_PROP_FRAME_HEIGHT, self.config.height as f64)
            .ok();
        cap.set(videoio::CAP_PROP_FPS, self.config.fps as f64).ok();

        self.capture = Some(cap);
        self.frame_count = 0;
        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.capture = None;
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    /// Check if driver is available
    pub fn is_available(&self) -> bool {
        self.capture
            .as_ref()
            .map(|c| c.is_opened().unwrap_or(false))
            .unwrap_or(false)
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Read image data
    pub fn read(&mut self) -> HorusResult<Image> {
        use opencv::prelude::*;
        use std::time::{SystemTime, UNIX_EPOCH};

        let cap = self
            .capture
            .as_mut()
            .ok_or_else(|| HorusError::driver("Camera not initialized"))?;

        let mut frame = opencv::core::Mat::default();
        cap.read(&mut frame)
            .map_err(|e| HorusError::driver(format!("Failed to read frame: {}", e)))?;

        if frame.empty() {
            return Err(HorusError::driver("Empty frame received"));
        }

        self.status = DriverStatus::Running;
        self.frame_count += 1;

        // Convert OpenCV Mat to Image
        let rows = frame.rows() as u32;
        let cols = frame.cols() as u32;
        let channels = frame.channels() as u32;

        let data = frame
            .data_bytes()
            .map_err(|e| HorusError::driver(format!("Failed to get frame data: {}", e)))?
            .to_vec();

        let encoding = match channels {
            1 => ImageEncoding::Mono8,
            3 => ImageEncoding::Bgr8, // OpenCV uses BGR
            4 => ImageEncoding::Bgra8,
            _ => ImageEncoding::Rgb8,
        };

        let mut frame_id = [0u8; 32];
        let id_bytes = format!("opencv_cam_{}", self.config.device_index);
        let id_len = id_bytes.len().min(32);
        frame_id[..id_len].copy_from_slice(&id_bytes.as_bytes()[..id_len]);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        Ok(Image {
            width: cols,
            height: rows,
            encoding,
            step: cols * channels,
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

impl Default for OpenCvCameraDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create OpenCV camera driver")
    }
}
