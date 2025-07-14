# iOS Integration Example for noise-mobile-rust

This example demonstrates how to integrate the noise-mobile-rust library into an iOS application using Swift.

## Features

- Swift wrapper around the C FFI API
- BLE (Bluetooth Low Energy) transport implementation
- Complete Noise_XX handshake example
- Encrypted message exchange
- SwiftUI demo app

## Project Structure

```
examples/ios/
├── Package.swift              # Swift Package Manager configuration
├── Sources/
│   ├── NoiseSession.swift     # Swift wrapper for Noise protocol
│   ├── BLENoiseTransport.swift # BLE transport implementation
│   ├── ExampleApp.swift       # SwiftUI demo application
│   └── NoiseMobile-Bridging-Header.h # C bridging header
└── README.md                  # This file
```

## Building the Library

First, build the Rust library for iOS:

```bash
# From the project root
cd ../..
./build-ios.sh
```

This creates `target/NoiseMobile.xcframework` containing the library for all iOS architectures.

## Integration Methods

### Method 1: Swift Package Manager (Recommended)

1. Open the example in Xcode:
   ```bash
   cd examples/ios
   open Package.swift
   ```

2. The package is already configured to use the built XCFramework.

3. Build and run on simulator or device.

### Method 2: Direct Xcode Integration

1. Create a new iOS app in Xcode
2. Add the XCFramework:
   - Drag `target/NoiseMobile.xcframework` into your project
   - Ensure "Copy items if needed" is checked
   - Add to your app target

3. Configure bridging header:
   - Create a bridging header file
   - Add: `#include "noise_mobile.h"`
   - Set in Build Settings > Swift Compiler > Objective-C Bridging Header

4. Copy the Swift wrapper files:
   - `NoiseSession.swift`
   - `BLENoiseTransport.swift` (if using BLE)

### Method 3: CocoaPods (Future)

A podspec can be created for easier distribution:

```ruby
Pod::Spec.new do |s|
  s.name         = "NoiseMobile"
  s.version      = "0.1.0"
  s.summary      = "Mobile-optimized Noise Protocol Framework"
  s.platform     = :ios, "13.0"
  s.source       = { :git => "https://github.com/your-repo/noise-mobile-rust.git" }
  s.vendored_frameworks = "NoiseMobile.xcframework"
end
```

## Usage Example

### Basic Noise Session

```swift
import Foundation

// Create initiator and responder
let initiator = try NoiseSession(mode: .initiator)
let responder = try NoiseSession(mode: .responder)

// Perform handshake
try NoiseSession.performHandshake(initiator: initiator, responder: responder)

// Exchange encrypted messages
let plaintext = "Hello, Noise!".data(using: .utf8)!
let encrypted = try initiator.encrypt(plaintext)
let decrypted = try responder.decrypt(encrypted)

print(String(data: decrypted, encoding: .utf8)!) // "Hello, Noise!"
```

### BLE Transport

```swift
// Create BLE transport
let transport = BLENoiseTransport(mode: .initiator)

// Start as central (client)
try transport.startCentral { receivedData in
    print("Received: \(receivedData)")
}

// Send encrypted message after handshake
try transport.sendMessage("Hello over BLE!".data(using: .utf8)!)
```

### Custom Key Pair

```swift
// Generate or load a 32-byte private key
let privateKey = Data(repeating: 0x42, count: 32) // Example key

// Create session with custom key
let session = try NoiseSession(mode: .initiator, privateKey: privateKey)

// Get public key
let publicKey = try session.publicKey
print("Public key: \(publicKey.hexString)")
```

## SwiftUI Demo App

The included `ExampleApp.swift` demonstrates:

1. Mode selection (Initiator/Responder)
2. BLE scanning and advertising
3. Automatic handshake on connection
4. Secure message exchange
5. Connection status monitoring

To run the demo:

1. Build the app for two iOS devices (or device + simulator)
2. Start one as "Responder" (server)
3. Start the other as "Initiator" (client)
4. They will automatically connect and perform handshake
5. Send encrypted messages between devices

## Important Notes

### Info.plist Requirements

Add these entries for BLE support:

```xml
<key>NSBluetoothAlwaysUsageDescription</key>
<string>This app uses Bluetooth to communicate with nearby devices.</string>
<key>NSBluetoothPeripheralUsageDescription</key>
<string>This app uses Bluetooth to communicate with nearby devices.</string>
```

### Background Modes

For background BLE operation, add to Info.plist:

```xml
<key>UIBackgroundModes</key>
<array>
    <string>bluetooth-central</string>
    <string>bluetooth-peripheral</string>
</array>
```

### Memory Management

The Swift wrapper handles memory management automatically:
- `NoiseSession` deallocates the Rust session in `deinit`
- No manual memory management required
- Thread-safe for single session use

### Error Handling

All operations that can fail throw `NoiseError`:

```swift
do {
    let session = try NoiseSession(mode: .initiator)
    // Use session
} catch NoiseError.invalidParameter {
    print("Invalid parameter provided")
} catch NoiseError.handshakeFailed {
    print("Handshake failed")
} catch {
    print("Unexpected error: \(error)")
}
```

## Performance Considerations

Based on benchmarks:
- Handshake: ~390μs on Apple Silicon
- Encryption: 270-572 MiB/s depending on message size
- Use batching for multiple small messages
- BLE MTU affects chunking performance

## Security Notes

1. **Key Storage**: Use iOS Keychain for persistent key storage
2. **Random Keys**: Use `SecRandomCopyBytes` for key generation
3. **Public Key Validation**: Implement out-of-band verification
4. **Replay Protection**: Built-in via Noise protocol

## Troubleshooting

### Common Issues

1. **Undefined symbols**: Ensure XCFramework is properly linked
2. **BLE not working**: Check Info.plist permissions
3. **Handshake fails**: Verify both parties use same Noise pattern
4. **Module not found**: Check bridging header configuration

### Debug Tips

Enable Noise protocol logging:
```swift
// In bridging header or Swift file
setenv("RUST_LOG", "debug", 1)
```

## Next Steps

1. Implement iOS Keychain integration for key storage
2. Add Network Extension for background operation
3. Implement push notifications for offline messages
4. Add support for multiple simultaneous sessions
5. Create a full chat UI example

## License

This example is dual-licensed under Apache 2.0 and MIT, same as the parent project.