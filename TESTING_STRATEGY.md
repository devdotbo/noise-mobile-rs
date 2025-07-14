# Testing Strategy

## Overview

Comprehensive testing is crucial for a cryptographic library. This document outlines our multi-layered testing approach, from unit tests to real device validation.

## Testing Principles

1. **No Mocks for Crypto**: Real cryptographic operations only
2. **FFI Boundary Focus**: Most bugs occur at language boundaries  
3. **Real Device Priority**: Simulators hide real issues
4. **Performance Regression**: Track metrics over time
5. **Security First**: Test attack scenarios explicitly

## Test Categories

### 1. Unit Tests (Rust)

Location: `src/` (inline) and `tests/unit/`

#### Core Functionality
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_handshake_state_machine() {
        let mut initiator = NoiseSession::new_initiator().unwrap();
        let mut responder = NoiseSession::new_responder().unwrap();
        
        // State should start in handshake
        assert!(initiator.is_handshake_state());
        assert!(responder.is_handshake_state());
        
        // Complete handshake
        let msg1 = initiator.write_message(&[]).unwrap();
        assert_eq!(msg1.len(), 32); // e
        
        responder.read_message(&msg1).unwrap();
        let msg2 = responder.write_message(&[]).unwrap();
        assert_eq!(msg2.len(), 96); // e, ee, s, es
        
        initiator.read_message(&msg2).unwrap();
        let msg3 = initiator.write_message(&[]).unwrap();
        assert_eq!(msg3.len(), 64); // s, se
        
        responder.read_message(&msg3).unwrap();
        
        // Both should now be in transport state
        assert!(initiator.is_transport_state());
        assert!(responder.is_transport_state());
    }
    
    #[test]
    fn test_encryption_decryption() {
        let (mut alice, mut bob) = create_connected_pair();
        
        let plaintext = b"Hello, Bob!";
        let ciphertext = alice.encrypt(plaintext).unwrap();
        
        // Ciphertext should be plaintext + 16 bytes tag
        assert_eq!(ciphertext.len(), plaintext.len() + 16);
        
        let decrypted = bob.decrypt(&ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }
    
    #[test]
    fn test_wrong_key_fails() {
        let (mut alice, _) = create_connected_pair();
        let (_, mut charlie) = create_connected_pair();
        
        let ciphertext = alice.encrypt(b"secret").unwrap();
        
        // Charlie shouldn't be able to decrypt Alice's message
        assert!(charlie.decrypt(&ciphertext).is_err());
    }
}
```

#### Property-Based Testing
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encrypt_decrypt_roundtrip(data: Vec<u8>) {
        let (mut alice, mut bob) = create_connected_pair();
        
        let ciphertext = alice.encrypt(&data).unwrap();
        let plaintext = bob.decrypt(&ciphertext).unwrap();
        
        prop_assert_eq!(data, plaintext);
    }
    
    #[test]
    fn test_ciphertext_malleability(
        data: Vec<u8>,
        corrupt_pos: usize,
        corrupt_byte: u8
    ) {
        let (mut alice, mut bob) = create_connected_pair();
        
        let mut ciphertext = alice.encrypt(&data).unwrap();
        
        if !ciphertext.is_empty() {
            let pos = corrupt_pos % ciphertext.len();
            ciphertext[pos] ^= corrupt_byte;
            
            // Any modification should cause decryption to fail
            prop_assert!(bob.decrypt(&ciphertext).is_err());
        }
    }
}
```

### 2. FFI Boundary Tests

Location: `tests/ffi/`

#### Memory Safety
```rust
#[test]
fn test_null_pointer_handling() {
    unsafe {
        // All functions should handle null gracefully
        assert_eq!(
            noise_session_free(std::ptr::null_mut()),
            NoiseErrorCode::Success as c_int
        );
        
        let mut error = 0;
        assert!(noise_write_message(
            std::ptr::null_mut(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ) != NoiseErrorCode::Success as c_int);
    }
}

#[test]
fn test_double_free_safety() {
    unsafe {
        let mut error = 0;
        let session = noise_session_new(0, &mut error);
        assert!(!session.is_null());
        
        noise_session_free(session);
        // Second free should be safe (no-op)
        noise_session_free(session);
    }
}

#[test]
fn test_buffer_overflow_protection() {
    unsafe {
        let mut error = 0;
        let session = noise_session_new(0, &mut error);
        
        // Provide buffer too small
        let mut small_buffer = [0u8; 10];
        let mut len = small_buffer.len();
        
        let result = noise_write_message(
            session,
            std::ptr::null(),
            0,
            small_buffer.as_mut_ptr(),
            &mut len
        );
        
        assert_eq!(result, NoiseErrorCode::BufferTooSmall as c_int);
        // Should have updated len to required size
        assert!(len > 10);
        
        noise_session_free(session);
    }
}
```

#### Thread Safety
```rust
use std::thread;
use std::sync::Arc;

#[test]
fn test_concurrent_sessions() {
    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                unsafe {
                    let mut error = 0;
                    let session = noise_session_new(i % 2, &mut error);
                    assert_eq!(error, 0);
                    
                    // Perform some operations
                    let mut buffer = vec![0u8; 1024];
                    let mut len = buffer.len();
                    
                    noise_write_message(
                        session,
                        std::ptr::null(),
                        0,
                        buffer.as_mut_ptr(),
                        &mut len
                    );
                    
                    noise_session_free(session);
                }
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
}
```

### 3. Integration Tests

Location: `tests/integration/`

#### Cross-Language Communication
```rust
#[test]
fn test_c_to_rust_handshake() {
    // Create Rust responder
    let mut responder = NoiseSession::new_responder().unwrap();
    
    // Create C initiator
    unsafe {
        let mut error = 0;
        let c_session = noise_session_new(0, &mut error);
        
        // Get first message from C
        let mut buffer = vec![0u8; 1024];
        let mut len = buffer.len();
        noise_write_message(
            c_session,
            std::ptr::null(),
            0,
            buffer.as_mut_ptr(),
            &mut len
        );
        
        // Process in Rust
        responder.read_message(&buffer[..len]).unwrap();
        
        // Continue handshake...
        
        noise_session_free(c_session);
    }
}
```

#### Platform-Specific Tests
```rust
#[cfg(target_os = "ios")]
#[test]
fn test_ios_keychain_integration() {
    use crate::mobile::ios::KeychainStorage;
    
    let storage = KeychainStorage::new();
    let key = vec![0u8; 32];
    
    storage.store_identity(&key, "test_key").unwrap();
    let loaded = storage.load_identity("test_key").unwrap();
    
    assert_eq!(key, loaded);
    
    storage.delete_identity("test_key").unwrap();
}

#[cfg(target_os = "android")]
#[test]
fn test_android_keystore_integration() {
    use crate::mobile::android::KeystoreStorage;
    
    let storage = KeystoreStorage::new();
    let key = vec![0u8; 32];
    
    storage.store_identity(&key, "test_alias").unwrap();
    let loaded = storage.load_identity("test_alias").unwrap();
    
    assert_eq!(key, loaded);
}
```

### 4. Performance Tests

Location: `benches/`

#### Benchmarks
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_handshake_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("handshake");
    
    for pattern in &["XX", "IK", "NK"] {
        group.bench_with_input(
            BenchmarkId::new("pattern", pattern),
            pattern,
            |b, &pattern| {
                b.iter(|| {
                    let params = format!("Noise_{}_25519_ChaChaPoly_BLAKE2s", pattern);
                    perform_handshake(&params)
                });
            }
        );
    }
    
    group.finish();
}

fn bench_encryption_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption");
    let (mut alice, mut bob) = create_connected_pair();
    
    for size in &[64, 1024, 8192, 65536] {
        let data = vec![0u8; *size];
        
        group.bench_with_input(
            BenchmarkId::new("encrypt", size),
            &data,
            |b, data| {
                b.iter(|| {
                    black_box(alice.encrypt(data).unwrap())
                });
            }
        );
    }
    
    group.finish();
}

fn bench_mobile_scenarios(c: &mut Criterion) {
    c.bench_function("message_burst", |b| {
        let (mut alice, mut bob) = create_connected_pair();
        let messages: Vec<_> = (0..100).map(|i| vec![i as u8; 256]).collect();
        
        b.iter(|| {
            for msg in &messages {
                let ct = alice.encrypt(msg).unwrap();
                black_box(bob.decrypt(&ct).unwrap());
            }
        });
    });
}

criterion_group!(benches, bench_handshake_sizes, bench_encryption_sizes, bench_mobile_scenarios);
criterion_main!(benches);
```

### 5. Real Device Tests

Location: `tests/device/`

#### iOS Device Tests
```swift
// tests/device/ios/DeviceTests.swift
import XCTest

class NoiseDeviceTests: XCTestCase {
    func testBLEHandshake() {
        // Setup BLE central and peripheral
        let central = BLECentral()
        let peripheral = BLEPeripheral()
        
        // Create Noise sessions
        let initiator = try! NoiseSession(mode: .initiator)
        let responder = try! NoiseSession(mode: .responder)
        
        // Perform handshake over BLE
        let expectation = XCTestExpectation(description: "BLE handshake")
        
        peripheral.onReceive = { data in
            let response = try! responder.readMessage(data)
            peripheral.send(response)
        }
        
        central.onReceive = { data in
            if initiator.isHandshakeComplete {
                expectation.fulfill()
            } else {
                let response = try! initiator.readMessage(data)
                if let response = response {
                    central.send(response)
                }
            }
        }
        
        // Start handshake
        let firstMsg = try! initiator.writeMessage(Data())
        central.send(firstMsg)
        
        wait(for: [expectation], timeout: 10.0)
    }
    
    func testBatteryImpact() {
        measure(metrics: [XCTCPUMetric(), XCTMemoryMetric()]) {
            // Perform 1000 encryptions
            let (alice, bob) = createConnectedPair()
            let data = Data(repeating: 0, count: 1024)
            
            for _ in 0..<1000 {
                let encrypted = try! alice.encrypt(data)
                let _ = try! bob.decrypt(encrypted)
            }
        }
    }
}
```

#### Android Device Tests
```kotlin
// tests/device/android/DeviceTests.kt
@RunWith(AndroidJUnit4::class)
class NoiseDeviceTests {
    @Test
    fun testBLEHandshake() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        
        // Setup BLE
        val bleManager = BleManager(context)
        
        // Create sessions
        val initiator = NoiseSession(NoiseMode.INITIATOR)
        val responder = NoiseSession(NoiseMode.RESPONDER)
        
        // Perform handshake
        val latch = CountDownLatch(1)
        
        bleManager.startAdvertising { data ->
            val response = responder.readMessage(data)
            bleManager.sendResponse(response)
        }
        
        bleManager.startScanning { data ->
            if (initiator.isHandshakeComplete) {
                latch.countDown()
            } else {
                val response = initiator.readMessage(data)
                response?.let { bleManager.sendData(it) }
            }
        }
        
        // Start handshake
        val firstMsg = initiator.writeMessage(ByteArray(0))
        bleManager.sendData(firstMsg)
        
        assertTrue(latch.await(10, TimeUnit.SECONDS))
    }
    
    @Test
    fun testMemoryPressure() {
        val runtime = Runtime.getRuntime()
        val startMemory = runtime.totalMemory() - runtime.freeMemory()
        
        // Create many sessions
        val sessions = mutableListOf<NoiseSession>()
        for (i in 0..100) {
            sessions.add(NoiseSession(NoiseMode.INITIATOR))
        }
        
        val peakMemory = runtime.totalMemory() - runtime.freeMemory()
        val memoryPerSession = (peakMemory - startMemory) / 100
        
        // Should be less than 10KB per session
        assertTrue(memoryPerSession < 10 * 1024)
    }
}
```

### 6. Security Tests

Location: `tests/security/`

#### Attack Scenarios
```rust
#[test]
fn test_replay_attack_prevention() {
    let (mut alice, mut bob) = create_connected_pair();
    
    let plaintext = b"transfer $1000";
    let ciphertext = alice.encrypt(plaintext).unwrap();
    
    // First decryption should succeed
    assert_eq!(bob.decrypt(&ciphertext).unwrap(), plaintext);
    
    // Replay should fail
    assert!(bob.decrypt(&ciphertext).is_err());
}

#[test]
fn test_mitm_detection() {
    let mut alice = NoiseSession::new_initiator().unwrap();
    let mut bob = NoiseSession::new_responder().unwrap();
    let mut mallory = NoiseSession::new_responder().unwrap();
    
    // Alice sends to Mallory (thinking it's Bob)
    let msg1 = alice.write_message(&[]).unwrap();
    mallory.read_message(&msg1).unwrap();
    
    // Mallory forwards to Bob
    let msg1_forward = msg1.clone(); // In reality, Mallory would create new
    bob.read_message(&msg1_forward).unwrap();
    
    // Complete handshakes
    // ...
    
    // Verify Alice and Bob have different session keys
    let alice_test = alice.encrypt(b"test").unwrap();
    assert!(bob.decrypt(&alice_test).is_err());
}

#[test]
fn test_timing_attack_resistance() {
    use std::time::Instant;
    
    let (mut alice, mut bob) = create_connected_pair();
    let valid_ciphertext = alice.encrypt(b"secret").unwrap();
    
    // Create invalid ciphertext by corrupting tag
    let mut invalid_ciphertext = valid_ciphertext.clone();
    let len = invalid_ciphertext.len();
    invalid_ciphertext[len - 1] ^= 1;
    
    // Measure timing
    let mut valid_times = Vec::new();
    let mut invalid_times = Vec::new();
    
    for _ in 0..1000 {
        let start = Instant::now();
        let _ = bob.decrypt(&valid_ciphertext);
        valid_times.push(start.elapsed());
        
        let start = Instant::now();
        let _ = bob.decrypt(&invalid_ciphertext);
        invalid_times.push(start.elapsed());
    }
    
    // Statistical analysis to ensure constant time
    let valid_avg = valid_times.iter().sum::<Duration>() / valid_times.len() as u32;
    let invalid_avg = invalid_times.iter().sum::<Duration>() / invalid_times.len() as u32;
    
    // Difference should be negligible (< 5%)
    let diff = (valid_avg.as_nanos() as f64 - invalid_avg.as_nanos() as f64).abs();
    let avg = (valid_avg.as_nanos() + invalid_avg.as_nanos()) as f64 / 2.0;
    assert!(diff / avg < 0.05);
}
```

## Testing Infrastructure

### Continuous Integration
```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, beta, nightly]
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: ${{ matrix.rust }}
      - run: cargo test --all-features
      
  ffi-tests:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v2
      - run: cargo test --features ffi-tests
      - run: |
          cd tests/c
          make test
          
  security-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - run: |
          cargo install cargo-fuzz
          cargo fuzz run handshake -- -runs=10000
      - run: |
          cargo install cargo-audit
          cargo audit
```

### Device Testing Farm
```yaml
# .github/workflows/device-tests.yml
name: Device Tests

on:
  schedule:
    - cron: '0 0 * * *' # Daily

jobs:
  ios-tests:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v2
      - name: Run iOS Tests
        run: |
          xcodebuild test \
            -project tests/device/ios/NoiseTests.xcodeproj \
            -scheme NoiseTests \
            -destination 'platform=iOS,name=iPhone 12'
            
  android-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Run Android Tests
        run: |
          ./gradlew connectedAndroidTest
```

## Test Coverage Goals

- Unit Tests: > 90% coverage
- FFI Boundary: 100% coverage
- Integration: All major scenarios
- Performance: No regressions
- Security: All known attacks

## Test Execution

### Local Development
```bash
# Run all tests
cargo test

# Run specific test category
cargo test --test unit
cargo test --test ffi
cargo test --test integration

# Run with coverage
cargo tarpaulin --out Html

# Run benchmarks
cargo bench

# Run fuzzing
cargo fuzz run handshake
```

### Pre-commit Hook
```bash
#!/bin/bash
# .git/hooks/pre-commit

# Run fast tests
cargo test --lib
if [ $? -ne 0 ]; then
    echo "Tests failed. Commit aborted."
    exit 1
fi

# Check formatting
cargo fmt --check
if [ $? -ne 0 ]; then
    echo "Code not formatted. Run 'cargo fmt'"
    exit 1
fi
```

## Success Metrics

1. **Correctness**: All tests pass consistently
2. **Performance**: Benchmarks meet targets
3. **Security**: No vulnerabilities found
4. **Coverage**: > 90% line coverage
5. **Device**: Works on real iOS/Android devices