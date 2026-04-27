#![forbid(unsafe_code)]

//! Sans-IO decoders for event-camera data formats.
//!
//! All decoders operate on byte slices and emit events through
//! caller-provided callbacks. They do not read files, open sockets, or
//! allocate.

pub mod evt2;
pub mod evt3;

use evtide_core::{EventCd, EventExtTrigger};

/// An event emitted by a codec decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecEvent {
    /// A contrast-detection (CD) event.
    Cd(EventCd),
    /// An external trigger event.
    Trigger(EventExtTrigger),
}

/// Error returned when a byte stream ends with incomplete buffered bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrailingByteError;

impl core::fmt::Display for TrailingByteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("byte stream ended with an incomplete word")
    }
}

/// Push-based byte-level decoder.
///
/// Implementations decode a specific format (EVT2, EVT3, …) by consuming raw
/// bytes in arbitrary chunk sizes and emitting [`CodecEvent`]s through a
/// caller-provided callback.
pub trait ByteDecoder {
    /// The underlying word-level context for this format.
    type Context;

    /// Feed raw bytes. Returns the number of bytes consumed.
    fn decode(&mut self, bytes: &[u8], on_event: &mut impl FnMut(CodecEvent)) -> usize;

    /// Signal end of stream. Consumes the decoder.
    ///
    /// Returns the underlying context on success, or [`TrailingByteError`] if
    /// buffered partial bytes remain.
    fn finish(self) -> Result<Self::Context, TrailingByteError>;

    /// Shared reference to the underlying context.
    fn context(&self) -> &Self::Context;

    /// Reset all state, including the underlying context and any buffered
    /// partial bytes.
    fn reset(&mut self);
}
