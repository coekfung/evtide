pub mod context;
pub(crate) mod word;

use crate::assemble::assemble_words;
use crate::{ByteDecoder, CodecEvent, TrailingByteError};

use context::Evt21Context;

/// Sans-IO byte-level decoder for EVT2.1 data.
///
/// Converts raw little-endian bytes to 64-bit words, feeding them to an
/// [`Evt21Context`]. Supports arbitrary chunk boundaries by buffering
/// partial words between calls to [`decode`].
///
/// [`decode`]: ByteDecoder::decode
///
/// # Example
///
/// ```
/// use evtide_codec::evt21::Evt21ByteDecoder;
/// use evtide_codec::{ByteDecoder, CodecEvent};
///
/// let mut decoder = Evt21ByteDecoder::new();
/// let mut events = Vec::new();
///
/// let consumed = decoder.decode(
///     &[0x00; 16],
///     &mut |e| events.push(e),
/// );
/// assert_eq!(consumed, 16);
///
/// decoder.finish().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct Evt21ByteDecoder {
    pending: [u8; 8],
    pending_len: u8,

    ctx: Evt21Context,
}

impl Default for Evt21ByteDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteDecoder for Evt21ByteDecoder {
    type Context = Evt21Context;

    fn decode(&mut self, bytes: &[u8], on_event: &mut impl FnMut(CodecEvent)) -> usize {
        assemble_words::<8>(&mut self.pending, &mut self.pending_len, bytes, |word| {
            self.ctx.process_word(u64::from_le_bytes(word), on_event)
        })
    }

    fn finish(self) -> Result<Evt21Context, TrailingByteError> {
        if self.pending_len > 0 {
            return Err(TrailingByteError);
        }
        Ok(self.ctx)
    }

    fn context(&self) -> &Evt21Context {
        &self.ctx
    }

    fn reset(&mut self) {
        self.pending_len = 0;
        self.ctx.reset();
    }
}

impl Evt21ByteDecoder {
    pub fn new() -> Self {
        Self {
            pending: [0; 8],
            pending_len: 0,
            ctx: Evt21Context::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use evtide_core::Polarity;

    use super::*;

    fn words_to_bytes(words: &[u64]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    #[test]
    fn roundtrip_even_boundary() {
        let words: &[u64] = &[
            0x8000_0000_0000_0000,
            (0u64 << 60) | (12u64 << 54) | (40u64 << 43) | (100u64 << 32) | 5,
        ];
        let bytes = words_to_bytes(words);

        let mut decoder = Evt21ByteDecoder::new();
        let mut events = Vec::new();
        let consumed = ByteDecoder::decode(&mut decoder, &bytes, &mut |e| events.push(e));

        assert_eq!(consumed, bytes.len());
        ByteDecoder::finish(decoder).unwrap();

        assert_eq!(events.len(), 2);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd");
        };
        assert_eq!(e.x, 40);
        assert_eq!(e.y, 100);
        assert_eq!(e.polarity, Polarity::Off);
        assert_eq!(e.timestamp, 12);
    }

    #[test]
    fn chunk_boundaries() {
        let words: &[u64] = &[
            0x8000_0000_0000_0000,
            (1u64 << 60) | (5u64 << 54) | (100u64 << 43) | (10u64 << 32) | 3,
        ];
        let bytes = words_to_bytes(words);

        let mut decoder = Evt21ByteDecoder::new();
        let mut events = Vec::new();

        let c1 = ByteDecoder::decode(&mut decoder, &bytes[..10], &mut |e| events.push(e));
        assert_eq!(c1, 10);

        let c2 = ByteDecoder::decode(&mut decoder, &bytes[10..], &mut |e| events.push(e));
        assert_eq!(c2, 6);

        ByteDecoder::finish(decoder).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn single_byte_chunks() {
        let words: &[u64] = &[0x8000_0000_0000_0000];
        let bytes = words_to_bytes(words);

        let mut decoder = Evt21ByteDecoder::new();
        let mut consumed_total = 0;

        for &b in &bytes {
            consumed_total += ByteDecoder::decode(&mut decoder, &[b], &mut |_| {});
        }

        assert_eq!(consumed_total, 8);
        ByteDecoder::finish(decoder).unwrap();
    }

    #[test]
    fn finish_rejects_partial_word() {
        let mut decoder = Evt21ByteDecoder::new();
        ByteDecoder::decode(&mut decoder, &[0x00; 5], &mut |_| {});
        assert!(ByteDecoder::finish(decoder).is_err());
    }

    #[test]
    fn reset_clears_partial() {
        let mut decoder = Evt21ByteDecoder::new();
        ByteDecoder::decode(&mut decoder, &[0x00; 3], &mut |_| {});
        ByteDecoder::reset(&mut decoder);
        assert!(ByteDecoder::finish(decoder).is_ok());
    }
}
