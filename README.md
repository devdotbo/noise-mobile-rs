# noise-mobile-rust

A mobile-optimized Rust implementation of the Noise Protocol Framework, designed for iOS and Android integration in P2P messaging applications.

## Overview

`noise-mobile-rust` provides FFI-safe bindings for the Noise Protocol, making it easy to integrate secure, end-to-end encrypted communication into mobile applications. Built on top of the excellent [snow](https://github.com/mcginty/snow) library, this crate adds mobile-specific optimizations and a clean FFI interface.

## Features

- 🔐 **Noise Protocol XX** - Mutual authentication pattern
- 📱 **Mobile Optimized** - Battery-efficient, background-task aware
- 🔄 **Network Resilient** - Handles connection interruptions gracefully
- 🚀 **High Performance** - < 10ms handshakes on mobile CPUs
- 🛡️ **Memory Safe** - Rust's guarantees extend across FFI boundary
- 🧪 **Extensively Tested** - Unit, integration, and device tests

## Architecture

```
┌─────────────────┐     ┌─────────────────┐
│   iOS Swift     │     │ Android Kotlin  │
│      App        │     │      App        │
└────────┬────────┘     └────────┬────────┘
         │ FFI                   │ JNI
         ├───────────┬───────────┤
                     │
         ┌───────────▼───────────┐
         │   noise-mobile-rust   │
         │  (This Library)       │
         │                       │
         │ • C-compatible API    │
         │ • Mobile optimizations│
         │ • Network resilience  │
         └───────────┬───────────┘
                     │
         ┌───────────▼───────────┐
         │     snow library      │
         │ (Noise Protocol impl) │
         └───────────────────────┘
```

## Quick Start

### Rust Integration

```rust
use noise_mobile::{NoiseSession, NoiseMode};

// Initialize a session
let session = NoiseSession::new(NoiseMode::Initiator)?;

// Perform handshake
let handshake_msg = session.write_message(&[])?;
// Send handshake_msg to peer...

// After handshake completion
let encrypted = session.encrypt(b"Hello, secure world!")?;
```

### iOS Integration

```swift
import NoiseMobile

let session = NoiseSession(mode: .initiator)
let handshakeData = try session.writeMessage(Data())
// Send to peer...
```

### Android Integration

```kotlin
import com.noise.mobile.NoiseSession

val session = NoiseSession(NoiseMode.INITIATOR)
val handshakeData = session.writeMessage(ByteArray(0))
// Send to peer...
```

## Performance

| Operation | Target | Actual |
|-----------|--------|--------|
| Handshake | < 10ms | TBD |
| Encrypt 1MB | < 100ms | TBD |
| Memory/Session | < 1KB | TBD |

## Building

### Prerequisites

- Rust 1.75+ (2021 edition)
- For iOS: Xcode 14+
- For Android: NDK r25+

### Build Library

```bash
# Debug build
cargo build

# Release build with optimizations
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### Generate Mobile Bindings

```bash
# iOS
cargo build --target aarch64-apple-ios --release

# Android
cargo build --target aarch64-linux-android --release
```

## Testing

We prioritize testing on real devices:

```bash
# Unit tests
cargo test

# FFI boundary tests
cargo test --features ffi-tests

# Integration tests (requires device/simulator)
cargo test --features integration-tests

# Benchmarks
cargo bench
```

## Project Structure

```
noise-mobile-rust/
├── src/
│   ├── core/        # Pure Rust implementation
│   ├── ffi/         # FFI bindings
│   └── mobile/      # Mobile-specific features
├── tests/           # Comprehensive test suite
├── benches/         # Performance benchmarks
└── examples/        # Integration examples
```

## Security

- All cryptographic operations via audited `snow` library
- Zeroization of sensitive data
- Constant-time operations where required
- No runtime panics in FFI layer

## Contributing

Contributions are welcome! Please ensure:

1. All tests pass
2. No compiler warnings
3. Code is formatted with `rustfmt`
4. New features include tests
5. FFI changes are documented

## Use Cases

- P2P messaging apps (like BitChat)
- Secure file transfer
- IoT device communication
- Any app requiring end-to-end encryption

## License

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)

## Acknowledgments

Built on top of [snow](https://github.com/mcginty/snow) by @mcginty