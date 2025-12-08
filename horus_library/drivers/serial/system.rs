//! System serial port driver using serialport crate

use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Duration;

use serialport::SerialPort;

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use super::{SerialConfig, SerialFlowControl, SerialParity};

/// System serial port driver
///
/// Hardware serial driver using the serialport crate for real UART communication.
pub struct SystemSerialDriver {
    config: SerialConfig,
    status: Mutex<DriverStatus>,
    port: Mutex<Option<Box<dyn SerialPort + Send>>>,
}

impl SystemSerialDriver {
    pub fn new(config: SerialConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: Mutex::new(DriverStatus::Uninitialized),
            port: Mutex::new(None),
        })
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        let data_bits = match self.config.data_bits {
            5 => serialport::DataBits::Five,
            6 => serialport::DataBits::Six,
            7 => serialport::DataBits::Seven,
            8 => serialport::DataBits::Eight,
            _ => serialport::DataBits::Eight,
        };

        let stop_bits = match self.config.stop_bits {
            1 => serialport::StopBits::One,
            2 => serialport::StopBits::Two,
            _ => serialport::StopBits::One,
        };

        let parity = match self.config.parity {
            SerialParity::None => serialport::Parity::None,
            SerialParity::Even => serialport::Parity::Even,
            SerialParity::Odd => serialport::Parity::Odd,
        };

        let flow_control = match self.config.flow_control {
            SerialFlowControl::None => serialport::FlowControl::None,
            SerialFlowControl::Hardware => serialport::FlowControl::Hardware,
            SerialFlowControl::Software => serialport::FlowControl::Software,
        };

        let port = serialport::new(&self.config.port, self.config.baud_rate)
            .data_bits(data_bits)
            .stop_bits(stop_bits)
            .parity(parity)
            .flow_control(flow_control)
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .open()
            .map_err(|e| {
                HorusError::driver(format!(
                    "Failed to open serial port {}: {}",
                    self.config.port, e
                ))
            })?;

        *self
            .port
            .lock()
            .map_err(|_| HorusError::driver("Lock poisoned"))? = Some(port);
        *self
            .status
            .lock()
            .map_err(|_| HorusError::driver("Lock poisoned"))? = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        *self
            .port
            .lock()
            .map_err(|_| HorusError::driver("Lock poisoned"))? = None;
        *self
            .status
            .lock()
            .map_err(|_| HorusError::driver("Lock poisoned"))? = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        serialport::available_ports()
            .map(|ports| ports.iter().any(|p| p.port_name == self.config.port))
            .unwrap_or(false)
    }

    pub fn status(&self) -> DriverStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(DriverStatus::Error("Lock poisoned".to_string()))
    }

    // ========================================================================
    // Bus methods
    // ========================================================================

    pub fn read_bytes(&mut self, len: usize) -> HorusResult<Vec<u8>> {
        {
            let status = self
                .status
                .lock()
                .map_err(|_| HorusError::driver("Lock poisoned"))?;
            if !matches!(*status, DriverStatus::Ready | DriverStatus::Running) {
                return Err(HorusError::driver("Driver not initialized"));
            }
        }
        *self
            .status
            .lock()
            .map_err(|_| HorusError::driver("Lock poisoned"))? = DriverStatus::Running;

        let mut port_guard = self
            .port
            .lock()
            .map_err(|_| HorusError::driver("Lock poisoned"))?;
        let port = port_guard
            .as_mut()
            .ok_or_else(|| HorusError::driver("Serial port not opened"))?;

        let mut buffer = vec![0u8; len];
        let bytes_read = port
            .read(&mut buffer)
            .map_err(|e| HorusError::driver(format!("Read failed: {}", e)))?;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    pub fn write_bytes(&mut self, data: &[u8]) -> HorusResult<()> {
        {
            let status = self
                .status
                .lock()
                .map_err(|_| HorusError::driver("Lock poisoned"))?;
            if !matches!(*status, DriverStatus::Ready | DriverStatus::Running) {
                return Err(HorusError::driver("Driver not initialized"));
            }
        }
        *self
            .status
            .lock()
            .map_err(|_| HorusError::driver("Lock poisoned"))? = DriverStatus::Running;

        let mut port_guard = self
            .port
            .lock()
            .map_err(|_| HorusError::driver("Lock poisoned"))?;
        let port = port_guard
            .as_mut()
            .ok_or_else(|| HorusError::driver("Serial port not opened"))?;

        port.write_all(data)
            .map_err(|e| HorusError::driver(format!("Write failed: {}", e)))?;
        Ok(())
    }
}
