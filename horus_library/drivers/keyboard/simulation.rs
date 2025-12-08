//! Simulation Keyboard driver
//!
//! Always-available simulation driver that generates synthetic keyboard input.
//! Useful for testing and development without terminal access.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use super::keycodes;
use crate::KeyboardInput;

/// Simulation keyboard driver configuration
#[derive(Debug, Clone)]
pub struct SimulationKeyboardConfig {
    /// Sample rate for generating events
    pub sample_rate: f32,
    /// Generate demo key sequence
    pub demo_sequence: bool,
    /// Demo sequence interval in milliseconds
    pub demo_interval_ms: u64,
}

impl Default for SimulationKeyboardConfig {
    fn default() -> Self {
        Self {
            sample_rate: 60.0,
            demo_sequence: false,
            demo_interval_ms: 500,
        }
    }
}

/// Simulation Keyboard driver
///
/// Generates synthetic keyboard input events for testing purposes.
pub struct SimulationKeyboardDriver {
    config: SimulationKeyboardConfig,
    status: DriverStatus,
    last_event_time: u64,
    event_queue: VecDeque<KeyboardInput>,
    demo_index: usize,
}

impl SimulationKeyboardDriver {
    /// Create a new simulation keyboard driver with default configuration
    pub fn new() -> Self {
        Self {
            config: SimulationKeyboardConfig::default(),
            status: DriverStatus::Uninitialized,
            last_event_time: 0,
            event_queue: VecDeque::new(),
            demo_index: 0,
        }
    }

    /// Create a new simulation driver with custom configuration
    pub fn with_config(config: SimulationKeyboardConfig) -> Self {
        Self {
            config,
            status: DriverStatus::Uninitialized,
            last_event_time: 0,
            event_queue: VecDeque::new(),
            demo_index: 0,
        }
    }

    /// Simulate a key press
    pub fn simulate_key_press(&mut self, key_name: &str, keycode: u32) {
        let event = KeyboardInput::new(key_name.to_string(), keycode, vec![], true);
        self.event_queue.push_back(event);
    }

    /// Simulate a key release
    pub fn simulate_key_release(&mut self, key_name: &str, keycode: u32) {
        let event = KeyboardInput::new(key_name.to_string(), keycode, vec![], false);
        self.event_queue.push_back(event);
    }

    /// Simulate a key with modifiers
    pub fn simulate_key_with_modifiers(
        &mut self,
        key_name: &str,
        keycode: u32,
        modifiers: Vec<String>,
        pressed: bool,
    ) {
        let event = KeyboardInput::new(key_name.to_string(), keycode, modifiers, pressed);
        self.event_queue.push_back(event);
    }

    /// Poll for next event (non-blocking)
    pub fn poll_event(&mut self) -> Option<KeyboardInput> {
        self.event_queue.pop_front()
    }

    /// Get the current timestamp in milliseconds
    fn now_millis(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Generate demo sequence events
    fn generate_demo_events(&mut self) {
        if !self.config.demo_sequence {
            return;
        }

        let current_time = self.now_millis();
        if current_time - self.last_event_time < self.config.demo_interval_ms {
            return;
        }
        self.last_event_time = current_time;

        // Demo sequence: arrow keys
        let demo_keys = [
            ("ArrowUp", keycodes::KEY_ARROW_UP),
            ("ArrowRight", keycodes::KEY_ARROW_RIGHT),
            ("ArrowDown", keycodes::KEY_ARROW_DOWN),
            ("ArrowLeft", keycodes::KEY_ARROW_LEFT),
        ];

        let (key_name, keycode) = demo_keys[self.demo_index % demo_keys.len()];
        self.simulate_key_press(key_name, keycode);
        self.demo_index += 1;
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        self.event_queue.clear();
        self.last_event_time = 0;
        self.demo_index = 0;
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

    /// Read keyboard input
    pub fn read(&mut self) -> HorusResult<KeyboardInput> {
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
    fn generate_reading(&mut self) -> KeyboardInput {
        // First check if there are queued events
        if let Some(event) = self.event_queue.pop_front() {
            return event;
        }

        // Generate demo events if enabled
        self.generate_demo_events();

        // Return first queued event or a neutral event
        self.event_queue
            .pop_front()
            .unwrap_or_else(|| KeyboardInput::new("None".to_string(), 0, vec![], false))
    }
}

impl Default for SimulationKeyboardDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_driver_lifecycle() {
        let mut driver = SimulationKeyboardDriver::new();

        assert_eq!(driver.status(), DriverStatus::Uninitialized);
        assert!(driver.is_available());

        driver.init().unwrap();
        assert_eq!(driver.status(), DriverStatus::Ready);

        let _input = driver.read().unwrap();
        assert_eq!(driver.status(), DriverStatus::Running);

        driver.shutdown().unwrap();
        assert_eq!(driver.status(), DriverStatus::Shutdown);
    }

    #[test]
    fn test_key_simulation() {
        let mut driver = SimulationKeyboardDriver::new();
        driver.init().unwrap();

        driver.simulate_key_press("A", keycodes::KEY_A);
        let input = driver.read().unwrap();

        assert!(input.pressed);
        assert_eq!(input.code, keycodes::KEY_A);
    }
}
