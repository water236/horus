//! Simulation Motor driver

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::MotorCommand;

/// Simulation motor driver
///
/// Simulates motor behavior without hardware.
pub struct SimulationMotorDriver {
    status: DriverStatus,
    current_speed: f32,
    target_speed: f32,
}

impl SimulationMotorDriver {
    pub fn new() -> Self {
        Self {
            status: DriverStatus::Uninitialized,
            current_speed: 0.0,
            target_speed: 0.0,
        }
    }

    /// Get current motor speed (-1.0 to 1.0)
    pub fn current_speed(&self) -> f32 {
        self.current_speed
    }

    /// Get target motor speed (-1.0 to 1.0)
    pub fn target_speed(&self) -> f32 {
        self.target_speed
    }

    pub fn init(&mut self) -> HorusResult<()> {
        self.current_speed = 0.0;
        self.target_speed = 0.0;
        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.current_speed = 0.0;
        self.target_speed = 0.0;
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        true
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    pub fn write(&mut self, cmd: MotorCommand) -> HorusResult<()> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;

        // Clamp speed to valid range (mode 0 = velocity mode)
        self.target_speed = (cmd.target as f32).clamp(-1.0, 1.0);

        // Simulate motor response (instant for now)
        self.current_speed = self.target_speed;

        Ok(())
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        self.target_speed = 0.0;
        self.current_speed = 0.0;
        Ok(())
    }
}

impl Default for SimulationMotorDriver {
    fn default() -> Self {
        Self::new()
    }
}
