# Android Integration Example for noise-mobile-rust

This example demonstrates how to integrate the noise-mobile-rust library into an Android application using Kotlin and JNI.

## Features

- Kotlin wrapper around the C FFI API
- JNI bridge implementation
- BLE (Bluetooth Low Energy) transport
- Complete Noise_XX handshake example
- Encrypted message exchange
- Material Design UI

## Project Structure

```
examples/android/
├── app/
│   ├── build.gradle              # App build configuration
│   └── src/main/
│       ├── AndroidManifest.xml   # App manifest
│       ├── java/com/example/noisemobile/
│       │   ├── NoiseSession.kt   # Kotlin wrapper
│       │   ├── BLENoiseTransport.kt # BLE transport
│       │   └── MainActivity.kt   # Main activity
│       ├── jni/
│       │   └── noise_jni.c       # JNI implementation
│       ├── cpp/
│       │   └── CMakeLists.txt    # Native build config
│       └── res/                  # Resources
├── build.gradle                  # Project build config
├── gradle.properties             # Gradle properties
├── settings.gradle               # Project settings
└── README.md                     # This file
```

## Prerequisites

1. Android Studio (Arctic Fox or later)
2. Android SDK (API 21+)
3. Android NDK (r23 or later)
4. Rust toolchain with Android targets

## Building the Library

First, build the Rust library for Android:

```bash
# From the project root
cd ../..

# Install Android targets if not already installed
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add i686-linux-android
rustup target add x86_64-linux-android

# Install cargo-ndk
cargo install cargo-ndk

# Build for Android
./build-android.sh
```

This creates native libraries in `target/android-libs/` for all Android architectures.

## Opening in Android Studio

1. Open Android Studio
2. Select "Open an existing project"
3. Navigate to `examples/android/`
4. Click "OK"
5. Wait for Gradle sync to complete

## Running the Example

### Option 1: Using Android Studio

1. Connect two Android devices (or use device + emulator)
2. Select the device from the device dropdown
3. Click "Run" (green play button)
4. Repeat for the second device

### Option 2: Command Line

```bash
# Build the APK
./gradlew assembleDebug

# Install on device
adb install app/build/outputs/apk/debug/app-debug.apk
```

## Usage

1. **Start the app** on two Android devices
2. **Grant permissions** when prompted (Bluetooth, Location)
3. **Select mode**:
   - Device 1: Select "Responder (Server)" and tap "Start"
   - Device 2: Select "Initiator (Client)" and tap "Start"
4. **Wait for connection** - devices will automatically discover and connect
5. **Send messages** after handshake completes

## Implementation Details

### Kotlin Wrapper

The `NoiseSession` class provides a high-level Kotlin API:

```kotlin
// Create a session
val session = NoiseSession.create(NoiseMode.INITIATOR)

// Perform handshake
val handshakeMsg = session.writeMessage()
val response = session.readMessage(peerMessage)

// Encrypt/decrypt messages
val encrypted = session.encrypt("Hello!".toByteArray())
val decrypted = session.decrypt(encrypted)
```

### JNI Bridge

The JNI implementation in `noise_jni.c`:
- Manages session lifecycle
- Converts between Java and C types
- Handles error propagation
- Prevents memory leaks

### BLE Transport

The `BLENoiseTransport` class handles:
- BLE scanning and advertising
- GATT server/client setup
- Message framing and chunking
- Automatic handshake initiation

## Permissions

The app requires the following permissions:

### Android 11 and below
- `BLUETOOTH` - Basic Bluetooth access
- `BLUETOOTH_ADMIN` - Bluetooth administration
- `ACCESS_FINE_LOCATION` - Required for BLE scanning

### Android 12 and above
- `BLUETOOTH_SCAN` - Scan for BLE devices
- `BLUETOOTH_CONNECT` - Connect to devices
- `BLUETOOTH_ADVERTISE` - Advertise as peripheral

## Customization

### Using Custom Keys

```kotlin
// Generate or load a 32-byte private key
val privateKey = ByteArray(32) { it.toByte() }

// Create session with custom key
val session = NoiseSession.createWithKey(
    NoiseMode.INITIATOR,
    privateKey
)

// Get public key
val publicKey = session.publicKey
println("Public key: ${publicKey.toHexString()}")
```

### Modifying BLE UUIDs

Edit `BLENoiseTransport.kt`:
```kotlin
companion object {
    private val SERVICE_UUID = UUID.fromString("your-service-uuid")
    private val TX_CHAR_UUID = UUID.fromString("your-tx-uuid")
    private val RX_CHAR_UUID = UUID.fromString("your-rx-uuid")
}
```

### ProGuard Rules

If using ProGuard/R8, add to `proguard-rules.pro`:
```
-keep class com.example.noisemobile.NoiseSession { *; }
-keep class com.example.noisemobile.NoiseSession$Companion { *; }
```

## Troubleshooting

### Common Issues

1. **UnsatisfiedLinkError**: Library not found
   - Ensure the Rust library was built correctly
   - Check that `copyNativeLibs` task ran
   - Verify library name matches (`libnoise_mobile.so`)

2. **Bluetooth permission denied**
   - Grant all requested permissions
   - Enable location services (required for BLE)
   - Check app permissions in Settings

3. **Connection fails**
   - Ensure Bluetooth is enabled
   - Devices must be in range (~10 meters)
   - Try restarting Bluetooth

4. **Handshake fails**
   - Both devices must use same Noise pattern
   - Check for version mismatches

### Debug Tips

Enable logging:
```kotlin
// In MainActivity.onCreate()
if (BuildConfig.DEBUG) {
    System.setProperty("java.util.logging.ConsoleHandler.level", "ALL")
}
```

Use Android Studio debugger:
1. Set breakpoints in Kotlin code
2. For native debugging, use "Debug" configuration
3. Check Logcat for native crashes

## Performance

Based on benchmarks:
- Handshake: ~1-2ms on modern Android devices
- Encryption: 100-300 MB/s depending on device
- BLE throughput: ~1-20 KB/s (limited by BLE)

## Security Considerations

1. **Key Storage**: Use Android Keystore for production
2. **Permissions**: Request minimal required permissions
3. **BLE Security**: Consider pairing/bonding for additional security
4. **Background Operation**: Handle Doze mode appropriately

## Next Steps

1. Implement Android Keystore integration
2. Add connection persistence across app restarts
3. Implement message history with Room database
4. Add support for multiple simultaneous connections
5. Create a service for background operation
6. Add file transfer functionality

## License

This example is dual-licensed under Apache 2.0 and MIT, same as the parent project.