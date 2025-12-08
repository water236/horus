//! GPIO Ultrasonic driver
//!
//! Hardware driver for echo/trigger ultrasonic sensors using GPIO pins.
//! Supports HC-SR04, JSN-SR04T, and similar sensors.

use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};
use sysfs_gpio::{Direction, Pin};

use crate::Range;

/// GPIO ultrasonic driver configuration
#[derive(Debug, Clone)]
pub struct GpioUltrasonicConfig {
    /// Trigger GPIO pin number
    pub trigger_pin: u64,
    /// Echo GPIO pin number
    pub echo_pin: u64,
    /// Sample rate in Hz
    pub sample_rate: f32,
    /// Minimum range in meters
    pub min_range: f32,
    /// Maximum range in meters
    pub max_range: f32,
    /// Field of view in radians
    pub field_of_view: f32,
    /// Speed of sound in m/s (varies with temperature)
    pub speed_of_sound: f32,
    /// Timeout for echo in milliseconds
    pub timeout_ms: u64,
}

impl Default for GpioUltrasonicConfig {
    fn default() -> Self {
        Self {
            trigger_pin: 0,
            echo_pin: 0,
            sample_rate: 10.0,
            min_range: 0.02,       // 2cm
            max_range: 4.0,        // 4m
            field_of_view: 0.26,   // ~15 degrees
            speed_of_sound: 343.0, // m/s at 20C
            timeout_ms: 50,
        }
    }
}

/// GPIO Ultrasonic driver
///
/// Hardware driver for echo/trigger ultrasonic sensors.
///
/// # Example
///
/// ```rust,ignore
/// use horus_library::drivers::GpioUltrasonicDriver;
/// use horus_core::driver::{Driver, Sensor};
///
/// let mut driver = GpioUltrasonicDriver::with_pins(23, 24)?;
/// driver.init()?;
///
/// loop {
///     let range = driver.read()?;
///     println!("Distance: {:.3}m", range.range);
/// }
/// ```
pub struct GpioUltrasonicDriver {
    config: GpioUltrasonicConfig,
    status: DriverStatus,
    trigger: Option<Pin>,
    echo: Option<Pin>,
    sample_count: u64,
}

impl GpioUltrasonicDriver {
    /// Create a new GPIO ultrasonic driver (pins must be set before init)
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: GpioUltrasonicConfig::default(),
            status: DriverStatus::Uninitialized,
            trigger: None,
            echo: None,
            sample_count: 0,
        })
    }

    /// Create a new GPIO ultrasonic driver with specific pins
    pub fn with_pins(trigger_pin: u64, echo_pin: u64) -> HorusResult<Self> {
        let mut config = GpioUltrasonicConfig::default();
        config.trigger_pin = trigger_pin;
        config.echo_pin = echo_pin;
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            trigger: None,
            echo: None,
            sample_count: 0,
        })
    }

    /// Create a new GPIO ultrasonic driver with custom configuration
    pub fn with_config(config: GpioUltrasonicConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            trigger: None,
            echo: None,
            sample_count: 0,
        })
    }

    /// Set the trigger and echo pins
    pub fn set_pins(&mut self, trigger_pin: u64, echo_pin: u64) {
        self.config.trigger_pin = trigger_pin;
        self.config.echo_pin = echo_pin;
    }

    /// Set temperature for speed of sound calculation
    /// Formula: v = 331.3 + 0.606 * T (T in Celsius)
    pub fn set_temperature(&mut self, celsius: f32) {
        self.config.speed_of_sound = 331.3 + 0.606 * celsius;
    }

    /// Get the current timestamp in nanoseconds
    fn now_nanos(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// Perform a distance measurement
    fn measure(&mut self) -> HorusResult<Range> {
        let trigger = self
            .trigger
            .as_ref()
            .ok_or_else(|| HorusError::driver("Trigger pin not initialized"))?;
        let echo = self
            .echo
            .as_ref()
            .ok_or_else(|| HorusError::driver("Echo pin not initialized"))?;

        // Send 10us trigger pulse
        trigger
            .set_value(1)
            .map_err(|e| HorusError::driver(format!("Trigger set high: {}", e)))?;
        thread::sleep(Duration::from_micros(10));
        trigger
            .set_value(0)
            .map_err(|e| HorusError::driver(format!("Trigger set low: {}", e)))?;

        // Wait for echo to go HIGH (with timeout)
        let start_wait = SystemTime::now();
        let timeout = Duration::from_millis(self.config.timeout_ms);

        loop {
            let val = echo
                .get_value()
                .map_err(|e| HorusError::driver(format!("Read echo: {}", e)))?;
            if val == 1 {
                break;
            }
            if start_wait.elapsed().unwrap_or(Duration::from_secs(1)) > timeout {
                return Err(HorusError::driver("Timeout waiting for echo start"));
            }
            thread::sleep(Duration::from_micros(10));
        }

        // Record echo start time
        let echo_start = SystemTime::now();

        // Wait for echo to go LOW (measure pulse width)
        loop {
            let val = echo
                .get_value()
                .map_err(|e| HorusError::driver(format!("Read echo: {}", e)))?;
            if val == 0 {
                break;
            }
            if echo_start.elapsed().unwrap_or(Duration::from_secs(1)) > Duration::from_millis(40) {
                return Err(HorusError::driver("Echo too long - out of range"));
            }
            thread::sleep(Duration::from_micros(10));
        }

        // Calculate distance from echo duration
        let echo_duration = echo_start.elapsed().unwrap_or(Duration::ZERO);
        let echo_seconds = echo_duration.as_secs_f32();
        let distance = (echo_seconds * self.config.speed_of_sound) / 2.0;

        self.sample_count += 1;

        Ok(Range {
            sensor_type: Range::ULTRASONIC,
            field_of_view: self.config.field_of_view,
            min_range: self.config.min_range,
            max_range: self.config.max_range,
            range: distance,
            timestamp: self.now_nanos(),
        })
    }
}

impl Default for GpioUltrasonicDriver {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

// ========================================================================
// Lifecycle methods
// ========================================================================

impl GpioUltrasonicDriver {
    pub fn init(&mut self) -> HorusResult<()> {
        if self.config.trigger_pin == 0 || self.config.echo_pin == 0 {
            return Err(HorusError::driver("GPIO pins not configured"));
        }

        // Initialize trigger pin (output)
        let trigger = Pin::new(self.config.trigger_pin);
        trigger
            .export()
            .map_err(|e| HorusError::driver(format!("Export trigger: {}", e)))?;
        thread::sleep(Duration::from_millis(10)); // Give sysfs time
        trigger
            .set_direction(Direction::Out)
            .map_err(|e| HorusError::driver(format!("Set trigger direction: {}", e)))?;
        trigger
            .set_value(0)
            .map_err(|e| HorusError::driver(format!("Set trigger low: {}", e)))?;

        // Initialize echo pin (input)
        let echo = Pin::new(self.config.echo_pin);
        echo.export()
            .map_err(|e| HorusError::driver(format!("Export echo: {}", e)))?;
        thread::sleep(Duration::from_millis(10));
        echo.set_direction(Direction::In)
            .map_err(|e| HorusError::driver(format!("Set echo direction: {}", e)))?;

        self.trigger = Some(trigger);
        self.echo = Some(echo);
        self.sample_count = 0;
        self.status = DriverStatus::Ready;

        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        // Set trigger low and unexport pins
        if let Some(ref trigger) = self.trigger {
            let _ = trigger.set_value(0);
            let _ = trigger.unexport();
        }
        if let Some(ref echo) = self.echo {
            let _ = echo.unexport();
        }

        self.trigger = None;
        self.echo = None;
        self.status = DriverStatus::Shutdown;

        Ok(())
    }

    pub fn is_available(&self) -> bool {
        // Check if GPIO is accessible
        self.config.trigger_pin != 0 && self.config.echo_pin != 0
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    // ========================================================================
    // Sensor methods
    // ========================================================================

    pub fn read(&mut self) -> HorusResult<Range> {
        if self.status != DriverStatus::Ready && self.status != DriverStatus::Running {
            return Err(HorusError::driver("Driver not initialized"));
        }
        self.status = DriverStatus::Running;
        self.measure()
    }

    pub fn has_data(&self) -> bool {
        matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpio_driver_creation() {
        let driver = GpioUltrasonicDriver::new().unwrap();
        assert_eq!(driver.status(), DriverStatus::Uninitialized);
    }

    #[test]
    fn test_gpio_driver_with_pins() {
        let driver = GpioUltrasonicDriver::with_pins(23, 24).unwrap();
        assert!(driver.is_available());
    }
}
