# Architecture

## Overview

`noise-mobile-rust` is designed as a three-layer architecture optimized for mobile FFI integration:

1. **Core Layer** - Pure Rust implementation wrapping Snow
2. **FFI Layer** - C-compatible API with mobile considerations  
3. **Platform Layer** - iOS/Android specific optimizations

## Design Principles

### 1. FFI-First Design

Every public API is designed to be FFI-safe from the start:

```rust
// Instead of this:
pub fn create_session(config: Config) -> Result<Session, Error>

// We have this:
#[no_mangle]
pub extern "C" fn noise_session_new(
    mode: c_int, 
    error: *mut c_int
) -> *mut NoiseSession
```

### 2. Opaque Pointers

Complex Rust types are hidden behind opaque pointers:

```rust
// Rust side
pub struct NoiseSession {
    state: snow::HandshakeState,
    buffer: Vec<u8>,
    mode: SessionMode,
}

// C side sees only
typedef struct NoiseSession NoiseSession;
```

### 3. Error Codes, Not Exceptions

FFI doesn't support exceptions, so we use error codes:

```rust
#[repr(C)]
pub enum NoiseError {
    Success = 0,
    InvalidParameter = 1,
    OutOfMemory = 2,
    HandshakeFailed = 3,
    EncryptionFailed = 4,
    // ... etc
}
```

### 4. Explicit Memory Management

```rust
#[no_mangle]
pub extern "C" fn noise_session_free(session: *mut NoiseSession) {
    if !session.is_null() {
        unsafe { Box::from_raw(session); }
    }
}
```

## Core Architecture

### Session Lifecycle

```
┌─────────────┐
│   Created   │ ──── noise_session_new()
└──────┬──────┘
       │
┌──────▼──────┐
│ Handshaking │ ──── noise_write_message()
│             │ ──── noise_read_message()
└──────┬──────┘
       │
┌──────▼──────┐
│  Transport  │ ──── noise_encrypt()
│    Mode     │ ──── noise_decrypt()
└──────┬──────┘
       │
┌──────▼──────┐
│  Destroyed  │ ──── noise_session_free()
└─────────────┘
```

### Memory Layout

```rust
// Mobile-optimized memory layout
pub struct NoiseSession {
    // Hot data (frequently accessed)
    state: TransportState,    // 32 bytes aligned
    
    // Warm data (handshake phase)
    handshake_hash: [u8; 32], // 32 bytes aligned
    
    // Cold data (rarely accessed)  
    config: SessionConfig,     // Variable size
    
    // Buffers (page-aligned for DMA)
    encrypt_buffer: AlignedBuffer<4096>,
    decrypt_buffer: AlignedBuffer<4096>,
}
```

### Thread Safety

The library is designed for single-threaded use per session, but multiple sessions can run concurrently:

```rust
// Each session is Send but not Sync
unsafe impl Send for NoiseSession {}
// impl !Sync for NoiseSession
```

For thread-safe usage:
```rust
pub struct ThreadSafeSession {
    inner: Arc<Mutex<NoiseSession>>,
}
```

## FFI API Design

### Naming Convention

All FFI functions follow the pattern: `noise_<object>_<action>`

```c
// Creation/Destruction
NoiseSession* noise_session_new(int mode, int* error);
void noise_session_free(NoiseSession* session);

// Handshake Operations  
int noise_write_message(
    NoiseSession* session,
    const uint8_t* payload, size_t payload_len,
    uint8_t* output, size_t* output_len
);

// Transport Operations
int noise_encrypt(
    NoiseSession* session,
    const uint8_t* plaintext, size_t plaintext_len,
    uint8_t* ciphertext, size_t* ciphertext_len  
);
```

### Buffer Management

Two strategies for buffer management:

1. **Caller-Allocated** (Preferred for mobile)
```c
uint8_t buffer[NOISE_MAX_MESSAGE_LEN];
size_t len = sizeof(buffer);
int err = noise_encrypt(session, input, input_len, buffer, &len);
```

2. **Library-Allocated** (For convenience)
```c
uint8_t* output;
size_t output_len;
int err = noise_encrypt_alloc(session, input, input_len, &output, &output_len);
// Caller must call noise_buffer_free(output)
```

### Error Handling

Consistent error handling pattern:

```rust
#[no_mangle]
pub extern "C" fn noise_encrypt(
    session: *mut NoiseSession,
    plaintext: *const u8,
    plaintext_len: usize,
    ciphertext: *mut u8,
    ciphertext_len: *mut usize,
) -> c_int {
    // Null checks
    if session.is_null() || plaintext.is_null() || ciphertext.is_null() {
        return NoiseError::InvalidParameter as c_int;
    }
    
    // Convert to safe types
    let session = unsafe { &mut *session };
    let plaintext = unsafe { slice::from_raw_parts(plaintext, plaintext_len) };
    
    // Perform operation
    match session.encrypt(plaintext) {
        Ok(encrypted) => {
            // Copy to output buffer
            unsafe {
                ptr::copy_nonoverlapping(
                    encrypted.as_ptr(),
                    ciphertext,
                    encrypted.len()
                );
                *ciphertext_len = encrypted.len();
            }
            NoiseError::Success as c_int
        }
        Err(_) => NoiseError::EncryptionFailed as c_int,
    }
}
```

## Mobile Optimizations

### 1. Battery Efficiency

```rust
// Batch operations to reduce wake-ups
pub struct BatchedSession {
    session: NoiseSession,
    pending: Vec<Message>,
    flush_threshold: usize,
}

impl BatchedSession {
    pub fn encrypt_batch(&mut self, messages: &[&[u8]]) -> Result<Vec<Vec<u8>>, Error> {
        // Single crypto operation for multiple messages
        // Reduces CPU wake-ups on mobile
    }
}
```

### 2. Background Task Support

```rust
// Save/restore session state for iOS/Android background limits
impl NoiseSession {
    pub fn serialize(&self) -> Vec<u8> {
        // Serialize session state
    }
    
    pub fn deserialize(data: &[u8]) -> Result<Self, Error> {
        // Restore session state
    }
}
```

### 3. Network Resilience

```rust
pub struct ResilientSession {
    inner: NoiseSession,
    message_counter: u64,
    replay_window: BitVec,
}

impl ResilientSession {
    // Handle out-of-order messages
    pub fn decrypt_with_replay_protection(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, Error> {
        // Check replay window
        // Decrypt if valid
        // Update window
    }
}
```

### 4. Memory Pressure Handling

```rust
// Respond to mobile memory warnings
impl NoiseSession {
    pub fn reduce_memory_usage(&mut self) {
        self.encrypt_buffer.shrink_to_fit();
        self.decrypt_buffer.shrink_to_fit();
        // Clear any caches
    }
}
```

## Platform Integration

### iOS Specifics

```rust
#[cfg(target_os = "ios")]
mod ios {
    // Keychain integration
    pub fn store_key_in_keychain(key: &[u8], identifier: &str) -> Result<(), Error> {
        // Use Security framework
    }
    
    // Background task integration
    pub fn register_background_task() -> BackgroundTaskId {
        // Register with UIKit
    }
}
```

### Android Specifics

```rust
#[cfg(target_os = "android")]
mod android {
    // Keystore integration
    pub fn store_key_in_keystore(key: &[u8], alias: &str) -> Result<(), Error> {
        // Use Android Keystore
    }
    
    // Doze mode handling
    pub fn is_doze_mode_active() -> bool {
        // Check with PowerManager
    }
}
```

## Performance Considerations

### 1. Zero-Copy Operations

Where possible, avoid copying data:

```rust
// Bad: Copies data multiple times
pub fn encrypt(data: Vec<u8>) -> Vec<u8>

// Good: In-place encryption
pub fn encrypt_in_place(data: &mut [u8]) -> Result<usize, Error>
```

### 2. Cache-Friendly Data Layout

```rust
#[repr(C, align(64))] // Cache line aligned
pub struct NoiseSession {
    // Group frequently accessed fields
    hot_data: HotData,
    
    // Separate cold data
    cold_data: ColdData,
}
```

### 3. Vectorization Opportunities

```rust
// Enable SIMD where beneficial
#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

pub fn xor_blocks(a: &mut [u8], b: &[u8]) {
    // Use NEON instructions on ARM64
}
```

## Testing Strategy

### 1. Unit Tests
- Test each layer independently
- Mock the Snow library for core tests
- Test FFI boundary conditions

### 2. Integration Tests
- Full handshake scenarios
- Cross-platform communication
- Error injection tests

### 3. Device Tests
- Real BLE communication
- Battery usage profiling
- Memory pressure scenarios
- Background/foreground transitions

### 4. Performance Tests
- Benchmark each operation
- Profile on real devices
- Compare with native implementations

## Security Architecture

### 1. Key Management

```rust
pub trait KeyStorage: Send + Sync {
    fn store_identity(&self, key: &[u8]) -> Result<KeyId, Error>;
    fn load_identity(&self, id: KeyId) -> Result<Vec<u8>, Error>;
    fn delete_identity(&self, id: KeyId) -> Result<(), Error>;
}

// Platform-specific implementations
struct KeychainStorage;  // iOS
struct KeystoreStorage;  // Android
```

### 2. Zeroization

```rust
use zeroize::Zeroize;

impl Drop for NoiseSession {
    fn drop(&mut self) {
        self.keys.zeroize();
        self.handshake_hash.zeroize();
    }
}
```

### 3. Side-Channel Resistance

- Constant-time comparisons
- No branching on secret data
- Careful cache access patterns

## Future Considerations

### 1. Post-Quantum Readiness
Structure allows for future PQ algorithm integration:
```rust
pub enum KeyExchangeAlgorithm {
    X25519,
    X448,
    #[cfg(feature = "post-quantum")]
    Kyber1024,
}
```

### 2. Hardware Acceleration
Support for platform crypto accelerators:
```rust
#[cfg(feature = "hardware-crypto")]
mod hw_crypto {
    // iOS: CryptoKit
    // Android: Hardware-backed Keystore
}
```

### 3. Multi-Transport Support
Beyond BLE to WiFi Direct, etc:
```rust
pub trait Transport {
    fn send(&mut self, data: &[u8]) -> Result<(), Error>;
    fn recv(&mut self, buffer: &mut [u8]) -> Result<usize, Error>;
}
```