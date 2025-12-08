//! Intel RealSense Depth Camera driver
//!
//! Hardware driver for Intel RealSense D400 and L500 series cameras.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use super::DepthCameraFrame;

#[cfg(feature = "realsense")]
use realsense_rust::{
    config::Config,
    context::Context,
    kind::{Rs2Format, Rs2StreamKind},
    pipeline::{InactivePipeline, Pipeline},
};

/// RealSense depth camera driver configuration
#[derive(Debug, Clone)]
pub struct RealSenseConfig {
    /// Device serial number (empty for first available)
    pub serial: String,
    /// RGB resolution
    pub rgb_resolution: (u32, u32),
    /// Depth resolution
    pub depth_resolution: (u32, u32),
    /// Frame rate
    pub frame_rate: u32,
}

impl Default for RealSenseConfig {
    fn default() -> Self {
        Self {
            serial: String::new(),
            rgb_resolution: (640, 480),
            depth_resolution: (640, 480),
            frame_rate: 30,
        }
    }
}

/// Intel RealSense depth camera driver
pub struct RealSenseDriver {
    config: RealSenseConfig,
    status: DriverStatus,
    #[cfg(feature = "realsense")]
    pipeline: Option<Pipeline>,
}

impl RealSenseDriver {
    /// Create a new RealSense driver with default configuration
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: RealSenseConfig::default(),
            status: DriverStatus::Uninitialized,
            #[cfg(feature = "realsense")]
            pipeline: None,
        })
    }

    /// Create a new RealSense driver with custom configuration
    pub fn with_config(config: RealSenseConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            #[cfg(feature = "realsense")]
            pipeline: None,
        })
    }

    /// Get the current timestamp in nanoseconds
    fn now_nanos(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

impl Default for RealSenseDriver {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

// ========================================================================
// Lifecycle methods
// ========================================================================

impl RealSenseDriver {
    #[cfg(feature = "realsense")]
    pub fn init(&mut self) -> HorusResult<()> {
        // Create context
        let context = Context::new().map_err(|e| {
            HorusError::driver(format!("Failed to create RealSense context: {:?}", e))
        })?;

        // Query devices
        let devices = context
            .query_devices(None)
            .map_err(|e| HorusError::driver(format!("Failed to query devices: {:?}", e)))?;

        if devices.len() == 0 {
            return Err(HorusError::driver("No RealSense devices found"));
        }

        // Create pipeline
        let pipeline = InactivePipeline::try_from(&context)
            .map_err(|e| HorusError::driver(format!("Failed to create pipeline: {:?}", e)))?;

        // Configure streams
        let mut config = Config::new();

        config
            .enable_stream(
                Rs2StreamKind::Color,
                None,
                self.config.rgb_resolution.0 as usize,
                self.config.rgb_resolution.1 as usize,
                Rs2Format::Rgb8,
                self.config.frame_rate as usize,
            )
            .ok();

        config
            .enable_stream(
                Rs2StreamKind::Depth,
                None,
                self.config.depth_resolution.0 as usize,
                self.config.depth_resolution.1 as usize,
                Rs2Format::Z16,
                self.config.frame_rate as usize,
            )
            .ok();

        // Start pipeline
        let active_pipeline = pipeline
            .start(Some(config))
            .map_err(|e| HorusError::driver(format!("Failed to start pipeline: {:?}", e)))?;

        self.pipeline = Some(active_pipeline);
        self.status = DriverStatus::Ready;
        Ok(())
    }

    #[cfg(not(feature = "realsense"))]
    pub fn init(&mut self) -> HorusResult<()> {
        Err(HorusError::driver("RealSense feature not enabled"))
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        #[cfg(feature = "realsense")]
        {
            self.pipeline = None;
        }
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        cfg!(feature = "realsense")
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    // ========================================================================
    // Sensor methods
    // ========================================================================

    #[cfg(feature = "realsense")]
    pub fn read(&mut self) -> HorusResult<DepthCameraFrame> {
        if self.status != DriverStatus::Ready && self.status != DriverStatus::Running {
            return Err(HorusError::driver("Driver not initialized"));
        }

        let pipeline = self
            .pipeline
            .as_mut()
            .ok_or_else(|| HorusError::driver("Pipeline not initialized"))?;

        let frames = pipeline
            .wait(Some(std::time::Duration::from_millis(1000)))
            .map_err(|e| HorusError::driver(format!("Failed to get frames: {:?}", e)))?;

        let mut rgb_data = Vec::new();
        let mut depth_data = Vec::new();

        if let Some(color_frame) = frames.color_frame() {
            rgb_data = color_frame.get_data().to_vec();
        }

        if let Some(depth_frame) = frames.depth_frame() {
            let data_bytes = depth_frame.get_data();
            for chunk in data_bytes.chunks_exact(2) {
                let value = u16::from_le_bytes([chunk[0], chunk[1]]);
                depth_data.push(value);
            }
        }

        self.status = DriverStatus::Running;

        Ok(DepthCameraFrame {
            rgb_data,
            depth_data,
            rgb_resolution: self.config.rgb_resolution,
            depth_resolution: self.config.depth_resolution,
            timestamp: self.now_nanos(),
        })
    }

    #[cfg(not(feature = "realsense"))]
    pub fn read(&mut self) -> HorusResult<DepthCameraFrame> {
        Err(HorusError::driver("RealSense feature not enabled"))
    }

    pub fn has_data(&self) -> bool {
        matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.frame_rate as f32)
    }
}
