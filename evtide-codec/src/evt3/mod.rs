pub mod context;
pub(crate) mod word;

use crate::{ByteDecoder, CodecEvent, TrailingByteError};

use context::Evt3Context;

/// Sans-IO byte-level decoder for EVT3 data.
///
/// Converts raw little-endian bytes to 16-bit words, feeding them to an
/// [`Evt3Context`]. Supports arbitrary chunk boundaries by buffering a
/// single pending byte between calls to [`decode`].
///
/// [`decode`]: ByteDecoder::decode
///
/// # Example
///
/// ```
/// use evtide_codec::evt3::Evt3ByteDecoder;
/// use evtide_codec::{ByteDecoder, CodecEvent};
///
/// let mut decoder = Evt3ByteDecoder::new();
/// let mut events = Vec::new();
///
/// let consumed = decoder.decode(&[0x00, 0x80, 0x64, 0x60], &mut |e| events.push(e));
/// assert_eq!(consumed, 4);
///
/// decoder.finish().unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct Evt3ByteDecoder {
    /// Buffered byte from an odd-sized chunk.
    pending: Option<u8>,

    ctx: Evt3Context,
}

impl Default for Evt3ByteDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteDecoder for Evt3ByteDecoder {
    type Context = Evt3Context;

    fn decode(&mut self, bytes: &[u8], on_event: &mut impl FnMut(CodecEvent)) -> usize {
        // EVT3's 16-bit words need at most 1 buffered byte, so the assembly
        // is trivial and does not benefit from `assemble_words::<2>`.
        let mut consumed = 0;
        let mut remaining = bytes;

        if let Some(pending) = self.pending.take() {
            if let Some((&first, rest)) = remaining.split_first() {
                let word = u16::from_le_bytes([pending, first]);
                self.ctx.process_word(word, on_event);
                consumed += 1;
                remaining = rest;
            } else {
                self.pending = Some(pending);
                return 0;
            }
        }

        let mut chunks = remaining.chunks_exact(2);
        for chunk in chunks.by_ref() {
            let word = u16::from_le_bytes([chunk[0], chunk[1]]);
            self.ctx.process_word(word, on_event);
        }
        consumed += remaining.len() - chunks.remainder().len();

        if let Some(&last) = chunks.remainder().first() {
            self.pending = Some(last);
        }

        consumed
    }

    fn finish(self) -> Result<Self::Context, TrailingByteError> {
        if self.pending.is_some() {
            return Err(TrailingByteError);
        }
        Ok(self.ctx)
    }

    fn context(&self) -> &Self::Context {
        &self.ctx
    }

    fn reset(&mut self) {
        self.pending = None;
        self.ctx.reset();
    }
}

impl Evt3ByteDecoder {
    pub fn new() -> Self {
        Self {
            pending: None,
            ctx: Evt3Context::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use evtide_core::Polarity;

    use super::*;

    fn words_to_bytes(words: &[u16]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    #[test]
    fn roundtrip_even_boundary() {
        let words: &[u16] = &[
            0x8000, // TIME_HIGH = 0
            0x6064, // TIME_LOW = 100
            0x0032, // ADDR_Y y=50
            0x2864, // ADDR_X x=100, pol=1
        ];
        let bytes = words_to_bytes(words);

        let mut decoder = Evt3ByteDecoder::new();
        let mut events = Vec::new();
        let consumed = ByteDecoder::decode(&mut decoder, &bytes, &mut |e| events.push(e));

        assert_eq!(consumed, bytes.len());
        ByteDecoder::finish(decoder).unwrap();

        assert_eq!(events.len(), 1);
        let CodecEvent::Cd(e) = &events[0] else {
            panic!("expected Cd");
        };
        assert_eq!(e.x, 100);
        assert_eq!(e.y, 50);
        assert_eq!(e.polarity, Polarity::On);
        assert_eq!(e.timestamp, 100);
    }

    #[test]
    fn odd_chunk_boundaries() {
        let words: &[u16] = &[0x8000, 0x6064, 0x0032, 0x2864];
        let bytes = words_to_bytes(words);

        let mut decoder = Evt3ByteDecoder::new();
        let mut events = Vec::new();

        let c1 = ByteDecoder::decode(&mut decoder, &bytes[..5], &mut |e| events.push(e));
        assert_eq!(c1, 4);

        let c2 = ByteDecoder::decode(&mut decoder, &bytes[5..], &mut |e| events.push(e));
        assert_eq!(c2, 3);

        ByteDecoder::finish(decoder).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn single_byte_chunks() {
        let words: &[u16] = &[0x8000, 0x6064];
        let bytes = words_to_bytes(words);

        let mut decoder = Evt3ByteDecoder::new();
        let mut consumed_total = 0;

        for &b in &bytes {
            consumed_total += ByteDecoder::decode(&mut decoder, &[b], &mut |_| {});
        }

        assert_eq!(consumed_total, 4);
        ByteDecoder::finish(decoder).unwrap();
    }

    #[test]
    fn finish_rejects_trailing_byte() {
        let mut decoder = Evt3ByteDecoder::new();
        ByteDecoder::decode(&mut decoder, &[0x00], &mut |_| {});
        assert!(ByteDecoder::finish(decoder).is_err());
    }

    #[test]
    fn reset_clears_pending_byte() {
        let mut decoder = Evt3ByteDecoder::new();
        ByteDecoder::decode(&mut decoder, &[0x00], &mut |_| {});
        ByteDecoder::reset(&mut decoder);
        assert!(ByteDecoder::finish(decoder).is_ok());
    }
}
