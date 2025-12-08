//! Simulation Serial driver

use std::collections::VecDeque;

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use super::SerialConfig;

/// Simulation serial driver
///
/// Simulates serial port behavior with a loopback buffer.
pub struct SimulationSerialDriver {
    config: SerialConfig,
    status: DriverStatus,
    /// Loopback buffer for simulation
    rx_buffer: VecDeque<u8>,
    /// Bytes transmitted (for statistics)
    bytes_tx: u64,
    /// Bytes received (for statistics)
    bytes_rx: u64,
}

impl SimulationSerialDriver {
    pub fn new(config: SerialConfig) -> Self {
        Self {
            config,
            status: DriverStatus::Uninitialized,
            rx_buffer: VecDeque::new(),
            bytes_tx: 0,
            bytes_rx: 0,
        }
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        self.rx_buffer.clear();
        self.bytes_tx = 0;
        self.bytes_rx = 0;
        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.rx_buffer.clear();
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        true // Simulation is always available
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    // ========================================================================
    // Bus methods
    // ========================================================================

    pub fn read_bytes(&mut self, len: usize) -> HorusResult<Vec<u8>> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(HorusError::driver("Driver not initialized"));
        }
        self.status = DriverStatus::Running;

        let mut result = Vec::with_capacity(len);
        for _ in 0..len {
            if let Some(byte) = self.rx_buffer.pop_front() {
                result.push(byte);
                self.bytes_rx += 1;
            } else {
                break;
            }
        }
        Ok(result)
    }

    pub fn write_bytes(&mut self, data: &[u8]) -> HorusResult<()> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(HorusError::driver("Driver not initialized"));
        }
        self.status = DriverStatus::Running;

        // In simulation mode, loopback the data
        self.rx_buffer.extend(data);
        self.bytes_tx += data.len() as u64;
        Ok(())
    }

    // ========================================================================
    // Query methods
    // ========================================================================

    /// Get the configured port path
    pub fn port(&self) -> &str {
        &self.config.port
    }

    /// Get the configured baud rate
    pub fn baud_rate(&self) -> u32 {
        self.config.baud_rate
    }

    /// Get bytes transmitted
    pub fn bytes_transmitted(&self) -> u64 {
        self.bytes_tx
    }

    /// Get bytes received
    pub fn bytes_received(&self) -> u64 {
        self.bytes_rx
    }

    /// Check if data is available to read
    pub fn has_data(&self) -> bool {
        !self.rx_buffer.is_empty()
    }

    /// Inject data into the receive buffer (for testing)
    pub fn inject_rx_data(&mut self, data: &[u8]) {
        self.rx_buffer.extend(data);
    }
}

impl Default for SimulationSerialDriver {
    fn default() -> Self {
        Self::new(SerialConfig::default())
    }
}
