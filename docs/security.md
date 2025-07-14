# Security Considerations

## Overview

This document outlines the security architecture, threat model, and best practices for `noise-mobile-rust`. As a cryptographic library handling sensitive communications, security is our highest priority.

## Threat Model

### In Scope

1. **Network Attackers**
   - Passive eavesdropping
   - Active MITM attacks
   - Message replay attacks
   - Traffic analysis

2. **Application-Level Attacks**
   - Memory disclosure
   - Side-channel attacks
   - API misuse
   - Key management errors

3. **Mobile-Specific Threats**
   - Insecure storage
   - Background state exposure
   - Memory pressure attacks
   - Platform API vulnerabilities

### Out of Scope

1. **Physical Device Access**
   - Device theft with unlocked state
   - Hardware attacks (chip decapping)
   - Cold boot attacks

2. **Platform Compromise**
   - OS-level exploits
   - Jailbroken/rooted devices
   - Malicious system libraries

3. **Implementation Bugs in Dependencies**
   - Vulnerabilities in Snow library
   - Platform crypto library bugs

## Security Architecture

### Cryptographic Primitives

We use the Noise Protocol Framework with the following configuration:

```
Noise_XX_25519_ChaChaPoly_BLAKE2s
```

- **DH**: Curve25519 (128-bit security)
- **Cipher**: ChaCha20-Poly1305 (256-bit key)
- **Hash**: BLAKE2s (256-bit output)

### Key Hierarchy

```
┌─────────────────────┐
│ Static Identity Key │ (Long-term, stored securely)
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│ Ephemeral Keys      │ (Per-session, never stored)
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│ Session Keys        │ (Derived via Noise handshake)
└─────────────────────┘
```

### Memory Security

#### Zeroization

All sensitive data is zeroized when no longer needed:

```rust
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(ZeroizeOnDrop)]
pub struct SensitiveData {
    #[zeroize(skip)] // Only skip non-sensitive fields
    pub id: u64,
    pub key: Vec<u8>,
    pub secret: [u8; 32],
}

impl Drop for NoiseSession {
    fn drop(&mut self) {
        // Explicit zeroization of all key material
        if let Some(key) = &mut self.symmetric_key {
            key.zeroize();
        }
        self.handshake_hash.zeroize();
    }
}
```

#### Memory Locking

For extremely sensitive operations:

```rust
#[cfg(unix)]
fn lock_memory(data: &[u8]) -> Result<(), Error> {
    use libc::{mlock, munlock};
    
    unsafe {
        if mlock(data.as_ptr() as *const _, data.len()) != 0 {
            return Err(Error::MemoryLockFailed);
        }
    }
    Ok(())
}
```

### Side-Channel Resistance

#### Constant-Time Operations

Critical operations must be constant-time:

```rust
// Bad: Timing leak
fn verify_tag(expected: &[u8], actual: &[u8]) -> bool {
    expected == actual  // DON'T DO THIS
}

// Good: Constant-time comparison
fn verify_tag_ct(expected: &[u8], actual: &[u8]) -> bool {
    use subtle::ConstantTimeEq;
    expected.ct_eq(actual).into()
}
```

#### Cache Attack Mitigation

```rust
// Avoid secret-dependent memory access
fn process_secret(secret: &[u8], index: usize) {
    // Bad: Table lookup based on secret
    let value = TABLE[secret[index] as usize];
    
    // Good: Process all values, select result
    let mut result = 0;
    for (i, &table_val) in TABLE.iter().enumerate() {
        let mask = constant_time_eq(i, secret[index] as usize);
        result |= mask & table_val;
    }
}
```

## Mobile-Specific Security

### iOS Security

#### Keychain Integration

```swift
// Store keys with highest security attributes
let query: [String: Any] = [
    kSecClass: kSecClassKey,
    kSecAttrApplicationTag: "com.example.noise.identity",
    kSecAttrAccessible: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
    kSecAttrSynchronizable: false,
    kSecUseDataProtectionKeychain: true,
    kSecAttrAccessControl: SecAccessControlCreateWithFlags(
        nil,
        kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
        [.privateKeyUsage, .biometryCurrentSet],
        nil
    )!,
    kSecValueData: keyData
]
```

#### Background Security

```swift
// Clear sensitive data when entering background
func applicationDidEnterBackground(_ application: UIApplication) {
    // Mark sensitive views as hidden
    sensitiveView.isHidden = true
    
    // Clear in-memory sessions
    NoiseSessionManager.shared.clearAll()
    
    // Request time to complete cleanup
    var bgTask: UIBackgroundTaskIdentifier = .invalid
    bgTask = application.beginBackgroundTask {
        application.endBackgroundTask(bgTask)
    }
    
    DispatchQueue.global().async {
        // Perform cleanup
        self.secureCleanup()
        application.endBackgroundTask(bgTask)
    }
}
```

### Android Security

#### Keystore Integration

```kotlin
// Generate and store keys in hardware-backed keystore
val keyGenerator = KeyGenerator.getInstance(
    KeyProperties.KEY_ALGORITHM_AES,
    "AndroidKeyStore"
)

val keyGenSpec = KeyGenParameterSpec.Builder(
    "noise_identity_key",
    KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
)
    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
    .setKeySize(256)
    .setUserAuthenticationRequired(true)
    .setUserAuthenticationValidityDurationSeconds(30)
    .setUnlockedDeviceRequired(true)
    .build()

keyGenerator.init(keyGenSpec)
val key = keyGenerator.generateKey()
```

#### Memory Security

```kotlin
// Respond to memory pressure
override fun onTrimMemory(level: Int) {
    when (level) {
        ComponentCallbacks2.TRIM_MEMORY_UI_HIDDEN -> {
            // UI is hidden, clear sensitive data
            NoiseSessionManager.clearInactiveSessions()
        }
        ComponentCallbacks2.TRIM_MEMORY_RUNNING_CRITICAL -> {
            // System is low on memory
            NoiseSessionManager.emergencyCleanup()
        }
    }
}
```

## API Security

### Input Validation

All inputs must be validated:

```rust
impl NoiseSession {
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, Error> {
        // Validate state
        if !self.is_transport_mode() {
            return Err(Error::InvalidState);
        }
        
        // Validate input size
        if plaintext.len() > MAX_PLAINTEXT_SIZE {
            return Err(Error::MessageTooLarge);
        }
        
        // Validate buffer won't overflow
        let ciphertext_len = plaintext.len() + TAG_SIZE;
        if ciphertext_len > usize::MAX - HEADER_SIZE {
            return Err(Error::IntegerOverflow);
        }
        
        // Proceed with encryption
        self.do_encrypt(plaintext)
    }
}
```

### Error Handling

Never leak sensitive information in errors:

```rust
// Bad: Leaks timing information
pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, Error> {
    let plaintext = self.do_decrypt(ciphertext)?;
    
    if !self.verify_mac(&plaintext) {
        return Err(Error::InvalidMAC); // Timing leak!
    }
    
    Ok(plaintext)
}

// Good: Constant-time verification
pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, Error> {
    // Decrypt and verify in one operation
    match self.aead_decrypt(ciphertext) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => Err(Error::DecryptionFailed), // Generic error
    }
}
```

## Key Management

### Key Generation

```rust
use rand_core::{RngCore, OsRng};

pub fn generate_keypair() -> Result<Keypair, Error> {
    // Always use OS RNG
    let mut rng = OsRng;
    
    // Generate with proper entropy
    let mut secret = [0u8; 32];
    rng.fill_bytes(&mut secret);
    
    // Derive public key
    let public = x25519_dalek::PublicKey::from(&secret);
    
    Ok(Keypair {
        secret: StaticSecret::from(secret),
        public,
    })
}

// Platform-specific entropy enhancement
#[cfg(target_os = "ios")]
fn enhance_entropy(data: &mut [u8]) {
    use security_framework::random::SecRandom;
    SecRandom::default().randomize(data).unwrap();
}
```

### Key Storage

```rust
pub trait SecureStorage: Send + Sync {
    fn store_key(&self, id: &str, key: &[u8]) -> Result<(), Error>;
    fn retrieve_key(&self, id: &str) -> Result<Vec<u8>, Error>;
    fn delete_key(&self, id: &str) -> Result<(), Error>;
    
    // Require authentication
    fn require_auth(&self) -> bool { true }
    
    // Hardware backing
    fn is_hardware_backed(&self) -> bool;
}
```

### Key Rotation

```rust
pub struct RotatingSession {
    current: NoiseSession,
    next: Option<NoiseSession>,
    rotation_interval: Duration,
    last_rotation: Instant,
}

impl RotatingSession {
    pub fn maybe_rotate(&mut self) -> Result<bool, Error> {
        if self.last_rotation.elapsed() > self.rotation_interval {
            // Generate new keypair
            let new_keypair = generate_keypair()?;
            
            // Create new session
            self.next = Some(NoiseSession::new_with_keys(new_keypair)?);
            
            // Signal rotation to peer
            return Ok(true);
        }
        Ok(false)
    }
}
```

## Attack Mitigations

### Replay Attack Prevention

```rust
pub struct AntiReplayWindow {
    last_seen: u64,
    window: BitVec,
    window_size: usize,
}

impl AntiReplayWindow {
    pub fn check_and_update(&mut self, sequence: u64) -> Result<(), Error> {
        if sequence <= self.last_seen {
            // Check if in window
            let diff = self.last_seen - sequence;
            if diff >= self.window_size as u64 {
                return Err(Error::ReplayDetected);
            }
            
            // Check bit in window
            if self.window[diff as usize] {
                return Err(Error::ReplayDetected);
            }
            
            // Mark as seen
            self.window.set(diff as usize, true);
        } else {
            // Advance window
            let advance = sequence - self.last_seen;
            if advance > self.window_size as u64 {
                // Reset window
                self.window.clear();
            } else {
                // Shift window
                self.window.rotate_right(advance as usize);
            }
            self.last_seen = sequence;
        }
        
        Ok(())
    }
}
```

### MITM Prevention

The Noise XX pattern provides mutual authentication:

```rust
// After handshake, verify peer identity
pub fn verify_peer(&self, expected_public_key: &[u8]) -> Result<(), Error> {
    let peer_static = self.get_remote_static()
        .ok_or(Error::NoPeerKey)?;
    
    if !constant_time_eq(peer_static, expected_public_key) {
        return Err(Error::PeerVerificationFailed);
    }
    
    Ok(())
}

// Generate verification code for out-of-band verification
pub fn verification_code(&self) -> String {
    let hash = self.get_handshake_hash();
    
    // Convert to human-readable code
    let mut code = String::new();
    for chunk in hash.chunks(2) {
        let num = u16::from_be_bytes([chunk[0], chunk[1]]);
        code.push_str(&format!("{:05}", num % 100000));
        code.push('-');
    }
    code.pop(); // Remove trailing dash
    
    code
}
```

## Security Testing

### Fuzzing

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    
    let mode = data[0] % 2;
    let mut session = NoiseSession::new(mode).unwrap();
    
    // Fuzz handshake messages
    let _ = session.read_message(&data[1..]);
    
    // If transport mode, fuzz encryption
    if session.is_transport_mode() {
        let _ = session.decrypt(&data[1..]);
    }
});
```

### Static Analysis

```toml
# .cargo/config.toml
[target.'cfg(all())']
rustflags = [
    "-D", "warnings",
    "-D", "clippy::all",
    "-D", "clippy::pedantic",
    "-D", "clippy::cargo",
    "-D", "unsafe-code",
]
```

### Security Checklist

- [ ] All key material zeroized on drop
- [ ] No timing leaks in crypto operations
- [ ] Input validation on all public APIs
- [ ] No panics in FFI layer
- [ ] Platform secure storage used
- [ ] Memory locked for sensitive operations
- [ ] Replay protection implemented
- [ ] MITM prevention via peer verification
- [ ] Fuzz testing passes
- [ ] Security audit performed

## Vulnerability Response

### Reporting

Security vulnerabilities should be reported to: [security@example.com]

### Response Process

1. **Acknowledge** within 48 hours
2. **Investigate** and verify issue
3. **Develop** fix with test
4. **Review** by security team
5. **Release** patch version
6. **Disclose** after users updated

### Severity Levels

| Level | Description | Example | Response Time |
|-------|-------------|---------|---------------|
| Critical | Remote code execution | Buffer overflow in FFI | < 24 hours |
| High | Authentication bypass | Weak randomness | < 72 hours |
| Medium | Information disclosure | Timing leak | < 1 week |
| Low | Denial of service | Crash on malformed input | < 2 weeks |

## Best Practices for Users

### DO

- ✅ Verify peer identities after handshake
- ✅ Store keys in platform secure storage
- ✅ Clear sessions when done
- ✅ Handle errors gracefully
- ✅ Keep library updated
- ✅ Use secure channels for key exchange

### DON'T

- ❌ Store keys in plain text
- ❌ Reuse nonces or keys
- ❌ Ignore verification codes
- ❌ Log sensitive data
- ❌ Use on jailbroken devices
- ❌ Trust the network

## References

- [Noise Protocol Specification](https://noiseprotocol.org/)
- [OWASP Mobile Security](https://owasp.org/www-project-mobile-security/)
- [iOS Security Guide](https://support.apple.com/guide/security/welcome/web)
- [Android Security](https://source.android.com/security)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)