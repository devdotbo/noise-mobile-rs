# Updated TODO List for noise-mobile-rust

**Last Updated**: July 14, 2025 (After Session 2)

## ✅ Completed in Session 2

### 1. ~~Add Dual License Files~~ ✅
- Added LICENSE-APACHE and LICENSE-MIT
- Updated lib.rs with SPDX identifier
- Commit: f78ef0b

### 2. ~~Complete Network Resilience Implementation~~ ✅
- Implemented full replay protection with sliding window
- Added sequence number tracking
- Created serialization/deserialization
- 6 comprehensive tests added
- Commit: 4241e74

### 3. ~~Complete Battery Optimization Implementation~~ ✅
- Implemented message queuing
- Added threshold and time-based auto-flush
- Batch processing to minimize CPU wake-ups
- 7 comprehensive tests added
- Commit: 2fcb075

## 🔴 High Priority Tasks (Remaining)

### 1. FFI Boundary Tests
**File to create**: `tests/ffi_tests.rs`

Required test cases:
```rust
// Double-free protection
#[test]
fn test_double_free_protection() {
    unsafe {
        let mut error = 0;
        let session = noise_session_new(0, &mut error);
        noise_session_free(session);
        noise_session_free(session); // Should not crash
    }
}

// Null pointer handling for every function
#[test]
fn test_null_session_handling() {
    unsafe {
        let mut len = 100;
        let mut buffer = vec![0u8; 100];
        
        // Should return error, not crash
        let result = noise_encrypt(
            std::ptr::null_mut(),
            std::ptr::null(),
            0,
            buffer.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_INVALID_PARAMETER);
    }
}

// Buffer overflow protection
#[test]
fn test_buffer_overflow_protection() {
    unsafe {
        let mut error = 0;
        let session = noise_session_new(0, &mut error);
        
        let mut small_buffer = [0u8; 10];
        let mut len = 10;
        
        // Should fail with buffer too small
        let result = noise_write_message(
            session,
            std::ptr::null(),
            0,
            small_buffer.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_BUFFER_TOO_SMALL);
        assert!(len > 10); // Should indicate required size
        
        noise_session_free(session);
    }
}

// Add tests for:
// - Invalid enum values
// - Concurrent session creation
// - Very large buffer sizes
// - Zero-length buffers
// - Misaligned pointers
```

### 2. Integration Tests
**File to create**: `tests/integration_tests.rs`

Required test scenarios:
```rust
// Full handshake via FFI
#[test]
fn test_ffi_handshake_complete() {
    unsafe {
        let mut error = 0;
        let initiator = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        let responder = noise_session_new(NOISE_MODE_RESPONDER, &mut error);
        
        // Perform complete handshake
        // Verify both reach transport mode
        // Test encryption/decryption works
        
        noise_session_free(initiator);
        noise_session_free(responder);
    }
}

// Session persistence
#[test]
fn test_session_persistence() {
    // Create session
    // Perform partial handshake
    // Serialize state
    // Deserialize into new session
    // Complete handshake
    // Verify encryption works
}

// Network failure simulation
#[test]
fn test_network_interruption() {
    // Start handshake
    // Simulate packet loss
    // Retry with resilient session
    // Verify recovery
}
```

### 3. Security Test Suite
**File to create**: `tests/security_tests.rs`

Required security tests:
```rust
// Replay attack prevention
#[test]
fn test_replay_attack_blocked() {
    // Use ResilientSession
    // Capture encrypted message
    // Try to replay it
    // Verify rejection
}

// MITM detection
#[test]
fn test_mitm_detection() {
    // Create three sessions (Alice, Bob, Mallory)
    // Attempt MITM attack
    // Verify detection via key mismatch
}

// Timing attack resistance
#[test]
fn test_constant_time_operations() {
    // Measure decryption time for valid vs invalid tags
    // Statistical analysis to ensure constant time
}

// Malformed input handling
#[test]
fn test_malformed_message_handling() {
    // Send truncated messages
    // Send oversized messages
    // Send random data
    // Verify graceful failure
}
```

### 4. Platform Build Scripts

**File to create**: `build-ios.sh`
```bash
#!/bin/bash
set -e

echo "Building noise-mobile-rust for iOS..."

# Build for iOS device (arm64)
cargo build --target aarch64-apple-ios --release

# Build for iOS simulator (x86_64)
cargo build --target x86_64-apple-ios --release

# Build for iOS simulator (arm64 - M1 Macs)
cargo build --target aarch64-apple-ios-sim --release

# Create universal library
mkdir -p target/universal/release

# Device library
lipo -create \
    target/aarch64-apple-ios/release/libnoise_mobile.a \
    -output target/universal/release/libnoise_mobile_device.a

# Simulator library
lipo -create \
    target/x86_64-apple-ios/release/libnoise_mobile.a \
    target/aarch64-apple-ios-sim/release/libnoise_mobile.a \
    -output target/universal/release/libnoise_mobile_sim.a

# Create XCFramework
xcodebuild -create-xcframework \
    -library target/universal/release/libnoise_mobile_device.a \
    -library target/universal/release/libnoise_mobile_sim.a \
    -output target/NoiseM obile.xcframework

# Generate header
cbindgen --config cbindgen.toml --crate noise-mobile-rust --output include/noise_mobile.h

echo "iOS build complete!"
```

**File to create**: `build-android.sh`
```bash
#!/bin/bash
set -e

echo "Building noise-mobile-rust for Android..."

# Set up Android NDK paths
export ANDROID_NDK_HOME=${ANDROID_NDK_HOME:-$HOME/Android/Sdk/ndk/25.2.9519653}

# Build for all Android architectures
cargo ndk -t armeabi-v7a -t arm64-v8a -t x86 -t x86_64 \
    -o ./target/android-libs build --release

# Copy to jniLibs structure
mkdir -p android/app/src/main/jniLibs/{armeabi-v7a,arm64-v8a,x86,x86_64}

cp target/android-libs/armeabi-v7a/libnoise_mobile.so android/app/src/main/jniLibs/armeabi-v7a/
cp target/android-libs/arm64-v8a/libnoise_mobile.so android/app/src/main/jniLibs/arm64-v8a/
cp target/android-libs/x86/libnoise_mobile.so android/app/src/main/jniLibs/x86/
cp target/android-libs/x86_64/libnoise_mobile.so android/app/src/main/jniLibs/x86_64/

echo "Android build complete!"
```

**File to create**: `cbindgen.toml`
```toml
language = "C"
header = "/* Generated by cbindgen - DO NOT EDIT */"
include_guard = "NOISE_MOBILE_H"
autogen_warning = "/* Warning: This file is auto-generated. Do not modify. */"
include_version = true
namespace = "noise"
namespaces = []

[export]
include = ["NoiseError", "NoiseMode"]
exclude = []
prefix = "NOISE_"

[fn]
prefix = "noise_"
postfix = ""

[struct]
prefix = "Noise"
postfix = ""

[enum]
prefix = "NOISE_"
```

## 🟡 Medium Priority Tasks

### 5. iOS Integration Example
**Directory to create**: `examples/ios/`

Files needed:
- `NoiseSession.swift` - High-level Swift wrapper
- `NoiseMobile-Bridging-Header.h` - Bridge to C API
- `BLEExample.swift` - BLE integration demo
- `README.md` - Setup and usage instructions
- `Podfile` - CocoaPods configuration

### 6. Android Integration Example
**Directory to create**: `examples/android/`

Files needed:
- `NoiseSession.kt` - Kotlin wrapper
- `NoiseJNI.c` - JNI bridge implementation
- `BLEActivity.kt` - BLE integration demo
- `build.gradle` - Gradle configuration
- `README.md` - Setup instructions

### 7. Performance Benchmarks
**File to create**: `benches/noise_benchmarks.rs`

Using Criterion:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_handshake(c: &mut Criterion) {
    c.bench_function("full_xx_handshake", |b| {
        b.iter(|| {
            // Measure complete handshake time
        });
    });
}

fn benchmark_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption");
    
    for size in &[64, 1024, 8192, 65536] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, &size| {
                // Setup connected session
                let data = vec![0u8; size];
                b.iter(|| {
                    session.encrypt(&data)
                });
            },
        );
    }
    group.finish();
}

fn benchmark_batch_vs_individual(c: &mut Criterion) {
    // Compare BatchedCrypto vs individual operations
}

criterion_group!(benches, benchmark_handshake, benchmark_encryption, benchmark_batch_vs_individual);
criterion_main!(benches);
```

## 🟢 Low Priority Tasks

### 8. CI/CD Setup
**File to create**: `.github/workflows/ci.yml`

### 9. Platform-Specific Key Storage
- iOS Keychain implementation
- Android Keystore implementation

### 10. Advanced Features
- Post-quantum readiness
- Hardware acceleration
- Custom transports

## Implementation Priority Order

1. **FFI Tests First** - Critical for safety
2. **Integration Tests** - Verify end-to-end functionality
3. **Build Scripts** - Enable platform testing
4. **Security Tests** - Ensure protocol security
5. **Examples** - Help users integrate
6. **Benchmarks** - Optimize performance
7. **Documentation** - Polish for release

## Notes for Implementation

### FFI Testing Tips
- Use `valgrind` for memory leak detection
- Test with `RUST_BACKTRACE=1` for debugging
- Consider using `cargo-fuzz` for fuzzing

### Build Script Requirements
- Install `cargo-ndk` for Android: `cargo install cargo-ndk`
- Install `cbindgen`: `cargo install cbindgen`
- Ensure Xcode and Android NDK are installed

### Documentation Needs
- Add examples to every public function
- Create architecture diagrams
- Document threat model
- Add performance tuning guide

## Success Metrics

Each completed task should:
- ✅ Have comprehensive tests
- ✅ Handle all error cases
- ✅ Include documentation
- ✅ Pass CI checks
- ✅ Work on real devices