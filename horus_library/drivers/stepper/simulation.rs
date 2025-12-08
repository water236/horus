//! Simulation Stepper driver

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use super::{StepperConfig, StepperDriverCommand};

/// Simulation stepper motor driver
///
/// Simulates stepper motor behavior without hardware.
pub struct SimulationStepperDriver {
    config: StepperConfig,
    status: DriverStatus,
    /// Current position in steps per motor
    current_position: [i64; 8],
    /// Target position in steps per motor
    target_position: [i64; 8],
    /// Current velocity in steps/sec per motor
    current_velocity: [f64; 8],
    /// Target velocity in steps/sec per motor
    target_velocity: [f64; 8],
    /// Motor enabled state
    enabled: [bool; 8],
    /// Remaining steps to move
    remaining_steps: [i64; 8],
}

impl SimulationStepperDriver {
    pub fn new(config: StepperConfig) -> Self {
        Self {
            config,
            status: DriverStatus::Uninitialized,
            current_position: [0; 8],
            target_position: [0; 8],
            current_velocity: [0.0; 8],
            target_velocity: [0.0; 8],
            enabled: [false; 8],
            remaining_steps: [0; 8],
        }
    }

    /// Get current position for a motor (in steps)
    pub fn get_position(&self, motor_id: u8) -> Option<i64> {
        if motor_id < 8 {
            Some(self.current_position[motor_id as usize])
        } else {
            None
        }
    }

    /// Get current velocity for a motor (in steps/sec)
    pub fn get_velocity(&self, motor_id: u8) -> Option<f64> {
        if motor_id < 8 {
            Some(self.current_velocity[motor_id as usize])
        } else {
            None
        }
    }

    /// Check if motor is enabled
    pub fn is_enabled(&self, motor_id: u8) -> bool {
        if motor_id < 8 {
            self.enabled[motor_id as usize]
        } else {
            false
        }
    }

    /// Check if motor is moving
    pub fn is_moving(&self, motor_id: u8) -> bool {
        if motor_id < 8 {
            self.remaining_steps[motor_id as usize] != 0
                || self.current_velocity[motor_id as usize].abs() > 0.001
        } else {
            false
        }
    }

    /// Simulate a tick of motion (call periodically)
    pub fn simulate_tick(&mut self, dt: f64) {
        for i in 0..self.config.num_motors as usize {
            if !self.enabled[i] {
                self.current_velocity[i] = 0.0;
                continue;
            }

            // Apply acceleration toward target velocity
            let vel_diff = self.target_velocity[i] - self.current_velocity[i];
            let max_delta = self.config.acceleration * dt;

            if vel_diff.abs() <= max_delta {
                self.current_velocity[i] = self.target_velocity[i];
            } else {
                self.current_velocity[i] += vel_diff.signum() * max_delta;
            }

            // Update position based on velocity
            let steps_this_tick = (self.current_velocity[i] * dt) as i64;

            if self.remaining_steps[i] != 0 {
                // Position mode: count down remaining steps
                let direction = self.remaining_steps[i].signum();
                let steps_to_move = steps_this_tick.abs().min(self.remaining_steps[i].abs());

                self.current_position[i] += direction * steps_to_move;
                self.remaining_steps[i] -= direction * steps_to_move;

                // Check if we've reached target
                if self.remaining_steps[i] == 0 {
                    self.target_velocity[i] = 0.0;
                }
            } else if self.current_velocity[i].abs() > 0.001 {
                // Velocity mode: just update position
                self.current_position[i] += steps_this_tick;
            }
        }
    }

    /// Zero the position for a motor
    pub fn zero_position(&mut self, motor_id: u8) {
        if motor_id < 8 {
            self.current_position[motor_id as usize] = 0;
            self.target_position[motor_id as usize] = 0;
        }
    }

    pub fn init(&mut self) -> HorusResult<()> {
        // Reset all motors
        for i in 0..8 {
            self.current_position[i] = 0;
            self.target_position[i] = 0;
            self.current_velocity[i] = 0.0;
            self.target_velocity[i] = 0.0;
            self.enabled[i] = false;
            self.remaining_steps[i] = 0;
        }
        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        // Stop all motors
        for i in 0..8 {
            self.current_velocity[i] = 0.0;
            self.target_velocity[i] = 0.0;
            self.enabled[i] = false;
            self.remaining_steps[i] = 0;
        }
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        true
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    pub fn write(&mut self, cmd: StepperDriverCommand) -> HorusResult<()> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;

        let idx = cmd.motor_id as usize;
        if idx >= self.config.num_motors as usize {
            return Err(horus_core::error::HorusError::driver(format!(
                "Invalid motor ID: {} (max: {})",
                cmd.motor_id,
                self.config.num_motors - 1
            )));
        }

        self.enabled[idx] = cmd.enable;

        if cmd.enable {
            // Clamp velocity to max
            let velocity = cmd.velocity.min(self.config.max_velocity);

            if cmd.steps == i64::MAX || cmd.steps == i64::MIN {
                // Velocity mode
                self.target_velocity[idx] = if cmd.steps > 0 { velocity } else { -velocity };
                self.remaining_steps[idx] = 0;
            } else {
                // Position mode
                self.remaining_steps[idx] = cmd.steps;
                self.target_velocity[idx] = if cmd.steps >= 0 { velocity } else { -velocity };
            }
        } else {
            self.target_velocity[idx] = 0.0;
            self.remaining_steps[idx] = 0;
        }

        Ok(())
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        for i in 0..self.config.num_motors as usize {
            self.target_velocity[i] = 0.0;
            self.remaining_steps[i] = 0;
            self.enabled[i] = false;
        }
        Ok(())
    }
}

impl Default for SimulationStepperDriver {
    fn default() -> Self {
        Self::new(StepperConfig::default())
    }
}
