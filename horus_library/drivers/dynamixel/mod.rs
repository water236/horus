//! Dynamixel servo drivers
//!
//! This module provides drivers for Dynamixel smart servos.
//!
//! # Available Drivers
//!
//! - `SimulationDynamixelDriver` - Always available, simulates Dynamixel behavior
//! - `SerialDynamixelDriver` - Serial protocol (requires `serial-hardware` feature)

mod simulation;

#[cfg(feature = "serial-hardware")]
mod serial;

pub use simulation::SimulationDynamixelDriver;

#[cfg(feature = "serial-hardware")]
pub use serial::SerialDynamixelDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

/// Dynamixel servo command
#[derive(Debug, Clone, Copy, Default)]
pub struct DynamixelCommand {
    /// Servo ID (0-253)
    pub servo_id: u8,
    /// Control mode
    pub mode: DynamixelMode,
    /// Target value (position in degrees, velocity in RPM, or PWM duty)
    pub target: f64,
    /// Enable torque
    pub torque_enable: bool,
}

impl DynamixelCommand {
    pub fn position(servo_id: u8, degrees: f64) -> Self {
        Self {
            servo_id,
            mode: DynamixelMode::Position,
            target: degrees,
            torque_enable: true,
        }
    }

    pub fn velocity(servo_id: u8, rpm: f64) -> Self {
        Self {
            servo_id,
            mode: DynamixelMode::Velocity,
            target: rpm,
            torque_enable: true,
        }
    }

    pub fn stop(servo_id: u8) -> Self {
        Self {
            servo_id,
            mode: DynamixelMode::Position,
            target: 0.0,
            torque_enable: false,
        }
    }
}

/// Dynamixel control mode
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DynamixelMode {
    #[default]
    Position,
    Velocity,
    ExtendedPosition,
    PWM,
    CurrentBasedPosition,
}

/// Dynamixel protocol version
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DynamixelProtocol {
    #[default]
    V1,
    V2,
}

/// Dynamixel configuration
#[derive(Debug, Clone)]
pub struct DynamixelConfig {
    /// Serial port path
    pub port: String,
    /// Baud rate
    pub baud_rate: u32,
    /// Protocol version
    pub protocol: DynamixelProtocol,
    /// Servo IDs to manage
    pub servo_ids: Vec<u8>,
}

impl Default for DynamixelConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 1000000,
            protocol: DynamixelProtocol::V2,
            servo_ids: vec![1],
        }
    }
}

/// Dynamixel driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DynamixelDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "serial-hardware")]
    Serial,
}

/// Type-erased Dynamixel driver
pub enum DynamixelDriver {
    Simulation(SimulationDynamixelDriver),
    #[cfg(feature = "serial-hardware")]
    Serial(SerialDynamixelDriver),
}

impl DynamixelDriver {
    pub fn new(backend: DynamixelDriverBackend, config: DynamixelConfig) -> HorusResult<Self> {
        match backend {
            DynamixelDriverBackend::Simulation => {
                Ok(Self::Simulation(SimulationDynamixelDriver::new(config)))
            }
            #[cfg(feature = "serial-hardware")]
            DynamixelDriverBackend::Serial => Ok(Self::Serial(SerialDynamixelDriver::new(config)?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationDynamixelDriver::new(DynamixelConfig::default()))
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.status(),
        }
    }

    // ========================================================================
    // Actuator methods
    // ========================================================================

    pub fn write(&mut self, cmd: DynamixelCommand) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.write(cmd),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.write(cmd),
        }
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.stop(),
            #[cfg(feature = "serial-hardware")]
            Self::Serial(d) => d.stop(),
        }
    }
}
