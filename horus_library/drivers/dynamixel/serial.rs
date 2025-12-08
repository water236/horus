//! Serial Dynamixel driver using serialport crate

use std::io::{Read, Write};
use std::sync::Mutex;
use std::time::Duration;

use serialport::SerialPort;

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use super::{DynamixelCommand, DynamixelConfig, DynamixelMode, DynamixelProtocol};

/// Serial Dynamixel driver
///
/// Hardware driver for Dynamixel servos over serial (TTL/RS-485).
pub struct SerialDynamixelDriver {
    config: DynamixelConfig,
    status: Mutex<DriverStatus>,
    port: Mutex<Option<Box<dyn SerialPort + Send>>>,
}

impl SerialDynamixelDriver {
    pub fn new(config: DynamixelConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: Mutex::new(DriverStatus::Uninitialized),
            port: Mutex::new(None),
        })
    }

    /// Build a Dynamixel protocol 1.0 packet
    fn build_packet_v1(&self, id: u8, instruction: u8, params: &[u8]) -> Vec<u8> {
        let length = (params.len() + 2) as u8;
        let mut packet = vec![0xFF, 0xFF, id, length, instruction];
        packet.extend_from_slice(params);
        let checksum = (!packet[2..].iter().fold(0u8, |acc, &x| acc.wrapping_add(x))) & 0xFF;
        packet.push(checksum);
        packet
    }

    /// Build a Dynamixel protocol 2.0 packet
    fn build_packet_v2(&self, id: u8, instruction: u8, params: &[u8]) -> Vec<u8> {
        let length = (params.len() + 3) as u16;
        let mut packet = vec![0xFF, 0xFF, 0xFD, 0x00, id];
        packet.extend_from_slice(&length.to_le_bytes());
        packet.push(instruction);
        packet.extend_from_slice(params);
        let crc = self.calculate_crc16(&packet);
        packet.extend_from_slice(&crc.to_le_bytes());
        packet
    }

    fn calculate_crc16(&self, data: &[u8]) -> u16 {
        let mut crc: u16 = 0;
        for &byte in data {
            crc = crc.wrapping_add(byte as u16);
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

        let mut response = [0u8; 16];
        let _ = port.read(&mut response);
        Ok(())
    }

    fn write_goal_position(&self, servo_id: u8, position: u16) -> HorusResult<()> {
        let params = match self.config.protocol {
            DynamixelProtocol::V1 => vec![30, position as u8, (position >> 8) as u8],
            DynamixelProtocol::V2 => {
                let pos32 = position as u32;
                vec![
                    116,
                    0,
                    pos32 as u8,
                    (pos32 >> 8) as u8,
                    (pos32 >> 16) as u8,
                    (pos32 >> 24) as u8,
                ]
            }
        };

        let packet = match self.config.protocol {
            DynamixelProtocol::V1 => self.build_packet_v1(servo_id, 0x03, &params),
            DynamixelProtocol::V2 => self.build_packet_v2(servo_id, 0x03, &params),
        };

        self.send_packet(&packet)
    }

    fn set_torque(&self, servo_id: u8, enable: bool) -> HorusResult<()> {
        let (addr, value) = match self.config.protocol {
            DynamixelProtocol::V1 => (24u8, if enable { 1u8 } else { 0u8 }),
            DynamixelProtocol::V2 => (64u8, if enable { 1u8 } else { 0u8 }),
        };

        let params = vec![addr, value];
        let packet = match self.config.protocol {
            DynamixelProtocol::V1 => self.build_packet_v1(servo_id, 0x03, &params),
            DynamixelProtocol::V2 => self.build_packet_v2(servo_id, 0x03, &params),
        };

        self.send_packet(&packet)
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
        let servo_ids = self.config.servo_ids.clone();
        for id in servo_ids {
            let _ = self.set_torque(id, false);
        }
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

    pub fn write(&mut self, cmd: DynamixelCommand) -> HorusResult<()> {
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

        self.set_torque(cmd.servo_id, cmd.torque_enable)?;

        if cmd.torque_enable {
            match cmd.mode {
                DynamixelMode::Position
                | DynamixelMode::ExtendedPosition
                | DynamixelMode::CurrentBasedPosition => {
                    let raw_pos = ((cmd.target / 360.0) * 4095.0).clamp(0.0, 4095.0) as u16;
                    self.write_goal_position(cmd.servo_id, raw_pos)?;
                }
                DynamixelMode::Velocity | DynamixelMode::PWM => {
                    let raw_pos = ((cmd.target / 360.0) * 4095.0).clamp(0.0, 4095.0) as u16;
                    self.write_goal_position(cmd.servo_id, raw_pos)?;
                }
            }
        }

        Ok(())
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        let servo_ids = self.config.servo_ids.clone();
        for id in servo_ids {
            self.set_torque(id, false)?;
        }
        Ok(())
    }
}
