cmake_minimum_required(VERSION 3.22.1)

project("noisemobile")

# Include directories
include_directories(${CMAKE_SOURCE_DIR}/../../../../include)

# Add the JNI source
add_library(
    noise_jni
    SHARED
    ../../jni/noise_jni.c
)

# Find required libraries
find_library(log-lib log)

# Link against the Rust library and system libraries
target_link_libraries(
    noise_jni
    ${CMAKE_SOURCE_DIR}/../jniLibs/${ANDROID_ABI}/libnoise_mobile.so
    ${log-lib}
)

# Set library output name to match what Java expects
set_target_properties(
    noise_jni
    PROPERTIES
    OUTPUT_NAME "noise_mobile"
)