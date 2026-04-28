pub mod context;
pub(crate) mod word;

use crate::assemble::assemble_words;
use crate::{ByteDecoder, CodecEvent, TrailingByteError};

use context::Evt2Context;

/// Sans-IO byte-level decoder for EVT2 data.
///
/// Converts raw little-endian bytes to 32-bit words, feeding them to an
/// [`Evt2Context`]. Supports arbitrary chunk boundaries by buffering
/// partial words between calls to [`decode`].
///
/// [`decode`]: ByteDecoder::decode
///
/// # Example
///
/// ```
/// use evtide_codec::evt2::Evt2ByteDecoder;
/// use evtide_codec::{ByteDecoder, CodecEvent};
///
/// let mut decoder = Evt2ByteDecoder::new();
/// let mut events = Vec::new();
///
/// let consumed = decoder.decode(
///     &[0x00, 0x00, 0x00, 0x80, 0x64, 0x00, 0x00, 0x00],
///     &mut |e| events.push(e),
/// );
/// assert_eq!(consumed, 8);
///
/// decoder.finish().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct Evt2ByteDecoder {
    /// Buffered partial bytes. `pending[..pending_len]` contains bytes from the
    /// previous chunk that did not form a complete 32-bit word.
    pending: [u8; 4],
    pending_len: u8,

    ctx: Evt2Context,
}

impl Default for Evt2ByteDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteDecoder for Evt2ByteDecoder {
    type Context = Evt2Context;

    fn decode(&mut self, bytes: &[u8], on_event: &mut impl FnMut(CodecEvent)) -> usize {
        assemble_words::<4>(&mut self.pending, &mut self.pending_len, bytes, |word| {
            self.ctx.process_word(u32::from_le_bytes(word), on_event)
        })
    }

    fn finish(self) -> Result<Evt2Context, TrailingByteError> {
        if self.pending_len > 0 {
            return Err(TrailingByteError);
        }
        Ok(self.ctx)
    }

    fn context(&self) -> &Evt2Context {
        &self.ctx
    }

    fn reset(&mut self) {
        self.pending_len = 0;
        self.ctx.reset();
    }
}

impl Evt2ByteDecoder {
    pub fn new() -> Self {
        Self {
            pending: [0; 4],
            pending_len: 0,
            ctx: Evt2Context::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use evtide_core::Polarity;

    use super::*;

    fn words_to_bytes(words: &[u32]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    #[test]
    fn roundtrip_even_boundary() {
        let words: &[u32] = &[
            0x8000_0000,                              // TIME_HIGH = 0
            0b0000_001100_00000001010_00001100100u32, // CD_OFF ts=12, x=10, y=100
        ];
        let bytes = words_to_bytes(words);

        let mut decoder = Evt2ByteDecoder::new();
        let mut events = Vec::new();
        let consumed = ByteDecoder::decode(&mut decoder, &bytes, &mut |e| events.push(e));

        assert_eq!(consumed, bytes.len());
        ByteDecoder::finish(decoder).unwrap();

        assert_eq!(events.len(), 1);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd");
        };
        assert_eq!(e.x, 10);
        assert_eq!(e.y, 100);
        assert_eq!(e.polarity, Polarity::Off);
        assert_eq!(e.timestamp, 12);
    }

    #[test]
    fn chunk_boundaries() {
        let words: &[u32] = &[0x8000_0000, 0b0000_000101_00001100100_00000001010u32];
        let bytes = words_to_bytes(words);

        let mut decoder = Evt2ByteDecoder::new();
        let mut events = Vec::new();

        let c1 = ByteDecoder::decode(&mut decoder, &bytes[..6], &mut |e| events.push(e));
        assert_eq!(c1, 6);

        let c2 = ByteDecoder::decode(&mut decoder, &bytes[6..], &mut |e| events.push(e));
        assert_eq!(c2, 2);

        ByteDecoder::finish(decoder).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn single_byte_chunks() {
        let words: &[u32] = &[0x8000_0000];
        let bytes = words_to_bytes(words);

        let mut decoder = Evt2ByteDecoder::new();
        let mut consumed_total = 0;

        for &b in &bytes {
            consumed_total += ByteDecoder::decode(&mut decoder, &[b], &mut |_| {});
        }

        assert_eq!(consumed_total, 4);
        ByteDecoder::finish(decoder).unwrap();
    }

    #[test]
    fn finish_rejects_partial_word() {
        let mut decoder = Evt2ByteDecoder::new();
        ByteDecoder::decode(&mut decoder, &[0x00, 0x00, 0x00], &mut |_| {});
        assert!(ByteDecoder::finish(decoder).is_err());
    }

    #[test]
    fn reset_clears_partial() {
        let mut decoder = Evt2ByteDecoder::new();
        ByteDecoder::decode(&mut decoder, &[0x00, 0x00], &mut |_| {});
        ByteDecoder::reset(&mut decoder);
        assert!(ByteDecoder::finish(decoder).is_ok());
    }
}
