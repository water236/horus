//! I2C Battery Monitor driver
//!
//! Hardware driver for I2C power monitors like INA219 and INA226.
//! These chips measure bus voltage and current through a shunt resistor.

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

use super::{BatteryChemistry, BatteryConfig};
use crate::BatteryState;

#[cfg(feature = "i2c-hardware")]
use i2cdev::core::I2CDevice;
#[cfg(feature = "i2c-hardware")]
use i2cdev::linux::LinuxI2CDevice;

/// I2C power monitor type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PowerMonitorType {
    /// Texas Instruments INA219 (12-bit, up to 26V)
    INA219,
    /// Texas Instruments INA226 (16-bit, up to 36V)
    INA226,
}

impl Default for PowerMonitorType {
    fn default() -> Self {
        Self::INA219
    }
}

/// I2C battery driver configuration
#[derive(Debug, Clone)]
pub struct I2cBatteryConfig {
    /// Battery configuration
    pub battery: BatteryConfig,
    /// I2C bus number (default: 1)
    pub i2c_bus: u8,
    /// I2C address (default: 0x40 for INA219)
    pub i2c_address: u16,
    /// Shunt resistor value in milliohms
    pub shunt_resistance_mohm: f32,
    /// Power monitor type
    pub monitor_type: PowerMonitorType,
    /// Sample rate in Hz
    pub sample_rate: f32,
}

impl Default for I2cBatteryConfig {
    fn default() -> Self {
        Self {
            battery: BatteryConfig::default(),
            i2c_bus: 1,
            i2c_address: 0x40,
            shunt_resistance_mohm: 100.0, // 100mΩ default
            monitor_type: PowerMonitorType::INA219,
            sample_rate: 10.0, // 10 Hz
        }
    }
}

impl I2cBatteryConfig {
    /// Create configuration for INA219
    pub fn ina219(bus: u8, address: u16, shunt_mohm: f32) -> Self {
        Self {
            i2c_bus: bus,
            i2c_address: address,
            shunt_resistance_mohm: shunt_mohm,
            monitor_type: PowerMonitorType::INA219,
            ..Default::default()
        }
    }

    /// Create configuration for INA226
    pub fn ina226(bus: u8, address: u16, shunt_mohm: f32) -> Self {
        Self {
            i2c_bus: bus,
            i2c_address: address,
            shunt_resistance_mohm: shunt_mohm,
            monitor_type: PowerMonitorType::INA226,
            ..Default::default()
        }
    }

    /// Set battery configuration
    pub fn with_battery(mut self, battery: BatteryConfig) -> Self {
        self.battery = battery;
        self
    }
}

/// I2C Battery Monitor driver
///
/// Reads voltage and current from I2C power monitors (INA219, INA226).
/// Calculates state of charge using Coulomb counting.
pub struct I2cBatteryDriver {
    config: I2cBatteryConfig,
    status: DriverStatus,
    #[cfg(feature = "i2c-hardware")]
    device: Option<LinuxI2CDevice>,

    // Calculated state
    charge_mah: f32,
    percentage: f32,
    temperature: f32,
    power_supply_status: u8,
    last_read_time: u64,

    // Voltage filtering
    voltage_history: [f32; 10],
    history_index: usize,
}

impl I2cBatteryDriver {
    /// Create a new I2C battery driver with default configuration
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: I2cBatteryConfig::default(),
            status: DriverStatus::Uninitialized,
            #[cfg(feature = "i2c-hardware")]
            device: None,
            charge_mah: 5000.0, // Start at full
            percentage: 100.0,
            temperature: 25.0,
            power_supply_status: BatteryState::STATUS_UNKNOWN,
            last_read_time: 0,
            voltage_history: [11.1; 10],
            history_index: 0,
        })
    }

    /// Create a new I2C battery driver with custom configuration
    pub fn with_config(config: I2cBatteryConfig) -> HorusResult<Self> {
        Ok(Self {
            charge_mah: config.battery.capacity_mah,
            config,
            status: DriverStatus::Uninitialized,
            #[cfg(feature = "i2c-hardware")]
            device: None,
            percentage: 100.0,
            temperature: 25.0,
            power_supply_status: BatteryState::STATUS_UNKNOWN,
            last_read_time: 0,
            voltage_history: [11.1; 10],
            history_index: 0,
        })
    }

    /// Get the current timestamp in nanoseconds
    fn now_nanos(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// Apply voltage filtering (moving average)
    fn filter_voltage(&mut self, raw_voltage: f32) -> f32 {
        self.voltage_history[self.history_index] = raw_voltage;
        self.history_index = (self.history_index + 1) % self.voltage_history.len();

        let sum: f32 = self.voltage_history.iter().sum();
        sum / self.voltage_history.len() as f32
    }

    /// Update charge estimate using Coulomb counting
    fn update_charge(&mut self, current: f32, dt: f32) {
        if current.abs() < 0.01 {
            return;
        }

        // Convert current (A) and time (s) to charge (mAh)
        let delta_charge = (current * dt * 1000.0) / 3600.0;
        self.charge_mah += delta_charge;
        self.charge_mah = self.charge_mah.clamp(0.0, self.config.battery.capacity_mah);

        // Update percentage
        self.percentage =
            (self.charge_mah / self.config.battery.capacity_mah * 100.0).clamp(0.0, 100.0);

        // Detect charging state
        if current > 0.1 {
            self.power_supply_status = BatteryState::STATUS_CHARGING;
        } else if current < -0.1 {
            self.power_supply_status = BatteryState::STATUS_DISCHARGING;
        } else if self.percentage >= 99.0 {
            self.power_supply_status = BatteryState::STATUS_FULL;
        } else {
            self.power_supply_status = BatteryState::STATUS_UNKNOWN;
        }
    }

    /// Configure INA219 chip
    #[cfg(feature = "i2c-hardware")]
    fn configure_ina219(device: &mut LinuxI2CDevice) -> std::io::Result<()> {
        // INA219 Configuration register (0x00)
        // Config: Bus voltage range = 16V, Gain = /1 40mV
        //         Bus ADC = 12-bit, Shunt ADC = 12-bit
        //         Mode = Shunt and Bus continuous
        let config: [u8; 3] = [0x00, 0x01, 0x9F];
        device.write(&config)?;
        Ok(())
    }

    /// Configure INA226 chip
    #[cfg(feature = "i2c-hardware")]
    fn configure_ina226(device: &mut LinuxI2CDevice) -> std::io::Result<()> {
        // INA226 Configuration register (0x00)
        // Averaging mode, conversion times, operating mode
        let config: [u8; 3] = [0x00, 0x41, 0x27];
        device.write(&config)?;
        Ok(())
    }

    /// Read voltage and current from INA219
    #[cfg(feature = "i2c-hardware")]
    fn read_ina219(&mut self) -> HorusResult<(f32, f32)> {
        let device = self
            .device
            .as_mut()
            .ok_or_else(|| HorusError::driver("I2C device not initialized"))?;

        // Read bus voltage (register 0x02)
        let reg_bus: [u8; 1] = [0x02];
        device
            .write(&reg_bus)
            .map_err(|e| HorusError::driver(format!("I2C write failed: {}", e)))?;
        let mut bus_data = [0u8; 2];
        device
            .read(&mut bus_data)
            .map_err(|e| HorusError::driver(format!("I2C read failed: {}", e)))?;
        let bus_raw = u16::from_be_bytes(bus_data) >> 3;
        let voltage = (bus_raw as f32) * 0.004; // LSB = 4mV

        // Read shunt voltage (register 0x01)
        let reg_shunt: [u8; 1] = [0x01];
        device
            .write(&reg_shunt)
            .map_err(|e| HorusError::driver(format!("I2C write failed: {}", e)))?;
        let mut shunt_data = [0u8; 2];
        device
            .read(&mut shunt_data)
            .map_err(|e| HorusError::driver(format!("I2C read failed: {}", e)))?;
        let shunt_raw = i16::from_be_bytes(shunt_data);
        let shunt_voltage_mv = (shunt_raw as f32) * 0.01; // LSB = 10μV

        // Calculate current: I = V_shunt / R_shunt
        let current = shunt_voltage_mv / self.config.shunt_resistance_mohm;

        Ok((voltage, current))
    }

    /// Read voltage and current from INA226
    #[cfg(feature = "i2c-hardware")]
    fn read_ina226(&mut self) -> HorusResult<(f32, f32)> {
        let device = self
            .device
            .as_mut()
            .ok_or_else(|| HorusError::driver("I2C device not initialized"))?;

        // Read bus voltage (register 0x02)
        let reg_bus: [u8; 1] = [0x02];
        device
            .write(&reg_bus)
            .map_err(|e| HorusError::driver(format!("I2C write failed: {}", e)))?;
        let mut bus_data = [0u8; 2];
        device
            .read(&mut bus_data)
            .map_err(|e| HorusError::driver(format!("I2C read failed: {}", e)))?;
        let bus_raw = u16::from_be_bytes(bus_data);
        let voltage = (bus_raw as f32) * 0.00125; // LSB = 1.25mV

        // Read shunt voltage (register 0x01)
        let reg_shunt: [u8; 1] = [0x01];
        device
            .write(&reg_shunt)
            .map_err(|e| HorusError::driver(format!("I2C write failed: {}", e)))?;
        let mut shunt_data = [0u8; 2];
        device
            .read(&mut shunt_data)
            .map_err(|e| HorusError::driver(format!("I2C read failed: {}", e)))?;
        let shunt_raw = i16::from_be_bytes(shunt_data);
        let shunt_voltage_uv = (shunt_raw as f32) * 2.5; // LSB = 2.5μV

        // Calculate current: I = V_shunt / R_shunt (convert μV to mV)
        let current = (shunt_voltage_uv / 1000.0) / self.config.shunt_resistance_mohm;

        Ok((voltage, current))
    }
}

impl Default for I2cBatteryDriver {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

// ========================================================================
// Lifecycle methods
// ========================================================================

impl I2cBatteryDriver {
    #[cfg(feature = "i2c-hardware")]
    pub fn init(&mut self) -> HorusResult<()> {
        let device_path = format!("/dev/i2c-{}", self.config.i2c_bus);
        let mut device =
            LinuxI2CDevice::new(&device_path, self.config.i2c_address).map_err(|e| {
                HorusError::driver(format!(
                "Failed to open I2C device {}: {}. Check I2C is enabled and device is connected.",
                device_path, e
            ))
            })?;

        // Configure the power monitor
        match self.config.monitor_type {
            PowerMonitorType::INA219 => Self::configure_ina219(&mut device)?,
            PowerMonitorType::INA226 => Self::configure_ina226(&mut device)?,
        }

        self.device = Some(device);
        self.charge_mah = self.config.battery.capacity_mah;
        self.percentage = 100.0;
        self.last_read_time = 0;
        self.status = DriverStatus::Ready;
        Ok(())
    }

    #[cfg(not(feature = "i2c-hardware"))]
    pub fn init(&mut self) -> HorusResult<()> {
        Err(HorusError::driver("I2C hardware feature not enabled"))
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        #[cfg(feature = "i2c-hardware")]
        {
            self.device = None;
        }
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        cfg!(feature = "i2c-hardware")
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    // ========================================================================
    // Sensor methods
    // ========================================================================

    #[cfg(feature = "i2c-hardware")]
    pub fn read(&mut self) -> HorusResult<BatteryState> {
        if self.status != DriverStatus::Ready && self.status != DriverStatus::Running {
            return Err(HorusError::driver("Driver not initialized"));
        }

        let current_time = self.now_nanos();
        let dt = if self.last_read_time > 0 {
            (current_time - self.last_read_time) as f32 / 1_000_000_000.0
        } else {
            0.0
        };
        self.last_read_time = current_time;

        // Read from hardware
        let (voltage, current) = match self.config.monitor_type {
            PowerMonitorType::INA219 => self.read_ina219()?,
            PowerMonitorType::INA226 => self.read_ina226()?,
        };

        // Filter voltage
        let filtered_voltage = self.filter_voltage(voltage);

        // Update charge estimate
        self.update_charge(current, dt);

        self.status = DriverStatus::Running;

        Ok(BatteryState {
            voltage: filtered_voltage,
            current,
            charge: self.charge_mah / 1000.0,
            capacity: self.config.battery.capacity_mah / 1000.0,
            percentage: self.percentage,
            power_supply_status: self.power_supply_status,
            temperature: self.temperature,
            cell_voltages: [0.0; 16], // Individual cell monitoring requires additional hardware
            cell_count: 0,
            timestamp: current_time,
        })
    }

    #[cfg(not(feature = "i2c-hardware"))]
    pub fn read(&mut self) -> HorusResult<BatteryState> {
        Err(HorusError::driver("I2C hardware feature not enabled"))
    }

    pub fn has_data(&self) -> bool {
        matches!(self.status, DriverStatus::Ready | DriverStatus::Running)
    }

    pub fn sample_rate(&self) -> Option<f32> {
        Some(self.config.sample_rate)
    }
}
