//! Low-level bit extraction for EVT3 16-bit words.

use evtide_core::Polarity;

/// Event type constants from bits [15:12] of an EVT3 word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Type {
    AddrY = 0x0,
    AddrX = 0x2,
    VectBaseX = 0x3,
    Vect12 = 0x4,
    Vect8 = 0x5,
    TimeLow = 0x6,
    Continued4 = 0x7,
    TimeHigh = 0x8,
    ExtTrigger = 0xA,
    Others = 0xE,
    Continued12 = 0xF,
}

impl Type {
    #[inline]
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x0 => Some(Self::AddrY),
            0x2 => Some(Self::AddrX),
            0x3 => Some(Self::VectBaseX),
            0x4 => Some(Self::Vect12),
            0x5 => Some(Self::Vect8),
            0x6 => Some(Self::TimeLow),
            0x7 => Some(Self::Continued4),
            0x8 => Some(Self::TimeHigh),
            0xA => Some(Self::ExtTrigger),
            0xE => Some(Self::Others),
            0xF => Some(Self::Continued12),
            _ => None,
        }
    }
}

/// Extract the 4-bit event type from bits [15:12].
#[inline]
pub fn event_type(word: u16) -> Option<Type> {
    Type::from_u8(((word >> 12) & 0xF) as u8)
}

/// Y coordinate from an EVT_ADDR_Y word (bits [10:0]).
#[inline]
pub fn addr_y_y(word: u16) -> u16 {
    word & 0x07FF
}

/// X coordinate from an EVT_ADDR_X word (bits [10:0]).
#[inline]
pub fn addr_x_x(word: u16) -> u16 {
    word & 0x07FF
}

/// Polarity from bit [11].
#[inline]
pub fn addr_x_polarity(word: u16) -> Polarity {
    if (word >> 11) & 1 != 0 {
        Polarity::On
    } else {
        Polarity::Off
    }
}

/// Base X coordinate from a VECT_BASE_X word (bits [10:0]).
#[inline]
pub fn vect_base_x_x(word: u16) -> u16 {
    word & 0x07FF
}

/// Polarity from bit [11].
#[inline]
pub fn vect_base_x_polarity(word: u16) -> Polarity {
    if (word >> 11) & 1 != 0 {
        Polarity::On
    } else {
        Polarity::Off
    }
}

/// 12-bit validity mask from bits [11:0].
#[inline]
pub fn vect_12_valid(word: u16) -> u16 {
    word & 0x0FFF
}

/// 8-bit validity mask from bits [7:0].
#[inline]
pub fn vect_8_valid(word: u16) -> u8 {
    (word & 0xFF) as u8
}

/// 12-bit time value from bits [11:0]. Used for both TIME_LOW and TIME_HIGH.
#[inline]
pub fn time_value(word: u16) -> u16 {
    word & 0x0FFF
}

/// Trigger channel ID from bits [11:8].
#[inline]
pub fn ext_trigger_id(word: u16) -> u8 {
    ((word >> 8) & 0xF) as u8
}

/// Trigger edge polarity from bit [0] (false = falling, true = rising).
#[inline]
pub fn ext_trigger_value(word: u16) -> bool {
    word & 1 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_roundtrip() {
        assert_eq!(event_type(0x0000), Some(Type::AddrY));
        assert_eq!(event_type(0x2000), Some(Type::AddrX));
        assert_eq!(event_type(0x3000), Some(Type::VectBaseX));
        assert_eq!(event_type(0x4000), Some(Type::Vect12));
        assert_eq!(event_type(0x5000), Some(Type::Vect8));
        assert_eq!(event_type(0x6000), Some(Type::TimeLow));
        assert_eq!(event_type(0x7000), Some(Type::Continued4));
        assert_eq!(event_type(0x8000), Some(Type::TimeHigh));
        assert_eq!(event_type(0xA000), Some(Type::ExtTrigger));
        assert_eq!(event_type(0xE000), Some(Type::Others));
        assert_eq!(event_type(0xF000), Some(Type::Continued12));
        assert_eq!(event_type(0x1000), None);
        assert_eq!(event_type(0x9000), None);
    }

    #[test]
    fn addr_y() {
        let word = 0b0000_0_00111110100u16;
        assert_eq!(addr_y_y(word), 500);

        let word2 = 0b0000_1_00001100100u16;
        assert_eq!(addr_y_y(word2), 100);
    }

    #[test]
    fn addr_x() {
        let word = 0b0010_1_00100101100u16;
        assert_eq!(addr_x_x(word), 300);
        assert_eq!(addr_x_polarity(word), Polarity::On);
    }

    #[test]
    fn vect_base_x() {
        let word = 0b0011_1_0000001010u16;
        assert_eq!(vect_base_x_x(word), 10);
        assert_eq!(vect_base_x_polarity(word), Polarity::On);
    }

    #[test]
    fn vect_12() {
        let word = 0b0100_101010101010u16;
        assert_eq!(vect_12_valid(word), 0b101010101010);
    }

    #[test]
    fn vect_8() {
        let word = 0b0101_0000_10101010u16;
        assert_eq!(vect_8_valid(word), 0b10101010);
    }

    #[test]
    fn time_value_extraction() {
        assert_eq!(time_value(0x8ABC), 0xABC);
        assert_eq!(time_value(0x6000), 0);
        assert_eq!(time_value(0x6FFF), 0xFFF);
    }

    #[test]
    fn ext_trigger() {
        let word = 0b1010_0010_0000000_1u16;
        assert_eq!(ext_trigger_id(word), 2);
        assert!(ext_trigger_value(word));
    }
}
