//! Linux I2C driver
//!
//! I2C driver using Linux i2cdev interface.
//! Requires the `i2c-hardware` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;

#[cfg(feature = "i2c-hardware")]
extern crate libc;

/// Linux I2C configuration
#[derive(Debug, Clone)]
pub struct LinuxI2cConfig {
    /// I2C device path (e.g., "/dev/i2c-1")
    pub device: String,
}

impl Default for LinuxI2cConfig {
    fn default() -> Self {
        Self {
            device: "/dev/i2c-1".to_string(),
        }
    }
}

/// Linux I2C driver using i2cdev
pub struct LinuxI2cDriver {
    config: LinuxI2cConfig,
    status: DriverStatus,
    device: Option<File>,
    current_addr: u16,
}

// I2C ioctl constants
#[cfg(feature = "i2c-hardware")]
const I2C_SLAVE: libc::c_ulong = 0x0703;

impl LinuxI2cDriver {
    /// Create a new Linux I2C driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: LinuxI2cConfig::default(),
            status: DriverStatus::Uninitialized,
            device: None,
            current_addr: 0,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: LinuxI2cConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            device: None,
            current_addr: 0,
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        let device = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.config.device)
            .map_err(|e| HorusError::driver(format!("Failed to open I2C device: {}", e)))?;

        self.device = Some(device);
        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.device = None;
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    /// Check if driver is available
    pub fn is_available(&self) -> bool {
        self.device.is_some()
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Set the I2C slave address
    fn set_address(&mut self, addr: u16) -> HorusResult<()> {
        if self.current_addr == addr {
            return Ok(());
        }

        let device = self
            .device
            .as_ref()
            .ok_or_else(|| HorusError::driver("I2C device not initialized"))?;

        // Set slave address via ioctl
        let ret = unsafe { libc::ioctl(device.as_raw_fd(), I2C_SLAVE, addr as libc::c_ulong) };

        if ret < 0 {
            return Err(HorusError::driver(format!(
                "Failed to set I2C address 0x{:02x}: {}",
                addr,
                std::io::Error::last_os_error()
            )));
        }

        self.current_addr = addr;
        Ok(())
    }

    /// Read bytes from I2C device
    pub fn read_bytes(&mut self, addr: u16, len: usize) -> HorusResult<Vec<u8>> {
        self.set_address(addr)?;

        let device = self
            .device
            .as_mut()
            .ok_or_else(|| HorusError::driver("I2C device not initialized"))?;

        let mut buf = vec![0u8; len];
        device
            .read_exact(&mut buf)
            .map_err(|e| HorusError::driver(format!("I2C read failed: {}", e)))?;

        self.status = DriverStatus::Running;
        Ok(buf)
    }

    /// Write bytes to I2C device
    pub fn write_bytes(&mut self, addr: u16, data: &[u8]) -> HorusResult<()> {
        self.set_address(addr)?;

        let device = self
            .device
            .as_mut()
            .ok_or_else(|| HorusError::driver("I2C device not initialized"))?;

        device
            .write_all(data)
            .map_err(|e| HorusError::driver(format!("I2C write failed: {}", e)))?;

        self.status = DriverStatus::Running;
        Ok(())
    }

    /// Write then read (common I2C pattern for register reads)
    pub fn write_read(
        &mut self,
        addr: u16,
        write_data: &[u8],
        read_len: usize,
    ) -> HorusResult<Vec<u8>> {
        self.write_bytes(addr, write_data)?;
        self.read_bytes(addr, read_len)
    }
}

impl Default for LinuxI2cDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create Linux I2C driver")
    }
}
