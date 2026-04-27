# Changelog

## [Unreleased]

### Added

- EVT2 sans-IO decoder: `Evt2Context` state machine and `Evt2ByteDecoder` byte-level wrapper.
- EVT3 sans-IO decoder: `Evt3Context` state machine and `Evt3ByteDecoder` byte-level wrapper.
- `CodecEvent` sum type for unified CD and trigger event output.
- `ByteDecoder` trait as uniform interface for all format decoders.
- `TrailingByteError` for incomplete byte stream detection.
