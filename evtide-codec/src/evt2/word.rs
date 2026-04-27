//! Low-level bit extraction for EVT2 32-bit words.

/// Event type constants from bits [31:28] of an EVT2 word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Type {
    CdOff = 0x0,
    CdOn = 0x1,
    TimeHigh = 0x8,
    ExtTrigger = 0xA,
    Others = 0xE,
    Continued = 0xF,
}

impl Type {
    #[inline]
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x0 => Some(Self::CdOff),
            0x1 => Some(Self::CdOn),
            0x8 => Some(Self::TimeHigh),
            0xA => Some(Self::ExtTrigger),
            0xE => Some(Self::Others),
            0xF => Some(Self::Continued),
            _ => None,
        }
    }
}

/// Extract the 4-bit event type from bits [31:28].
#[inline]
pub fn event_type(word: u32) -> Option<Type> {
    Type::from_u8((word >> 28) as u8)
}

/// 6-bit timestamp LSB from a CD event (bits [27:22]).
#[inline]
pub fn cd_ts(word: u32) -> u8 {
    ((word >> 22) & 0x3F) as u8
}

/// X coordinate from a CD event (bits [21:11]).
#[inline]
pub fn cd_x(word: u32) -> u16 {
    ((word >> 11) & 0x7FF) as u16
}

/// Y coordinate from a CD event (bits [10:0]).
#[inline]
pub fn cd_y(word: u32) -> u16 {
    (word & 0x7FF) as u16
}

/// 28-bit timestamp high bits from an EVT_TIME_HIGH word (bits [27:0]).
#[inline]
pub fn time_high_ts(word: u32) -> u32 {
    word & 0x0FFF_FFFF
}

/// 6-bit timestamp LSB from an EXT_TRIGGER word (bits [27:22]).
#[inline]
pub fn ext_trigger_ts(word: u32) -> u8 {
    ((word >> 22) & 0x3F) as u8
}

/// Trigger channel ID from an EXT_TRIGGER word (bits [12:8]).
#[inline]
pub fn ext_trigger_id(word: u32) -> u8 {
    ((word >> 8) & 0x1F) as u8
}

/// Trigger edge polarity from an EXT_TRIGGER word (bit [0]).
#[inline]
pub fn ext_trigger_value(word: u32) -> bool {
    word & 1 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_types() {
        assert_eq!(event_type(0x00000000), Some(Type::CdOff));
        assert_eq!(event_type(0x10000000), Some(Type::CdOn));
        assert_eq!(event_type(0x80000000), Some(Type::TimeHigh));
        assert_eq!(event_type(0xA0000000), Some(Type::ExtTrigger));
        assert_eq!(event_type(0x90000000), None);
    }

    #[test]
    fn cd_fields() {
        // type=0, ts=10, x=300, y=200
        let word = 0b0000_001010_00100101100_00011001000u32;
        assert_eq!(cd_ts(word), 10);
        assert_eq!(cd_x(word), 300);
        assert_eq!(cd_y(word), 200);
    }

    #[test]
    fn time_high() {
        let word = 0x8ABCDEF;
        assert_eq!(time_high_ts(word), 0xABCDEF);
    }

    #[test]
    fn ext_trigger_fields() {
        let word = 0b1010_000011_000000000_00101_0000000_1u32;
        assert_eq!(ext_trigger_ts(word), 3);
        assert_eq!(ext_trigger_id(word), 5);
        assert!(ext_trigger_value(word));
    }
}
