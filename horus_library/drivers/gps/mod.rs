//! GPS drivers
//!
//! This module provides drivers for GPS sensors.

mod simulation;

#[cfg(feature = "nmea-gps")]
mod nmea;

pub use simulation::SimulationGpsDriver;

#[cfg(feature = "nmea-gps")]
pub use nmea::NmeaGpsDriver;

use horus_core::driver::DriverStatus;
use horus_core::error::HorusResult;

use crate::NavSatFix;

/// GPS driver backend selection
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum GpsDriverBackend {
    #[default]
    Simulation,
    #[cfg(feature = "nmea-gps")]
    Nmea,
}

/// Type-erased GPS driver
pub enum GpsDriver {
    Simulation(SimulationGpsDriver),
    #[cfg(feature = "nmea-gps")]
    Nmea(NmeaGpsDriver),
}

impl GpsDriver {
    pub fn new(backend: GpsDriverBackend) -> HorusResult<Self> {
        match backend {
            GpsDriverBackend::Simulation => Ok(Self::Simulation(SimulationGpsDriver::new())),
            #[cfg(feature = "nmea-gps")]
            GpsDriverBackend::Nmea => Ok(Self::Nmea(NmeaGpsDriver::new()?)),
        }
    }

    pub fn simulation() -> Self {
        Self::Simulation(SimulationGpsDriver::new())
    }

    // ========================================================================
    // Lifecycle methods
    // ========================================================================

    pub fn init(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.init(),
            #[cfg(feature = "nmea-gps")]
            Self::Nmea(d) => d.init(),
        }
    }

    pub fn shutdown(&mut self) -> HorusResult<()> {
        match self {
            Self::Simulation(d) => d.shutdown(),
            #[cfg(feature = "nmea-gps")]
            Self::Nmea(d) => d.shutdown(),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            Self::Simulation(d) => d.is_available(),
            #[cfg(feature = "nmea-gps")]
            Self::Nmea(d) => d.is_available(),
        }
    }

    pub fn status(&self) -> DriverStatus {
        match self {
            Self::Simulation(d) => d.status(),
            #[cfg(feature = "nmea-gps")]
            Self::Nmea(d) => d.status(),
        }
    }

    // ========================================================================
    // Sensor methods
    // ========================================================================

    pub fn read(&mut self) -> HorusResult<NavSatFix> {
        match self {
            Self::Simulation(d) => d.read(),
            #[cfg(feature = "nmea-gps")]
            Self::Nmea(d) => d.read(),
        }
    }

    pub fn has_data(&self) -> bool {
        match self {
            Self::Simulation(d) => d.has_data(),
            #[cfg(feature = "nmea-gps")]
            Self::Nmea(d) => d.has_data(),
        }
    }

    pub fn sample_rate(&self) -> Option<f32> {
        match self {
            Self::Simulation(d) => d.sample_rate(),
            #[cfg(feature = "nmea-gps")]
            Self::Nmea(d) => d.sample_rate(),
        }
    }
}
