//! Simulation Joystick driver
//!
//! Always-available simulation driver that generates synthetic joystick input.
//! Useful for testing and development without hardware.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use super::JoystickConfig;
use crate::JoystickInput;

/// Simulation joystick driver configuration
#[derive(Debug, Clone)]
pub struct SimulationJoystickConfig {
    /// Base joystick configuration
    pub joystick: JoystickConfig,
    /// Sample rate for generating events
    pub sample_rate: f32,
    /// Generate random button presses
    pub random_buttons: bool,
    /// Generate axis movement patterns
    pub axis_patterns: bool,
    /// Pattern interval in milliseconds
    pub pattern_interval_ms: u64,
}

impl Default for SimulationJoystickConfig {
    fn default() -> Self {
        Self {
            joystick: JoystickConfig::default(),
            sample_rate: 60.0, // 60 Hz
            random_buttons: false,
            axis_patterns: true,
            pattern_interval_ms: 100,
        }
    }
}

/// Simulation Joystick driver
///
/// Generates synthetic joystick input events for testing purposes.
/// Can simulate button presses, axis movements, and connection events.
pub struct SimulationJoystickDriver {
    config: SimulationJoystickConfig,
    status: DriverStatus,
    last_event_time: u64,
    event_queue: VecDeque<JoystickInput>,
    pattern_phase: f32,
    simulated_connected: bool,
}

impl SimulationJoystickDriver {
    /// Create a new simulation joystick driver with default configuration
    pub fn new() -> Self {
        Self {
            config: SimulationJoystickConfig::default(),
            status: DriverStatus::Uninitialized,
            last_event_time: 0,
            event_queue: VecDeque::new(),
            pattern_phase: 0.0,
            simulated_connected: true,
        }
    }

    /// Create a new simulation driver with custom configuration
    pub fn with_config(config: SimulationJoystickConfig) -> Self {
        Self {
            config,
            status: DriverStatus::Uninitialized,
            last_event_time: 0,
            event_queue: VecDeque::new(),
            pattern_phase: 0.0,
            simulated_connected: true,
        }
    }

    /// Set whether a controller is simulated as connected
    pub fn set_connected(&mut self, connected: bool) {
        if connected != self.simulated_connected {
            self.simulated_connected = connected;
            // Queue connection event
            let event = JoystickInput::new_connection(self.config.joystick.device_id, connected);
            self.event_queue.push_back(event);
        }
    }

    /// Simulate a button press
    pub fn simulate_button_press(&mut self, button_id: u32, button_name: &str) {
        let event = JoystickInput::new_button(
            self.config.joystick.device_id,
            button_id,
            button_name.to_string(),
            true,
        );
        self.event_queue.push_back(event);
    }

    /// Simulate a button release
    pub fn simulate_button_release(&mut self, button_id: u32, button_name: &str) {
        let event = JoystickInput::new_button(
            self.config.joystick.device_id,
            button_id,
            button_name.to_string(),
            false,
        );
        self.event_queue.push_back(event);
    }

    /// Simulate an axis movement
    pub fn simulate_axis(&mut self, axis_id: u32, axis_name: &str, value: f32) {
        let event = JoystickInput::new_axis(
            self.config.joystick.device_id,
            axis_id,
            axis_name.to_string(),
            value.clamp(-1.0, 1.0),
        );
        self.event_queue.push_back(event);
    }

    /// Check if controller is simulated as connected
    pub fn is_controller_connected(&self) -> bool {
        self.simulated_connected
    }

    /// Get simulated battery level
    pub fn get_battery_level(&self) -> Option<f32> {
        Some(0.75) // Simulated 75% battery
    }

    /// Poll for next event (non-blocking)
    pub fn poll_event(&mut self) -> Option<JoystickInput> {
        self.event_queue.pop_front()
    }

    /// Get the current timestamp in milliseconds
    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Generate axis pattern events
    fn generate_axis_patterns(&mut self) {
        if !self.config.axis_patterns {
            return;
        }

        let current_time = self.now_millis();
        if current_time - self.last_event_time < self.config.pattern_interval_ms {
            return;
        }
        self.last_event_time = current_time;

        // Advance phase
        self.pattern_phase += 0.1;
        if self.pattern_phase > std::f32::consts::TAU {
            self.pattern_phase -= std::f32::consts::TAU;
        }

        // Generate smooth circular pattern on left stick
        let x = self.pattern_phase.cos();
        let y = self.pattern_phase.sin();

        self.simulate_axis(0, "LeftStickX", x * 0.5);
        self.simulate_axis(1, "LeftStickY", y * 0.5);
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        self.event_queue.clear();
        self.last_event_time = 0;
        self.pattern_phase = 0.0;

        // Queue initial connection event
        if self.simulated_connected {
            let event = JoystickInput::new_connection(self.config.joystick.device_id, true);
            self.event_queue.push_back(event);
        }

        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    /// Check if driver is available
    pub fn is_available(&self) -> bool {
        true // Simulation is always available
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Read joystick input
    pub fn read(&mut self) -> HorusResult<JoystickInput> {
        if self.status != DriverStatus::Ready && self.status != DriverStatus::Running {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;
        Ok(self.generate_reading())
    }

    /// Check if data is available
    pub fn has_data(&self) -> bool {
        matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.sample_rate)
    }

    /// Generate a reading
    fn generate_reading(&mut self) -> JoystickInput {
        // First check if there are queued events
        if let Some(event) = self.event_queue.pop_front() {
            return event;
        }

        // Generate pattern events if enabled
        self.generate_axis_patterns();

        // Return first queued event or a neutral axis event
        self.event_queue.pop_front().unwrap_or_else(|| {
            JoystickInput::new_axis(
                self.config.joystick.device_id,
                0,
                "LeftStickX".to_string(),
                0.0,
            )
        })
    }
}

impl Default for SimulationJoystickDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_driver_lifecycle() {
        let mut driver = SimulationJoystickDriver::new();

        assert_eq!(driver.status(), DriverStatus::Uninitialized);
        assert!(driver.is_available());

        driver.init().unwrap();
        assert_eq!(driver.status(), DriverStatus::Ready);

        let input = driver.read().unwrap();
        assert_eq!(driver.status(), DriverStatus::Running);
        assert!(input.timestamp > 0);

        driver.shutdown().unwrap();
        assert_eq!(driver.status(), DriverStatus::Shutdown);
    }

    #[test]
    fn test_button_simulation() {
        let mut driver = SimulationJoystickDriver::new();
        driver.init().unwrap();

        // Clear initial connection event
        let _ = driver.read();

        // Simulate button press
        driver.simulate_button_press(0, "A");
        let input = driver.read().unwrap();

        assert!(input.pressed);
        assert_eq!(input.element_id, 0);
    }

    #[test]
    fn test_axis_simulation() {
        let mut driver = SimulationJoystickDriver::new();
        driver.init().unwrap();

        // Clear initial connection event
        let _ = driver.read();

        // Simulate axis movement
        driver.simulate_axis(0, "LeftStickX", 0.75);
        let input = driver.read().unwrap();

        assert!((input.value - 0.75).abs() < 0.001);
    }
}
