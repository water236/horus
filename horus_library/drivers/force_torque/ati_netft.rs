//! ATI NetFT driver
//!
//! Driver for ATI Industrial Automation NetFT sensors.
//! Requires the `netft` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};
use std::net::UdpSocket;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::WrenchStamped;

/// ATI NetFT configuration
#[derive(Debug, Clone)]
pub struct AtiNetFtConfig {
    /// Sensor IP address
    pub ip_address: String,
    /// UDP port (default: 49152)
    pub port: u16,
    /// Sample rate in Hz
    pub sample_rate: f32,
    /// Force scale factor
    pub force_scale: f64,
    /// Torque scale factor
    pub torque_scale: f64,
}

impl Default for AtiNetFtConfig {
    fn default() -> Self {
        Self {
            ip_address: "192.168.1.1".to_string(),
            port: 49152,
            sample_rate: 1000.0,
            force_scale: 1.0,
            torque_scale: 1.0,
        }
    }
}

/// ATI NetFT driver
pub struct AtiNetFtDriver {
    config: AtiNetFtConfig,
    status: DriverStatus,
    socket: Option<UdpSocket>,
}

impl AtiNetFtDriver {
    /// Create a new ATI NetFT driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: AtiNetFtConfig::default(),
            status: DriverStatus::Uninitialized,
            socket: None,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: AtiNetFtConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            socket: None,
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .map_err(|e| HorusError::driver(format!("Failed to create UDP socket: {}", e)))?;

        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .ok();

        // Connect to sensor
        let addr = format!("{}:{}", self.config.ip_address, self.config.port);
        socket
            .connect(&addr)
            .map_err(|e| HorusError::driver(format!("Failed to connect to sensor: {}", e)))?;

        // Send start streaming command
        let start_cmd: [u8; 8] = [0x12, 0x34, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01];
        socket.send(&start_cmd).ok();

        self.socket = Some(socket);
        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
        // Send stop streaming command
        if let Some(socket) = &self.socket {
            let stop_cmd: [u8; 8] = [0x12, 0x34, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00];
            socket.send(&stop_cmd).ok();
        }

        self.socket = None;
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    /// Check if driver is available
    pub fn is_available(&self) -> bool {
        self.socket.is_some()
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Read force/torque data
    pub fn read(&mut self) -> HorusResult<WrenchStamped> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| HorusError::driver("Sensor not initialized"))?;

        let mut buf = [0u8; 36];
        socket
            .recv(&mut buf)
            .map_err(|e| HorusError::driver(format!("Failed to receive data: {}", e)))?;

        self.status = DriverStatus::Running;

        // Parse NetFT packet (simplified - actual format depends on sensor)
        let fx = i32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]) as f64
            * self.config.force_scale
            / 1000000.0;
        let fy = i32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]) as f64
            * self.config.force_scale
            / 1000000.0;
        let fz = i32::from_be_bytes([buf[20], buf[21], buf[22], buf[23]]) as f64
            * self.config.force_scale
            / 1000000.0;
        let tx = i32::from_be_bytes([buf[24], buf[25], buf[26], buf[27]]) as f64
            * self.config.torque_scale
            / 1000000.0;
        let ty = i32::from_be_bytes([buf[28], buf[29], buf[30], buf[31]]) as f64
            * self.config.torque_scale
            / 1000000.0;
        let tz = i32::from_be_bytes([buf[32], buf[33], buf[34], buf[35]]) as f64
            * self.config.torque_scale
            / 1000000.0;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut frame_id = [0u8; 32];
        let id_bytes = b"ati_netft";
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

impl Default for AtiNetFtDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create ATI NetFT driver")
    }
}
