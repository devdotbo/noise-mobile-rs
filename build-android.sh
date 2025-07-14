#!/bin/bash
# Build script for Android - creates libraries for all architectures
# Copyright 2025 PermissionlessTech Contributors
# SPDX-License-Identifier: MIT OR Apache-2.0

set -e

echo "Building noise-mobile-rust for Android..."

# Check if necessary tools are installed
if ! command -v rustup &> /dev/null; then
    echo "Error: rustup is not installed. Please install Rust."
    exit 1
fi

if ! command -v cargo-ndk &> /dev/null; then
    echo "cargo-ndk not found. Installing..."
    cargo install cargo-ndk
fi

if ! command -v cbindgen &> /dev/null; then
    echo "cbindgen not found. Installing..."
    cargo install cbindgen
fi

# Check for Android NDK
if [ -z "$ANDROID_NDK_HOME" ] && [ -z "$ANDROID_HOME" ]; then
    echo "Error: Neither ANDROID_NDK_HOME nor ANDROID_HOME is set."
    echo "Please set ANDROID_NDK_HOME to your Android NDK path."
    exit 1
fi

# Set NDK path
if [ -z "$ANDROID_NDK_HOME" ]; then
    # Try to find NDK in common locations
    if [ -d "$ANDROID_HOME/ndk" ]; then
        # Find the latest NDK version
        NDK_VERSION=$(ls -1 "$ANDROID_HOME/ndk" | sort -V | tail -n 1)
        export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/$NDK_VERSION"
    else
        echo "Error: Could not find Android NDK."
        echo "Please install Android NDK and set ANDROID_NDK_HOME."
        exit 1
    fi
fi

echo "Using Android NDK at: $ANDROID_NDK_HOME"

# Add Android targets if not already installed
echo "Checking Android targets..."
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android 2>/dev/null || true

# Clean previous builds
echo "Cleaning previous builds..."
rm -rf target/android-libs
rm -rf android/jniLibs

# Build for all Android architectures
echo "Building for Android architectures..."
cargo ndk \
    --target aarch64-linux-android \
    --target armv7-linux-androideabi \
    --target i686-linux-android \
    --target x86_64-linux-android \
    --output-dir ./target/android-libs \
    -- build --release

# Create jniLibs directory structure
echo "Creating JNI library structure..."
mkdir -p android/jniLibs/{arm64-v8a,armeabi-v7a,x86,x86_64}

# Copy libraries to JNI structure
cp target/android-libs/arm64-v8a/libnoise_mobile.so android/jniLibs/arm64-v8a/
cp target/android-libs/armeabi-v7a/libnoise_mobile.so android/jniLibs/armeabi-v7a/
cp target/android-libs/x86/libnoise_mobile.so android/jniLibs/x86/
cp target/android-libs/x86_64/libnoise_mobile.so android/jniLibs/x86_64/

# Generate C header
echo "Generating C header..."
mkdir -p include
cbindgen --config cbindgen.toml --crate noise-mobile-rust --output include/noise_mobile.h

# Create a sample JNI wrapper header
echo "Creating JNI wrapper template..."
cat > include/noise_mobile_jni.h << 'EOF'
/* JNI wrapper for noise-mobile-rust
 * Copyright 2025 PermissionlessTech Contributors
 * SPDX-License-Identifier: MIT OR Apache-2.0
 */

#ifndef NOISE_MOBILE_JNI_H
#define NOISE_MOBILE_JNI_H

#include <jni.h>
#include "noise_mobile.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Example JNI function declarations
 * Implement these in your Android project's native code
 */

JNIEXPORT jlong JNICALL
Java_com_example_noise_NoiseSession_createInitiator(JNIEnv *env, jobject thiz);

JNIEXPORT jlong JNICALL
Java_com_example_noise_NoiseSession_createResponder(JNIEnv *env, jobject thiz);

JNIEXPORT void JNICALL
Java_com_example_noise_NoiseSession_destroy(JNIEnv *env, jobject thiz, jlong session_ptr);

JNIEXPORT jbyteArray JNICALL
Java_com_example_noise_NoiseSession_writeMessage(JNIEnv *env, jobject thiz, 
                                                  jlong session_ptr, jbyteArray payload);

JNIEXPORT jbyteArray JNICALL
Java_com_example_noise_NoiseSession_readMessage(JNIEnv *env, jobject thiz,
                                                 jlong session_ptr, jbyteArray message);

#ifdef __cplusplus
}
#endif

#endif /* NOISE_MOBILE_JNI_H */
EOF

# Create AAR build configuration
echo "Creating AAR build files..."
mkdir -p android/aar
cat > android/aar/build.gradle << 'EOF'
apply plugin: 'com.android.library'

android {
    compileSdkVersion 33
    
    defaultConfig {
        minSdkVersion 21
        targetSdkVersion 33
        versionCode 1
        versionName "1.0"
    }
    
    sourceSets {
        main {
            jniLibs.srcDirs = ['../jniLibs']
        }
    }
}

dependencies {
    implementation fileTree(dir: 'libs', include: ['*.jar'])
}
EOF

echo "✅ Android build complete!"
echo ""
echo "Output files:"
echo "  - Libraries: android/jniLibs/{arm64-v8a,armeabi-v7a,x86,x86_64}/libnoise_mobile.so"
echo "  - C Header: include/noise_mobile.h"
echo "  - JNI Template: include/noise_mobile_jni.h"
echo ""
echo "To use in Android Studio:"
echo "1. Copy android/jniLibs to your app/src/main/ directory"
echo "2. Create JNI wrapper functions based on the template"
echo "3. Load the library in your Kotlin/Java code:"
echo "   System.loadLibrary(\"noise_mobile\")"