//! ARINC 429 Avionics Bus Driver
//!
//! Implementation of ARINC 429 digital information transfer system
//! for aerospace applications. This protocol is the primary data bus
//! standard for commercial and transport aircraft.
//!
//! # Standards Compliance
//! - ARINC Specification 429P1-18 (Mark 33 DITS)
//!
//! # Requirements
//! - HLR-A429-001: Driver shall support both high-speed (100 kbps) and low-speed (12.5 kbps)
//! - HLR-A429-002: Driver shall support transmit and receive operations
//! - HLR-A429-003: Driver shall validate label and SDI fields

#![no_std]

use core::convert::TryFrom;

/// ARINC 429 bus speed configuration
///
/// HLR-A429-001: Speed selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusSpeed {
    /// 12.5 kbps - low speed
    Low = 12500,
    /// 100 kbps - high speed
    High = 100000,
}

/// Source/Destination Identifier
///
/// LLR-A429-010: SDI field encoding (bits 9-10)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Sdi {
    /// All systems
    All = 0b00,
    /// System 1
    System1 = 0b01,
    /// System 2
    System2 = 0b10,
    /// System 3
    System3 = 0b11,
}

impl TryFrom<u8> for Sdi {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b00 => Ok(Sdi::All),
            0b01 => Ok(Sdi::System1),
            0b10 => Ok(Sdi::System2),
            0b11 => Ok(Sdi::System3),
            _ => Err(()),
        }
    }
}

/// Sign/Status Matrix
///
/// LLR-A429-011: SSM field encoding (bits 30-31)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Ssm {
    /// Normal operation
    Normal = 0b00,
    /// No computed data
    NoComputedData = 0b01,
    /// Functional test
    FunctionalTest = 0b10,
    /// Failure warning
    FailureWarning = 0b11,
}

impl TryFrom<u8> for Ssm {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b00 => Ok(Ssm::Normal),
            0b01 => Ok(Ssm::NoComputedData),
            0b10 => Ok(Ssm::FunctionalTest),
            0b11 => Ok(Ssm::FailureWarning),
            _ => Err(()),
        }
    }
}

/// ARINC 429 Word structure
///
/// HLR-A429-010: 32-bit word format
///
/// Bit layout:
/// - Bits 1-8: Label (octal encoded, transmitted LSB first)
/// - Bits 9-10: SDI (Source/Destination Identifier)
/// - Bits 11-29: Data field
/// - Bits 30-31: SSM (Sign/Status Matrix)
/// - Bit 32: Parity (odd)
#[derive(Debug, Clone, Copy)]
pub struct Word {
    raw: u32,
}

impl Word {
    /// Create a new ARINC 429 word
    ///
    /// LLR-A429-020: Word construction with parity calculation
    pub fn new(label: u8, sdi: Sdi, data: u32, ssm: Ssm) -> Self {
        let mut raw = 0u32;

        // LLR-A429-021: Set label (bits 1-8, reversed)
        raw |= Self::reverse_label(label) as u32;

        // LLR-A429-022: Set SDI (bits 9-10)
        raw |= ((sdi as u32) & 0x03) << 8;

        // LLR-A429-023: Set data (bits 11-29)
        raw |= (data & 0x7FFFF) << 10;

        // LLR-A429-024: Set SSM (bits 30-31)
        raw |= ((ssm as u32) & 0x03) << 29;

        // LLR-A429-025: Calculate and set parity (bit 32)
        let parity = Self::calc_odd_parity(raw);
        raw |= parity << 31;

        Self { raw }
    }

    /// Create word from raw 32-bit value
    ///
    /// SAF-A429-001: Validate parity on incoming words
    pub fn from_raw(raw: u32) -> Result<Self, Arinc429Error> {
        let word = Self { raw };

        if !word.verify_parity() {
            return Err(Arinc429Error::ParityError);
        }

        Ok(word)
    }

    /// Get the label (reversed to standard octal form)
    ///
    /// HLR-A429-003: Label extraction
    pub fn label(&self) -> u8 {
        Self::reverse_label((self.raw & 0xFF) as u8)
    }

    /// Get the SDI field
    pub fn sdi(&self) -> Sdi {
        Sdi::try_from(((self.raw >> 8) & 0x03) as u8).unwrap()
    }

    /// Get the data field (19 bits)
    pub fn data(&self) -> u32 {
        (self.raw >> 10) & 0x7FFFF
    }

    /// Get the SSM field
    pub fn ssm(&self) -> Ssm {
        Ssm::try_from(((self.raw >> 29) & 0x03) as u8).unwrap()
    }

    /// Get raw 32-bit word
    pub fn raw(&self) -> u32 {
        self.raw
    }

    /// Verify parity
    ///
    /// SAF-A429-001: Parity verification
    fn verify_parity(&self) -> bool {
        self.raw.count_ones() % 2 == 1
    }

    /// Calculate odd parity
    fn calc_odd_parity(value: u32) -> u32 {
        if value.count_ones() % 2 == 0 { 1 } else { 0 }
    }

    /// Reverse label bits (ARINC 429 transmits LSB first)
    fn reverse_label(label: u8) -> u8 {
        label.reverse_bits()
    }
}

/// BNR (Binary Number Representation) data encoding
///
/// LLR-A429-030: BNR format support
pub struct BnrFormat {
    /// Most significant bit position (11-29)
    pub msb: u8,
    /// Resolution (LSB value)
    pub resolution: f32,
    /// Signed or unsigned
    pub signed: bool,
}

impl BnrFormat {
    /// Encode a floating-point value to BNR
    ///
    /// LLR-A429-031: BNR encoding
    pub fn encode(&self, value: f32) -> u32 {
        let bits = self.msb - 10; // Data field starts at bit 11
        let max_val = (1u32 << bits) - 1;

        let scaled = (value / self.resolution) as i32;

        if self.signed {
            // LLR-A429-032: Two's complement for signed
            (scaled as u32) & max_val
        } else {
            (scaled as u32).min(max_val)
        }
    }

    /// Decode BNR to floating-point value
    ///
    /// LLR-A429-033: BNR decoding
    pub fn decode(&self, data: u32) -> f32 {
        let bits = self.msb - 10;

        if self.signed {
            // LLR-A429-034: Sign extension for signed values
            let sign_bit = 1u32 << (bits - 1);
            let value = if data & sign_bit != 0 {
                (data | !((1u32 << bits) - 1)) as i32
            } else {
                data as i32
            };
            value as f32 * self.resolution
        } else {
            data as f32 * self.resolution
        }
    }
}

/// BCD (Binary Coded Decimal) data encoding
///
/// LLR-A429-040: BCD format support
pub struct BcdFormat {
    /// Number of digits
    pub digits: u8,
}

impl BcdFormat {
    /// Encode integer to BCD
    ///
    /// LLR-A429-041: BCD encoding
    pub fn encode(&self, value: u32) -> u32 {
        let mut result = 0u32;
        let mut remaining = value;

        for i in 0..self.digits {
            let digit = remaining % 10;
            result |= digit << (i * 4);
            remaining /= 10;
        }

        result
    }

    /// Decode BCD to integer
    ///
    /// LLR-A429-042: BCD decoding
    pub fn decode(&self, data: u32) -> u32 {
        let mut result = 0u32;
        let mut multiplier = 1u32;

        for i in 0..self.digits {
            let digit = (data >> (i * 4)) & 0x0F;
            result += digit * multiplier;
            multiplier *= 10;
        }

        result
    }
}

/// ARINC 429 driver errors
///
/// SAF-A429-010: Error enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arinc429Error {
    /// Parity error on received word
    ParityError,
    /// FIFO overflow
    Overflow,
    /// Gap timing violation
    GapError,
    /// Hardware not ready
    NotReady,
    /// Invalid configuration
    InvalidConfig,
}

/// ARINC 429 transmitter configuration
///
/// LLR-A429-050: TX configuration
pub struct TxConfig {
    /// Bus speed
    pub speed: BusSpeed,
    /// Minimum gap between words (bit times, typically 4)
    /// LLR-A429-051: Inter-word gap timing
    pub gap_bits: u8,
}

/// ARINC 429 receiver configuration
///
/// LLR-A429-060: RX configuration
pub struct RxConfig {
    /// Bus speed
    pub speed: BusSpeed,
    /// Label filter (bitmask, None = accept all)
    /// LLR-A429-061: Hardware label filtering
    pub label_filter: Option<[u8; 32]>,
    /// SDI filter (None = accept all)
    /// LLR-A429-062: SDI filtering
    pub sdi_filter: Option<Sdi>,
}

/// ARINC 429 transmitter
///
/// HLR-A429-020: Transmit capability
pub struct Tx<HW> {
    hw: HW,
    config: TxConfig,
}

impl<HW: TxHardware> Tx<HW> {
    /// Create new transmitter
    pub fn new(hw: HW, config: TxConfig) -> Result<Self, Arinc429Error> {
        let mut tx = Self { hw, config };
        tx.hw.configure(&tx.config)?;
        Ok(tx)
    }

    /// Transmit a word
    ///
    /// HLR-A429-021: Word transmission
    /// SAF-A429-020: Transmit shall verify word format
    pub fn send(&mut self, word: Word) -> Result<(), Arinc429Error> {
        // LLR-A429-070: Check TX FIFO not full
        if !self.hw.tx_ready() {
            return Err(Arinc429Error::NotReady);
        }

        // LLR-A429-071: Write word to hardware FIFO
        self.hw.write_word(word.raw());

        Ok(())
    }

    /// Check if transmitter is ready
    pub fn is_ready(&self) -> bool {
        self.hw.tx_ready()
    }
}

/// ARINC 429 receiver
///
/// HLR-A429-030: Receive capability
pub struct Rx<HW> {
    hw: HW,
    config: RxConfig,
}

impl<HW: RxHardware> Rx<HW> {
    /// Create new receiver
    pub fn new(hw: HW, config: RxConfig) -> Result<Self, Arinc429Error> {
        let mut rx = Self { hw, config };
        rx.hw.configure(&rx.config)?;
        Ok(rx)
    }

    /// Receive a word (non-blocking)
    ///
    /// HLR-A429-031: Word reception
    /// SAF-A429-001: Verify parity on receive
    pub fn receive(&mut self) -> Result<Option<Word>, Arinc429Error> {
        // LLR-A429-080: Check RX FIFO not empty
        if !self.hw.rx_available() {
            return Ok(None);
        }

        // LLR-A429-081: Read word from hardware FIFO
        let raw = self.hw.read_word();

        // SAF-A429-001: Validate parity
        let word = Word::from_raw(raw)?;

        Ok(Some(word))
    }

    /// Check if data is available
    pub fn is_available(&self) -> bool {
        self.hw.rx_available()
    }

    /// Get FIFO fill level
    ///
    /// LLR-A429-082: FIFO monitoring for overflow prevention
    pub fn fifo_count(&self) -> usize {
        self.hw.rx_fifo_count()
    }
}

/// Hardware abstraction trait for TX
///
/// LLR-A429-090: Hardware interface
pub trait TxHardware {
    fn configure(&mut self, config: &TxConfig) -> Result<(), Arinc429Error>;
    fn tx_ready(&self) -> bool;
    fn write_word(&mut self, word: u32);
}

/// Hardware abstraction trait for RX
///
/// LLR-A429-091: Hardware interface
pub trait RxHardware {
    fn configure(&mut self, config: &RxConfig) -> Result<(), Arinc429Error>;
    fn rx_available(&self) -> bool;
    fn read_word(&mut self) -> u32;
    fn rx_fifo_count(&self) -> usize;
}

/// Common ARINC 429 labels (subset)
///
/// Reference: ARINC 429 Attachment 6
pub mod labels {
    /// HLR-A429-100: Standard label definitions

    /// Latitude (BNR)
    pub const LAT: u8 = 0o310;
    /// Longitude (BNR)
    pub const LON: u8 = 0o311;
    /// Ground speed (BNR)
    pub const GSPD: u8 = 0o312;
    /// True track angle (BNR)
    pub const TTRK: u8 = 0o313;
    /// Altitude (BNR)
    pub const ALT: u8 = 0o203;
    /// Baro corrected altitude (BNR)
    pub const BALT: u8 = 0o204;
    /// Vertical speed (BNR)
    pub const IVSI: u8 = 0o212;
    /// True airspeed (BNR)
    pub const TAS: u8 = 0o210;
    /// Mach number (BNR)
    pub const MACH: u8 = 0o205;
    /// Total air temperature (BNR)
    pub const TAT: u8 = 0o213;
    /// UTC time (BCD)
    pub const UTC: u8 = 0o150;
    /// Date (BCD)
    pub const DATE: u8 = 0o260;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SAF-A429-100: Unit test for parity calculation
    #[test]
    fn test_parity() {
        let word = Word::new(0o310, Sdi::All, 0x1234, Ssm::Normal);
        assert!(word.verify_parity(), "Parity should be valid");
    }

    /// LLR-A429-100: Unit test for label encoding
    #[test]
    fn test_label_encoding() {
        let word = Word::new(0o310, Sdi::All, 0, Ssm::Normal);
        assert_eq!(word.label(), 0o310, "Label should round-trip correctly");
    }

    /// LLR-A429-101: Unit test for BNR encoding
    #[test]
    fn test_bnr_encoding() {
        let format = BnrFormat {
            msb: 29,
            resolution: 0.01,
            signed: true,
        };

        let encoded = format.encode(123.45);
        let decoded = format.decode(encoded);

        assert!((decoded - 123.45).abs() < 0.02, "BNR round-trip error too large");
    }
}
