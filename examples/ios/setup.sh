#!/bin/bash

# Setup script for iOS example project

set -e

echo "Setting up iOS example project..."

# Check if we're in the right directory
if [ ! -f "Package.swift" ]; then
    echo "Error: Please run this script from the examples/ios directory"
    exit 1
fi

# Build the Rust library if needed
if [ ! -d "../../target/NoiseMobile.xcframework" ]; then
    echo "Building Rust library for iOS..."
    cd ../..
    ./build-ios.sh
    cd examples/ios
fi

# Check for Xcode
if ! command -v xcodebuild &> /dev/null; then
    echo "Error: Xcode is not installed"
    exit 1
fi

# Option 1: Open with Swift Package Manager
echo "Opening project with Swift Package Manager..."
open Package.swift

echo ""
echo "Setup complete! The project should now be open in Xcode."
echo ""
echo "To run the example:"
echo "1. Select a simulator or device"
echo "2. Build and run (Cmd+R)"
echo "3. For BLE testing, you'll need two devices or a device + simulator"
echo ""
echo "Make sure to add the following to your Info.plist:"
echo "- NSBluetoothAlwaysUsageDescription"
echo "- NSBluetoothPeripheralUsageDescription"
echo ""
echo "For more details, see README.md"