#!/bin/bash

# Setup script for Android example project

set -e

echo "Setting up Android example project..."

# Check if we're in the right directory
if [ ! -f "build.gradle" ]; then
    echo "Error: Please run this script from the examples/android directory"
    exit 1
fi

# Check for Android SDK
if [ -z "$ANDROID_HOME" ] && [ -z "$ANDROID_SDK_ROOT" ]; then
    echo "Error: ANDROID_HOME or ANDROID_SDK_ROOT environment variable not set"
    echo "Please set it to your Android SDK location"
    exit 1
fi

# Check for Android NDK
if [ -z "$ANDROID_NDK_HOME" ] && [ -z "$NDK_HOME" ]; then
    echo "Warning: ANDROID_NDK_HOME not set"
    echo "The build will try to use the NDK bundled with Android Studio"
fi

# Build the Rust library if needed
if [ ! -d "../../target/android-libs" ]; then
    echo "Building Rust library for Android..."
    cd ../..
    
    # Check if cargo-ndk is installed
    if ! command -v cargo-ndk &> /dev/null; then
        echo "Installing cargo-ndk..."
        cargo install cargo-ndk
    fi
    
    # Check if Android targets are installed
    for target in aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android; do
        if ! rustup target list --installed | grep -q $target; then
            echo "Installing Rust target: $target"
            rustup target add $target
        fi
    done
    
    # Build the library
    ./build-android.sh
    cd examples/android
fi

# Create local.properties if it doesn't exist
if [ ! -f "local.properties" ]; then
    echo "Creating local.properties..."
    SDK_DIR="${ANDROID_HOME:-$ANDROID_SDK_ROOT}"
    echo "sdk.dir=$SDK_DIR" > local.properties
    
    if [ -n "$ANDROID_NDK_HOME" ]; then
        echo "ndk.dir=$ANDROID_NDK_HOME" >> local.properties
    elif [ -n "$NDK_HOME" ]; then
        echo "ndk.dir=$NDK_HOME" >> local.properties
    fi
fi

# Make gradlew executable if it exists
if [ -f "gradlew" ]; then
    chmod +x gradlew
fi

echo ""
echo "Setup complete!"
echo ""
echo "To open in Android Studio:"
echo "1. Open Android Studio"
echo "2. Select 'Open an existing project'"
echo "3. Navigate to this directory and click OK"
echo ""
echo "To build from command line:"
echo "  ./gradlew assembleDebug"
echo ""
echo "To install on connected device:"
echo "  adb install app/build/outputs/apk/debug/app-debug.apk"
echo ""
echo "For more details, see README.md"