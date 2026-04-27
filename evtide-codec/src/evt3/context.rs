use evtide_core::{EventCd, EventExtTrigger, Polarity};

use super::word;

use crate::CodecEvent;

/// Duration of one TIME_HIGH period in microseconds (4096 us).
const TIME_HIGH_PERIOD_US: u64 = 4096;

/// Full wraparound period: TIME_HIGH values 0..4095, each 4096 us.
const WRAP_PERIOD_US: u64 = 4096 * TIME_HIGH_PERIOD_US;

/// Max backward TIME_HIGH jump (in us) that is NOT a wraparound.
const WRAP_THRESHOLD_US: u64 = WRAP_PERIOD_US - 10 * TIME_HIGH_PERIOD_US;

/// Decoding state for the EVT3 format.
///
/// `Evt3Context` is a pure sans-IO state machine. It processes individual
/// 16-bit EVT3 words and emits decoded events through a caller-provided
/// callback. It does not allocate, does not perform I/O, and can be tested
/// with hand-crafted word sequences.
///
/// # State
///
/// EVT3 encodes data relative to a base state — coordinates, polarity, and
/// timestamp are only transmitted when they change. The context tracks these
/// values across words.
///
/// Before the first `EVT_TIME_HIGH` word is received no events can be emitted
/// because the timestamp is unknown. Words received in this phase are silently
/// dropped.
#[derive(Debug, Clone)]
pub struct Evt3Context {
    /// Most recent TIME_HIGH value. `None` before the first one arrives.
    time_high: Option<u16>,

    /// Most recent TIME_LOW value.
    time_low: u16,

    /// Number of 4096*4096 us wraparound periods that have elapsed.
    wraps: u64,

    /// Current Y coordinate (set by EVT_ADDR_Y).
    y: u16,

    /// Base X coordinate for vector events (set by VECT_BASE_X, advanced by
    /// VECT_12/VECT_8).
    base_x: u16,

    /// Current polarity for vector events (set by VECT_BASE_X).
    polarity: Polarity,
}

impl Default for Evt3Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Evt3Context {
    /// Default initial state.
    pub fn new() -> Self {
        Self {
            time_high: None,
            time_low: 0,
            wraps: 0,
            y: 0,
            base_x: 0,
            polarity: Polarity::Off,
        }
    }

    /// Resets all decoding state to defaults.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Processes a single 16-bit EVT3 word.
    ///
    /// Decoded events are passed to `on_event`. No allocation is performed.
    pub fn process_word(&mut self, word: u16, on_event: &mut impl FnMut(CodecEvent)) {
        let Some(ty) = word::event_type(word) else {
            return; // reserved / unknown — skip
        };

        // Before the first TIME_HIGH the time base is unknown.
        // Skip all words except TIME_HIGH — they cannot produce meaningful
        // events and setting state from them would be misleading.
        if self.time_high.is_none() && ty != word::Type::TimeHigh {
            return;
        }

        match ty {
            word::Type::TimeHigh => self.on_time_high(word),
            word::Type::TimeLow => self.time_low = word::time_value(word),
            word::Type::AddrY => self.y = word::addr_y_y(word),
            word::Type::VectBaseX => {
                self.base_x = word::vect_base_x_x(word);
                self.polarity = word::vect_base_x_polarity(word);
            }
            word::Type::Vect12 => self.emit_vector(word::vect_12_valid(word) as u32, 12, on_event),
            word::Type::Vect8 => self.emit_vector(word::vect_8_valid(word) as u32, 8, on_event),
            word::Type::AddrX => {
                if let Some(ts) = self.timestamp_us() {
                    let x = word::addr_x_x(word);
                    let pol = word::addr_x_polarity(word);
                    on_event(CodecEvent::Cd(EventCd {
                        x,
                        y: self.y,
                        polarity: pol,
                        timestamp: ts,
                    }));
                }
            }
            word::Type::ExtTrigger => {
                if let Some(ts) = self.timestamp_us() {
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
            // These event types carry extension data not used for CD/trigger
            // event reconstruction. Silently consume them.
            word::Type::Continued4 | word::Type::Others | word::Type::Continued12 => {}
        }
    }

    /// Full timestamp in microseconds, or `None` before the first TIME_HIGH.
    pub fn timestamp_us(&self) -> Option<u64> {
        self.time_high
            .map(|th| ((th as u64) << 12) + (self.time_low as u64) + self.wraps * WRAP_PERIOD_US)
    }

    #[inline]
    fn on_time_high(&mut self, word: u16) {
        let new_th = word::time_value(word);

        let Some(old_th) = self.time_high else {
            self.time_high = Some(new_th);
            return;
        };

        let new_base = (new_th as u64) << 12;
        let old_base = (old_th as u64) << 12;

        if new_base < old_base && (old_base - new_base) > WRAP_THRESHOLD_US {
            self.wraps += 1;
        }

        self.time_high = Some(new_th);
    }

    #[inline]
    fn emit_vector(&mut self, valid: u32, count: u16, on_event: &mut impl FnMut(CodecEvent)) {
        let Some(ts) = self.timestamp_us() else {
            // No TIME_HIGH yet, can't emit events.
            // Still advance base_x per spec: "After processing this event, the
            // X position value on the receiver side should be incremented."
            self.base_x += count;
            return;
        };

        let start = self.base_x;

        for i in 0..count {
            if valid & (1 << i) != 0 {
                on_event(CodecEvent::Cd(EventCd {
                    x: start + i,
                    y: self.y,
                    polarity: self.polarity,
                    timestamp: ts,
                }));
            }
        }

        self.base_x = start + count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(words: &[u16]) -> Vec<CodecEvent> {
        let mut ctx = Evt3Context::new();
        let mut events = Vec::new();
        for &w in words {
            ctx.process_word(w, &mut |e| events.push(e));
        }
        events
    }

    #[test]
    fn state_not_set_before_first_time_high() {
        // ADDR_Y and VECT_BASE_X before TIME_HIGH must not affect state.
        // The emitted event should use default y=0 and polarity=Off.
        let events = decode(&[
            0x0064, // ADDR_Y y=100 — should be ignored (no TIME_HIGH yet)
            0x3800, // VECT_BASE_X x=0, pol=On — should be ignored
            0x8000, // TIME_HIGH = 0
            0x600A, // TIME_LOW = 10
            0x280A, // ADDR_X x=10, pol=On
        ]);
        assert_eq!(events.len(), 1);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd");
        };
        // y should be default 0, not 100 from the suppressed ADDR_Y
        assert_eq!(e.y, 0);
        assert_eq!(e.x, 10);
    }

    #[test]
    fn drops_events_before_first_time_high() {
        let events = decode(&[
            0x0032, // ADDR_Y y=50
            0x2864, // ADDR_X x=100, pol=1
            0xA801, // EXT_TRIGGER val=1, id=0
        ]);
        assert!(events.is_empty());
    }

    #[test]
    fn decodes_simple_cd_event() {
        let events = decode(&[
            0x8000, // TIME_HIGH = 0
            0x6064, // TIME_LOW = 100
            0x0032, // ADDR_Y y=50
            0x2864, // ADDR_X x=100, pol=1
        ]);
        assert_eq!(events.len(), 1);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd event");
        };
        assert_eq!(e.x, 100);
        assert_eq!(e.y, 50);
        assert_eq!(e.polarity, Polarity::On);
        assert_eq!(e.timestamp, 100);
    }

    #[test]
    fn decodes_vector_events() {
        let events = decode(&[
            0x8000, // TIME_HIGH = 0
            0x60C8, // TIME_LOW = 200
            0x0064, // ADDR_Y y=100
            0x3000, // VECT_BASE_X x=0, pol=Off
            0x4E38, // VECT_12 valid=0b111000111000 → x=3,4,5,9,10,11
            0x5004, // VECT_8 valid=0b00000100 → x=12+2=14 (base was 12 after VECT_12)
        ]);
        assert_eq!(events.len(), 7);

        let xs: Vec<u16> = events
            .iter()
            .filter_map(|e| {
                if let CodecEvent::Cd(cd) = e {
                    Some(cd.x)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(xs, vec![3, 4, 5, 9, 10, 11, 14]);

        // All should share y, polarity, timestamp
        for event in &events {
            if let CodecEvent::Cd(cd) = event {
                assert_eq!(cd.y, 100);
                assert_eq!(cd.polarity, Polarity::Off);
                assert_eq!(cd.timestamp, 200);
            }
        }
    }

    #[test]
    fn decodes_trigger_event() {
        let events = decode(&[
            0x8000, // TIME_HIGH = 0
            0x600A, // TIME_LOW = 10
            0xA801, // EXT_TRIGGER val=1, id=0
        ]);
        assert_eq!(events.len(), 1);
        let CodecEvent::Trigger(t) = &events[0] else {
            panic!("expected Trigger event");
        };
        assert_eq!(t.polarity, Polarity::On);
        assert_eq!(t.id, 0);
        assert_eq!(t.timestamp, 10);
    }

    #[test]
    fn time_high_wraparound() {
        // Simulate a wraparound: TIME_HIGH near max then wraps to 0.
        let mut ctx = Evt3Context::new();
        let mut events = Vec::new();
        let push = &mut |e| events.push(e);

        ctx.process_word(0x8FFF, push); // TIME_HIGH = 4095, base = 4095 << 12 = 16773120
        ctx.process_word(0x6FFF, push); // TIME_LOW = 4095
        // timestamp = 16773120 + 4095 = 16777215
        assert_eq!(ctx.timestamp_us(), Some(16777215));

        // Now TIME_HIGH wraps to 0
        ctx.process_word(0x8000, push); // TIME_HIGH = 0
        ctx.process_word(0x600A, push); // TIME_LOW = 10
        // Wraparound detected: 0 + WRAP_PERIOD_US.
        // WRAP_PERIOD_US = 4096 * 4096 = 16777216
        // timestamp = 16777216 + 10 = 16777226
        assert_eq!(ctx.timestamp_us(), Some(16777226));
    }

    #[test]
    fn addr_x_uses_vect_base_x_polarity_not_word_polarity() {
        // ADDR_X has its own polarity bit; it should NOT use the
        // VECT_BASE_X polarity.
        let events = decode(&[
            0x8000, // TIME_HIGH = 0
            0x600A, // TIME_LOW = 10
            0x0032, // ADDR_Y y=50
            0x3800, // VECT_BASE_X x=0, pol=On
            0x200A, // ADDR_X x=10, pol=Off
        ]);
        assert_eq!(events.len(), 1);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd");
        };
        // ADDR_X polarity comes from the ADDR_X word, not VECT_BASE_X
        assert_eq!(e.polarity, Polarity::Off);
        assert_eq!(e.x, 10);
        assert_eq!(e.y, 50);
    }

    #[test]
    fn vect_base_x_polarity_used_for_vectors() {
        let events = decode(&[
            0x8000, // TIME_HIGH = 0
            0x600A, // TIME_LOW = 10
            0x0064, // ADDR_Y y=100
            0x3800, // VECT_BASE_X x=0, pol=On
            0x4003, // VECT_12 valid=0b11 → x=0,1 both On
        ]);
        assert_eq!(events.len(), 2);
        for event in &events {
            if let CodecEvent::Cd(cd) = event {
                assert_eq!(cd.polarity, Polarity::On);
            }
        }
    }
}
