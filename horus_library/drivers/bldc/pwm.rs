//! PWM-based BLDC driver for hardware ESC control
//!
//! Uses hardware PWM to control ESCs on Raspberry Pi or similar platforms.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use rppal::pwm::{Channel, Polarity, Pwm};

use super::{BldcCommand, BldcConfig, BldcProtocol};

/// PWM-based BLDC motor driver
///
/// Controls BLDC motors via ESC using hardware PWM signals.
pub struct PwmBldcDriver {
    config: BldcConfig,
    status: DriverStatus,
    /// PWM channels for each motor
    pwm_channels: [Option<Pwm>; 8],
    /// Current throttle per motor
    current_throttle: [f64; 8],
    /// Armed state per motor
    armed: [bool; 8],
}

impl PwmBldcDriver {
    /// Create a new PWM BLDC driver
    pub fn new(config: BldcConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            pwm_channels: [None, None, None, None, None, None, None, None],
            current_throttle: [0.0; 8],
            armed: [false; 8],
        })
    }

    /// Initialize PWM for a specific motor
    fn init_pwm(&mut self, motor_id: u8) -> HorusResult<()> {
        let idx = motor_id as usize;
        let gpio_pin = self.config.gpio_pins[idx];

        if gpio_pin == 0 {
            return Err(HorusError::driver(format!(
                "GPIO pin not configured for motor {}",
                motor_id
            )));
        }

        // Map GPIO pin to PWM channel (Raspberry Pi specific)
        // GPIO 12 & 13 = PWM0, GPIO 18 & 19 = PWM1
        let channel = match gpio_pin {
            12 | 13 => Channel::Pwm0,
            18 | 19 => Channel::Pwm1,
            _ => {
                return Err(HorusError::driver(format!(
                    "Invalid PWM GPIO pin {} (use 12, 13, 18, or 19)",
                    gpio_pin
                )));
            }
        };

        // Initialize PWM
        let pwm = Pwm::with_frequency(
            channel,
            self.config.pwm_frequency_hz,
            0.0, // Start at 0% duty cycle
            Polarity::Normal,
            true, // enabled
        )
        .map_err(|e| HorusError::driver(format!("Failed to initialize PWM: {}", e)))?;

        self.pwm_channels[idx] = Some(pwm);
        Ok(())
    }

    /// Send PWM signal to ESC
    fn send_pwm(&mut self, motor_id: u8, throttle: f64) -> HorusResult<()> {
        let idx = motor_id as usize;

        let pwm = self.pwm_channels[idx]
            .as_mut()
            .ok_or_else(|| HorusError::driver("PWM not initialized"))?;

        // Calculate PWM pulse width based on protocol
        let pwm_us = match self.config.protocol {
            BldcProtocol::StandardPwm => {
                let range = (self.config.pwm_max_us - self.config.pwm_min_us) as f64;
                self.config.pwm_min_us as f64 + (throttle * range)
            }
            BldcProtocol::OneShot125 => 125.0 + (throttle * 125.0),
            BldcProtocol::OneShot42 => 42.0 + (throttle * 42.0),
            BldcProtocol::DShot150
            | BldcProtocol::DShot300
            | BldcProtocol::DShot600
            | BldcProtocol::DShot1200 => {
                // DShot uses digital protocol, approximate with PWM for now
                if throttle < 0.001 {
                    1000.0 // Disarmed
                } else {
                    1000.0 + (throttle * 1000.0)
                }
            }
        };

        // Convert microseconds to duty cycle percentage
        let period_us = 1_000_000.0 / self.config.pwm_frequency_hz;
        let duty_cycle = (pwm_us / period_us).clamp(0.0, 1.0);

        pwm.set_duty_cycle(duty_cycle)
            .map_err(|e| HorusError::driver(format!("Failed to set PWM: {}", e)))?;

        Ok(())
    }
}

// ========================================================================
// Lifecycle methods
// ========================================================================

impl PwmBldcDriver {
    pub fn init(&mut self) -> HorusResult<()> {
        // Initialize PWM for each configured motor
        for motor_id in 0..self.config.num_motors {
            if self.config.gpio_pins[motor_id as usize] != 0 {
                self.init_pwm(motor_id)?;
            }
        }

        // Reset all motors
        for i in 0..8 {
            self.current_throttle[i] = 0.0;
            self.armed[i] = false;
        }

        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        // Stop all motors
        for motor_id in 0..self.config.num_motors {
            let idx = motor_id as usize;
            if self.pwm_channels[idx].is_some() {
                let _ = self.send_pwm(motor_id, 0.0);
            }
            self.current_throttle[idx] = 0.0;
            self.armed[idx] = false;
        }

        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        // Check if any PWM channel is configured
        self.config.gpio_pins.iter().any(|&p| p != 0)
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    // ========================================================================
    // Actuator methods
    // ========================================================================

    pub fn write(&mut self, cmd: BldcCommand) -> HorusResult<()> {
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

        // Update state
        self.armed[idx] = cmd.armed;

        let throttle = if cmd.armed { cmd.throttle } else { 0.0 };
        self.current_throttle[idx] = throttle;

        // Send PWM signal
        self.send_pwm(cmd.motor_id, throttle)
    }

    pub fn stop(&mut self) -> HorusResult<()> {
        for motor_id in 0..self.config.num_motors {
            let idx = motor_id as usize;
            self.current_throttle[idx] = 0.0;
            self.armed[idx] = false;
            if self.pwm_channels[idx].is_some() {
                self.send_pwm(motor_id, 0.0)?;
            }
        }
        Ok(())
    }
}
