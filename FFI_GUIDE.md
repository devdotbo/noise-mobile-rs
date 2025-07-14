# FFI Integration Guide

## Overview

This guide explains how to integrate `noise-mobile-rust` into iOS and Android applications. The library provides C-compatible FFI bindings that work with both platforms.

## General Principles

1. **Opaque Pointers**: Rust structs are hidden behind opaque pointers
2. **Error Codes**: All functions return error codes, not exceptions
3. **Memory Management**: Rust owns all memory; explicit free required
4. **Buffer Sizing**: Caller allocates buffers with size checking

## C API Reference

### Types

```c
// Opaque session pointer
typedef struct NoiseSession NoiseSession;

// Error codes
typedef enum {
    NOISE_ERROR_SUCCESS = 0,
    NOISE_ERROR_INVALID_PARAMETER = 1,
    NOISE_ERROR_OUT_OF_MEMORY = 2,
    NOISE_ERROR_HANDSHAKE_FAILED = 3,
    NOISE_ERROR_ENCRYPTION_FAILED = 4,
    NOISE_ERROR_DECRYPTION_FAILED = 5,
    NOISE_ERROR_BUFFER_TOO_SMALL = 6,
    NOISE_ERROR_INVALID_STATE = 7,
} NoiseError;

// Session modes
typedef enum {
    NOISE_MODE_INITIATOR = 0,
    NOISE_MODE_RESPONDER = 1,
} NoiseMode;
```

### Core Functions

```c
// Create a new session
NoiseSession* noise_session_new(int mode, int* error);

// Free a session
void noise_session_free(NoiseSession* session);

// Handshake operations
int noise_write_message(
    NoiseSession* session,
    const uint8_t* payload, size_t payload_len,
    uint8_t* output, size_t* output_len
);

int noise_read_message(
    NoiseSession* session,
    const uint8_t* input, size_t input_len,
    uint8_t* payload, size_t* payload_len
);

// Check if handshake is complete
int noise_is_handshake_complete(NoiseSession* session);

// Transport operations
int noise_encrypt(
    NoiseSession* session,
    const uint8_t* plaintext, size_t plaintext_len,
    uint8_t* ciphertext, size_t* ciphertext_len
);

int noise_decrypt(
    NoiseSession* session,
    const uint8_t* ciphertext, size_t ciphertext_len,
    uint8_t* plaintext, size_t* plaintext_len
);

// Utility functions
size_t noise_max_message_len(void);
size_t noise_max_payload_len(void);
const char* noise_error_string(int error);
```

## iOS Integration

### Setup

1. **Build for iOS**
```bash
# Build universal library
./build-ios.sh

# Output:
# - target/universal/release/libnoise_mobile.a
# - include/noise_mobile.h
```

2. **Xcode Integration**
- Add `libnoise_mobile.a` to your project
- Add `noise_mobile.h` to your project
- Link with system libraries: `libresolv.tbd`

### Swift Wrapper

Create a Swift wrapper for better ergonomics:

```swift
// NoiseSession.swift
import Foundation

enum NoiseError: Error {
    case invalidParameter
    case outOfMemory
    case handshakeFailed
    case encryptionFailed
    case decryptionFailed
    case bufferTooSmall
    case invalidState
    
    init?(code: Int32) {
        switch code {
        case 0: return nil // Success
        case 1: self = .invalidParameter
        case 2: self = .outOfMemory
        case 3: self = .handshakeFailed
        case 4: self = .encryptionFailed
        case 5: self = .decryptionFailed
        case 6: self = .bufferTooSmall
        case 7: self = .invalidState
        default: self = .invalidParameter
        }
    }
}

class NoiseSession {
    private let ptr: OpaquePointer
    private let maxMessageLen = noise_max_message_len()
    
    init(mode: NoiseMode) throws {
        var error: Int32 = 0
        guard let session = noise_session_new(mode.rawValue, &error),
              error == 0 else {
            throw NoiseError(code: error) ?? NoiseError.handshakeFailed
        }
        self.ptr = session
    }
    
    deinit {
        noise_session_free(ptr)
    }
    
    var isHandshakeComplete: Bool {
        return noise_is_handshake_complete(ptr) != 0
    }
    
    func writeMessage(_ payload: Data = Data()) throws -> Data {
        let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: Int(maxMessageLen))
        defer { buffer.deallocate() }
        
        var outputLen = maxMessageLen
        
        let result = payload.withUnsafeBytes { payloadPtr in
            noise_write_message(
                ptr,
                payloadPtr.bindMemory(to: UInt8.self).baseAddress,
                payload.count,
                buffer,
                &outputLen
            )
        }
        
        if let error = NoiseError(code: result) {
            throw error
        }
        
        return Data(bytes: buffer, count: Int(outputLen))
    }
    
    func readMessage(_ input: Data) throws -> Data? {
        let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: Int(maxMessageLen))
        defer { buffer.deallocate() }
        
        var outputLen = maxMessageLen
        
        let result = input.withUnsafeBytes { inputPtr in
            noise_read_message(
                ptr,
                inputPtr.bindMemory(to: UInt8.self).baseAddress,
                input.count,
                buffer,
                &outputLen
            )
        }
        
        if let error = NoiseError(code: result) {
            throw error
        }
        
        return outputLen > 0 ? Data(bytes: buffer, count: Int(outputLen)) : nil
    }
    
    func encrypt(_ plaintext: Data) throws -> Data {
        let bufferSize = plaintext.count + 16 // AEAD tag
        let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: bufferSize)
        defer { buffer.deallocate() }
        
        var outputLen = bufferSize
        
        let result = plaintext.withUnsafeBytes { plaintextPtr in
            noise_encrypt(
                ptr,
                plaintextPtr.bindMemory(to: UInt8.self).baseAddress,
                plaintext.count,
                buffer,
                &outputLen
            )
        }
        
        if let error = NoiseError(code: result) {
            throw error
        }
        
        return Data(bytes: buffer, count: Int(outputLen))
    }
    
    func decrypt(_ ciphertext: Data) throws -> Data {
        let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: ciphertext.count)
        defer { buffer.deallocate() }
        
        var outputLen = ciphertext.count
        
        let result = ciphertext.withUnsafeBytes { ciphertextPtr in
            noise_decrypt(
                ptr,
                ciphertextPtr.bindMemory(to: UInt8.self).baseAddress,
                ciphertext.count,
                buffer,
                &outputLen
            )
        }
        
        if let error = NoiseError(code: result) {
            throw error
        }
        
        return Data(bytes: buffer, count: Int(outputLen))
    }
}

enum NoiseMode: Int32 {
    case initiator = 0
    case responder = 1
}
```

### Usage Example

```swift
// BLE Integration
class SecurePeripheral: NSObject {
    private var noiseSession: NoiseSession?
    private var peripheral: CBPeripheralManager!
    
    func startAdvertising() throws {
        // Initialize Noise as responder
        noiseSession = try NoiseSession(mode: .responder)
        
        // Start BLE advertising
        peripheral = CBPeripheralManager(delegate: self, queue: nil)
    }
    
    func peripheral(_ peripheral: CBPeripheralManager, 
                    didReceiveRead request: CBATTRequest) {
        guard let session = noiseSession else { return }
        
        do {
            // Process incoming handshake message
            let response = try session.readMessage(request.value ?? Data())
            
            if let response = response {
                // Send response
                request.value = response
                peripheral.respond(to: request, withResult: .success)
            }
            
            if session.isHandshakeComplete {
                print("Secure connection established!")
            }
        } catch {
            peripheral.respond(to: request, withResult: .unlikelyError)
        }
    }
}
```

## Android Integration

### Setup

1. **Build for Android**
```bash
# Build for all Android architectures
./build-android.sh

# Output:
# - target/aarch64-linux-android/release/libnoise_mobile.so
# - target/armv7-linux-androideabi/release/libnoise_mobile.so
# - target/i686-linux-android/release/libnoise_mobile.so
# - target/x86_64-linux-android/release/libnoise_mobile.so
```

2. **Android Studio Integration**

Project structure:
```
app/
├── src/
│   └── main/
│       ├── java/
│       │   └── com/example/
│       │       └── NoiseSession.kt
│       └── jniLibs/
│           ├── arm64-v8a/
│           │   └── libnoise_mobile.so
│           ├── armeabi-v7a/
│           │   └── libnoise_mobile.so
│           ├── x86/
│           │   └── libnoise_mobile.so
│           └── x86_64/
│               └── libnoise_mobile.so
```

### Kotlin Wrapper

```kotlin
// NoiseSession.kt
package com.example.noise

import java.nio.ByteBuffer

class NoiseSession(mode: Mode) : AutoCloseable {
    private val ptr: Long
    
    enum class Mode(val value: Int) {
        INITIATOR(0),
        RESPONDER(1)
    }
    
    sealed class NoiseException(message: String) : Exception(message) {
        object InvalidParameter : NoiseException("Invalid parameter")
        object OutOfMemory : NoiseException("Out of memory")
        object HandshakeFailed : NoiseException("Handshake failed")
        object EncryptionFailed : NoiseException("Encryption failed")
        object DecryptionFailed : NoiseException("Decryption failed")
        object BufferTooSmall : NoiseException("Buffer too small")
        object InvalidState : NoiseException("Invalid state")
    }
    
    init {
        System.loadLibrary("noise_mobile")
        ptr = nativeNew(mode.value)
        if (ptr == 0L) {
            throw NoiseException.HandshakeFailed
        }
    }
    
    val isHandshakeComplete: Boolean
        get() = nativeIsHandshakeComplete(ptr)
    
    fun writeMessage(payload: ByteArray = ByteArray(0)): ByteArray {
        return nativeWriteMessage(ptr, payload)
    }
    
    fun readMessage(input: ByteArray): ByteArray? {
        return nativeReadMessage(ptr, input)
    }
    
    fun encrypt(plaintext: ByteArray): ByteArray {
        if (!isHandshakeComplete) {
            throw NoiseException.InvalidState
        }
        return nativeEncrypt(ptr, plaintext)
    }
    
    fun decrypt(ciphertext: ByteArray): ByteArray {
        if (!isHandshakeComplete) {
            throw NoiseException.InvalidState
        }
        return nativeDecrypt(ptr, ciphertext)
    }
    
    override fun close() {
        if (ptr != 0L) {
            nativeFree(ptr)
        }
    }
    
    companion object {
        @JvmStatic
        private external fun nativeNew(mode: Int): Long
        
        @JvmStatic
        private external fun nativeFree(ptr: Long)
        
        @JvmStatic
        private external fun nativeIsHandshakeComplete(ptr: Long): Boolean
        
        @JvmStatic
        private external fun nativeWriteMessage(ptr: Long, payload: ByteArray): ByteArray
        
        @JvmStatic
        private external fun nativeReadMessage(ptr: Long, input: ByteArray): ByteArray?
        
        @JvmStatic
        private external fun nativeEncrypt(ptr: Long, plaintext: ByteArray): ByteArray
        
        @JvmStatic
        private external fun nativeDecrypt(ptr: Long, ciphertext: ByteArray): ByteArray
    }
}
```

### JNI Implementation

```c
// jni/noise_jni.c
#include <jni.h>
#include <stdlib.h>
#include <string.h>
#include "noise_mobile.h"

JNIEXPORT jlong JNICALL
Java_com_example_noise_NoiseSession_nativeNew(JNIEnv *env, jclass clazz, jint mode) {
    int error = 0;
    NoiseSession* session = noise_session_new(mode, &error);
    if (error != 0) {
        return 0;
    }
    return (jlong) session;
}

JNIEXPORT void JNICALL
Java_com_example_noise_NoiseSession_nativeFree(JNIEnv *env, jclass clazz, jlong ptr) {
    if (ptr != 0) {
        noise_session_free((NoiseSession*) ptr);
    }
}

JNIEXPORT jboolean JNICALL
Java_com_example_noise_NoiseSession_nativeIsHandshakeComplete(JNIEnv *env, jclass clazz, jlong ptr) {
    return noise_is_handshake_complete((NoiseSession*) ptr) ? JNI_TRUE : JNI_FALSE;
}

JNIEXPORT jbyteArray JNICALL
Java_com_example_noise_NoiseSession_nativeWriteMessage(JNIEnv *env, jclass clazz, jlong ptr, jbyteArray payload) {
    NoiseSession* session = (NoiseSession*) ptr;
    
    jsize payload_len = (*env)->GetArrayLength(env, payload);
    jbyte* payload_data = (*env)->GetByteArrayElements(env, payload, NULL);
    
    size_t output_len = noise_max_message_len();
    uint8_t* output = malloc(output_len);
    
    int result = noise_write_message(
        session,
        (const uint8_t*) payload_data, payload_len,
        output, &output_len
    );
    
    (*env)->ReleaseByteArrayElements(env, payload, payload_data, JNI_ABORT);
    
    if (result != 0) {
        free(output);
        // Throw exception
        jclass exception_class = (*env)->FindClass(env, "com/example/noise/NoiseSession$NoiseException$HandshakeFailed");
        (*env)->ThrowNew(env, exception_class, "Write message failed");
        return NULL;
    }
    
    jbyteArray result_array = (*env)->NewByteArray(env, output_len);
    (*env)->SetByteArrayRegion(env, result_array, 0, output_len, (const jbyte*) output);
    
    free(output);
    return result_array;
}

// Similar implementations for other native methods...
```

### Usage Example

```kotlin
// BLE Integration
class SecureBleService : Service() {
    private lateinit var noiseSession: NoiseSession
    private lateinit var bluetoothGatt: BluetoothGatt
    
    fun initializeSecurity() {
        noiseSession = NoiseSession(NoiseSession.Mode.INITIATOR)
    }
    
    private val gattCallback = object : BluetoothGattCallback() {
        override fun onCharacteristicWrite(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            status: Int
        ) {
            if (status == BluetoothGatt.GATT_SUCCESS) {
                // Read response
                gatt.readCharacteristic(characteristic)
            }
        }
        
        override fun onCharacteristicRead(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            status: Int
        ) {
            if (status == BluetoothGatt.GATT_SUCCESS) {
                val response = characteristic.value
                
                try {
                    val payload = noiseSession.readMessage(response)
                    
                    if (payload != null && !noiseSession.isHandshakeComplete) {
                        // Continue handshake
                        val nextMessage = noiseSession.writeMessage()
                        characteristic.value = nextMessage
                        gatt.writeCharacteristic(characteristic)
                    }
                    
                    if (noiseSession.isHandshakeComplete) {
                        Log.d(TAG, "Secure connection established!")
                        // Start encrypted communication
                    }
                } catch (e: NoiseSession.NoiseException) {
                    Log.e(TAG, "Handshake failed", e)
                }
            }
        }
    }
    
    fun sendEncryptedMessage(message: String) {
        if (!noiseSession.isHandshakeComplete) {
            throw IllegalStateException("Handshake not complete")
        }
        
        val encrypted = noiseSession.encrypt(message.toByteArray())
        // Send via BLE
    }
}
```

## Platform-Specific Considerations

### iOS

1. **Background Modes**: Add bluetooth-central and bluetooth-peripheral to Info.plist
2. **State Restoration**: Serialize session state for background/foreground transitions
3. **Keychain Integration**: Store long-term keys securely
4. **Memory Warnings**: Implement didReceiveMemoryWarning handling

### Android

1. **Doze Mode**: Handle Doze restrictions for background BLE
2. **Permissions**: Request BLUETOOTH, BLUETOOTH_ADMIN, ACCESS_FINE_LOCATION
3. **Keystore Integration**: Use Android Keystore for key storage
4. **ProGuard**: Add rules to keep JNI methods

ProGuard rules:
```
-keep class com.example.noise.NoiseSession { *; }
-keep class com.example.noise.NoiseSession$* { *; }
```

## Testing Integration

### iOS Test Example
```swift
func testCrossPlatformHandshake() {
    let ios = try! NoiseSession(mode: .initiator)
    let android = try! NoiseSession(mode: .responder)
    
    // Simulate communication
    var msg = try! ios.writeMessage()
    msg = try! android.readMessage(msg) ?? Data()
    msg = try! ios.readMessage(msg) ?? Data()
    msg = try! android.writeMessage()
    msg = try! ios.readMessage(msg) ?? Data()
    
    XCTAssertTrue(ios.isHandshakeComplete)
    XCTAssertTrue(android.isHandshakeComplete)
    
    // Test encryption
    let plaintext = "Hello from iOS"
    let encrypted = try! ios.encrypt(plaintext.data(using: .utf8)!)
    let decrypted = try! android.decrypt(encrypted)
    
    XCTAssertEqual(String(data: decrypted, encoding: .utf8), plaintext)
}
```

### Android Test Example
```kotlin
@Test
fun testCrossPlatformHandshake() {
    val android = NoiseSession(NoiseSession.Mode.INITIATOR)
    val ios = NoiseSession(NoiseSession.Mode.RESPONDER)
    
    // Simulate communication
    var msg = android.writeMessage()
    msg = ios.readMessage(msg) ?: ByteArray(0)
    msg = android.readMessage(msg) ?: ByteArray(0)
    msg = ios.writeMessage()
    android.readMessage(msg)
    
    assertTrue(android.isHandshakeComplete)
    assertTrue(ios.isHandshakeComplete)
    
    // Test encryption
    val plaintext = "Hello from Android".toByteArray()
    val encrypted = android.encrypt(plaintext)
    val decrypted = ios.decrypt(encrypted)
    
    assertArrayEquals(plaintext, decrypted)
}
```

## Performance Tips

1. **Buffer Reuse**: Allocate buffers once and reuse
2. **Batch Operations**: Encrypt multiple messages together
3. **Background Threads**: Perform crypto operations off main thread
4. **Memory Pressure**: Implement cleanup on memory warnings

## Common Issues

### iOS
- **Linker Errors**: Ensure `-lresolv` is in Other Linker Flags
- **Bitcode**: Disable bitcode or build library with bitcode support
- **Simulator**: Build separate simulator and device libraries

### Android
- **UnsatisfiedLinkError**: Check library is in correct jniLibs folder
- **Architecture Mismatch**: Ensure all target architectures are built
- **R8/ProGuard**: Keep all JNI methods from obfuscation

## Security Best Practices

1. **Key Storage**: Always use platform secure storage (Keychain/Keystore)
2. **Memory Cleanup**: Zero sensitive data after use
3. **Background Safety**: Don't leave sessions in memory during background
4. **Network Validation**: Verify peer identity after handshake
5. **Error Handling**: Don't leak information in error messages