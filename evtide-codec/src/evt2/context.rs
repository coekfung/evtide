use evtide_core::{EventCd, EventExtTrigger, Polarity};

use super::word;

use crate::CodecEvent;

/// Decoding state for the EVT2 format.
///
/// EVT2 is a 32-bit self-contained format (no vectorization). Each CD event
/// carries its own X, Y, and 6-bit timestamp LSB. The only accumulated state
/// is the most recent TIME_HIGH value (28 bits).
///
/// Before the first `EVT_TIME_HIGH` word is received no events can be emitted
/// because the full timestamp is unknown.
#[derive(Debug, Clone)]
pub struct Evt2Context {
    /// Most recent TIME_HIGH value (28 bits). `None` before the first one.
    time_high: Option<u32>,
}

impl Default for Evt2Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Evt2Context {
    /// Default initial state.
    pub fn new() -> Self {
        Self { time_high: None }
    }

    /// Resets all decoding state to defaults.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Processes a single 32-bit EVT2 word.
    ///
    /// Decoded events are passed to `on_event`. No allocation is performed.
    pub fn process_word(&mut self, word: u32, on_event: &mut impl FnMut(CodecEvent)) {
        let Some(ty) = word::event_type(word) else {
            return; // reserved — skip
        };

        if self.time_high.is_none() && ty != word::Type::TimeHigh {
            return;
        }

        match ty {
            word::Type::CdOff => self.emit_cd(word, Polarity::Off, on_event),
            word::Type::CdOn => self.emit_cd(word, Polarity::On, on_event),
            word::Type::TimeHigh => {
                self.time_high = Some(word::time_high_ts(word));
            }
            word::Type::ExtTrigger => self.emit_trigger(word, on_event),
            word::Type::Others | word::Type::Continued => {}
        }
    }

    fn emit_cd(&self, word: u32, pol: Polarity, on_event: &mut impl FnMut(CodecEvent)) {
        if let Some(ts) = self.timestamp_us(word::cd_ts(word)) {
            on_event(CodecEvent::Cd(EventCd {
                x: word::cd_x(word),
                y: word::cd_y(word),
                polarity: pol,
                timestamp: ts,
            }));
        }
    }

    fn emit_trigger(&self, word: u32, on_event: &mut impl FnMut(CodecEvent)) {
        if let Some(ts) = self.timestamp_us(word::ext_trigger_ts(word)) {
            on_event(CodecEvent::Trigger(EventExtTrigger {
                polarity: if word::ext_trigger_value(word) {
                    Polarity::On
                } else {
                    Polarity::Off
                },
                id: word::ext_trigger_id(word),
                timestamp: ts,
            }));
        }
    }

    /// Full timestamp in microseconds, or `None` before the first TIME_HIGH.
    pub fn timestamp_us(&self, ts_lsb: u8) -> Option<u64> {
        self.time_high
            .map(|th| ((th as u64) << 6) | (ts_lsb as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(words: &[u32]) -> Vec<CodecEvent> {
        let mut ctx = Evt2Context::new();
        let mut events = Vec::new();
        for &w in words {
            ctx.process_word(w, &mut |e| events.push(e));
        }
        events
    }

    #[test]
    fn drops_events_before_first_time_high() {
        let events = decode(&[
            0x0000_0064, // CD_OFF ts=0, x=0, y=100
            0x1000_0064, // CD_ON ts=0, x=0, y=100
        ]);
        assert!(events.is_empty());
    }

    #[test]
    fn decodes_cd_off() {
        let events = decode(&[
            0x8000_0000,                              // TIME_HIGH = 0
            0b0000_001100_00000001010_00001100100u32, // CD_OFF ts=12, x=10, y=100
        ]);
        assert_eq!(events.len(), 1);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd");
        };
        assert_eq!(e.polarity, Polarity::Off);
        assert_eq!(e.x, 10);
        assert_eq!(e.y, 100);
        assert_eq!(e.timestamp, 12);
    }

    #[test]
    fn decodes_cd_on() {
        let events = decode(&[
            0x8000_0000,                              // TIME_HIGH = 0
            0b0001_000101_00001100100_00000001010u32, // CD_ON ts=5, x=100, y=10
        ]);
        assert_eq!(events.len(), 1);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd");
        };
        assert_eq!(e.polarity, Polarity::On);
        assert_eq!(e.x, 100);
        assert_eq!(e.y, 10);
        assert_eq!(e.timestamp, 5);
    }

    #[test]
    fn decodes_trigger() {
        let events = decode(&[
            0x8000_0000,                                // TIME_HIGH = 0
            0b1010_001111_000000000_00010_0000000_1u32, // EXT_TRIGGER ts=15, id=2, val=1
        ]);
        assert_eq!(events.len(), 1);
        let CodecEvent::Trigger(t) = &events[0] else {
            panic!("expected Trigger");
        };
        assert_eq!(t.polarity, Polarity::On);
        assert_eq!(t.id, 2);
        assert_eq!(t.timestamp, 15);
    }

    #[test]
    fn timestamp_combines_time_high_and_lsb() {
        let events = decode(&[
            0x8ABCDEF,                                // TIME_HIGH = 0xABCDEF (28 bits)
            0b0000_000011_00000000000_00000000000u32, // CD_OFF ts=3
        ]);
        assert_eq!(events.len(), 1);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd");
        };
        // timestamp = (0xABCDEF << 6) | 3 = 0x2AF37BC << 2?
        // Actually: 0xABCDEF * 64 + 3
        assert_eq!(e.timestamp, (0xABCDEFu64 << 6) | 3);
    }

    #[test]
    fn state_not_set_before_first_time_high() {
        // CD events before TIME_HIGH must not be emitted (state not set).
        let events = decode(&[
            0x0000_0032,                              // CD_OFF y=50 — should be ignored
            0x8000_0000,                              // TIME_HIGH = 0
            0b0000_000000_00000000000_00000000000u32, // CD_OFF all zeros
        ]);
        assert_eq!(events.len(), 1);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd");
        };
        assert_eq!(e.y, 0); // default, not 50 from the suppressed event
    }
}
