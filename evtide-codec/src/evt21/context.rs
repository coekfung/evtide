use evtide_core::{EventCd, EventExtTrigger, Polarity};

use super::word;

use crate::CodecEvent;

/// Decoding state for the EVT2.1 format.
///
/// EVT2.1 is a 64-bit vectorized format. Each CD event carries a base X
/// coordinate (aligned to 32), Y, 6-bit timestamp LSB, and a 32-bit validity
/// mask. The only accumulated state is the most recent TIME_HIGH value
/// (28 bits).
///
/// Before the first `EVT_TIME_HIGH` word is received no events can be emitted
/// because the full timestamp is unknown.
#[derive(Debug, Clone)]
pub struct Evt21Context {
    time_high: Option<u32>,
}

impl Default for Evt21Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Evt21Context {
    /// Default initial state.
    pub fn new() -> Self {
        Self { time_high: None }
    }

    /// Resets all decoding state to defaults.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Processes a single 64-bit EVT2.1 word.
    ///
    /// Decoded events are passed to `on_event`. No allocation is performed.
    pub fn process_word(&mut self, word: u64, on_event: &mut impl FnMut(CodecEvent)) {
        let Some(ty) = word::event_type(word) else {
            return;
        };

        if self.time_high.is_none() && ty != word::Type::TimeHigh {
            return;
        }

        match ty {
            word::Type::EvtNeg => self.emit_cd(word, Polarity::Off, on_event),
            word::Type::EvtPos => self.emit_cd(word, Polarity::On, on_event),
            word::Type::TimeHigh => {
                self.time_high = Some(word::time_high_ts(word));
            }
            word::Type::ExtTrigger => self.emit_trigger(word, on_event),
            word::Type::Others => {}
        }
    }

    fn emit_cd(&self, word: u64, pol: Polarity, on_event: &mut impl FnMut(CodecEvent)) {
        let Some(ts) = self.timestamp_us(word::cd_ts(word)) else {
            return;
        };
        let base_x = word::cd_x(word);
        let y = word::cd_y(word);
        let mut valid = word::cd_valid(word);

        let mut x = base_x;
        while valid != 0 {
            if valid & 1 != 0 {
                on_event(CodecEvent::Cd(EventCd {
                    x,
                    y,
                    polarity: pol,
                    timestamp: ts,
                }));
            }
            valid >>= 1;
            x += 1;
        }
    }

    fn emit_trigger(&self, word: u64, on_event: &mut impl FnMut(CodecEvent)) {
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

    fn decode(words: &[u64]) -> Vec<CodecEvent> {
        let mut ctx = Evt21Context::new();
        let mut events = Vec::new();
        for &w in words {
            ctx.process_word(w, &mut |e| events.push(e));
        }
        events
    }

    #[test]
    fn drops_events_before_first_time_high() {
        let events = decode(&[
            0x0000_0000_0064_0000, // EVT_NEG
            0x1000_0000_0064_0000, // EVT_POS
        ]);
        assert!(events.is_empty());
    }

    #[test]
    fn decodes_evt_neg() {
        let word = (0u64 << 60) | (12u64 << 54) | (40u64 << 43) | (100u64 << 32) | 5;
        let events = decode(&[
            0x8000_0000_0000_0000, // TIME_HIGH = 0
            word,                  // EVT_NEG ts=12, x=40, y=100, valid bits 0,2
        ]);
        assert_eq!(events.len(), 2);
        let CodecEvent::Cd(e0) = &events[0] else {
            panic!("expected Cd");
        };
        assert_eq!(e0.polarity, Polarity::Off);
        assert_eq!(e0.x, 40);
        assert_eq!(e0.y, 100);
        assert_eq!(e0.timestamp, 12);

        let CodecEvent::Cd(e1) = &events[1] else {
            panic!("expected Cd");
        };
        assert_eq!(e1.x, 42);
        assert_eq!(e1.y, 100);
        assert_eq!(e1.timestamp, 12);
    }

    #[test]
    fn decodes_evt_pos() {
        let word = (1u64 << 60) | (5u64 << 54) | (100u64 << 43) | (10u64 << 32) | 3;
        let events = decode(&[
            0x8000_0000_0000_0000, // TIME_HIGH = 0
            word,                  // EVT_POS ts=5, x=100, y=10, valid bits 0,1
        ]);
        assert_eq!(events.len(), 2);
        for event in &events {
            let CodecEvent::Cd(e) = event else {
                panic!("expected Cd");
            };
            assert_eq!(e.polarity, Polarity::On);
            assert_eq!(e.y, 10);
            assert_eq!(e.timestamp, 5);
        }
    }

    #[test]
    fn decodes_trigger() {
        let word = (0xAu64 << 60) | (15u64 << 54) | (2u64 << 40) | (1u64 << 32);
        let events = decode(&[
            0x8000_0000_0000_0000, // TIME_HIGH
            word,                  // EXT_TRIGGER ts=15, id=2, val=1
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
    fn state_not_set_before_first_time_high() {
        let events = decode(&[
            0x0000_0000_0032_0000, // EVT_NEG y=50 — ignored
            0x8000_0000_0000_0000, // TIME_HIGH
            0x0000_0000_0000_0000, // EVT_NEG all zeros
        ]);
        assert_eq!(events.len(), 1);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd");
        };
        assert_eq!(e.y, 0);
    }

    #[test]
    fn emits_up_to_32_events() {
        let word = (1u64 << 60) | (100u64 << 32) | 0xFFFF_FFFF;
        let events = decode(&[
            0x8000_0000_0000_0000, // TIME_HIGH
            word,                  // EVT_POS x=0, y=100, all 32 bits valid
        ]);
        assert_eq!(events.len(), 32);
        for (i, event) in events.iter().enumerate() {
            let CodecEvent::Cd(e) = event else {
                panic!("expected Cd");
            };
            assert_eq!(e.x, i as u16);
            assert_eq!(e.y, 100);
        }
    }
}
