//! SocketCAN driver
//!
//! CAN bus driver using Linux SocketCAN interface.
//! Requires the `can-hardware` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

/// SocketCAN driver configuration
#[derive(Debug, Clone)]
pub struct SocketCanConfig {
    /// CAN interface name (e.g., "can0", "vcan0")
    pub interface: String,
    /// Bitrate in bits/second (only used when bringing up interface)
    pub bitrate: u32,
}

impl Default for SocketCanConfig {
    fn default() -> Self {
        Self {
            interface: "can0".to_string(),
            bitrate: 500_000,
        }
    }
}

/// CAN frame
#[derive(Debug, Clone)]
pub struct CanFrame {
    /// CAN ID (11 or 29 bit)
    pub id: u32,
    /// Extended frame flag
    pub extended: bool,
    /// Remote transmission request
    pub rtr: bool,
    /// Data (0-8 bytes for CAN 2.0, 0-64 for CAN FD)
    pub data: Vec<u8>,
}

/// SocketCAN driver
pub struct SocketCanDriver {
    config: SocketCanConfig,
    status: DriverStatus,
    socket: Option<socketcan::CANSocket>,
}

impl SocketCanDriver {
    /// Create a new SocketCAN driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: SocketCanConfig::default(),
            status: DriverStatus::Uninitialized,
            socket: None,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: SocketCanConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            socket: None,
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        let socket = socketcan::CANSocket::open(&self.config.interface)
            .map_err(|e| HorusError::driver(format!("Failed to open CAN socket: {}", e)))?;

        self.socket = Some(socket);
        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
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

    /// Send a CAN frame
    pub fn send(&mut self, frame: &CanFrame) -> HorusResult<()> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| HorusError::driver("CAN socket not initialized"))?;

        let can_frame = socketcan::CANFrame::new(frame.id, &frame.data, frame.rtr, frame.extended)
            .map_err(|e| HorusError::driver(format!("Failed to create CAN frame: {}", e)))?;

        socket
            .write_frame(&can_frame)
            .map_err(|e| HorusError::driver(format!("Failed to send CAN frame: {}", e)))?;

        self.status = DriverStatus::Running;
        Ok(())
    }

    /// Receive a CAN frame (blocking)
    pub fn receive(&mut self) -> HorusResult<CanFrame> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| HorusError::driver("CAN socket not initialized"))?;

        let frame = socket
            .read_frame()
            .map_err(|e| HorusError::driver(format!("Failed to receive CAN frame: {}", e)))?;

        self.status = DriverStatus::Running;

        Ok(CanFrame {
            id: frame.id(),
            extended: frame.is_extended(),
            rtr: frame.is_rtr(),
            data: frame.data().to_vec(),
        })
    }
}

impl Default for SocketCanDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create SocketCAN driver")
    }
}
