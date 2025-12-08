//! GPIO-based Stepper driver for hardware step/dir control
//!
//! Uses sysfs GPIO to control stepper drivers on Linux platforms.

use std::thread;
use std::time::Duration;

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use sysfs_gpio::{Direction, Pin};

use super::{StepperConfig, StepperDriverCommand};

/// GPIO-based stepper motor driver
///
/// Controls stepper motors via step/direction GPIO pins.
pub struct GpioStepperDriver {
    config: StepperConfig,
    status: DriverStatus,
    /// Step pins for each motor
    step_pins: [Option<Pin>; 8],
    /// Direction pins for each motor
    dir_pins: [Option<Pin>; 8],
    /// Enable pins for each motor
    enable_pins: [Option<Pin>; 8],
    /// Current position in steps per motor
    current_position: [i64; 8],
    /// Motor enabled state
    enabled: [bool; 8],
}

impl GpioStepperDriver {
    /// Create a new GPIO stepper driver
    pub fn new(config: StepperConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            step_pins: [None, None, None, None, None, None, None, None],
            dir_pins: [None, None, None, None, None, None, None, None],
            enable_pins: [None, None, None, None, None, None, None, None],
            current_position: [0; 8],
            enabled: [false; 8],
        })
    }

    /// Initialize GPIO for a specific motor
    fn init_gpio(&mut self, motor_id: u8) -> HorusResult<()> {
        let idx = motor_id as usize;
        let (step_num, dir_num, enable_num) = self.config.gpio_pins[idx];

        if step_num == 0 || dir_num == 0 {
            return Err(HorusError::driver(format!(
                "GPIO pins not configured for motor {}",
                motor_id
            )));
        }

        // Initialize step pin (output)
        let step = Pin::new(step_num);
        step.export()
            .map_err(|e| HorusError::driver(format!("Failed to export step pin: {}", e)))?;
        thread::sleep(Duration::from_millis(10));
        step.set_direction(Direction::Out)
            .map_err(|e| HorusError::driver(format!("Failed to set step direction: {}", e)))?;
        step.set_value(0)
            .map_err(|e| HorusError::driver(format!("Failed to set step value: {}", e)))?;

        // Initialize direction pin (output)
        let dir = Pin::new(dir_num);
        dir.export()
            .map_err(|e| HorusError::driver(format!("Failed to export dir pin: {}", e)))?;
        thread::sleep(Duration::from_millis(10));
        dir.set_direction(Direction::Out)
            .map_err(|e| HorusError::driver(format!("Failed to set dir direction: {}", e)))?;
        dir.set_value(0)
            .map_err(|e| HorusError::driver(format!("Failed to set dir value: {}", e)))?;

        self.step_pins[idx] = Some(step);
        self.dir_pins[idx] = Some(dir);

        // Initialize enable pin if configured (output, active-low on most drivers)
        if enable_num != 0 {
            let enable = Pin::new(enable_num);
            enable
                .export()
                .map_err(|e| HorusError::driver(format!("Failed to export enable pin: {}", e)))?;
            thread::sleep(Duration::from_millis(10));
            enable.set_direction(Direction::Out).map_err(|e| {
                HorusError::driver(format!("Failed to set enable direction: {}", e))
            })?;
            enable
                .set_value(1) // Start disabled (active-low)
                .map_err(|e| HorusError::driver(format!("Failed to set enable value: {}", e)))?;
            self.enable_pins[idx] = Some(enable);
        }

        Ok(())
    }

    /// Generate step pulses
    fn step(&mut self, motor_id: u8, steps: i64, velocity: f64) -> HorusResult<()> {
        let idx = motor_id as usize;

        let step_pin = self.step_pins[idx]
            .as_ref()
            .ok_or_else(|| HorusError::driver("Step pin not initialized"))?;

        let dir_pin = self.dir_pins[idx]
            .as_ref()
            .ok_or_else(|| HorusError::driver("Dir pin not initialized"))?;

        // Set direction
        let direction = if steps >= 0 { 1 } else { 0 };
        dir_pin
            .set_value(direction)
            .map_err(|e| HorusError::driver(format!("Failed to set direction: {}", e)))?;

        // Enable motor if enable pin exists
        if let Some(ref enable_pin) = self.enable_pins[idx] {
            enable_pin
                .set_value(0) // Enable (active-low)
                .map_err(|e| HorusError::driver(format!("Failed to enable motor: {}", e)))?;
        }

        // Calculate step interval
        let interval_us = if velocity > 0.0 {
            (1_000_000.0 / velocity) as u64
        } else {
            1000 // Default 1ms
        };

        // Generate steps
        let num_steps = steps.abs() as u64;
        for _ in 0..num_steps {
            // Step pulse
            step_pin
                .set_value(1)
                .map_err(|e| HorusError::driver(format!("Failed to set step high: {}", e)))?;
            thread::sleep(Duration::from_micros(self.config.step_pulse_us));
            step_pin
                .set_value(0)
                .map_err(|e| HorusError::driver(format!("Failed to set step low: {}", e)))?;

            // Wait for next step
            if interval_us > self.config.step_pulse_us {
                thread::sleep(Duration::from_micros(
                    interval_us - self.config.step_pulse_us,
                ));
            }
        }

        // Update position
        self.current_position[idx] += steps;

        Ok(())
    }

    /// Get current position for a motor
    pub fn get_position(&self, motor_id: u8) -> Option<i64> {
        if motor_id < 8 {
            Some(self.current_position[motor_id as usize])
        } else {
            None
        }
    }
}

// ========================================================================
// Lifecycle methods
// ========================================================================

impl GpioStepperDriver {
    pub fn init(&mut self) -> HorusResult<()> {
        // Initialize GPIO for each configured motor
        for motor_id in 0..self.config.num_motors {
            let (step_num, dir_num, _) = self.config.gpio_pins[motor_id as usize];
            if step_num != 0 && dir_num != 0 {
                self.init_gpio(motor_id)?;
            }
        }

        // Reset state
        for i in 0..8 {
            self.current_position[i] = 0;
            self.enabled[i] = false;
        }

        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        // Disable all motors
        for motor_id in 0..self.config.num_motors {
            let idx = motor_id as usize;
            if let Some(ref enable_pin) = self.enable_pins[idx] {
                let _ = enable_pin.set_value(1); // Disable (active-low)
            }
            self.enabled[idx] = false;
        }

        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        // Check if any GPIO pins are configured
        self.config
            .gpio_pins
            .iter()
            .any(|&(s, d, _)| s != 0 && d != 0)
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    // ========================================================================
    // Actuator methods
    // ========================================================================

    pub fn write(&mut self, cmd: StepperDriverCommand) -> HorusResult<()> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(HorusError::driver("Driver not initialized"));
        }
        self.status = DriverStatus::Running;

        let idx = cmd.motor_id as usize;
        if idx >= self.config.num_motors as usize {
            return Err(HorusError::driver(format!(
                "Invalid motor ID: {} (max: {})",
                cmd.motor_id,
                self.config.num_motors - 1
            )));
        }

        self.enabled[idx] = cmd.enable;

        if cmd.enable && cmd.steps != 0 {
            // Clamp velocity to max
            let velocity = cmd.velocity.min(self.config.max_velocity);
            self.step(cmd.motor_id, cmd.steps, velocity)?;
        } else if !cmd.enable {
            // Disable motor
            if let Some(ref enable_pin) = self.enable_pins[idx] {
                enable_pin
                    .set_value(1) // Disable (active-low)
                    .map_err(|e| HorusError::driver(format!("Failed to disable motor: {}", e)))?;
            }
        }

        Ok(())
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        // Disable all motors
        for motor_id in 0..self.config.num_motors {
            let idx = motor_id as usize;
            if let Some(ref enable_pin) = self.enable_pins[idx] {
                let _ = enable_pin.set_value(1); // Disable (active-low)
            }
            self.enabled[idx] = false;
        }
        Ok(())
    }
}
