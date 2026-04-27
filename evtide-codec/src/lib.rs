#![forbid(unsafe_code)]

//! Sans-IO decoders for event-camera data formats.
//!
//! All decoders operate on byte slices and emit events through
//! caller-provided callbacks. They do not read files, open sockets, or
//! allocate.

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
