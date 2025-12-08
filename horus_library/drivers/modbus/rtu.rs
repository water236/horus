//! Modbus RTU driver using serialport crate

use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Duration;

use serialport::SerialPort;

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use super::ModbusConfig;

/// Modbus RTU driver
///
/// Hardware driver for Modbus RTU over serial.
pub struct RtuModbusDriver {
    config: ModbusConfig,
    status: Mutex<DriverStatus>,
    port: Mutex<Option<Box<dyn SerialPort + Send>>>,
}

impl RtuModbusDriver {
    pub fn new(config: ModbusConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: Mutex::new(DriverStatus::Uninitialized),
            port: Mutex::new(None),
        })
    }

    /// Calculate CRC16 for Modbus RTU
    fn calculate_crc16(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &byte in data {
            crc ^= byte as u16;
            for _ in 0..8 {
                if crc & 0x0001 != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }

    /// Send a Modbus RTU request and receive response
    fn send_request(&self, function: u8, data: &[u8]) -> HorusResult<Vec<u8>> {
        let slave_id = self.config.slave_id;

        let mut request = vec![slave_id, function];
        request.extend_from_slice(data);

        let crc = Self::calculate_crc16(&request);
        request.push((crc & 0xFF) as u8);
        request.push((crc >> 8) as u8);

        let mut port_guard = self
            .port
            .lock()
            .map_err(|_| HorusError::driver("Lock poisoned"))?;
        let port = port_guard
            .as_mut()
            .ok_or_else(|| HorusError::driver("Serial port not opened"))?;

        port.write_all(&request)
            .map_err(|e| HorusError::driver(format!("Write failed: {}", e)))?;

        let mut response = vec![0u8; 256];
        let bytes_read = port
            .read(&mut response)
            .map_err(|e| HorusError::driver(format!("Read failed: {}", e)))?;

        if bytes_read < 5 {
            return Err(HorusError::driver("Response too short"));
        }

        response.truncate(bytes_read);

        let response_crc =
            ((response[bytes_read - 1] as u16) << 8) | (response[bytes_read - 2] as u16);
        let calculated_crc = Self::calculate_crc16(&response[..bytes_read - 2]);

        if response_crc != calculated_crc {
            return Err(HorusError::driver("CRC mismatch"));
        }

        if response[1] & 0x80 != 0 {
            return Err(HorusError::driver(format!(
                "Modbus exception: 0x{:02X}",
                response[2]
            )));
        }

        Ok(response)
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        let port = serialport::new(&self.config.port, self.config.baud_rate)
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

    pub fn read_bytes(&mut self, addr: u16, len: usize) -> HorusResult<Vec<u8>> {
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

        let count = ((len + 1) / 2) as u16;
        let data = vec![
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            (count >> 8) as u8,
            (count & 0xFF) as u8,
        ];

        let response = self.send_request(0x03, &data)?;

        if response.len() > 5 {
            Ok(response[3..response.len() - 2].to_vec())
        } else {
            Ok(Vec::new())
        }
    }

    pub fn write_bytes(&mut self, addr: u16, data: &[u8]) -> HorusResult<()> {
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

        let count = ((data.len() + 1) / 2) as u16;
        let mut request_data = vec![
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            (count >> 8) as u8,
            (count & 0xFF) as u8,
            data.len() as u8,
        ];
        request_data.extend_from_slice(data);

        let _ = self.send_request(0x10, &request_data)?;
        Ok(())
    }

    // ========================================================================
    // Modbus-specific methods
    // ========================================================================

    pub fn read_holding_registers(&mut self, addr: u16, count: u16) -> HorusResult<Vec<u16>> {
        let data = vec![
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            (count >> 8) as u8,
            (count & 0xFF) as u8,
        ];

        let response = self.send_request(0x03, &data)?;

        let mut registers = Vec::new();
        let data_bytes = &response[3..response.len() - 2];

        for chunk in data_bytes.chunks(2) {
            if chunk.len() == 2 {
                registers.push(((chunk[0] as u16) << 8) | (chunk[1] as u16));
            }
        }

        Ok(registers)
    }

    pub fn read_input_registers(&mut self, addr: u16, count: u16) -> HorusResult<Vec<u16>> {
        let data = vec![
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            (count >> 8) as u8,
            (count & 0xFF) as u8,
        ];

        let response = self.send_request(0x04, &data)?;

        let mut registers = Vec::new();
        let data_bytes = &response[3..response.len() - 2];

        for chunk in data_bytes.chunks(2) {
            if chunk.len() == 2 {
                registers.push(((chunk[0] as u16) << 8) | (chunk[1] as u16));
            }
        }

        Ok(registers)
    }

    pub fn write_single_register(&mut self, addr: u16, value: u16) -> HorusResult<()> {
        let data = vec![
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            (value >> 8) as u8,
            (value & 0xFF) as u8,
        ];

        let _ = self.send_request(0x06, &data)?;
        Ok(())
    }

    pub fn write_multiple_registers(&mut self, addr: u16, values: &[u16]) -> HorusResult<()> {
        let count = values.len() as u16;
        let mut data = vec![
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            (count >> 8) as u8,
            (count & 0xFF) as u8,
            (count * 2) as u8,
        ];

        for &value in values {
            data.push((value >> 8) as u8);
            data.push((value & 0xFF) as u8);
        }

        let _ = self.send_request(0x10, &data)?;
        Ok(())
    }

    pub fn read_coils(&mut self, addr: u16, count: u16) -> HorusResult<Vec<bool>> {
        let data = vec![
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            (count >> 8) as u8,
            (count & 0xFF) as u8,
        ];

        let response = self.send_request(0x01, &data)?;

        let mut coils = Vec::new();
        let data_bytes = &response[3..response.len() - 2];

        for (byte_idx, &byte) in data_bytes.iter().enumerate() {
            for bit in 0..8 {
                let coil_idx = byte_idx * 8 + bit;
                if coil_idx >= count as usize {
                    break;
                }
                coils.push((byte >> bit) & 1 != 0);
            }
        }

        Ok(coils)
    }

    pub fn write_single_coil(&mut self, addr: u16, value: bool) -> HorusResult<()> {
        let data = vec![
            (addr >> 8) as u8,
            (addr & 0xFF) as u8,
            if value { 0xFF } else { 0x00 },
            0x00,
        ];

        let _ = self.send_request(0x05, &data)?;
        Ok(())
    }
}
