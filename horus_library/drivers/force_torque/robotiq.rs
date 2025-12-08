//! Robotiq Force/Torque sensor driver
//!
//! Driver for Robotiq FT-300 and similar sensors via serial.
//! Requires the `robotiq-serial` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::WrenchStamped;

/// Robotiq configuration
#[derive(Debug, Clone)]
pub struct RobotiqConfig {
    /// Serial port path
    pub port: String,
    /// Baud rate
    pub baud_rate: u32,
    /// Sample rate in Hz
    pub sample_rate: f32,
}

impl Default for RobotiqConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            sample_rate: 100.0,
        }
    }
}

/// Robotiq serial driver
pub struct RobotiqSerialDriver {
    config: RobotiqConfig,
    status: DriverStatus,
    port: Option<Box<dyn serialport::SerialPort>>,
}

impl RobotiqSerialDriver {
    /// Create a new Robotiq serial driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: RobotiqConfig::default(),
            status: DriverStatus::Uninitialized,
            port: None,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: RobotiqConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            port: None,
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        let port = serialport::new(&self.config.port, self.config.baud_rate)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| HorusError::driver(format!("Failed to open serial port: {}", e)))?;

        self.port = Some(port);
        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.port = None;
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    /// Check if driver is available
    pub fn is_available(&self) -> bool {
        self.port.is_some()
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Read force/torque data
    pub fn read(&mut self) -> HorusResult<WrenchStamped> {
        use std::io::Read;

        let port = self
            .port
            .as_mut()
            .ok_or_else(|| HorusError::driver("Sensor not initialized"))?;

        // Send read command (simplified)
        // Real protocol would need proper Modbus RTU framing
        let mut buf = [0u8; 24];
        port.read_exact(&mut buf)
            .map_err(|e| HorusError::driver(format!("Failed to read: {}", e)))?;

        self.status = DriverStatus::Running;

        // Parse data (simplified - real parsing depends on protocol)
        let fx = i16::from_le_bytes([buf[0], buf[1]]) as f64 / 100.0;
        let fy = i16::from_le_bytes([buf[2], buf[3]]) as f64 / 100.0;
        let fz = i16::from_le_bytes([buf[4], buf[5]]) as f64 / 100.0;
        let tx = i16::from_le_bytes([buf[6], buf[7]]) as f64 / 1000.0;
        let ty = i16::from_le_bytes([buf[8], buf[9]]) as f64 / 1000.0;
        let tz = i16::from_le_bytes([buf[10], buf[11]]) as f64 / 1000.0;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut frame_id = [0u8; 32];
        let id_bytes = b"robotiq_ft";
        frame_id[..id_bytes.len()].copy_from_slice(id_bytes);

        Ok(WrenchStamped {
            fx,
            fy,
            fz,
            tx,
            ty,
            tz,
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
        Some(self.config.sample_rate)
    }
}

impl Default for RobotiqSerialDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create Robotiq serial driver")
    }
}
