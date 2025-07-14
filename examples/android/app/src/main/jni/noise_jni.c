#include <jni.h>
#include <string.h>
#include <stdlib.h>
#include "noise_mobile.h"

// Cache for method IDs and class references
static jclass noiseExceptionClass = NULL;
static jmethodID noiseExceptionConstructor = NULL;

// Error tracking per session
typedef struct {
    struct NoiseSessionFFI* session;
    int last_error;
} SessionWrapper;

// Initialize JNI cache
JNIEXPORT jint JNI_OnLoad(JavaVM* vm, void* reserved) {
    JNIEnv* env;
    if ((*vm)->GetEnv(vm, (void**)&env, JNI_VERSION_1_6) != JNI_OK) {
        return JNI_ERR;
    }
    
    // Cache exception class
    jclass localClass = (*env)->FindClass(env, "com/example/noisemobile/NoiseException$Unknown");
    if (localClass == NULL) {
        return JNI_ERR;
    }
    noiseExceptionClass = (*env)->NewGlobalRef(env, localClass);
    (*env)->DeleteLocalRef(env, localClass);
    
    // Cache constructor
    noiseExceptionConstructor = (*env)->GetMethodID(env, noiseExceptionClass, "<init>", "(I)V");
    if (noiseExceptionConstructor == NULL) {
        return JNI_ERR;
    }
    
    return JNI_VERSION_1_6;
}

// Cleanup JNI cache
JNIEXPORT void JNI_OnUnload(JavaVM* vm, void* reserved) {
    JNIEnv* env;
    if ((*vm)->GetEnv(vm, (void**)&env, JNI_VERSION_1_6) != JNI_OK) {
        return;
    }
    
    if (noiseExceptionClass != NULL) {
        (*env)->DeleteGlobalRef(env, noiseExceptionClass);
        noiseExceptionClass = NULL;
    }
}

// Helper function to create byte array from buffer
static jbyteArray create_byte_array(JNIEnv* env, const uint8_t* data, size_t len) {
    jbyteArray array = (*env)->NewByteArray(env, (jsize)len);
    if (array == NULL) {
        return NULL;
    }
    (*env)->SetByteArrayRegion(env, array, 0, (jsize)len, (const jbyte*)data);
    return array;
}

// Create initiator session
JNIEXPORT jlong JNICALL
Java_com_example_noisemobile_NoiseSession_nativeCreateInitiator(JNIEnv* env, jclass clazz) {
    int error = 0;
    struct NoiseSessionFFI* session = noise_session_new(0, &error);
    
    if (error != 0 || session == NULL) {
        return 0;
    }
    
    SessionWrapper* wrapper = malloc(sizeof(SessionWrapper));
    if (wrapper == NULL) {
        noise_session_free(session);
        return 0;
    }
    
    wrapper->session = session;
    wrapper->last_error = 0;
    
    return (jlong)(intptr_t)wrapper;
}

// Create responder session
JNIEXPORT jlong JNICALL
Java_com_example_noisemobile_NoiseSession_nativeCreateResponder(JNIEnv* env, jclass clazz) {
    int error = 0;
    struct NoiseSessionFFI* session = noise_session_new(1, &error);
    
    if (error != 0 || session == NULL) {
        return 0;
    }
    
    SessionWrapper* wrapper = malloc(sizeof(SessionWrapper));
    if (wrapper == NULL) {
        noise_session_free(session);
        return 0;
    }
    
    wrapper->session = session;
    wrapper->last_error = 0;
    
    return (jlong)(intptr_t)wrapper;
}

// Create initiator with key
JNIEXPORT jlong JNICALL
Java_com_example_noisemobile_NoiseSession_nativeCreateInitiatorWithKey(
    JNIEnv* env, jclass clazz, jbyteArray privateKey
) {
    jbyte* keyBytes = (*env)->GetByteArrayElements(env, privateKey, NULL);
    jsize keyLen = (*env)->GetArrayLength(env, privateKey);
    
    if (keyBytes == NULL || keyLen != 32) {
        if (keyBytes != NULL) {
            (*env)->ReleaseByteArrayElements(env, privateKey, keyBytes, JNI_ABORT);
        }
        return 0;
    }
    
    int error = 0;
    struct NoiseSessionFFI* session = noise_session_new_with_key(
        (const uint8_t*)keyBytes, (size_t)keyLen, 0, &error
    );
    
    (*env)->ReleaseByteArrayElements(env, privateKey, keyBytes, JNI_ABORT);
    
    if (error != 0 || session == NULL) {
        return 0;
    }
    
    SessionWrapper* wrapper = malloc(sizeof(SessionWrapper));
    if (wrapper == NULL) {
        noise_session_free(session);
        return 0;
    }
    
    wrapper->session = session;
    wrapper->last_error = 0;
    
    return (jlong)(intptr_t)wrapper;
}

// Create responder with key
JNIEXPORT jlong JNICALL
Java_com_example_noisemobile_NoiseSession_nativeCreateResponderWithKey(
    JNIEnv* env, jclass clazz, jbyteArray privateKey
) {
    jbyte* keyBytes = (*env)->GetByteArrayElements(env, privateKey, NULL);
    jsize keyLen = (*env)->GetArrayLength(env, privateKey);
    
    if (keyBytes == NULL || keyLen != 32) {
        if (keyBytes != NULL) {
            (*env)->ReleaseByteArrayElements(env, privateKey, keyBytes, JNI_ABORT);
        }
        return 0;
    }
    
    int error = 0;
    struct NoiseSessionFFI* session = noise_session_new_with_key(
        (const uint8_t*)keyBytes, (size_t)keyLen, 1, &error
    );
    
    (*env)->ReleaseByteArrayElements(env, privateKey, keyBytes, JNI_ABORT);
    
    if (error != 0 || session == NULL) {
        return 0;
    }
    
    SessionWrapper* wrapper = malloc(sizeof(SessionWrapper));
    if (wrapper == NULL) {
        noise_session_free(session);
        return 0;
    }
    
    wrapper->session = session;
    wrapper->last_error = 0;
    
    return (jlong)(intptr_t)wrapper;
}

// Destroy session
JNIEXPORT void JNICALL
Java_com_example_noisemobile_NoiseSession_nativeDestroy(JNIEnv* env, jobject obj, jlong handle) {
    if (handle == 0) return;
    
    SessionWrapper* wrapper = (SessionWrapper*)(intptr_t)handle;
    if (wrapper->session != NULL) {
        noise_session_free(wrapper->session);
    }
    free(wrapper);
}

// Get public key
JNIEXPORT jbyteArray JNICALL
Java_com_example_noisemobile_NoiseSession_nativeGetPublicKey(JNIEnv* env, jobject obj, jlong handle) {
    if (handle == 0) return NULL;
    
    SessionWrapper* wrapper = (SessionWrapper*)(intptr_t)handle;
    uint8_t pubkey[32];
    size_t pubkey_len = sizeof(pubkey);
    
    int result = noise_get_public_key(wrapper->session, pubkey, &pubkey_len);
    if (result != 0) {
        wrapper->last_error = result;
        return NULL;
    }
    
    return create_byte_array(env, pubkey, pubkey_len);
}

// Check if handshake is complete
JNIEXPORT jboolean JNICALL
Java_com_example_noisemobile_NoiseSession_nativeIsHandshakeComplete(JNIEnv* env, jobject obj, jlong handle) {
    if (handle == 0) return JNI_FALSE;
    
    SessionWrapper* wrapper = (SessionWrapper*)(intptr_t)handle;
    return noise_is_handshake_complete(wrapper->session) ? JNI_TRUE : JNI_FALSE;
}

// Write message
JNIEXPORT jbyteArray JNICALL
Java_com_example_noisemobile_NoiseSession_nativeWriteMessage(
    JNIEnv* env, jobject obj, jlong handle, jbyteArray payload
) {
    if (handle == 0) return NULL;
    
    SessionWrapper* wrapper = (SessionWrapper*)(intptr_t)handle;
    
    // Get payload data
    jbyte* payloadBytes = NULL;
    jsize payloadLen = 0;
    
    if (payload != NULL) {
        payloadBytes = (*env)->GetByteArrayElements(env, payload, NULL);
        payloadLen = (*env)->GetArrayLength(env, payload);
        if (payloadBytes == NULL) {
            wrapper->last_error = 1; // Invalid parameter
            return NULL;
        }
    }
    
    // Allocate output buffer
    uint8_t* output = malloc(65535);
    if (output == NULL) {
        if (payloadBytes != NULL) {
            (*env)->ReleaseByteArrayElements(env, payload, payloadBytes, JNI_ABORT);
        }
        wrapper->last_error = 2; // Out of memory
        return NULL;
    }
    
    size_t output_len = 65535;
    int result = noise_write_message(
        wrapper->session,
        payloadBytes != NULL ? (const uint8_t*)payloadBytes : NULL,
        (size_t)payloadLen,
        output,
        &output_len
    );
    
    if (payloadBytes != NULL) {
        (*env)->ReleaseByteArrayElements(env, payload, payloadBytes, JNI_ABORT);
    }
    
    if (result != 0) {
        free(output);
        wrapper->last_error = result;
        return NULL;
    }
    
    jbyteArray resultArray = create_byte_array(env, output, output_len);
    free(output);
    
    return resultArray;
}

// Read message
JNIEXPORT jbyteArray JNICALL
Java_com_example_noisemobile_NoiseSession_nativeReadMessage(
    JNIEnv* env, jobject obj, jlong handle, jbyteArray message
) {
    if (handle == 0 || message == NULL) return NULL;
    
    SessionWrapper* wrapper = (SessionWrapper*)(intptr_t)handle;
    
    // Get message data
    jbyte* messageBytes = (*env)->GetByteArrayElements(env, message, NULL);
    jsize messageLen = (*env)->GetArrayLength(env, message);
    
    if (messageBytes == NULL) {
        wrapper->last_error = 1; // Invalid parameter
        return NULL;
    }
    
    // Allocate output buffer
    uint8_t* output = malloc(65535);
    if (output == NULL) {
        (*env)->ReleaseByteArrayElements(env, message, messageBytes, JNI_ABORT);
        wrapper->last_error = 2; // Out of memory
        return NULL;
    }
    
    size_t output_len = 65535;
    int result = noise_read_message(
        wrapper->session,
        (const uint8_t*)messageBytes,
        (size_t)messageLen,
        output,
        &output_len
    );
    
    (*env)->ReleaseByteArrayElements(env, message, messageBytes, JNI_ABORT);
    
    if (result != 0) {
        free(output);
        wrapper->last_error = result;
        return NULL;
    }
    
    jbyteArray resultArray = create_byte_array(env, output, output_len);
    free(output);
    
    return resultArray;
}

// Encrypt
JNIEXPORT jbyteArray JNICALL
Java_com_example_noisemobile_NoiseSession_nativeEncrypt(
    JNIEnv* env, jobject obj, jlong handle, jbyteArray plaintext
) {
    if (handle == 0 || plaintext == NULL) return NULL;
    
    SessionWrapper* wrapper = (SessionWrapper*)(intptr_t)handle;
    
    // Get plaintext data
    jbyte* plaintextBytes = (*env)->GetByteArrayElements(env, plaintext, NULL);
    jsize plaintextLen = (*env)->GetArrayLength(env, plaintext);
    
    if (plaintextBytes == NULL) {
        wrapper->last_error = 1; // Invalid parameter
        return NULL;
    }
    
    // Allocate output buffer (plaintext + 16 bytes for tag)
    size_t output_size = (size_t)plaintextLen + 16;
    uint8_t* output = malloc(output_size);
    if (output == NULL) {
        (*env)->ReleaseByteArrayElements(env, plaintext, plaintextBytes, JNI_ABORT);
        wrapper->last_error = 2; // Out of memory
        return NULL;
    }
    
    size_t output_len = output_size;
    int result = noise_encrypt(
        wrapper->session,
        (const uint8_t*)plaintextBytes,
        (size_t)plaintextLen,
        output,
        &output_len
    );
    
    (*env)->ReleaseByteArrayElements(env, plaintext, plaintextBytes, JNI_ABORT);
    
    if (result != 0) {
        free(output);
        wrapper->last_error = result;
        return NULL;
    }
    
    jbyteArray resultArray = create_byte_array(env, output, output_len);
    free(output);
    
    return resultArray;
}

// Decrypt
JNIEXPORT jbyteArray JNICALL
Java_com_example_noisemobile_NoiseSession_nativeDecrypt(
    JNIEnv* env, jobject obj, jlong handle, jbyteArray ciphertext
) {
    if (handle == 0 || ciphertext == NULL) return NULL;
    
    SessionWrapper* wrapper = (SessionWrapper*)(intptr_t)handle;
    
    // Get ciphertext data
    jbyte* ciphertextBytes = (*env)->GetByteArrayElements(env, ciphertext, NULL);
    jsize ciphertextLen = (*env)->GetArrayLength(env, ciphertext);
    
    if (ciphertextBytes == NULL) {
        wrapper->last_error = 1; // Invalid parameter
        return NULL;
    }
    
    // Allocate output buffer
    uint8_t* output = malloc((size_t)ciphertextLen);
    if (output == NULL) {
        (*env)->ReleaseByteArrayElements(env, ciphertext, ciphertextBytes, JNI_ABORT);
        wrapper->last_error = 2; // Out of memory
        return NULL;
    }
    
    size_t output_len = (size_t)ciphertextLen;
    int result = noise_decrypt(
        wrapper->session,
        (const uint8_t*)ciphertextBytes,
        (size_t)ciphertextLen,
        output,
        &output_len
    );
    
    (*env)->ReleaseByteArrayElements(env, ciphertext, ciphertextBytes, JNI_ABORT);
    
    if (result != 0) {
        free(output);
        wrapper->last_error = result;
        return NULL;
    }
    
    jbyteArray resultArray = create_byte_array(env, output, output_len);
    free(output);
    
    return resultArray;
}

// Get last error
JNIEXPORT jint JNICALL
Java_com_example_noisemobile_NoiseSession_nativeGetLastError(JNIEnv* env, jobject obj, jlong handle) {
    if (handle == 0) return 1; // Invalid parameter
    
    SessionWrapper* wrapper = (SessionWrapper*)(intptr_t)handle;
    return wrapper->last_error;
}