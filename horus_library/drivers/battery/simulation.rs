//! Simulation Battery driver
//!
//! Always-available simulation driver that generates synthetic battery data.
//! Useful for testing and development without hardware.

use std::time::{SystemTime, UNIX_EPOCH};

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use super::BatteryConfig;
use crate::BatteryState;

/// Simulation battery driver configuration
#[derive(Debug, Clone)]
pub struct SimulationBatteryConfig {
    /// Battery configuration
    pub battery: BatteryConfig,
    /// Sample rate in Hz
    pub sample_rate: f32,
    /// Simulated discharge current (A)
    pub discharge_current: f32,
    /// Initial state of charge (0-100%)
    pub initial_soc: f32,
}

impl Default for SimulationBatteryConfig {
    fn default() -> Self {
        Self {
            battery: BatteryConfig::default(),
            sample_rate: 1.0,       // 1 Hz
            discharge_current: 5.0, // 5A discharge
            initial_soc: 100.0,     // Start fully charged
        }
    }
}

/// Simulation Battery driver
///
/// Generates synthetic battery data including voltage, current, temperature,
/// and state of charge. Simulates realistic discharge curves based on
/// battery chemistry.
pub struct SimulationBatteryDriver {
    config: SimulationBatteryConfig,
    status: DriverStatus,
    start_time: Option<u64>,
    last_read_time: u64,

    // Current battery state
    charge_mah: f32,
    voltage: f32,
    current: f32,
    percentage: f32,
    temperature: f32,
    power_supply_status: u8,
    cell_voltages: [f32; 16],
}

impl SimulationBatteryDriver {
    /// Create a new simulation battery driver with default configuration
    pub fn new() -> Self {
        let config = SimulationBatteryConfig::default();
        let charge_mah = config.battery.capacity_mah * (config.initial_soc / 100.0);

        Self {
            voltage: config.battery.full_voltage * (config.initial_soc / 100.0),
            charge_mah,
            config,
            status: DriverStatus::Uninitialized,
            start_time: None,
            last_read_time: 0,
            current: 0.0,
            percentage: 100.0,
            temperature: 25.0,
            power_supply_status: BatteryState::STATUS_DISCHARGING,
            cell_voltages: [0.0; 16],
        }
    }

    /// Create a new simulation driver with custom configuration
    pub fn with_config(config: SimulationBatteryConfig) -> Self {
        let charge_mah = config.battery.capacity_mah * (config.initial_soc / 100.0);
        let voltage = config.battery.full_voltage * (config.initial_soc / 100.0);

        Self {
            voltage,
            charge_mah,
            config,
            status: DriverStatus::Uninitialized,
            start_time: None,
            last_read_time: 0,
            current: 0.0,
            percentage: 100.0,
            temperature: 25.0,
            power_supply_status: BatteryState::STATUS_DISCHARGING,
            cell_voltages: [0.0; 16],
        }
    }

    /// Create a new simulation driver with battery configuration
    pub fn with_battery_config(battery: BatteryConfig) -> Self {
        Self::with_config(SimulationBatteryConfig {
            battery,
            ..Default::default()
        })
    }

    /// Set discharge current
    pub fn set_discharge_current(&mut self, current: f32) {
        self.config.discharge_current = current;
    }

    /// Set the current for testing purposes
    pub fn set_current(&mut self, current: f32) {
        self.current = current;
    }

    /// Get the current timestamp in nanoseconds
    fn now_nanos(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// Simulate battery discharge
    fn simulate_discharge(&mut self, dt: f32) {
        // Simulate current based on configuration
        self.current = -self.config.discharge_current; // Negative = discharging

        // Update charge (Coulomb counting)
        if self.current.abs() > 0.01 {
            let delta_charge = (self.current * dt * 1000.0) / 3600.0; // A·s to mAh
            self.charge_mah += delta_charge;
            self.charge_mah = self.charge_mah.clamp(0.0, self.config.battery.capacity_mah);
        }

        // Update percentage
        self.percentage =
            (self.charge_mah / self.config.battery.capacity_mah * 100.0).clamp(0.0, 100.0);

        // Simulate voltage based on SOC and chemistry
        let soc_factor = self.percentage / 100.0;
        let load_factor = (self.current.abs() / 50.0).min(1.0);
        let voltage_drop = load_factor * 0.5;

        // Non-linear discharge curve
        let base_voltage = self.config.battery.critical_voltage
            + (self.config.battery.full_voltage - self.config.battery.critical_voltage)
                * soc_factor.powf(1.2);

        self.voltage = base_voltage - voltage_drop;

        // Simulate temperature (increases with current)
        let ambient = 25.0;
        let heating = (self.current.abs() / 10.0) * 10.0; // 10°C per 10A
        self.temperature = ambient + heating;

        // Update power supply status
        if self.current > 0.1 {
            self.power_supply_status = BatteryState::STATUS_CHARGING;
        } else if self.current < -0.1 {
            self.power_supply_status = BatteryState::STATUS_DISCHARGING;
        } else if self.percentage >= 99.0 {
            self.power_supply_status = BatteryState::STATUS_FULL;
        } else {
            self.power_supply_status = BatteryState::STATUS_UNKNOWN;
        }

        // Simulate cell voltages if enabled
        if self.config.battery.enable_cell_monitoring && self.config.battery.cell_count > 0 {
            let avg_cell_voltage = self.voltage / self.config.battery.cell_count as f32;
            for i in 0..self.config.battery.cell_count as usize {
                let variation = (i as f32 - (self.config.battery.cell_count / 2) as f32) * 0.02;
                self.cell_voltages[i] = avg_cell_voltage + variation;
            }
        }
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        self.start_time = Some(self.now_nanos());
        self.last_read_time = 0;
        self.charge_mah = self.config.battery.capacity_mah * (self.config.initial_soc / 100.0);
        self.percentage = self.config.initial_soc;
        self.voltage = self.config.battery.full_voltage * (self.config.initial_soc / 100.0);
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

    /// Read battery state
    pub fn read(&mut self) -> HorusResult<BatteryState> {
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

    /// Generate a battery state reading
    fn generate_reading(&mut self) -> BatteryState {
        let current_time = self.now_nanos();
        let dt = if self.last_read_time > 0 {
            (current_time - self.last_read_time) as f32 / 1_000_000_000.0
        } else {
            0.0
        };
        self.last_read_time = current_time;

        // Simulate battery behavior
        self.simulate_discharge(dt);

        BatteryState {
            voltage: self.voltage,
            current: self.current,
            charge: self.charge_mah / 1000.0, // Convert to Ah
            capacity: self.config.battery.capacity_mah / 1000.0,
            percentage: self.percentage,
            power_supply_status: self.power_supply_status,
            temperature: self.temperature,
            cell_voltages: self.cell_voltages,
            cell_count: if self.config.battery.enable_cell_monitoring {
                self.config.battery.cell_count
            } else {
                0
            },
            timestamp: current_time,
        }
    }
}

impl Default for SimulationBatteryDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulation_driver_lifecycle() {
        let mut driver = SimulationBatteryDriver::new();

        assert_eq!(driver.status(), DriverStatus::Uninitialized);
        assert!(driver.is_available());

        driver.init().unwrap();
        assert_eq!(driver.status(), DriverStatus::Ready);

        let state = driver.read().unwrap();
        assert_eq!(driver.status(), DriverStatus::Running);

        // Check state data
        assert!(state.voltage > 0.0);
        assert!(state.percentage >= 0.0 && state.percentage <= 100.0);

        driver.shutdown().unwrap();
        assert_eq!(driver.status(), DriverStatus::Shutdown);
    }

    #[test]
    fn test_custom_battery_config() {
        let config = BatteryConfig::lipo_4s(10000.0);
        let mut driver = SimulationBatteryDriver::with_battery_config(config);

        driver.init().unwrap();
        let state = driver.read().unwrap();

        // 4S LiPo should have ~16.8V full
        assert!(state.voltage > 14.0);
        assert_eq!(state.capacity, 10.0); // 10Ah = 10000mAh
    }
}
