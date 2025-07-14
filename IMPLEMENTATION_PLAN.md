# Implementation Plan

## Overview

This document provides a step-by-step implementation plan for `noise-mobile-rust`. Each phase builds on the previous one, with clear milestones and testing criteria.

## Phase 1: Core Foundation (Days 1-3)

### Step 1.1: Project Setup
```bash
# Initialize library
cargo init --lib

# Add dependencies to Cargo.toml
[dependencies]
snow = { path = "../snow" }
zeroize = "1.7"
thiserror = "1.0"

[dev-dependencies]
proptest = "1.0"
criterion = "0.5"
```

### Step 1.2: Core Types
```rust
// src/core/mod.rs
pub mod error;
pub mod session;
pub mod crypto;

// src/core/error.rs
#[derive(Debug, thiserror::Error)]
pub enum NoiseError {
    #[error("Invalid parameter")]
    InvalidParameter,
    #[error("Handshake failed")]
    HandshakeFailed,
    // etc...
}

// src/core/session.rs
pub struct NoiseSession {
    state: NoiseState,
    buffer: Vec<u8>,
}

enum NoiseState {
    Handshake(snow::HandshakeState),
    Transport(snow::TransportState),
}
```

### Step 1.3: Basic Noise Wrapper
```rust
impl NoiseSession {
    pub fn new_initiator() -> Result<Self, NoiseError> {
        let params = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
        let builder = snow::Builder::new(params.parse()?);
        let keypair = builder.generate_keypair()?;
        let state = builder
            .local_private_key(&keypair.private)
            .build_initiator()?;
        
        Ok(NoiseSession {
            state: NoiseState::Handshake(state),
            buffer: vec![0u8; 65535],
        })
    }
    
    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>, NoiseError> {
        match &mut self.state {
            NoiseState::Handshake(state) => {
                let len = state.write_message(payload, &mut self.buffer)?;
                Ok(self.buffer[..len].to_vec())
            }
            NoiseState::Transport(_) => Err(NoiseError::InvalidState),
        }
    }
}
```

### Testing Milestone 1
- [ ] Basic handshake works between initiator and responder
- [ ] Error cases handled properly
- [ ] No memory leaks (test with valgrind)

## Phase 2: FFI Layer (Days 4-6)

### Step 2.1: FFI Types
```rust
// src/ffi/types.rs
#[repr(C)]
pub enum NoiseErrorCode {
    Success = 0,
    InvalidParameter = 1,
    OutOfMemory = 2,
    HandshakeFailed = 3,
    EncryptionFailed = 4,
}

#[repr(C)]
pub enum NoiseMode {
    Initiator = 0,
    Responder = 1,
}

// Opaque pointer type
pub struct NoiseSessionFFI {
    _private: [u8; 0],
}
```

### Step 2.2: C API Implementation
```rust
// src/ffi/c_api.rs
use std::os::raw::{c_int, c_uchar};
use std::ptr;
use std::slice;

#[no_mangle]
pub extern "C" fn noise_session_new(
    mode: c_int,
    error: *mut c_int,
) -> *mut NoiseSessionFFI {
    if error.is_null() {
        return ptr::null_mut();
    }
    
    let session = match mode {
        0 => NoiseSession::new_initiator(),
        1 => NoiseSession::new_responder(),
        _ => {
            unsafe { *error = NoiseErrorCode::InvalidParameter as c_int; }
            return ptr::null_mut();
        }
    };
    
    match session {
        Ok(s) => {
            unsafe { *error = NoiseErrorCode::Success as c_int; }
            Box::into_raw(Box::new(s)) as *mut NoiseSessionFFI
        }
        Err(_) => {
            unsafe { *error = NoiseErrorCode::HandshakeFailed as c_int; }
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn noise_session_free(session: *mut NoiseSessionFFI) {
    if !session.is_null() {
        unsafe {
            let _ = Box::from_raw(session as *mut NoiseSession);
        }
    }
}
```

### Step 2.3: Buffer Management
```rust
#[no_mangle]
pub extern "C" fn noise_write_message(
    session: *mut NoiseSessionFFI,
    payload: *const c_uchar,
    payload_len: usize,
    output: *mut c_uchar,
    output_len: *mut usize,
) -> c_int {
    // Null checks
    if session.is_null() || output.is_null() || output_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &mut *(session as *mut NoiseSession) };
    let payload = if payload.is_null() || payload_len == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(payload, payload_len) }
    };
    
    match session.write_message(payload) {
        Ok(msg) => {
            let len = msg.len();
            if unsafe { *output_len } < len {
                unsafe { *output_len = len; }
                return NoiseErrorCode::BufferTooSmall as c_int;
            }
            
            unsafe {
                ptr::copy_nonoverlapping(msg.as_ptr(), output, len);
                *output_len = len;
            }
            NoiseErrorCode::Success as c_int
        }
        Err(_) => NoiseErrorCode::HandshakeFailed as c_int,
    }
}
```

### Testing Milestone 2
- [ ] C example program can complete handshake
- [ ] Memory safety verified with sanitizers
- [ ] FFI boundary tests pass
- [ ] No undefined behavior

## Phase 3: Mobile Optimizations (Days 7-9)

### Step 3.1: Key Storage Abstraction
```rust
// src/mobile/storage.rs
pub trait KeyStorage: Send + Sync {
    fn store_identity(&self, key: &[u8], id: &str) -> Result<(), NoiseError>;
    fn load_identity(&self, id: &str) -> Result<Vec<u8>, NoiseError>;
    fn delete_identity(&self, id: &str) -> Result<(), NoiseError>;
}

// Mock implementation for testing
pub struct MemoryKeyStorage {
    keys: std::sync::Mutex<HashMap<String, Vec<u8>>>,
}

#[cfg(target_os = "ios")]
pub struct KeychainStorage;

#[cfg(target_os = "android")]
pub struct KeystoreStorage;
```

### Step 3.2: Network Resilience
```rust
// src/mobile/network.rs
pub struct ResilientSession {
    inner: NoiseSession,
    last_sent: u64,
    last_received: u64,
    replay_window: BitVec,
}

impl ResilientSession {
    pub fn new(session: NoiseSession) -> Self {
        Self {
            inner: session,
            last_sent: 0,
            last_received: 0,
            replay_window: BitVec::with_capacity(64),
        }
    }
    
    pub fn encrypt_with_sequence(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, NoiseError> {
        self.last_sent += 1;
        let mut message = Vec::with_capacity(8 + plaintext.len() + 16);
        message.extend_from_slice(&self.last_sent.to_be_bytes());
        message.extend_from_slice(plaintext);
        self.inner.encrypt(&message)
    }
    
    pub fn decrypt_with_replay_check(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, NoiseError> {
        let decrypted = self.inner.decrypt(ciphertext)?;
        if decrypted.len() < 8 {
            return Err(NoiseError::InvalidMessage);
        }
        
        let sequence = u64::from_be_bytes(decrypted[..8].try_into().unwrap());
        // Check replay window
        if sequence <= self.last_received {
            return Err(NoiseError::ReplayDetected);
        }
        
        self.last_received = sequence;
        Ok(decrypted[8..].to_vec())
    }
}
```

### Step 3.3: Battery Optimization
```rust
// src/mobile/battery.rs
pub struct BatchedCrypto {
    session: NoiseSession,
    pending_encrypts: Vec<Vec<u8>>,
    pending_decrypts: Vec<Vec<u8>>,
}

impl BatchedCrypto {
    pub fn queue_encrypt(&mut self, plaintext: Vec<u8>) {
        self.pending_encrypts.push(plaintext);
    }
    
    pub fn flush_encrypts(&mut self) -> Result<Vec<Vec<u8>>, NoiseError> {
        let mut results = Vec::with_capacity(self.pending_encrypts.len());
        
        // Process all at once to minimize CPU wake-ups
        for plaintext in self.pending_encrypts.drain(..) {
            results.push(self.session.encrypt(&plaintext)?);
        }
        
        Ok(results)
    }
}
```

### Testing Milestone 3
- [ ] Key storage works on iOS simulator
- [ ] Key storage works on Android emulator
- [ ] Replay protection prevents attacks
- [ ] Batched operations improve performance

## Phase 4: Integration Examples (Days 10-12)

### Step 4.1: iOS Example
```swift
// examples/ios/NoiseMobileExample.swift
import Foundation

class NoiseExample {
    let session: OpaquePointer
    
    init() throws {
        var error: Int32 = 0
        guard let s = noise_session_new(0, &error) else {
            throw NoiseError(code: error)
        }
        self.session = s
    }
    
    deinit {
        noise_session_free(session)
    }
    
    func performHandshake() throws -> Data {
        let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: 1024)
        defer { buffer.deallocate() }
        
        var len = 1024
        let result = noise_write_message(session, nil, 0, buffer, &len)
        
        guard result == 0 else {
            throw NoiseError(code: result)
        }
        
        return Data(bytes: buffer, count: len)
    }
}
```

### Step 4.2: Android Example
```kotlin
// examples/android/NoiseExample.kt
class NoiseSession {
    private val ptr: Long
    
    init {
        System.loadLibrary("noise_mobile")
        ptr = nativeNew(MODE_INITIATOR)
        if (ptr == 0L) {
            throw RuntimeException("Failed to create session")
        }
    }
    
    protected fun finalize() {
        if (ptr != 0L) {
            nativeFree(ptr)
        }
    }
    
    fun performHandshake(): ByteArray {
        return nativeWriteMessage(ptr, ByteArray(0))
    }
    
    companion object {
        const val MODE_INITIATOR = 0
        const val MODE_RESPONDER = 1
        
        @JvmStatic
        external fun nativeNew(mode: Int): Long
        
        @JvmStatic
        external fun nativeFree(ptr: Long)
        
        @JvmStatic
        external fun nativeWriteMessage(ptr: Long, payload: ByteArray): ByteArray
    }
}
```

### Testing Milestone 4
- [ ] iOS example builds and runs
- [ ] Android example builds and runs
- [ ] Cross-platform communication works
- [ ] Real device testing successful

## Phase 5: Performance & Polish (Days 13-14)

### Step 5.1: Benchmarks
```rust
// benches/handshake.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_handshake(c: &mut Criterion) {
    c.bench_function("full_xx_handshake", |b| {
        b.iter(|| {
            let mut initiator = NoiseSession::new_initiator().unwrap();
            let mut responder = NoiseSession::new_responder().unwrap();
            
            // Message 1: initiator -> responder
            let msg1 = initiator.write_message(&[]).unwrap();
            responder.read_message(&msg1).unwrap();
            
            // Message 2: responder -> initiator
            let msg2 = responder.write_message(&[]).unwrap();
            initiator.read_message(&msg2).unwrap();
            
            // Message 3: initiator -> responder
            let msg3 = initiator.write_message(&[]).unwrap();
            responder.read_message(&msg3).unwrap();
            
            black_box((initiator, responder))
        });
    });
}

criterion_group!(benches, bench_handshake);
criterion_main!(benches);
```

### Step 5.2: Documentation
- Generate API documentation: `cargo doc --open`
- Add examples to all public functions
- Create integration guide
- Document security considerations

### Step 5.3: CI/CD Setup
```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test
      - run: cargo bench --no-run
```

### Final Testing Milestone
- [ ] All tests pass on CI
- [ ] Benchmarks meet performance targets
- [ ] Documentation complete
- [ ] Security review done

## Delivery Checklist

### Code Quality
- [ ] No compiler warnings
- [ ] No clippy warnings
- [ ] Code formatted with rustfmt
- [ ] All TODOs resolved

### Testing
- [ ] Unit test coverage > 80%
- [ ] FFI tests comprehensive
- [ ] Integration tests pass
- [ ] Benchmarks documented

### Documentation
- [ ] README complete
- [ ] API docs for all public items
- [ ] Examples for iOS and Android
- [ ] Security considerations documented

### Performance
- [ ] Handshake < 10ms
- [ ] Encryption > 10MB/s
- [ ] Memory usage < 1KB per session
- [ ] No memory leaks

## Risk Mitigation

### Technical Risks
1. **Snow API changes**: Pin to specific version
2. **FFI complexity**: Extensive testing, clear docs
3. **Platform differences**: Abstract behind traits

### Schedule Risks
1. **iOS complications**: Have fallback pure C example
2. **Android NDK issues**: Test early on real device
3. **Performance problems**: Profile early and often

## Success Criteria

The library is considered complete when:
1. Full Noise XX handshake works across FFI
2. iOS and Android examples communicate
3. Performance targets met on real devices
4. No memory safety issues
5. Comprehensive test suite passes

## Next Steps After Completion

1. **Integration with BitChat**
   - Fork BitChat repositories
   - Replace current crypto
   - Submit PRs

2. **Community Engagement**
   - Announce on relevant forums
   - Create blog post about mobile Noise
   - Engage with Jack on implementation

3. **Long-term Maintenance**
   - Set up issue templates
   - Create contribution guidelines
   - Plan for future features