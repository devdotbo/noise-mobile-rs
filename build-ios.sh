#!/bin/bash
# Build script for iOS - creates universal library and XCFramework
# Copyright 2025 PermissionlessTech Contributors
# SPDX-License-Identifier: MIT OR Apache-2.0

set -e

echo "Building noise-mobile-rust for iOS..."

# Check if necessary tools are installed
if ! command -v rustup &> /dev/null; then
    echo "Error: rustup is not installed. Please install Rust."
    exit 1
fi

if ! command -v cbindgen &> /dev/null; then
    echo "cbindgen not found. Installing..."
    cargo install cbindgen
fi

# Add iOS targets if not already installed
echo "Checking iOS targets..."
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim 2>/dev/null || true

# Clean previous builds
echo "Cleaning previous builds..."
rm -rf target/universal
rm -rf target/NoiseMobile.xcframework
mkdir -p target/universal/release

# Build for iOS device (arm64)
echo "Building for iOS device (arm64)..."
cargo build --target aarch64-apple-ios --release

# Build for iOS simulator (x86_64 - Intel Macs)
echo "Building for iOS simulator (x86_64)..."
cargo build --target x86_64-apple-ios --release

# Build for iOS simulator (arm64 - M1+ Macs)
echo "Building for iOS simulator (arm64)..."
cargo build --target aarch64-apple-ios-sim --release

# Create universal libraries
echo "Creating universal libraries..."

# Device library (just arm64)
cp target/aarch64-apple-ios/release/libnoise_mobile.a \
   target/universal/release/libnoise_mobile_device.a

# Simulator library (both x86_64 and arm64)
lipo -create \
    target/x86_64-apple-ios/release/libnoise_mobile.a \
    target/aarch64-apple-ios-sim/release/libnoise_mobile.a \
    -output target/universal/release/libnoise_mobile_sim.a

# Generate C header
echo "Generating C header..."
mkdir -p include
cbindgen --config cbindgen.toml --crate noise-mobile-rust --output include/noise_mobile.h

# Create module map for Swift
echo "Creating module map..."
mkdir -p include
cat > include/module.modulemap << EOF
module NoiseMobile {
    header "noise_mobile.h"
    export *
}
EOF

# Create XCFramework
echo "Creating XCFramework..."
xcodebuild -create-xcframework \
    -library target/universal/release/libnoise_mobile_device.a \
    -headers include \
    -library target/universal/release/libnoise_mobile_sim.a \
    -headers include \
    -output target/NoiseMobile.xcframework

echo "✅ iOS build complete!"
echo ""
echo "Output files:"
echo "  - XCFramework: target/NoiseMobile.xcframework"
echo "  - C Header: include/noise_mobile.h"
echo "  - Module Map: include/module.modulemap"
echo ""
echo "To use in Xcode:"
echo "1. Drag target/NoiseMobile.xcframework into your Xcode project"
echo "2. Make sure 'Embed & Sign' is selected in the framework settings"
echo "3. Import the module in Swift: import NoiseMobile"