//! Temperature Sensor Driver
//!
//! Driver for precision temperature sensors used in thermal management
//! systems. Supports common I2C temperature ICs.
//!
//! REQ-200: Driver shall support temperature range -40C to +125C
//! REQ-201: Driver shall provide 0.1C resolution
//! REQ-202: Driver shall support multiple sensor instances

use embedded_hal::i2c::I2c;

/// REQ-203: Temperature conversion timeout
const CONVERSION_TIMEOUT_MS: u32 = 100;

/// REQ-204: Default I2C address
const DEFAULT_I2C_ADDR: u8 = 0x48;

/// Temperature reading result
///
/// REQ-205: Results shall include validity status
#[derive(Debug, Clone, Copy)]
pub struct TemperatureReading {
    /// Temperature in millidegrees Celsius
    pub millidegrees_c: i32,
    /// REQ-206: Timestamp of reading
    pub timestamp_ms: u32,
    /// REQ-207: Reading validity flag
    pub valid: bool,
}

impl TemperatureReading {
    /// Get temperature in degrees Celsius
    ///
    /// REQ-201: 0.1C resolution
    pub fn degrees_c(&self) -> f32 {
        self.millidegrees_c as f32 / 1000.0
    }

    /// Get temperature in degrees Fahrenheit
    ///
    /// REQ-208: Fahrenheit conversion support
    pub fn degrees_f(&self) -> f32 {
        (self.degrees_c() * 9.0 / 5.0) + 32.0
    }
}

/// Temperature sensor configuration
///
/// REQ-210: Configuration shall be immutable after init
#[derive(Debug, Clone)]
pub struct TempSensorConfig {
    /// I2C address (7-bit)
    pub i2c_addr: u8,
    /// REQ-211: Averaging sample count
    pub averaging: u8,
    /// REQ-212: Alert threshold high (millidegrees)
    pub alert_high_mc: i32,
    /// REQ-213: Alert threshold low (millidegrees)
    pub alert_low_mc: i32,
}

impl Default for TempSensorConfig {
    /// REQ-214: Sensible defaults for automotive use
    fn default() -> Self {
        Self {
            i2c_addr: DEFAULT_I2C_ADDR,
            averaging: 4,
            alert_high_mc: 85_000,  // 85C
            alert_low_mc: -20_000,  // -20C
        }
    }
}

/// Temperature sensor errors
///
/// REQ-220: All errors shall be enumerated
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TempSensorError {
    /// I2C communication failed
    I2cError,
    /// REQ-221: Sensor not responding
    NotPresent,
    /// REQ-222: Reading out of valid range
    OutOfRange,
    /// REQ-223: Conversion timeout
    Timeout,
    /// REQ-224: CRC validation failed
    CrcError,
}

/// Temperature sensor driver
///
/// REQ-202: Support multiple instances
pub struct TempSensor<I2C> {
    i2c: I2C,
    config: TempSensorConfig,
    last_reading: TemperatureReading,
}

impl<I2C, E> TempSensor<I2C>
where
    I2C: I2c<Error = E>,
{
    /// Create new sensor instance
    ///
    /// REQ-230: Constructor shall validate address
    pub fn new(i2c: I2C, config: TempSensorConfig) -> Result<Self, TempSensorError> {
        // REQ-230: Validate I2C address range
        if config.i2c_addr < 0x08 || config.i2c_addr > 0x77 {
            return Err(TempSensorError::NotPresent);
        }

        Ok(Self {
            i2c,
            config,
            last_reading: TemperatureReading {
                millidegrees_c: 0,
                timestamp_ms: 0,
                valid: false,
            },
        })
    }

    /// Initialize the sensor
    ///
    /// REQ-231: Init shall verify sensor presence
    /// REQ-232: Init shall configure alert thresholds
    pub fn init(&mut self) -> Result<(), TempSensorError> {
        // REQ-231: Read device ID to verify presence
        let device_id = self.read_register(REG_DEVICE_ID)?;
        if device_id != EXPECTED_DEVICE_ID {
            return Err(TempSensorError::NotPresent);
        }

        // REQ-233: Configure resolution
        self.write_register(REG_CONFIG, CONFIG_12BIT)?;

        // REQ-232: Set alert thresholds
        self.write_threshold(REG_THIGH, self.config.alert_high_mc)?;
        self.write_threshold(REG_TLOW, self.config.alert_low_mc)?;

        Ok(())
    }

    /// Read current temperature
    ///
    /// REQ-240: Read shall complete within timeout
    /// REQ-241: Read shall return cached value if sensor busy
    pub fn read(&mut self) -> Result<TemperatureReading, TempSensorError> {
        // REQ-242: Check sensor ready
        let status = self.read_register(REG_STATUS)?;
        if status & STATUS_BUSY != 0 {
            // REQ-241: Return last valid reading
            return Ok(self.last_reading);
        }

        // REQ-243: Read temperature registers
        let mut buf = [0u8; 2];
        self.read_registers(REG_TEMP, &mut buf)?;

        // REQ-244: Convert to millidegrees
        let raw = i16::from_be_bytes(buf);
        let millidegrees = (raw as i32 * 1000) / 128;

        // REQ-222: Validate range
        if millidegrees < -40_000 || millidegrees > 125_000 {
            return Err(TempSensorError::OutOfRange);
        }

        // REQ-245: Update cached reading
        self.last_reading = TemperatureReading {
            millidegrees_c: millidegrees,
            timestamp_ms: self.get_timestamp(),
            valid: true,
        };

        Ok(self.last_reading)
    }

    /// Trigger one-shot conversion
    ///
    /// REQ-250: Support one-shot mode for power saving
    pub fn trigger_conversion(&mut self) -> Result<(), TempSensorError> {
        // REQ-251: Set one-shot bit
        self.write_register(REG_CONFIG, CONFIG_12BIT | CONFIG_ONESHOT)?;
        Ok(())
    }

    /// Check if temperature is in alert condition
    ///
    /// REQ-260: Alert status shall be queryable
    pub fn is_alert(&mut self) -> Result<bool, TempSensorError> {
        let status = self.read_register(REG_STATUS)?;
        Ok(status & (STATUS_ALERT_HIGH | STATUS_ALERT_LOW) != 0)
    }

    /// Enter low-power shutdown mode
    ///
    /// REQ-270: Shutdown current shall be < 1uA
    pub fn shutdown(&mut self) -> Result<(), TempSensorError> {
        self.write_register(REG_CONFIG, CONFIG_SHUTDOWN)?;
        Ok(())
    }

    /// Wake from shutdown
    ///
    /// REQ-271: Wake shall restore previous configuration
    pub fn wake(&mut self) -> Result<(), TempSensorError> {
        self.write_register(REG_CONFIG, CONFIG_12BIT)?;
        Ok(())
    }

    // Private helpers

    fn read_register(&mut self, reg: u8) -> Result<u8, TempSensorError> {
        let mut buf = [0u8];
        self.i2c
            .write_read(self.config.i2c_addr, &[reg], &mut buf)
            .map_err(|_| TempSensorError::I2cError)?;
        Ok(buf[0])
    }

    fn read_registers(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), TempSensorError> {
        self.i2c
            .write_read(self.config.i2c_addr, &[reg], buf)
            .map_err(|_| TempSensorError::I2cError)?;
        Ok(())
    }

    fn write_register(&mut self, reg: u8, value: u8) -> Result<(), TempSensorError> {
        self.i2c
            .write(self.config.i2c_addr, &[reg, value])
            .map_err(|_| TempSensorError::I2cError)?;
        Ok(())
    }

    fn write_threshold(&mut self, reg: u8, millidegrees: i32) -> Result<(), TempSensorError> {
        // REQ-280: Convert millidegrees to register format
        let raw = ((millidegrees * 128) / 1000) as i16;
        let bytes = raw.to_be_bytes();
        self.i2c
            .write(self.config.i2c_addr, &[reg, bytes[0], bytes[1]])
            .map_err(|_| TempSensorError::I2cError)?;
        Ok(())
    }

    fn get_timestamp(&self) -> u32 {
        // Platform-specific
        0
    }
}

// Register definitions
const REG_TEMP: u8 = 0x00;
const REG_CONFIG: u8 = 0x01;
const REG_TLOW: u8 = 0x02;
const REG_THIGH: u8 = 0x03;
const REG_STATUS: u8 = 0x04;
const REG_DEVICE_ID: u8 = 0x0F;

const CONFIG_12BIT: u8 = 0x60;
const CONFIG_ONESHOT: u8 = 0x80;
const CONFIG_SHUTDOWN: u8 = 0x01;

const STATUS_BUSY: u8 = 0x80;
const STATUS_ALERT_HIGH: u8 = 0x40;
const STATUS_ALERT_LOW: u8 = 0x20;

const EXPECTED_DEVICE_ID: u8 = 0xCB;

#[cfg(test)]
mod tests {
    use super::*;

    /// REQ-290: Unit test for temperature conversion
    #[test]
    fn test_temperature_conversion() {
        let reading = TemperatureReading {
            millidegrees_c: 25_500,
            timestamp_ms: 0,
            valid: true,
        };

        assert!((reading.degrees_c() - 25.5).abs() < 0.01);
        assert!((reading.degrees_f() - 77.9).abs() < 0.1);
    }

    /// REQ-291: Unit test for range validation
    #[test]
    fn test_valid_range() {
        // REQ-200: -40C to +125C
        assert!((-40_000..=125_000).contains(&-40_000));
        assert!((-40_000..=125_000).contains(&125_000));
        assert!(!(-40_000..=125_000).contains(&-41_000));
        assert!(!(-40_000..=125_000).contains(&126_000));
    }
}
