//! NMEA GPS driver
//!
//! GPS driver using NMEA 0183 protocol over serial.
//! Requires the `nmea-gps` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};
use std::io::{BufRead, BufReader};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::NavSatFix;

/// NMEA GPS configuration
#[derive(Debug, Clone)]
pub struct NmeaGpsConfig {
    /// Serial port path
    pub port: String,
    /// Baud rate
    pub baud_rate: u32,
}

impl Default for NmeaGpsConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 9600,
        }
    }
}

/// NMEA GPS driver
pub struct NmeaGpsDriver {
    config: NmeaGpsConfig,
    status: DriverStatus,
    reader: Option<BufReader<Box<dyn serialport::SerialPort>>>,
    last_fix: Option<NavSatFix>,
}

impl NmeaGpsDriver {
    /// Create a new NMEA GPS driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: NmeaGpsConfig::default(),
            status: DriverStatus::Uninitialized,
            reader: None,
            last_fix: None,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: NmeaGpsConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            reader: None,
            last_fix: None,
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        let port = serialport::new(&self.config.port, self.config.baud_rate)
            .timeout(Duration::from_millis(1000))
            .open()
            .map_err(|e| HorusError::driver(format!("Failed to open serial port: {}", e)))?;

        self.reader = Some(BufReader::new(port));
        self.status = DriverStatus::Ready;
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
        self.reader = None;
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    /// Check if driver is available
    pub fn is_available(&self) -> bool {
        self.reader.is_some()
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Read GPS fix
    pub fn read(&mut self) -> HorusResult<NavSatFix> {
        let reader = self
            .reader
            .as_mut()
            .ok_or_else(|| HorusError::driver("GPS not initialized"))?;

        let mut line = String::new();

        // Read lines until we get a GGA or RMC sentence
        loop {
            line.clear();
            reader
                .read_line(&mut line)
                .map_err(|e| HorusError::driver(format!("Failed to read: {}", e)))?;

            if line.starts_with("$GPGGA") || line.starts_with("$GNGGA") {
                if let Some(fix) = self.parse_gga(&line) {
                    self.status = DriverStatus::Running;
                    self.last_fix = Some(fix.clone());
                    return Ok(fix);
                }
            }
        }
    }

    fn parse_gga(&self, sentence: &str) -> Option<NavSatFix> {
        let parts: Vec<&str> = sentence.split(',').collect();
        if parts.len() < 10 {
            return None;
        }

        // Parse latitude
        let lat_str = parts.get(2)?;
        let lat_dir = parts.get(3)?;
        let latitude = self.parse_nmea_coord(lat_str, *lat_dir == "S")?;

        // Parse longitude
        let lon_str = parts.get(4)?;
        let lon_dir = parts.get(5)?;
        let longitude = self.parse_nmea_coord(lon_str, *lon_dir == "W")?;

        // Parse altitude
        let altitude = parts.get(9)?.parse::<f64>().ok()?;

        // Parse fix quality
        let fix_quality = parts.get(6)?.parse::<u8>().unwrap_or(0);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut frame_id = [0u8; 32];
        let id_bytes = b"gps";
        frame_id[..id_bytes.len()].copy_from_slice(id_bytes);

        Some(NavSatFix {
            latitude,
            longitude,
            altitude,
            position_covariance: [0.0; 9],
            status: fix_quality as i8,
            service: 1, // GPS
            frame_id,
            timestamp,
        })
    }

    fn parse_nmea_coord(&self, coord: &str, negative: bool) -> Option<f64> {
        if coord.len() < 4 {
            return None;
        }

        // NMEA format: DDDMM.MMMMM
        let dot_pos = coord.find('.')?;
        let deg_len = dot_pos - 2;

        let degrees: f64 = coord[..deg_len].parse().ok()?;
        let minutes: f64 = coord[deg_len..].parse().ok()?;

        let mut result = degrees + minutes / 60.0;
        if negative {
            result = -result;
        }

        Some(result)
    }

    /// Check if data is available
    pub fn has_data(&self) -> bool {
        self.last_fix.is_some()
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> Option<f32> {
        Some(1.0) // Most GPS modules output at 1 Hz
    }
}

impl Default for NmeaGpsDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create NMEA GPS driver")
    }
}
