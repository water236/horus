//! Serial RoboClaw driver using serialport crate

use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Duration;

use serialport::SerialPort;

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use super::{RoboclawCommand, RoboclawConfig, RoboclawMode};

/// Serial RoboClaw driver
///
/// Hardware driver for RoboClaw motor controllers over serial.
pub struct SerialRoboclawDriver {
    config: RoboclawConfig,
    status: Mutex<DriverStatus>,
    port: Mutex<Option<Box<dyn SerialPort + Send>>>,
}

impl SerialRoboclawDriver {
    pub fn new(config: RoboclawConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: Mutex::new(DriverStatus::Uninitialized),
            port: Mutex::new(None),
        })
    }

    /// Calculate CRC16 checksum for RoboClaw packet mode
    fn calculate_crc16(data: &[u8]) -> u16 {
        let mut crc: u16 = 0;
        for &byte in data {
            crc ^= (byte as u16) << 8;
            for _ in 0..8 {
                if crc & 0x8000 != 0 {
                    crc = (crc << 1) ^ 0x1021;
                } else {
                    crc <<= 1;
                }
            }
        }
        crc
    }

    fn send_packet(&self, packet: &[u8]) -> HorusResult<()> {
        let mut port_guard = self
            .port
            .lock()
            .map_err(|_| HorusError::driver("Port lock poisoned"))?;
        let port = port_guard
            .as_mut()
            .ok_or_else(|| HorusError::driver("Serial port not opened"))?;

        port.write_all(packet)
            .map_err(|e| HorusError::driver(format!("Write failed: {}", e)))?;

        let mut ack = [0u8; 1];
        let _ = port.read(&mut ack);
        Ok(())
    }

    /// Send a command with CRC
    fn send_command(&self, cmd: u8, data: &[u8]) -> HorusResult<()> {
        let mut packet = vec![self.config.address, cmd];
        packet.extend_from_slice(data);

        let crc = Self::calculate_crc16(&packet);
        packet.push((crc >> 8) as u8);
        packet.push((crc & 0xFF) as u8);

        self.send_packet(&packet)
    }

    /// Set motor duty (-32767 to 32767)
    fn set_duty(&self, channel: u8, duty: i16) -> HorusResult<()> {
        let cmd = if channel == 1 { 32 } else { 33 };
        let data = [(duty >> 8) as u8, (duty & 0xFF) as u8];
        self.send_command(cmd, &data)
    }

    /// Set motor velocity (requires encoder feedback)
    fn set_velocity(&self, channel: u8, velocity: i32, accel: u32) -> HorusResult<()> {
        let cmd = if channel == 1 { 35 } else { 36 };
        let mut data = Vec::new();
        data.extend_from_slice(&velocity.to_be_bytes());
        data.extend_from_slice(&accel.to_be_bytes());
        self.send_command(cmd, &data)
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        let port = serialport::new(&self.config.port, self.config.baud_rate)
            .timeout(Duration::from_millis(100))
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
        let _ = self.set_duty(1, 0);
        let _ = self.set_duty(2, 0);
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
    // Actuator methods
    // ========================================================================

    pub fn write(&mut self, cmd: RoboclawCommand) -> HorusResult<()> {
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

        match cmd.mode {
            RoboclawMode::Duty => {
                let duty = (cmd.target * 32767.0).clamp(-32767.0, 32767.0) as i16;
                self.set_duty(cmd.channel, duty)?;
            }
            RoboclawMode::Velocity => {
                self.set_velocity(cmd.channel, cmd.target as i32, cmd.acceleration)?;
            }
            RoboclawMode::Position => {
                self.set_velocity(cmd.channel, cmd.target as i32, cmd.acceleration)?;
            }
        }

        Ok(())
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        self.set_duty(1, 0)?;
        self.set_duty(2, 0)?;
        Ok(())
    }
}
