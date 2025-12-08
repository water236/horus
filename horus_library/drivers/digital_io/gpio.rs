//! GPIO-based Digital I/O driver
//!
//! Uses Linux sysfs GPIO interface.

use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use sysfs_gpio::{Direction, Pin};

use super::{DigitalIoConfig, PinMode};

/// GPIO-based digital I/O driver
pub struct GpioDigitalIoDriver {
    config: DigitalIoConfig,
    status: DriverStatus,
    /// GPIO pin handles
    pins: HashMap<u64, Pin>,
    /// Pin modes
    pin_modes: HashMap<u64, PinMode>,
}

impl GpioDigitalIoDriver {
    pub fn new(config: DigitalIoConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            pins: HashMap::new(),
            pin_modes: HashMap::new(),
        })
    }

    fn init_pin(&mut self, pin_num: u64, mode: PinMode, initial_value: bool) -> HorusResult<()> {
        let pin = Pin::new(pin_num);

        pin.export()
            .map_err(|e| HorusError::driver(format!("Failed to export GPIO {}: {}", pin_num, e)))?;

        // Wait for sysfs to settle
        thread::sleep(Duration::from_millis(10));

        let direction = match mode {
            PinMode::Input | PinMode::InputPullUp | PinMode::InputPullDown => Direction::In,
            PinMode::Output => Direction::Out,
        };

        pin.set_direction(direction).map_err(|e| {
            HorusError::driver(format!("Failed to set GPIO {} direction: {}", pin_num, e))
        })?;

        // Set initial value for outputs
        if mode == PinMode::Output {
            pin.set_value(if initial_value { 1 } else { 0 })
                .map_err(|e| {
                    HorusError::driver(format!("Failed to set GPIO {} value: {}", pin_num, e))
                })?;
        }

        self.pins.insert(pin_num, pin);
        self.pin_modes.insert(pin_num, mode);

        Ok(())
    }
}

// ========================================================================
// Lifecycle methods
// ========================================================================

impl GpioDigitalIoDriver {
    pub fn init(&mut self) -> HorusResult<()> {
        // Initialize all configured pins
        for pin_config in self.config.pins.clone() {
            self.init_pin(pin_config.pin, pin_config.mode, pin_config.initial_value)?;
        }

        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        // Set all outputs to low and unexport
        for (pin_num, pin) in &self.pins {
            if self.pin_modes.get(pin_num) == Some(&PinMode::Output) {
                let _ = pin.set_value(0);
            }
            let _ = pin.unexport();
        }

        self.pins.clear();
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        // Check if sysfs GPIO is available
        std::path::Path::new("/sys/class/gpio").exists()
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    // ========================================================================
    // Digital I/O methods
    // ========================================================================

    pub fn read_pin(&mut self, pin_num: u64) -> HorusResult<bool> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(HorusError::driver("Driver not initialized"));
        }
        self.status = DriverStatus::Running;

        let pin = self
            .pins
            .get(&pin_num)
            .ok_or_else(|| HorusError::driver(format!("Pin {} not configured", pin_num)))?;

        let value = pin
            .get_value()
            .map_err(|e| HorusError::driver(format!("Failed to read GPIO {}: {}", pin_num, e)))?;

        // Check for inversion
        let inverted = self
            .config
            .pins
            .iter()
            .find(|p| p.pin == pin_num)
            .map(|p| p.inverted)
            .unwrap_or(false);

        let result = value != 0;
        Ok(if inverted { !result } else { result })
    }

    pub fn write_pin(&mut self, pin_num: u64, value: bool) -> HorusResult<()> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(HorusError::driver("Driver not initialized"));
        }
        self.status = DriverStatus::Running;

        // Check if pin is output
        if self.pin_modes.get(&pin_num) != Some(&PinMode::Output) {
            return Err(HorusError::driver(format!(
                "Pin {} is not configured as output",
                pin_num
            )));
        }

        let pin = self
            .pins
            .get(&pin_num)
            .ok_or_else(|| HorusError::driver(format!("Pin {} not configured", pin_num)))?;

        // Check for inversion
        let inverted = self
            .config
            .pins
            .iter()
            .find(|p| p.pin == pin_num)
            .map(|p| p.inverted)
            .unwrap_or(false);

        let actual_value = if inverted { !value } else { value };

        pin.set_value(if actual_value { 1 } else { 0 })
            .map_err(|e| HorusError::driver(format!("Failed to write GPIO {}: {}", pin_num, e)))?;

        Ok(())
    }

    pub fn read_all(&mut self) -> HorusResult<Vec<(u64, bool)>> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(HorusError::driver("Driver not initialized"));
        }
        self.status = DriverStatus::Running;

        let mut results = Vec::new();
        for pin_config in &self.config.pins {
            if matches!(
                pin_config.mode,
                PinMode::Input | PinMode::InputPullUp | PinMode::InputPullDown
            ) {
                if let Some(pin) = self.pins.get(&pin_config.pin) {
                    if let Ok(value) = pin.get_value() {
                        let result = value != 0;
                        let actual = if pin_config.inverted { !result } else { result };
                        results.push((pin_config.pin, actual));
                    }
                }
            }
        }
        Ok(results)
    }

    pub fn set_mode(&mut self, pin_num: u64, mode: PinMode) -> HorusResult<()> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(HorusError::driver("Driver not initialized"));
        }

        // If pin exists, update its direction
        if let Some(pin) = self.pins.get(&pin_num) {
            let direction = match mode {
                PinMode::Input | PinMode::InputPullUp | PinMode::InputPullDown => Direction::In,
                PinMode::Output => Direction::Out,
            };

            pin.set_direction(direction).map_err(|e| {
                HorusError::driver(format!("Failed to set GPIO {} direction: {}", pin_num, e))
            })?;

            self.pin_modes.insert(pin_num, mode);
        } else {
            // Initialize the pin
            self.init_pin(pin_num, mode, false)?;
        }

        Ok(())
    }
}
