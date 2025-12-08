//! Crossterm-based Keyboard driver
//!
//! Terminal keyboard input using the crossterm library.
//! Requires the `crossterm` feature to be enabled.

use std::collections::VecDeque;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use super::keycodes;
use crate::KeyboardInput;

/// Crossterm driver configuration
#[derive(Debug, Clone)]
pub struct CrosstermConfig {
    /// Poll timeout in milliseconds (0 for non-blocking)
    pub poll_timeout_ms: u64,
    /// Enable raw mode on init
    pub enable_raw_mode: bool,
}

impl Default for CrosstermConfig {
    fn default() -> Self {
        Self {
            poll_timeout_ms: 0, // Non-blocking by default
            enable_raw_mode: true,
        }
    }
}

/// Crossterm-based Keyboard driver
///
/// Captures terminal keyboard input using the crossterm library.
pub struct CrosstermKeyboardDriver {
    config: CrosstermConfig,
    status: DriverStatus,
    event_queue: VecDeque<KeyboardInput>,
    raw_mode_enabled: bool,
}

impl CrosstermKeyboardDriver {
    /// Create a new crossterm keyboard driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: CrosstermConfig::default(),
            status: DriverStatus::Uninitialized,
            event_queue: VecDeque::new(),
            raw_mode_enabled: false,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: CrosstermConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            event_queue: VecDeque::new(),
            raw_mode_enabled: false,
        })
    }

    /// Poll for next event (non-blocking)
    pub fn poll_event(&mut self) -> Option<KeyboardInput> {
        self.process_events();
        self.event_queue.pop_front()
    }

    /// Process crossterm events into the queue
    fn process_events(&mut self) {
        let timeout = Duration::from_millis(self.config.poll_timeout_ms);

        if event::poll(timeout).unwrap_or(false) {
            if let Ok(Event::Key(key_event)) = event::read() {
                if let Some(input) = self.process_key_event(key_event) {
                    self.event_queue.push_back(input);
                }
            }
        }
    }

    /// Process crossterm KeyEvent into KeyboardInput
    fn process_key_event(&self, key_event: KeyEvent) -> Option<KeyboardInput> {
        let (key_name, keycode) = match key_event.code {
            KeyCode::Up => ("ArrowUp".to_string(), keycodes::KEY_ARROW_UP),
            KeyCode::Down => ("ArrowDown".to_string(), keycodes::KEY_ARROW_DOWN),
            KeyCode::Left => ("ArrowLeft".to_string(), keycodes::KEY_ARROW_LEFT),
            KeyCode::Right => ("ArrowRight".to_string(), keycodes::KEY_ARROW_RIGHT),
            KeyCode::Char(c) => {
                let key_str = c.to_uppercase().to_string();
                let code = if c.is_ascii_alphabetic() {
                    c.to_ascii_uppercase() as u32
                } else if c.is_ascii_digit() {
                    c as u32
                } else {
                    match c {
                        ' ' => keycodes::KEY_SPACE,
                        _ => return None,
                    }
                };
                (key_str, code)
            }
            KeyCode::Enter => ("Enter".to_string(), keycodes::KEY_ENTER),
            KeyCode::Esc => ("Escape".to_string(), keycodes::KEY_ESCAPE),
            KeyCode::Tab => ("Tab".to_string(), keycodes::KEY_TAB),
            KeyCode::Backspace => ("Backspace".to_string(), keycodes::KEY_BACKSPACE),
            KeyCode::F(n) if (1..=12).contains(&n) => (format!("F{}", n), 111 + n as u32),
            _ => return None,
        };

        // Build modifiers list
        let mut modifiers = Vec::new();
        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            modifiers.push("Ctrl".to_string());
        }
        if key_event.modifiers.contains(KeyModifiers::ALT) {
            modifiers.push("Alt".to_string());
        }
        if key_event.modifiers.contains(KeyModifiers::SHIFT) {
            modifiers.push("Shift".to_string());
        }

        Some(KeyboardInput::new(key_name, keycode, modifiers, true))
    }
}

// ========================================================================
// Lifecycle methods
// ========================================================================

impl CrosstermKeyboardDriver {
    pub fn init(&mut self) -> HorusResult<()> {
        self.event_queue.clear();

        if self.config.enable_raw_mode {
            enable_raw_mode().map_err(|e| {
                horus_core::error::HorusError::driver(format!("Failed to enable raw mode: {}", e))
            })?;
            self.raw_mode_enabled = true;
        }

        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        if self.raw_mode_enabled {
            let _ = disable_raw_mode();
            self.raw_mode_enabled = false;
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

    // ========================================================================
    // Sensor methods
    // ========================================================================

    pub fn read(&mut self) -> HorusResult<KeyboardInput> {
        if self.status != DriverStatus::Ready && self.status != DriverStatus::Running {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;

        self.process_events();

        self.event_queue
            .pop_front()
            .ok_or_else(|| horus_core::error::HorusError::driver("No keyboard events available"))
    }

    pub fn has_data(&self) -> bool {
        !self.event_queue.is_empty()
            || matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    pub fn sample_rate(&self) -> Option<f32> {
        Some(1000.0) // Poll-based
    }
}

impl Drop for CrosstermKeyboardDriver {
    fn drop(&mut self) {
        if self.raw_mode_enabled {
            let _ = disable_raw_mode();
        }
    }
}
