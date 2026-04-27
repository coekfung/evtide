#![forbid(unsafe_code)]

/// Polarity of an event.
///
/// For CD events: `On` indicates an increase in illumination (CD_ON),
/// `Off` indicates a decrease (CD_OFF).
/// For trigger events: `On` indicates a rising edge, `Off` a falling edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    Off = 0,
    On = 1,
}

impl From<bool> for Polarity {
    fn from(v: bool) -> Self {
        if v { Self::On } else { Self::Off }
    }
}

impl From<Polarity> for bool {
    fn from(p: Polarity) -> Self {
        matches!(p, Polarity::On)
    }
}

/// A contrast-detection event from an event-based vision sensor.
///
/// CD events represent a change in illumination at a specific pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventCd {
    /// Pixel X coordinate.
    pub x: u16,
    /// Pixel Y coordinate.
    pub y: u16,
    /// Event polarity.
    pub polarity: Polarity,
    /// Timestamp in microseconds.
    pub timestamp: u64,
}

/// An external trigger event from an event-based vision sensor.
///
/// Trigger events indicate a change on an external signal pin
/// (e.g. EXTTRIG, TDRSTN, PXRSTN).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventExtTrigger {
    /// Edge polarity: `On` for rising edge, `Off` for falling edge.
    pub polarity: Polarity,
    /// Trigger channel ID.
    ///
    /// `0x00`: EXTTRIG pin, `0x01`: Reset pin.
    pub id: u8,
    /// Timestamp in microseconds.
    pub timestamp: u64,
}
