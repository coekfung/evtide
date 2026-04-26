# evtide

Experimental cross-platform event-camera I/O and visualization toolkit.

[![Crates.io](https://img.shields.io/crates/v/evtide.svg)](https://crates.io/crates/evtide)
[![Docs](https://docs.rs/evtide/badge.svg)](https://docs.rs/evtide)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Overview

**evtide** is a Rust library for working with neuromorphic event-based cameras. It provides tools for reading, writing, processing, and visualizing event-camera data across platforms.

## Features

- Cross-platform event-camera I/O
- Event stream processing and filtering
- Visualization utilities for event data

## Installation

Add `evtide` to your `Cargo.toml`:

```toml
[dependencies]
evtide = "0.1"
```

## Usage

```rust
use evtide;

fn main() {
    println!("evtide version: {}", evtide::VERSION);
}
```

## Documentation

Full API documentation is available at [docs.rs/evtide](https://docs.rs/evtide).

## License

This project is licensed under the [MIT License](LICENSE).
