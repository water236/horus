//! Linux SPI driver
//!
//! SPI driver using Linux spidev interface.
//! Requires the `spi-hardware` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};

/// Linux SPI driver configuration
#[derive(Debug, Clone)]
pub struct LinuxSpiConfig {
    /// SPI device path (e.g., "/dev/spidev0.0")
    pub device: String,
    /// SPI clock speed in Hz
    pub speed_hz: u32,
    /// SPI mode (0-3)
    pub mode: u8,
    /// Bits per word
    pub bits_per_word: u8,
}

impl Default for LinuxSpiConfig {
    fn default() -> Self {
        Self {
            device: "/dev/spidev0.0".to_string(),
            speed_hz: 1_000_000,
            mode: 0,
            bits_per_word: 8,
        }
    }
}

/// Linux SPI driver using spidev
pub struct LinuxSpiDriver {
    config: LinuxSpiConfig,
    status: DriverStatus,
    device: Option<spidev::Spidev>,
}

impl LinuxSpiDriver {
    /// Create a new Linux SPI driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: LinuxSpiConfig::default(),
            status: DriverStatus::Uninitialized,
            device: None,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: LinuxSpiConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            device: None,
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        use spidev::{SpiModeFlags, Spidev, SpidevOptions};

        let mut spi = Spidev::open(&self.config.device)
            .map_err(|e| HorusError::driver(format!("Failed to open SPI device: {}", e)))?;

        let mode = match self.config.mode {
            0 => SpiModeFlags::SPI_MODE_0,
            1 => SpiModeFlags::SPI_MODE_1,
            2 => SpiModeFlags::SPI_MODE_2,
            3 => SpiModeFlags::SPI_MODE_3,
            _ => SpiModeFlags::SPI_MODE_0,
        };

        let options = SpidevOptions::new()
            .bits_per_word(self.config.bits_per_word)
            .max_speed_hz(self.config.speed_hz)
            .mode(mode)
            .build();

        spi.configure(&options)
            .map_err(|e| HorusError::driver(format!("Failed to configure SPI: {}", e)))?;

        self.device = Some(spi);
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

    /// Transfer data (full duplex)
    pub fn transfer(&mut self, tx: &[u8], rx: &mut [u8]) -> HorusResult<()> {
        use spidev::SpidevTransfer;

        let dev = self
            .device
            .as_mut()
            .ok_or_else(|| HorusError::driver("SPI not initialized"))?;

        let mut transfer = SpidevTransfer::read_write(tx, rx);
        dev.transfer(&mut transfer)
            .map_err(|e| HorusError::driver(format!("SPI transfer failed: {}", e)))?;

        self.status = DriverStatus::Running;
        Ok(())
    }

    /// Write data
    pub fn write(&mut self, data: &[u8]) -> HorusResult<()> {
        use std::io::Write;

        let dev = self
            .device
            .as_mut()
            .ok_or_else(|| HorusError::driver("SPI not initialized"))?;

        dev.write_all(data)
            .map_err(|e| HorusError::driver(format!("SPI write failed: {}", e)))?;

        self.status = DriverStatus::Running;
        Ok(())
    }
}

impl Default for LinuxSpiDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create Linux SPI driver")
    }
}
