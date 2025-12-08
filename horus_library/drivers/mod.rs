//! Hardware drivers for HORUS
//!
//! This module contains driver implementations that provide hardware abstraction
//! for built-in nodes. Drivers implement traits from `horus_core::driver`.
//!
//! # Architecture
//!
//! ```text
//! Nodes (horus_library/nodes/)
//!   │
//!   └── use Driver traits (horus_core::driver)
//!           │
//!           ├── Built-in Drivers (this module)
//!           │   ├── Simulation drivers (always available)
//!           │   └── Hardware drivers (feature-gated)
//!           │
//!           └── External Drivers (marketplace crates)
//! ```
//!
//! # Available Driver Categories
//!
//! ## Sensors
//! - `imu` - Inertial Measurement Units (accelerometer, gyroscope, magnetometer)
//! - `camera` - Vision sensors (RGB, depth, stereo)
//! - `lidar` - Laser range finders
//! - `gps` - Global positioning systems
//! - `encoder` - Rotary encoders for odometry
//! - `ultrasonic` - Ultrasonic distance sensors
//! - `depth_camera` - Depth cameras (RealSense, etc.)
//! - `battery` - Battery monitoring
//! - `force_torque` - Force/torque sensors
//!
//! ## Actuators
//! - `motor` - DC motors
//! - `servo` - Position-controlled servos
//! - `bldc` - Brushless DC motors with ESC
//! - `stepper` - Stepper motors
//! - `dynamixel` - Dynamixel smart servos
//! - `roboclaw` - RoboClaw motor controllers
//!
//! ## Buses
//! - `bus` - Communication buses (I2C, SPI, CAN)
//! - `serial` - Serial port (UART)
//! - `modbus` - Modbus protocol
//!
//! ## Input
//! - `joystick` - Gamepad/joystick input
//! - `keyboard` - Keyboard input
//!
//! ## Other
//! - `digital_io` - Digital GPIO input/output
//!
//! # Adding a New Driver
//!
//! 1. Create a new module under the appropriate category (e.g., `drivers/imu/`)
//! 2. Implement the appropriate trait (`Sensor`, `Actuator`, or `Bus`)
//! 3. Add feature gate if hardware-specific
//! 4. Re-export from this module

// Sensor drivers
pub mod battery;
pub mod camera;
pub mod depth_camera;
pub mod encoder;
pub mod force_torque;
pub mod gps;
pub mod imu;
pub mod lidar;
pub mod ultrasonic;

// Actuator drivers
pub mod bldc;
pub mod dynamixel;
pub mod motor;
pub mod roboclaw;
pub mod servo;
pub mod stepper;

// Bus drivers
pub mod bus;
pub mod modbus;
pub mod serial;

// Input drivers
pub mod joystick;
pub mod keyboard;

// Other drivers
pub mod digital_io;

// ============================================================================
// IMU Drivers
// ============================================================================
pub use imu::{ImuDriver, SimulationImuDriver};

#[cfg(feature = "mpu6050-imu")]
pub use imu::Mpu6050Driver;

#[cfg(feature = "bno055-imu")]
pub use imu::Bno055Driver;

// ============================================================================
// Camera Drivers
// ============================================================================
pub use camera::{CameraDriver, SimulationCameraDriver};

#[cfg(feature = "opencv-backend")]
pub use camera::OpenCvCameraDriver;

#[cfg(feature = "v4l2-backend")]
pub use camera::V4l2CameraDriver;

// ============================================================================
// LiDAR Drivers
// ============================================================================
pub use lidar::{LidarDriver, SimulationLidarDriver};

#[cfg(feature = "rplidar")]
pub use lidar::RplidarDriver;

// ============================================================================
// GPS Drivers
// ============================================================================
pub use gps::{GpsDriver, SimulationGpsDriver};

#[cfg(feature = "nmea-gps")]
pub use gps::NmeaGpsDriver;

// ============================================================================
// Encoder Drivers
// ============================================================================
pub use encoder::{EncoderDriver, SimulationEncoderDriver};

#[cfg(feature = "gpio-hardware")]
pub use encoder::GpioEncoderDriver;

// ============================================================================
// Ultrasonic Drivers
// ============================================================================
pub use ultrasonic::{SimulationUltrasonicDriver, UltrasonicDriver};

#[cfg(feature = "gpio-hardware")]
pub use ultrasonic::GpioUltrasonicDriver;

// ============================================================================
// Depth Camera Drivers
// ============================================================================
pub use depth_camera::{DepthCameraDriver, DepthCameraFrame, SimulationDepthCameraDriver};

#[cfg(feature = "realsense")]
pub use depth_camera::RealSenseDriver;

// ============================================================================
// Battery Drivers
// ============================================================================
pub use battery::{
    BatteryChemistry, BatteryConfig, BatteryDriver, BatteryDriverBackend, SimulationBatteryDriver,
};

#[cfg(feature = "i2c-hardware")]
pub use battery::I2cBatteryDriver;

// ============================================================================
// Force/Torque Drivers
// ============================================================================
pub use force_torque::{
    ForceTorqueDriver, ForceTorqueDriverBackend, FtSensorModel, SimulationForceTorqueDriver,
};

#[cfg(feature = "netft")]
pub use force_torque::AtiNetFtDriver;

#[cfg(feature = "robotiq-serial")]
pub use force_torque::RobotiqSerialDriver;

// ============================================================================
// Motor Drivers (DC)
// ============================================================================
pub use motor::{MotorDriver, SimulationMotorDriver};

#[cfg(feature = "gpio-hardware")]
pub use motor::GpioMotorDriver;

// ============================================================================
// Servo Drivers
// ============================================================================
pub use servo::{ServoDriver, SimulationServoDriver};

#[cfg(feature = "i2c-hardware")]
pub use servo::Pca9685ServoDriver;

// ============================================================================
// BLDC Motor Drivers
// ============================================================================
pub use bldc::{
    BldcCommand, BldcConfig, BldcDriver, BldcDriverBackend, BldcProtocol, SimulationBldcDriver,
};

#[cfg(feature = "gpio-hardware")]
pub use bldc::PwmBldcDriver;

// ============================================================================
// Stepper Motor Drivers
// ============================================================================
pub use stepper::{
    SimulationStepperDriver, StepperConfig, StepperDriver, StepperDriverBackend,
    StepperDriverCommand,
};

#[cfg(feature = "gpio-hardware")]
pub use stepper::GpioStepperDriver;

// ============================================================================
// Dynamixel Drivers
// ============================================================================
pub use dynamixel::{
    DynamixelCommand, DynamixelConfig, DynamixelDriver, DynamixelDriverBackend, DynamixelMode,
    DynamixelProtocol, SimulationDynamixelDriver,
};

#[cfg(feature = "serial-hardware")]
pub use dynamixel::SerialDynamixelDriver;

// ============================================================================
// RoboClaw Drivers
// ============================================================================
pub use roboclaw::{
    RoboclawCommand, RoboclawConfig, RoboclawDriver, RoboclawDriverBackend, RoboclawMode,
    SimulationRoboclawDriver,
};

#[cfg(feature = "serial-hardware")]
pub use roboclaw::SerialRoboclawDriver;

// ============================================================================
// Bus Drivers (I2C, SPI, CAN)
// ============================================================================
pub use bus::{
    CanDriver, I2cDriver, SimulationCanDriver, SimulationI2cDriver, SimulationSpiDriver, SpiDriver,
};

#[cfg(feature = "i2c-hardware")]
pub use bus::LinuxI2cDriver;

#[cfg(feature = "spi-hardware")]
pub use bus::LinuxSpiDriver;

#[cfg(feature = "can-hardware")]
pub use bus::SocketCanDriver;

// ============================================================================
// Serial Drivers
// ============================================================================
pub use serial::{
    SerialConfig, SerialDriver, SerialDriverBackend, SerialFlowControl, SerialParity,
    SimulationSerialDriver,
};

#[cfg(feature = "serial-hardware")]
pub use serial::SystemSerialDriver;

// ============================================================================
// Modbus Drivers
// ============================================================================
pub use modbus::{ModbusConfig, ModbusDriver, ModbusDriverBackend, SimulationModbusDriver};

#[cfg(feature = "serial-hardware")]
pub use modbus::RtuModbusDriver;

// ============================================================================
// Joystick Drivers
// ============================================================================
pub use joystick::{
    ButtonMapping, JoystickConfig, JoystickDriver, JoystickDriverBackend, SimulationJoystickDriver,
};

#[cfg(feature = "gilrs")]
pub use joystick::GilrsJoystickDriver;

// ============================================================================
// Keyboard Drivers
// ============================================================================
pub use keyboard::{KeyboardDriver, KeyboardDriverBackend, SimulationKeyboardDriver};

#[cfg(feature = "crossterm")]
pub use keyboard::CrosstermKeyboardDriver;

// ============================================================================
// Digital I/O Drivers
// ============================================================================
pub use digital_io::{
    DigitalIoConfig, DigitalIoDriver, DigitalIoDriverBackend, DigitalIoPin, PinMode,
    SimulationDigitalIoDriver,
};

#[cfg(feature = "gpio-hardware")]
pub use digital_io::GpioDigitalIoDriver;

// ============================================================================
// Driver Factory (runtime driver instantiation from config)
// ============================================================================
pub mod factory;
pub use factory::{
    create_battery_driver, create_camera_driver, create_drivers_from_config, create_encoder_driver,
    create_force_torque_driver, create_gps_driver, create_imu_driver, create_joystick_driver,
    create_keyboard_driver, create_lidar_driver, create_motor_driver, create_servo_driver,
    create_ultrasonic_driver, list_available_backends, CreatedDrivers,
};
