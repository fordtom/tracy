//! Inertial Measurement Unit (IMU) Driver
//!
//! This module provides a driver for 6-axis IMU sensors commonly used
//! in automotive and aerospace applications for attitude estimation.
//!
//! # Requirements Coverage
//! - SRS-IMU-001: IMU shall provide 3-axis accelerometer data
//! - SRS-IMU-002: IMU shall provide 3-axis gyroscope data
//! - SRS-IMU-003: IMU shall support sample rates up to 1kHz

use core::fmt;
use embedded_hal::spi::SpiDevice;

/// HLR-IMU-001: Accelerometer full-scale range options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccelRange {
    /// +/- 2g (highest resolution)
    G2 = 0,
    /// +/- 4g
    G4 = 1,
    /// +/- 8g
    G8 = 2,
    /// +/- 16g (highest range)
    G16 = 3,
}

/// HLR-IMU-002: Gyroscope full-scale range options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GyroRange {
    /// +/- 250 deg/s
    Dps250 = 0,
    /// +/- 500 deg/s
    Dps500 = 1,
    /// +/- 1000 deg/s
    Dps1000 = 2,
    /// +/- 2000 deg/s
    Dps2000 = 3,
}

/// HLR-IMU-003: Output data rate configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputDataRate {
    /// 100 Hz sample rate
    Hz100 = 0,
    /// 200 Hz sample rate
    Hz200 = 1,
    /// 500 Hz sample rate
    Hz500 = 2,
    /// 1000 Hz sample rate (SRS-IMU-003)
    Hz1000 = 3,
}

/// IMU configuration structure
///
/// LLR-IMU-010: Configuration shall be validated before applying
#[derive(Debug, Clone)]
pub struct ImuConfig {
    /// Accelerometer range
    pub accel_range: AccelRange,
    /// Gyroscope range
    pub gyro_range: GyroRange,
    /// Output data rate
    pub odr: OutputDataRate,
    /// LLR-IMU-011: Enable low-pass filter
    pub lpf_enabled: bool,
    /// LLR-IMU-012: Low-pass filter cutoff (Hz)
    pub lpf_cutoff_hz: u16,
}

impl Default for ImuConfig {
    /// LLR-IMU-013: Default configuration for automotive use
    fn default() -> Self {
        Self {
            accel_range: AccelRange::G8,
            gyro_range: GyroRange::Dps500,
            odr: OutputDataRate::Hz200,
            lpf_enabled: true,
            lpf_cutoff_hz: 50,
        }
    }
}

/// Raw IMU reading from sensor
///
/// SRS-IMU-010: Data structure shall include timestamp
#[derive(Debug, Clone, Copy, Default)]
pub struct ImuReading {
    /// X-axis acceleration (raw ADC value)
    pub accel_x: i16,
    /// Y-axis acceleration
    pub accel_y: i16,
    /// Z-axis acceleration
    pub accel_z: i16,
    /// X-axis angular rate (raw ADC value)
    pub gyro_x: i16,
    /// Y-axis angular rate
    pub gyro_y: i16,
    /// Z-axis angular rate
    pub gyro_z: i16,
    /// Temperature sensor (for compensation)
    pub temperature: i16,
    /// SRS-IMU-010: Timestamp in microseconds
    pub timestamp_us: u64,
}

/// Scaled IMU data in engineering units
///
/// SRS-IMU-011: Driver shall provide data in SI units
#[derive(Debug, Clone, Copy, Default)]
pub struct ImuData {
    /// X acceleration in m/s^2
    pub accel_x: f32,
    /// Y acceleration in m/s^2
    pub accel_y: f32,
    /// Z acceleration in m/s^2
    pub accel_z: f32,
    /// X angular rate in rad/s
    pub gyro_x: f32,
    /// Y angular rate in rad/s
    pub gyro_y: f32,
    /// Z angular rate in rad/s
    pub gyro_z: f32,
    /// Temperature in Celsius
    pub temperature_c: f32,
    /// Timestamp in microseconds
    pub timestamp_us: u64,
}

/// IMU driver errors
///
/// LLR-IMU-020: All error conditions shall be enumerated
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImuError {
    /// SPI communication failed
    SpiFailed,
    /// Device ID mismatch (wrong sensor)
    WrongDevice,
    /// Self-test failed
    SelfTestFailed,
    /// Configuration invalid
    InvalidConfig,
    /// Sensor not ready
    NotReady,
    /// Data overrun (missed samples)
    Overrun,
}

impl fmt::Display for ImuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImuError::SpiFailed => write!(f, "SPI communication error"),
            ImuError::WrongDevice => write!(f, "Device ID mismatch"),
            ImuError::SelfTestFailed => write!(f, "Self-test failed"),
            ImuError::InvalidConfig => write!(f, "Invalid configuration"),
            ImuError::NotReady => write!(f, "Sensor not ready"),
            ImuError::Overrun => write!(f, "Data overrun"),
        }
    }
}

/// IMU driver instance
///
/// SRS-IMU-020: Driver shall manage single sensor instance
pub struct Imu<SPI> {
    spi: SPI,
    config: ImuConfig,
    accel_scale: f32,
    gyro_scale: f32,
    initialized: bool,
}

impl<SPI, E> Imu<SPI>
where
    SPI: SpiDevice<Error = E>,
{
    /// Create new IMU driver instance
    ///
    /// LLR-IMU-030: Constructor shall not access hardware
    pub fn new(spi: SPI) -> Self {
        Self {
            spi,
            config: ImuConfig::default(),
            accel_scale: 0.0,
            gyro_scale: 0.0,
            initialized: false,
        }
    }

    /// Initialize the IMU sensor
    ///
    /// SRS-IMU-030: Initialization shall verify device identity
    /// SRS-IMU-031: Initialization shall run self-test
    /// SRS-IMU-032: Initialization shall complete within 100ms
    pub fn init(&mut self, config: ImuConfig) -> Result<(), ImuError> {
        // LLR-IMU-040: Read and verify device ID
        let device_id = self.read_register(REG_WHO_AM_I)?;
        if device_id != EXPECTED_DEVICE_ID {
            return Err(ImuError::WrongDevice);
        }

        // LLR-IMU-041: Perform soft reset
        self.write_register(REG_CTRL1, CTRL1_SOFT_RESET)?;
        self.delay_ms(10);

        // LLR-IMU-042: Run built-in self-test
        if !self.run_self_test()? {
            return Err(ImuError::SelfTestFailed);
        }

        // LLR-IMU-043: Apply configuration
        self.apply_config(&config)?;
        self.config = config;

        // LLR-IMU-044: Calculate scale factors
        self.accel_scale = self.calc_accel_scale();
        self.gyro_scale = self.calc_gyro_scale();

        self.initialized = true;
        Ok(())
    }

    /// Read raw sensor data
    ///
    /// SRS-IMU-040: Read shall be atomic (all axes from same sample)
    pub fn read_raw(&mut self) -> Result<ImuReading, ImuError> {
        if !self.initialized {
            return Err(ImuError::NotReady);
        }

        // LLR-IMU-050: Check data ready status
        let status = self.read_register(REG_STATUS)?;
        if status & STATUS_DATA_READY == 0 {
            return Err(ImuError::NotReady);
        }

        // LLR-IMU-051: Check for overrun
        if status & STATUS_OVERRUN != 0 {
            // SAF-IMU-001: Log overrun events
            return Err(ImuError::Overrun);
        }

        // LLR-IMU-052: Burst read all data registers
        let mut buffer = [0u8; 14];
        self.read_registers(REG_DATA_START, &mut buffer)?;

        Ok(ImuReading {
            accel_x: i16::from_be_bytes([buffer[0], buffer[1]]),
            accel_y: i16::from_be_bytes([buffer[2], buffer[3]]),
            accel_z: i16::from_be_bytes([buffer[4], buffer[5]]),
            gyro_x: i16::from_be_bytes([buffer[6], buffer[7]]),
            gyro_y: i16::from_be_bytes([buffer[8], buffer[9]]),
            gyro_z: i16::from_be_bytes([buffer[10], buffer[11]]),
            temperature: i16::from_be_bytes([buffer[12], buffer[13]]),
            timestamp_us: self.get_timestamp(),
        })
    }

    /// Read and convert sensor data to engineering units
    ///
    /// SRS-IMU-011: Convert to SI units (m/s^2, rad/s, Celsius)
    pub fn read(&mut self) -> Result<ImuData, ImuError> {
        let raw = self.read_raw()?;

        // LLR-IMU-060: Apply calibration scale factors
        Ok(ImuData {
            accel_x: raw.accel_x as f32 * self.accel_scale,
            accel_y: raw.accel_y as f32 * self.accel_scale,
            accel_z: raw.accel_z as f32 * self.accel_scale,
            gyro_x: raw.gyro_x as f32 * self.gyro_scale,
            gyro_y: raw.gyro_y as f32 * self.gyro_scale,
            gyro_z: raw.gyro_z as f32 * self.gyro_scale,
            temperature_c: self.convert_temperature(raw.temperature),
            timestamp_us: raw.timestamp_us,
        })
    }

    /// Enter low-power sleep mode
    ///
    /// SRS-IMU-050: Driver shall support low-power mode
    /// LLR-IMU-070: Sleep current shall be < 10uA
    pub fn sleep(&mut self) -> Result<(), ImuError> {
        self.write_register(REG_PWR_MGMT, PWR_SLEEP)?;
        self.initialized = false;  // LLR-IMU-071: Require re-init after wake
        Ok(())
    }

    // Private helper methods

    fn read_register(&mut self, reg: u8) -> Result<u8, ImuError> {
        let mut buffer = [reg | 0x80, 0];
        self.spi.transfer_in_place(&mut buffer).map_err(|_| ImuError::SpiFailed)?;
        Ok(buffer[1])
    }

    fn read_registers(&mut self, reg: u8, buffer: &mut [u8]) -> Result<(), ImuError> {
        let mut cmd = [reg | 0x80 | 0x40]; // Read + auto-increment
        self.spi.transfer_in_place(&mut cmd).map_err(|_| ImuError::SpiFailed)?;
        self.spi.transfer_in_place(buffer).map_err(|_| ImuError::SpiFailed)?;
        Ok(())
    }

    fn write_register(&mut self, reg: u8, value: u8) -> Result<(), ImuError> {
        let buffer = [reg, value];
        self.spi.write(&buffer).map_err(|_| ImuError::SpiFailed)?;
        Ok(())
    }

    fn run_self_test(&mut self) -> Result<bool, ImuError> {
        // LLR-IMU-080: Self-test procedure per datasheet
        self.write_register(REG_SELF_TEST, SELF_TEST_ENABLE)?;
        self.delay_ms(50);

        let result = self.read_register(REG_SELF_TEST)?;
        self.write_register(REG_SELF_TEST, 0)?;

        Ok(result & SELF_TEST_PASS != 0)
    }

    fn apply_config(&mut self, config: &ImuConfig) -> Result<(), ImuError> {
        // LLR-IMU-090: Configure accelerometer
        let accel_cfg = (config.accel_range as u8) << 3;
        self.write_register(REG_ACCEL_CFG, accel_cfg)?;

        // LLR-IMU-091: Configure gyroscope
        let gyro_cfg = (config.gyro_range as u8) << 3;
        self.write_register(REG_GYRO_CFG, gyro_cfg)?;

        // LLR-IMU-092: Configure output data rate
        let odr_cfg = config.odr as u8;
        self.write_register(REG_ODR_CFG, odr_cfg)?;

        // LLR-IMU-093: Configure low-pass filter
        if config.lpf_enabled {
            let lpf_cfg = self.calc_lpf_config(config.lpf_cutoff_hz);
            self.write_register(REG_LPF_CFG, lpf_cfg)?;
        }

        Ok(())
    }

    fn calc_accel_scale(&self) -> f32 {
        // LLR-IMU-100: Scale factor in m/s^2 per LSB
        const G: f32 = 9.80665;
        match self.config.accel_range {
            AccelRange::G2 => (2.0 * G) / 32768.0,
            AccelRange::G4 => (4.0 * G) / 32768.0,
            AccelRange::G8 => (8.0 * G) / 32768.0,
            AccelRange::G16 => (16.0 * G) / 32768.0,
        }
    }

    fn calc_gyro_scale(&self) -> f32 {
        // LLR-IMU-101: Scale factor in rad/s per LSB
        const DEG_TO_RAD: f32 = core::f32::consts::PI / 180.0;
        match self.config.gyro_range {
            GyroRange::Dps250 => (250.0 * DEG_TO_RAD) / 32768.0,
            GyroRange::Dps500 => (500.0 * DEG_TO_RAD) / 32768.0,
            GyroRange::Dps1000 => (1000.0 * DEG_TO_RAD) / 32768.0,
            GyroRange::Dps2000 => (2000.0 * DEG_TO_RAD) / 32768.0,
        }
    }

    fn convert_temperature(&self, raw: i16) -> f32 {
        // LLR-IMU-102: Temperature conversion per datasheet
        (raw as f32 / 340.0) + 36.53
    }

    fn calc_lpf_config(&self, cutoff_hz: u16) -> u8 {
        // LLR-IMU-103: Map cutoff frequency to register value
        match cutoff_hz {
            0..=20 => 0,
            21..=50 => 1,
            51..=100 => 2,
            _ => 3,
        }
    }

    fn delay_ms(&self, _ms: u32) {
        // Platform-specific delay
    }

    fn get_timestamp(&self) -> u64 {
        // Platform-specific timestamp
        0
    }
}

// Register definitions
const REG_WHO_AM_I: u8 = 0x75;
const REG_CTRL1: u8 = 0x6B;
const REG_STATUS: u8 = 0x3A;
const REG_DATA_START: u8 = 0x3B;
const REG_PWR_MGMT: u8 = 0x6B;
const REG_ACCEL_CFG: u8 = 0x1C;
const REG_GYRO_CFG: u8 = 0x1B;
const REG_ODR_CFG: u8 = 0x19;
const REG_LPF_CFG: u8 = 0x1A;
const REG_SELF_TEST: u8 = 0x0D;

const EXPECTED_DEVICE_ID: u8 = 0x71;
const CTRL1_SOFT_RESET: u8 = 0x80;
const STATUS_DATA_READY: u8 = 0x01;
const STATUS_OVERRUN: u8 = 0x10;
const PWR_SLEEP: u8 = 0x40;
const SELF_TEST_ENABLE: u8 = 0x01;
const SELF_TEST_PASS: u8 = 0x80;
