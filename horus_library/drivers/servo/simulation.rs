//! Simulation Servo driver

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::ServoCommand;

/// Simulation servo driver
///
/// Simulates servo behavior without hardware.
pub struct SimulationServoDriver {
    status: DriverStatus,
    positions: [f32; 32], // Support up to 32 servos
    enabled: [bool; 32],
}

impl SimulationServoDriver {
    pub fn new() -> Self {
        Self {
            status: DriverStatus::Uninitialized,
            positions: [0.0; 32],
            enabled: [false; 32],
        }
    }

    /// Get current position for a servo (in radians)
    pub fn get_position(&self, servo_id: u8) -> Option<f32> {
        if servo_id < 32 {
            Some(self.positions[servo_id as usize])
        } else {
            None
        }
    }

    /// Check if a servo is enabled
    pub fn is_enabled(&self, servo_id: u8) -> bool {
        if servo_id < 32 {
            self.enabled[servo_id as usize]
        } else {
            false
        }
    }

    pub fn init(&mut self) -> HorusResult<()> {
        self.positions = [0.0; 32];
        self.enabled = [false; 32];
        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        // Disable all servos on shutdown
        self.enabled = [false; 32];
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        true
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    pub fn write(&mut self, cmd: ServoCommand) -> HorusResult<()> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;

        if cmd.servo_id >= 32 {
            return Err(horus_core::error::HorusError::driver(
                "Servo ID out of range (max 31)",
            ));
        }

        let idx = cmd.servo_id as usize;
        self.enabled[idx] = cmd.enable;

        if cmd.enable {
            // Simulate servo movement (instant for simulation)
            self.positions[idx] = cmd.position;
        }

        Ok(())
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        // Disable all servos
        self.enabled = [false; 32];
        Ok(())
    }
}

impl Default for SimulationServoDriver {
    fn default() -> Self {
        Self::new()
    }
}
