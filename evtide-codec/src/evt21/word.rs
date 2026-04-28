//! Low-level bit extraction for EVT2.1 64-bit words.

/// Event type constants from bits [63:60] of an EVT2.1 word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Type {
    EvtNeg = 0x0,
    EvtPos = 0x1,
    TimeHigh = 0x8,
    ExtTrigger = 0xA,
    Others = 0xE,
}

impl Type {
    #[inline]
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x0 => Some(Self::EvtNeg),
            0x1 => Some(Self::EvtPos),
            0x8 => Some(Self::TimeHigh),
            0xA => Some(Self::ExtTrigger),
            0xE => Some(Self::Others),
            _ => None,
        }
    }
}

/// Extract the 4-bit event type from bits [63:60].
#[inline]
pub fn event_type(word: u64) -> Option<Type> {
    Type::from_u8((word >> 60) as u8)
}

/// 6-bit timestamp LSB from a CD event (bits [59:54]).
#[inline]
pub fn cd_ts(word: u64) -> u8 {
    ((word >> 54) & 0x3F) as u8
}

/// Base X coordinate from a CD event (bits [53:43]).
#[inline]
pub fn cd_x(word: u64) -> u16 {
    ((word >> 43) & 0x7FF) as u16
}

/// Y coordinate from a CD event (bits [42:32]).
#[inline]
pub fn cd_y(word: u64) -> u16 {
    ((word >> 32) & 0x7FF) as u16
}

/// 32-bit validity mask from a CD event (bits [31:0]).
#[inline]
pub fn cd_valid(word: u64) -> u32 {
    word as u32
}

/// 28-bit timestamp high bits from an EVT_TIME_HIGH word (bits [59:32]).
#[inline]
pub fn time_high_ts(word: u64) -> u32 {
    ((word >> 32) & 0x0FFF_FFFF) as u32
}

/// 6-bit timestamp LSB from an EXT_TRIGGER word (bits [59:54]).
#[inline]
pub fn ext_trigger_ts(word: u64) -> u8 {
    ((word >> 54) & 0x3F) as u8
}

/// Trigger channel ID from an EXT_TRIGGER word (bits [44:40]).
#[inline]
pub fn ext_trigger_id(word: u64) -> u8 {
    ((word >> 40) & 0x1F) as u8
}

/// Trigger edge polarity from an EXT_TRIGGER word (bit [32]).
#[inline]
pub fn ext_trigger_value(word: u64) -> bool {
    (word >> 32) & 1 != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_types() {
        assert_eq!(event_type(0x0000000000000000), Some(Type::EvtNeg));
        assert_eq!(event_type(0x1000000000000000), Some(Type::EvtPos));
        assert_eq!(event_type(0x8000000000000000), Some(Type::TimeHigh));
        assert_eq!(event_type(0xA000000000000000), Some(Type::ExtTrigger));
        assert_eq!(event_type(0x9000000000000000), None);
    }

    #[test]
    fn cd_fields() {
        // type=0, ts=10, x=32, y=200, valid=0x80000001 (bits 0 and 31)
        let word = 0b0000_001010_00000100000_00011001000_10000000000000000000000000000001u64;
        assert_eq!(cd_ts(word), 10);
        assert_eq!(cd_x(word), 32);
        assert_eq!(cd_y(word), 200);
        assert_eq!(cd_valid(word), 0x8000_0001);
    }

    #[test]
    fn time_high() {
        let word = (0x8u64 << 60) | (0xABCDEFu64 << 32);
        assert_eq!(time_high_ts(word), 0xABCDEF);
    }

    #[test]
    fn ext_trigger_fields() {
        let word = 0b1010_000011_000000000_00101_0000000_1_00000000000000000000000000000000u64;
        assert_eq!(ext_trigger_ts(word), 3);
        assert_eq!(ext_trigger_id(word), 5);
        assert!(ext_trigger_value(word));
    }
}
