pub mod context;
pub(crate) mod word;

use crate::CodecEvent;

use context::Evt3Context;

/// Error returned when a byte stream ends with a pending half-word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrailingByteError;

impl core::fmt::Display for TrailingByteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("EVT3 byte stream ended with an incomplete 16-bit word")
    }
}

/// Sans-IO byte-level decoder for EVT3 data.
///
/// Converts raw little-endian bytes to 16-bit words, feeding them to an
/// [`Evt3Context`]. Supports arbitrary chunk boundaries by buffering a
/// single pending byte between calls to [`decode`].
///
/// [`decode`]: Self::decode
///
/// # Example
///
/// ```
/// use evtide_codec::evt3::Evt3ByteDecoder;
/// use evtide_codec::CodecEvent;
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

    context: Evt3Context,
}

impl Default for Evt3ByteDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Evt3ByteDecoder {
    pub fn new() -> Self {
        Self {
            pending: None,
            context: Evt3Context::new(),
        }
    }

    /// Resets all state, including the underlying [`Evt3Context`] and any
    /// buffered pending byte.
    pub fn reset(&mut self) {
        self.pending = None;
        self.context.reset();
    }

    /// Feeds raw little-endian bytes into the decoder.
    ///
    /// Each complete 16-bit word is passed to the underlying [`Evt3Context`],
    /// which calls `on_event` for each decoded event.
    ///
    /// Returns the number of bytes consumed from `bytes`. A byte is consumed
    /// once it has been paired into a complete `u16` word. Trailing odd bytes
    /// are buffered and not counted as consumed.
    pub fn decode(&mut self, bytes: &[u8], on_event: &mut impl FnMut(CodecEvent)) -> usize {
        let mut consumed = 0;
        let mut remaining = bytes;

        // Pair a pending byte with the first byte of this chunk.
        if let Some(pending) = self.pending.take() {
            if let Some((&first, rest)) = remaining.split_first() {
                let word = u16::from_le_bytes([pending, first]);
                self.context.process_word(word, on_event);
                consumed += 1; // Byte paired with the buffered pending byte.
                remaining = rest;
            } else {
                self.pending = Some(pending);
                return 0;
            }
        }

        // Process complete pairs.
        let mut chunks = remaining.chunks_exact(2);
        for chunk in chunks.by_ref() {
            let word = u16::from_le_bytes([chunk[0], chunk[1]]);
            self.context.process_word(word, on_event);
        }
        consumed += remaining.len() - chunks.remainder().len();

        // Buffer any odd trailing byte.
        if let Some(&last) = chunks.remainder().first() {
            self.pending = Some(last);
        }

        consumed
    }

    /// Finalizes the byte stream.
    ///
    /// Returns the underlying [`Evt3Context`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`TrailingByteError`] if a single byte from a previous
    /// [`decode`](Self::decode) call is still buffered, indicating an
    /// incomplete stream.
    pub fn finish(self) -> Result<Evt3Context, TrailingByteError> {
        if self.pending.is_some() {
            return Err(TrailingByteError);
        }
        Ok(self.context)
    }

    /// Returns a shared reference to the underlying [`Evt3Context`].
    pub fn context(&self) -> &Evt3Context {
        &self.context
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
        let consumed = decoder.decode(&bytes, &mut |e| events.push(e));

        assert_eq!(consumed, bytes.len());
        decoder.finish().unwrap();

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

        // Feed 5 bytes (2.5 words)
        let c1 = decoder.decode(&bytes[..5], &mut |e| events.push(e));
        assert_eq!(c1, 4); // Only 4 bytes consumed (2 complete words)

        // Feed remaining 3 bytes
        let c2 = decoder.decode(&bytes[5..], &mut |e| events.push(e));
        assert_eq!(c2, 3); // 2 consumed + 1 pending from before = 3

        decoder.finish().unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn single_byte_chunks() {
        let words: &[u16] = &[0x8000, 0x6064];
        let bytes = words_to_bytes(words);

        let mut decoder = Evt3ByteDecoder::new();
        let mut consumed_total = 0;

        // Feed one byte at a time
        for &b in &bytes {
            consumed_total += decoder.decode(&[b], &mut |_| {});
        }

        assert_eq!(consumed_total, 4);
        decoder.finish().unwrap();
    }

    #[test]
    fn finish_rejects_trailing_byte() {
        let mut decoder = Evt3ByteDecoder::new();
        decoder.decode(&[0x00], &mut |_| {});
        assert!(decoder.finish().is_err());
    }

    #[test]
    fn reset_clears_pending_byte() {
        let mut decoder = Evt3ByteDecoder::new();

        // Feed an odd byte to create pending state
        decoder.decode(&[0x00], &mut |_| {});

        // Reset should clear it
        decoder.reset();
        assert!(decoder.finish().is_ok());
    }
}
