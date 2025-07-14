# CLAUDE.md - AI Agent Instructions

## Project Overview

You are implementing `noise-mobile-rust`, a mobile-optimized Rust library for the Noise Protocol Framework. This library will provide FFI-safe bindings for iOS and Android applications, specifically designed for P2P messaging apps like BitChat.

## Critical Context

1. **Snow Library Version**: 
   **IMPORTANT UPDATE**: We are using snow 0.10.0-beta.2 from crates.io:
   ```toml
   [dependencies]
   snow = "0.10.0-beta.2"
   ```
   Do NOT use the local path reference. The library is available on crates.io and should be used from there.

2. **Purpose**: This is NOT a standalone app. It's a Rust library designed to be integrated into iOS (Swift) and Android (Kotlin) applications via FFI.

3. **Testing Priority**: The user has physical iOS and Android devices. All code must be testable on real hardware, not just simulators.

## Architecture Requirements

### Core Design Principles

1. **FFI-First**: Every public API must be FFI-safe
   ```rust
   #[no_mangle]
   pub extern "C" fn noise_session_new() -> *mut NoiseSession { }
   ```

2. **No Panics**: Use Result types internally, return error codes via FFI
   ```rust
   #[repr(C)]
   pub enum NoiseError {
       Success = 0,
       InvalidKey = 1,
       HandshakeFailed = 2,
       // etc
   }
   ```

3. **Memory Safety**: Clear ownership rules for FFI
   - Rust owns all allocated memory
   - Provide explicit free functions
   - Use opaque pointers for complex types

4. **Platform Agnostic**: No platform-specific code in core library
   - Platform-specific features go in separate modules
   - Use trait abstractions for platform differences

## Implementation Priorities

### Phase 1: Core Wrapper (Week 1)
1. Create safe Rust wrapper around Snow
2. Implement Noise_XX pattern (mutual authentication)
3. Design FFI-safe API surface
4. Basic error handling

### Phase 2: FFI Layer (Week 1)
1. C-compatible API
2. Memory management functions
3. Error code system
4. Opaque pointer types

### Phase 3: Mobile Helpers (Week 2)
1. Key storage abstraction (for Keychain/Keystore)
2. Network resilience features
3. Background task handling
4. Battery-optimized crypto scheduling

### Phase 4: Testing (Week 2)
1. Comprehensive Rust unit tests
2. FFI boundary tests
3. Example iOS/Android integration
4. Performance benchmarks

## Project Structure

```
noise-mobile-rust/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Main library entry
│   ├── core/            # Pure Rust implementation
│   │   ├── mod.rs
│   │   ├── session.rs   # Noise session management
│   │   ├── crypto.rs    # Crypto operations
│   │   └── error.rs     # Error types
│   ├── ffi/             # FFI bindings
│   │   ├── mod.rs
│   │   ├── c_api.rs     # C-compatible API
│   │   └── types.rs     # FFI-safe types
│   └── mobile/          # Mobile-specific helpers
│       ├── mod.rs
│       ├── storage.rs   # Key storage traits
│       └── network.rs   # Network resilience
├── tests/
│   ├── unit/            # Rust unit tests
│   ├── ffi/             # FFI boundary tests
│   └── integration/     # Full integration tests
├── benches/             # Performance benchmarks
├── examples/
│   ├── ios_integration.rs
│   └── android_integration.rs
└── ios-binding/         # iOS-specific binding code
    └── android-binding/ # Android-specific binding code
```

## Code Style Guidelines

1. **Use Rust 2021 Edition**
2. **Follow Rust API Guidelines**: https://rust-lang.github.io/api-guidelines/
3. **Document all public APIs** with examples
4. **Use `#[must_use]` on Result-returning functions**
5. **Prefer explicit over implicit** - no magic

## Testing Requirements

### Unit Tests
- Test every public function
- Test error conditions explicitly
- Use proptest for property-based testing where appropriate

### FFI Tests
- Test memory allocation/deallocation
- Test error propagation across FFI boundary
- Verify no undefined behavior

### Integration Tests
- Full handshake scenarios
- Network interruption handling
- Concurrent session management
- Performance under load

### Example Test:
```rust
#[test]
fn test_noise_handshake() {
    let initiator = NoiseSession::new_initiator(&keypair);
    let responder = NoiseSession::new_responder(&keypair);
    
    // Perform full XX handshake
    let msg1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg1).unwrap();
    // ... etc
}
```

## Performance Targets

- Handshake completion: < 10ms on mobile CPU
- Message encryption: > 10MB/s on mobile CPU
- Memory overhead: < 1KB per session
- Battery impact: Minimal (test with Android Battery Historian)

## Security Considerations

1. **Zeroize sensitive data** when dropped
2. **Constant-time operations** where required
3. **No logging of sensitive data**
4. **Secure random number generation** via platform APIs

## FFI Integration Guide

### iOS (Swift)
```swift
// The library should be usable like this:
let session = NoiseSession(mode: .initiator)
let handshakeMsg = try session.writeMessage(Data())
```

### Android (Kotlin)
```kotlin
// The library should be usable like this:
val session = NoiseSession(NoiseMode.INITIATOR)
val handshakeMsg = session.writeMessage(ByteArray(0))
```

## Common Pitfalls to Avoid

1. **Don't use `std::panic!`** - Return errors instead
2. **Don't leak memory** - Provide cleanup functions
3. **Don't assume platform features** - Abstract them
4. **Don't over-optimize early** - Focus on correctness first
5. **Don't forget mobile constraints** - Limited CPU, memory, battery

## Resources

- Snow documentation: Check `../snow/README.md`
- Noise Protocol Specification: https://noiseprotocol.org/noise.html
- Rust FFI Omnibus: http://jakegoulding.com/rust-ffi-omnibus/
- Mobile best practices: Research iOS Background Tasks, Android Doze Mode

## Questions to Consider

1. How will the library handle app backgrounding?
2. What happens to sessions during network changes?
3. How to minimize battery impact during crypto operations?
4. How to handle key rotation and forward secrecy?
5. What telemetry/debugging is needed for mobile?

## Next Steps

1. ~~Run `cargo init --lib` to create the library~~ ✅
2. ~~Set up the module structure as outlined~~ ✅
3. ~~Implement core Noise wrapper first~~ ✅
4. ~~Add FFI layer incrementally~~ ✅
5. Test on real devices early and often

Remember: This library's success depends on being genuinely useful for mobile developers. Keep the API simple, the performance good, and the integration painless.

## Current Implementation Status (July 14, 2025)

### What's Been Completed

1. **Core Implementation** ✅
   - Full Noise_XX pattern implementation using snow 0.10.0-beta.2
   - Complete session lifecycle (handshake → transport)
   - Three-state model to handle state transitions safely
   - All core tests passing

2. **FFI Layer** ✅
   - Complete C API with all necessary functions
   - Memory safety helpers in `ffi/helpers.rs`
   - Proper error handling without panics
   - Buffer size validation

3. **Mobile Features** (Partial)
   - Key storage abstraction ✅ (memory implementation only)
   - Network resilience 🚧 (structure only)
   - Battery optimization 🚧 (structure only)

### Important Implementation Details

1. **State Transition Handling**: The implementation uses a three-state model (Handshake, Transitioning, Transport) to safely handle the ownership transfer when moving from handshake to transport mode.

2. **Memory Safety**: All FFI functions use helper functions for:
   - Pointer validation (`validate_session_ptr`)
   - Safe buffer operations (`copy_to_c_buffer`)
   - Null-safe slice conversion (`c_to_slice`)

3. **Error Handling**: No panics in FFI code. All errors are converted to error codes.

4. **Testing**: 13 tests currently passing covering core functionality, FFI helpers, and key storage.

### What Remains

See `CURRENT_STATUS.md` and `TODO.md` for detailed information about remaining tasks. Key items:
- Complete network resilience implementation
- Complete battery optimization
- Add comprehensive test suite
- Create platform build scripts
- Add integration examples

The library is functional but needs the remaining features for production use.