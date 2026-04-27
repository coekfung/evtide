# Changelog

## [0.0.2] - 2026-04-27

### Added

- EVT2 sans-IO decoder: `Evt2Context` state machine and `Evt2ByteDecoder` byte-level wrapper.
- EVT2.1 sans-IO decoder: `Evt21Context` state machine and `Evt21ByteDecoder` byte-level wrapper.
- EVT3 sans-IO decoder: `Evt3Context` state machine and `Evt3ByteDecoder` byte-level wrapper.
- `CodecEvent` sum type for unified CD and trigger event output.
- `ByteDecoder` trait as uniform interface for all format decoders.
- `TrailingByteError` for incomplete byte stream detection.
- `assemble_words::<N>` reusable const-generic byte assembly function.

## [0.0.1]
