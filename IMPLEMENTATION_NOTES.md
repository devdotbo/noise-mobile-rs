# Implementation Notes for noise-mobile-rust

## Critical Technical Details

### 1. State Transition Pattern

The `NoiseSession` uses a three-state pattern to handle the ownership issue when transitioning from handshake to transport mode:

```rust
pub enum NoiseState {
    Handshake(Box<HandshakeState>),
    Transport(Box<TransportState>),
    Transitioning,  // Temporary state during transition
}
```

This prevents the "use after move" error that would occur with a simple two-state model. The transition happens like this:
1. Replace current state with `Transitioning`
2. Extract the `HandshakeState` from the old state
3. Call `into_transport_mode()` to consume it
4. Set the new `Transport` state

### 2. FFI Memory Safety Pattern

All FFI functions follow this pattern:
```rust
pub extern "C" fn noise_function(
    session: *mut NoiseSessionFFI,
    // other params
) -> c_int {
    // 1. Validate pointers
    if !validate_session_ptr(session) {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    // 2. Convert to safe types
    let session = unsafe { &mut *(session as *mut NoiseSession) };
    
    // 3. Perform operation
    match session.some_method() {
        Ok(result) => {
            // 4. Copy result to C buffer safely
            if unsafe { copy_to_c_buffer(&result, output, output_len) } {
                NoiseErrorCode::Success as c_int
            } else {
                NoiseErrorCode::BufferTooSmall as c_int
            }
        }
        Err(e) => NoiseErrorCode::from(e) as c_int,
    }
}
```

### 3. Buffer Size Management

The library uses these constants:
- `NOISE_MAX_MESSAGE_LEN`: 65535 bytes (maximum Noise message)
- `NOISE_MAX_PAYLOAD_LEN`: 65519 bytes (minus 16-byte AEAD tag)
- `NOISE_TAG_LEN`: 16 bytes (ChaCha20Poly1305 tag)

FFI functions always update the size parameter to indicate required buffer size, even on failure.

### 4. Error Handling Philosophy

- No `unwrap()` in production code
- No `panic!()` in FFI layer
- All errors mapped to C-compatible error codes
- Error strings available via `noise_error_string()`

### 5. Zeroization Strategy

All sensitive data is zeroized:
- Session buffers on drop
- Keys when replaced or deleted
- Temporary buffers after use

The `zeroize` crate with `derive` feature handles this automatically for marked types.

### 6. Testing Approach

Tests are organized by layer:
- Core tests: Pure Rust functionality
- FFI tests: Boundary conditions and safety
- Integration tests: End-to-end scenarios
- Security tests: Attack scenarios

Run all tests with: `cargo test`
Run specific test module: `cargo test core::session::tests`

### 7. Platform Conditional Compilation

Platform-specific code uses:
```rust
#[cfg(target_os = "ios")]
pub struct KeychainStorage;

#[cfg(target_os = "android")]
pub struct KeystoreStorage;
```

### 8. Performance Considerations

- Buffers are reused within sessions (not reallocated)
- Boxing used for large state objects to reduce stack usage
- Release profile optimized for mobile (LTO, single codegen unit)

### 9. Concurrency Model

- Each `NoiseSession` is `Send` but not `Sync`
- Sessions can be used from different threads but not simultaneously
- For concurrent access, wrap in `Arc<Mutex<NoiseSession>>`

### 10. Debugging Tips

- Set `RUST_BACKTRACE=1` for panic debugging
- Use `cargo test -- --nocapture` to see println! output
- Check pointer alignment with the validation helpers
- Use ASan/valgrind for memory leak detection

### 11. Common Pitfalls Avoided

1. **Double-free protection**: FFI free functions handle null/invalid pointers
2. **Buffer overflows**: Size validation before all copies
3. **Null pointer derefs**: Explicit null checks
4. **Uninitialized memory**: All buffers zero-initialized
5. **Race conditions**: Single-threaded session design

### 12. Future Compatibility

The design allows for:
- Additional Noise patterns (IK, NK, etc.)
- Different crypto primitives
- Post-quantum hybrid modes
- Custom transports

These can be added without breaking the existing API.