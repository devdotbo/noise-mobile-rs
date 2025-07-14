# TODO List for noise-mobile-rust

## High Priority Tasks

### 1. Complete Network Resilience Implementation
**File**: `src/mobile/network.rs`
**Current State**: Basic struct exists but no implementation

```rust
// Implementation needed:
impl ResilientSession {
    // Add sequence number to messages for ordering
    pub fn encrypt_with_sequence(&mut self, plaintext: &[u8]) -> Result<Vec<u8>>
    
    // Check sequence numbers to prevent replay
    pub fn decrypt_with_replay_check(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>>
    
    // Handle out-of-order messages with sliding window
    pub fn set_replay_window_size(&mut self, size: usize)
    
    // Serialize/deserialize for session resumption
    pub fn serialize(&self) -> Vec<u8>
    pub fn deserialize(data: &[u8]) -> Result<Self>
}
```

**Testing Required**:
- Test replay attack prevention
- Test out-of-order message handling
- Test session serialization/deserialization

### 2. Complete Battery Optimization Implementation
**File**: `src/mobile/battery.rs`
**Current State**: Basic struct exists but no implementation

```rust
// Implementation needed:
impl BatchedCrypto {
    // Actually implement the flush operation
    pub fn flush_encrypts(&mut self) -> Result<Vec<Vec<u8>>>
    
    // Add decrypt queueing
    pub fn queue_decrypt(&mut self, ciphertext: Vec<u8>)
    pub fn flush_decrypts(&mut self) -> Result<Vec<Vec<u8>>>
    
    // Add threshold-based auto-flush
    pub fn set_flush_threshold(&mut self, threshold: usize)
    
    // Add time-based auto-flush for latency control
    pub fn set_flush_interval(&mut self, interval: Duration)
}
```

**Testing Required**:
- Test batch encryption/decryption
- Test auto-flush behavior
- Benchmark performance improvements

### 3. FFI Boundary Tests
**File**: Create `tests/ffi_tests.rs`

Required tests:
- Double-free protection
- Null pointer handling for all functions
- Buffer overflow protection
- Invalid enum values
- Concurrent session creation/destruction
- Memory leak detection (using valgrind in CI)

Example test structure:
```rust
#[test]
fn test_double_free_protection() {
    unsafe {
        let session = noise_session_new(0, &mut 0);
        noise_session_free(session);
        noise_session_free(session); // Should not crash
    }
}
```

### 4. Integration Tests
**File**: Create `tests/integration_tests.rs`

Required tests:
- Full handshake between C and Rust APIs
- Interop between iOS and Android examples
- Network failure scenarios
- Session persistence and resumption
- Performance under load

### 5. Security Test Suite
**File**: Create `tests/security_tests.rs`

Required tests:
- Replay attack prevention
- Man-in-the-middle detection
- Timing attack resistance
- Malformed message handling
- Key compromise scenarios

## Medium Priority Tasks

### 6. Platform Build Scripts
**File**: Create `build-ios.sh`

```bash
#!/bin/bash
# Build for iOS simulator and device
cargo build --target aarch64-apple-ios --release
cargo build --target x86_64-apple-ios --release
# Create universal library
lipo -create target/aarch64-apple-ios/release/libnoise_mobile.a \
             target/x86_64-apple-ios/release/libnoise_mobile.a \
     -output target/universal/release/libnoise_mobile.a
# Generate C header
cbindgen --config cbindgen.toml --crate noise-mobile-rust --output include/noise_mobile.h
```

**File**: Create `build-android.sh`

```bash
#!/bin/bash
# Build for all Android architectures
cargo build --target aarch64-linux-android --release
cargo build --target armv7-linux-androideabi --release
cargo build --target i686-linux-android --release
cargo build --target x86_64-linux-android --release
```

### 7. iOS Integration Example
**Directory**: Create `examples/ios/`

Files needed:
- `NoiseSession.swift` - Swift wrapper class
- `BLEIntegration.swift` - Example BLE usage
- `README.md` - Setup instructions
- `Podfile` - CocoaPods integration

### 8. Android Integration Example  
**Directory**: Create `examples/android/`

Files needed:
- `NoiseSession.kt` - Kotlin wrapper class
- `NoiseJNI.c` - JNI bridge implementation
- `BLEActivity.kt` - Example BLE usage
- `build.gradle` - Gradle configuration
- `README.md` - Setup instructions

### 9. Performance Benchmarks
**File**: Create `benches/handshake.rs` and `benches/encryption.rs`

Benchmarks needed:
- Handshake completion time
- Encryption/decryption throughput
- Memory usage per session
- Batch vs individual operations
- FFI overhead measurement

### 10. Documentation Updates
- Add rustdoc comments to all public APIs
- Create `SECURITY.md` with threat model
- Create `PERFORMANCE.md` with tuning guide
- Update `README.md` with usage examples
- Add inline examples in code

## Low Priority Tasks

### 11. CI/CD Setup
**File**: Create `.github/workflows/ci.yml`

- Run tests on multiple platforms
- Check for memory leaks
- Run benchmarks and track regressions
- Build for all target platforms
- Generate and deploy documentation

### 12. Platform-Specific Implementations

#### iOS Keychain Integration
**File**: `src/mobile/storage.rs`
- Implement `KeychainStorage` using Security framework
- Requires conditional compilation and iOS-specific dependencies

#### Android Keystore Integration  
**File**: `src/mobile/storage.rs`
- Implement `KeystoreStorage` using Android Keystore
- Requires JNI calls to Java Keystore API

### 13. Advanced Features
- Post-quantum crypto readiness (hybrid modes)
- Hardware crypto acceleration support
- Custom transport abstraction
- Noise pattern negotiation
- Multi-session management

## Implementation Notes

### For Network Resilience
- Use a sliding window bitmap for replay protection
- Consider using `bitvec` crate for efficient bitmap
- Sequence numbers should be 8 bytes
- Window size should be configurable (default 64)

### For Battery Optimization  
- Use `tokio::time::interval` for time-based flushing
- Consider wake lock implications
- Benchmark optimal batch sizes
- Add metrics collection

### For Testing
- Use `proptest` for property-based testing
- Use `criterion` for benchmarks
- Consider `loom` for concurrency testing
- Add fuzzing targets for security tests

### For Platform Integration
- Keep platform-specific code behind feature flags
- Use `cfg` attributes for conditional compilation
- Document platform requirements clearly
- Test on real devices, not just simulators

## Success Metrics

Each task should meet these criteria:
- All tests pass
- No memory leaks (verified with valgrind/ASan)
- Performance meets targets (handshake <10ms, encryption >10MB/s)
- Code has documentation and examples
- FFI functions handle all error cases without panicking