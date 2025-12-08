//! RPLidar driver
//!
//! Driver for SLAMTEC RPLidar A2/A3 sensors.
//! Requires the `rplidar` feature.

use horus_core::driver::DriverStatus;
use horus_core::error::{HorusError, HorusResult};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::LaserScan;

/// RPLidar configuration
#[derive(Debug, Clone)]
pub struct RplidarConfig {
    /// Serial port path
    pub port: String,
    /// Baud rate (default: 115200 for A2, 256000 for A3)
    pub baud_rate: u32,
    /// Scan mode (0 = standard, 1 = express, 2 = boost)
    pub scan_mode: u8,
}

impl Default for RplidarConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: 115200,
            scan_mode: 0,
        }
    }
}

/// RPLidar driver
pub struct RplidarDriver {
    config: RplidarConfig,
    status: DriverStatus,
    port: Option<Box<dyn serialport::SerialPort>>,
    scan_data: Vec<(f32, f32)>, // (angle, distance)
}

impl RplidarDriver {
    /// Create a new RPLidar driver
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            config: RplidarConfig::default(),
            status: DriverStatus::Uninitialized,
            port: None,
            scan_data: Vec::with_capacity(360),
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: RplidarConfig) -> HorusResult<Self> {
        Ok(Self {
            config,
            status: DriverStatus::Uninitialized,
            port: None,
            scan_data: Vec::with_capacity(360),
        })
    }

    /// Initialize the driver
    pub fn init(&mut self) -> HorusResult<()> {
        let port = serialport::new(&self.config.port, self.config.baud_rate)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|e| HorusError::driver(format!("Failed to open serial port: {}", e)))?;

        self.port = Some(port);

        // Send reset and start scan commands
        self.send_command(0x40)?; // Reset
        std::thread::sleep(Duration::from_millis(100));
        self.send_command(0x20)?; // Start scan

        self.status = DriverStatus::Ready;
        Ok(())
    }

    fn send_command(&mut self, cmd: u8) -> HorusResult<()> {
        use std::io::Write;

        if let Some(port) = &mut self.port {
            let packet = [0xA5, cmd];
            port.write_all(&packet)
                .map_err(|e| HorusError::driver(format!("Failed to send command: {}", e)))?;
        }
        Ok(())
    }

    /// Shutdown the driver
    pub fn shutdown(&mut self) -> HorusResult<()> {
        if self.port.is_some() {
            self.send_command(0x25)?; // Stop scan
        }
        self.port = None;
        self.status = DriverStatus::Shutdown;
        Ok(())
    }

    /// Check if driver is available
    pub fn is_available(&self) -> bool {
        self.port.is_some()
    }

    /// Get driver status
    pub fn status(&self) -> DriverStatus {
        self.status.clone()
    }

    /// Read laser scan
    pub fn read(&mut self) -> HorusResult<LaserScan> {
        use std::io::Read;

        let port = self
            .port
            .as_mut()
            .ok_or_else(|| HorusError::driver("LiDAR not initialized"))?;

        self.scan_data.clear();

        // Read scan data (simplified - real protocol is more complex)
        let mut buf = [0u8; 5];
        let mut last_angle = 0.0f32;
        let start_new_scan = false;

        // Collect points for one revolution
        loop {
            if port.read_exact(&mut buf).is_err() {
                break;
            }

            // Parse scan point (simplified)
            let quality = buf[0] >> 2;
            if quality == 0 {
                continue;
            }

            let angle_raw = ((buf[1] as u16) | ((buf[2] as u16) << 8)) >> 1;
            let angle = (angle_raw as f32) / 64.0;

            let distance_raw = (buf[3] as u16) | ((buf[4] as u16) << 8);
            let distance = (distance_raw as f32) / 4.0 / 1000.0; // Convert to meters

            // Check for new scan (angle wrapped around)
            if angle < last_angle && self.scan_data.len() > 100 {
                break;
            }
            last_angle = angle;

            self.scan_data.push((angle.to_radians(), distance));
        }

        self.status = DriverStatus::Running;

        // Convert to LaserScan message
        let num_points = 360;
        let angle_min = 0.0;
        let angle_max = 2.0 * std::f32::consts::PI;
        let angle_increment = (angle_max - angle_min) / num_points as f32;

        let mut ranges = vec![f32::INFINITY; num_points];
        let intensities = vec![0.0; num_points];

        for (angle, distance) in &self.scan_data {
            let idx = ((angle / angle_increment) as usize).min(num_points - 1);
            if *distance > 0.0 && *distance < ranges[idx] {
                ranges[idx] = *distance;
            }
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut frame_id = [0u8; 32];
        let id_bytes = b"rplidar";
        frame_id[..id_bytes.len()].copy_from_slice(id_bytes);

        Ok(LaserScan {
            angle_min,
            angle_max,
            angle_increment,
            time_increment: 0.0,
            scan_time: 0.1,
            range_min: 0.15,
            range_max: 12.0,
            ranges,
            intensities,
            frame_id,
            timestamp,
        })
    }

    /// Check if data is available
    pub fn has_data(&self) -> bool {
        self.is_available()
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> Option<f32> {
        Some(10.0) // ~10 Hz scan rate
    }
}

impl Default for RplidarDriver {
    fn default() -> Self {
        Self::new().expect("Failed to create RPLidar driver")
    }
}
