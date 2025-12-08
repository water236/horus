//! Simulation Bus drivers
//!
//! Provides simulated I2C, SPI, and CAN interfaces for testing.

use std::collections::HashMap;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

/// Simulation I2C bus driver
///
/// Simulates an I2C bus with configurable device registers.
pub struct SimulationI2cDriver {
    status: DriverStatus,
    /// Simulated device registers: device_addr -> (register_addr -> value)
    devices: HashMap<u8, HashMap<u8, u8>>,
    /// Current register pointer for each device
    register_pointers: HashMap<u8, u8>,
}

impl SimulationI2cDriver {
    pub fn new() -> Self {
        Self {
            status: DriverStatus::Uninitialized,
            devices: HashMap::new(),
            register_pointers: HashMap::new(),
        }
    }

    /// Add a simulated device with initial register values
    pub fn add_device(&mut self, addr: u8, registers: HashMap<u8, u8>) {
        self.devices.insert(addr, registers);
        self.register_pointers.insert(addr, 0);
    }

    /// Set a register value for a device
    pub fn set_register(&mut self, addr: u8, reg: u8, value: u8) {
        self.devices.entry(addr).or_default().insert(reg, value);
    }

    /// Get a register value from a device
    pub fn get_register(&self, addr: u8, reg: u8) -> Option<u8> {
        self.devices
            .get(&addr)
            .and_then(|regs| regs.get(&reg).copied())
    }

    pub fn init(&mut self) -> HorusResult<()> {
        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        true
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    pub fn read_bytes(&mut self, addr: u16, len: usize) -> HorusResult<Vec<u8>> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;

        let addr8 = addr as u8;
        let reg_ptr = *self.register_pointers.get(&addr8).unwrap_or(&0);

        let mut result = Vec::with_capacity(len);
        if let Some(device) = self.devices.get(&addr8) {
            for i in 0..len {
                let reg = reg_ptr.wrapping_add(i as u8);
                result.push(*device.get(&reg).unwrap_or(&0xFF));
            }
        } else {
            // Device not found, return 0xFF
            result.resize(len, 0xFF);
        }

        Ok(result)
    }

    pub fn write_bytes(&mut self, addr: u16, data: &[u8]) -> HorusResult<()> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;

        let addr8 = addr as u8;

        if data.is_empty() {
            return Ok(());
        }

        // First byte is typically the register address
        let reg = data[0];
        self.register_pointers.insert(addr8, reg);

        // Remaining bytes are data to write
        if data.len() > 1 {
            let device = self.devices.entry(addr8).or_default();
            for (i, &byte) in data[1..].iter().enumerate() {
                let target_reg = reg.wrapping_add(i as u8);
                device.insert(target_reg, byte);
            }
        }

        Ok(())
    }
}

impl Default for SimulationI2cDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Simulation SPI bus driver
///
/// Simulates an SPI bus with configurable response data.
pub struct SimulationSpiDriver {
    status: DriverStatus,
    /// Response data to return for reads
    response_data: Vec<u8>,
    /// Last written data
    last_write: Vec<u8>,
    /// Current chip select (stored as u16 addr)
    current_addr: u16,
}

impl SimulationSpiDriver {
    pub fn new() -> Self {
        Self {
            status: DriverStatus::Uninitialized,
            response_data: vec![0xFF; 256],
            last_write: Vec::new(),
            current_addr: 0,
        }
    }

    /// Set the response data that will be returned on reads
    pub fn set_response(&mut self, data: Vec<u8>) {
        self.response_data = data;
    }

    /// Get the last written data
    pub fn last_write(&self) -> &[u8] {
        &self.last_write
    }

    pub fn init(&mut self) -> HorusResult<()> {
        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        true
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    pub fn read_bytes(&mut self, addr: u16, len: usize) -> HorusResult<Vec<u8>> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;
        self.current_addr = addr;

        // Return from response buffer, cycling if needed
        let mut result = Vec::with_capacity(len);
        for i in 0..len {
            let idx = i % self.response_data.len().max(1);
            result.push(self.response_data.get(idx).copied().unwrap_or(0xFF));
        }

        Ok(result)
    }

    pub fn write_bytes(&mut self, addr: u16, data: &[u8]) -> HorusResult<()> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;
        self.current_addr = addr;
        self.last_write = data.to_vec();

        Ok(())
    }
}

impl Default for SimulationSpiDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Simulation CAN bus driver
///
/// Simulates a CAN bus with message injection/capture capabilities.
pub struct SimulationCanDriver {
    status: DriverStatus,
    /// Queue of frames to return on read (ID, data)
    rx_queue: Vec<(u32, Vec<u8>)>,
    /// History of sent frames (ID, data)
    tx_history: Vec<(u32, Vec<u8>)>,
}

impl SimulationCanDriver {
    pub fn new() -> Self {
        Self {
            status: DriverStatus::Uninitialized,
            rx_queue: Vec::new(),
            tx_history: Vec::new(),
        }
    }

    /// Inject a CAN frame to be received on next read
    pub fn inject_frame(&mut self, id: u32, data: Vec<u8>) {
        self.rx_queue.push((id, data));
    }

    /// Get history of sent frames
    pub fn tx_history(&self) -> &[(u32, Vec<u8>)] {
        &self.tx_history
    }

    /// Clear TX history
    pub fn clear_history(&mut self) {
        self.tx_history.clear();
    }

    pub fn init(&mut self) -> HorusResult<()> {
        self.rx_queue.clear();
        self.tx_history.clear();
        self.status = DriverStatus::Ready;
        Ok(())
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        true
    }

    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    pub fn read_bytes(&mut self, _addr: u16, len: usize) -> HorusResult<Vec<u8>> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;

        // Return next frame from queue if available
        if let Some((id, data)) = self.rx_queue.pop() {
            // Pack ID + data into result
            let mut result = Vec::with_capacity(4 + data.len());
            result.extend_from_slice(&id.to_le_bytes());
            result.extend(data);

            // Pad or truncate to requested length
            result.resize(len, 0);
            Ok(result)
        } else {
            // No data available
            Ok(vec![0; len])
        }
    }

    pub fn write_bytes(&mut self, addr: u16, data: &[u8]) -> HorusResult<()> {
        if !matches!(self.status, DriverStatus::Ready | DriverStatus::Running) {
            return Err(horus_core::error::HorusError::driver(
                "Driver not initialized",
            ));
        }
        self.status = DriverStatus::Running;

        // Store the frame (use addr as CAN ID)
        self.tx_history.push((addr as u32, data.to_vec()));

        Ok(())
    }
}

impl Default for SimulationCanDriver {
    fn default() -> Self {
        Self::new()
    }
}
