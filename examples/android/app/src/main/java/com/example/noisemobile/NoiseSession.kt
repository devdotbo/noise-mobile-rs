package com.example.noisemobile

import java.nio.ByteBuffer

/**
 * Noise session mode
 */
enum class NoiseMode(val value: Int) {
    INITIATOR(0),
    RESPONDER(1)
}

/**
 * Noise protocol exceptions
 */
sealed class NoiseException(message: String) : Exception(message) {
    class InvalidParameter : NoiseException("Invalid parameter provided")
    class OutOfMemory : NoiseException("Out of memory")
    class HandshakeFailed : NoiseException("Handshake failed")
    class EncryptionFailed : NoiseException("Encryption failed")
    class DecryptionFailed : NoiseException("Decryption failed")
    class BufferTooSmall(val needed: Int) : NoiseException("Buffer too small, need $needed bytes")
    class InvalidState : NoiseException("Operation invalid in current state")
    class ProtocolError : NoiseException("Protocol error")
    class Unknown(val code: Int) : NoiseException("Unknown error: $code")
}

/**
 * Kotlin wrapper for Noise protocol sessions
 */
class NoiseSession private constructor(
    private val nativeHandle: Long
) : AutoCloseable {
    
    companion object {
        init {
            System.loadLibrary("noise_mobile")
        }
        
        /**
         * Create a new Noise session
         */
        @JvmStatic
        fun create(mode: NoiseMode): NoiseSession {
            val handle = when (mode) {
                NoiseMode.INITIATOR -> nativeCreateInitiator()
                NoiseMode.RESPONDER -> nativeCreateResponder()
            }
            
            if (handle == 0L) {
                throw NoiseException.OutOfMemory()
            }
            
            return NoiseSession(handle)
        }
        
        /**
         * Create a new Noise session with a specific private key
         */
        @JvmStatic
        fun createWithKey(mode: NoiseMode, privateKey: ByteArray): NoiseSession {
            if (privateKey.size != 32) {
                throw NoiseException.InvalidParameter()
            }
            
            val handle = when (mode) {
                NoiseMode.INITIATOR -> nativeCreateInitiatorWithKey(privateKey)
                NoiseMode.RESPONDER -> nativeCreateResponderWithKey(privateKey)
            }
            
            if (handle == 0L) {
                throw NoiseException.OutOfMemory()
            }
            
            return NoiseSession(handle)
        }
        
        // Native method declarations
        @JvmStatic
        private external fun nativeCreateInitiator(): Long
        
        @JvmStatic
        private external fun nativeCreateResponder(): Long
        
        @JvmStatic
        private external fun nativeCreateInitiatorWithKey(privateKey: ByteArray): Long
        
        @JvmStatic
        private external fun nativeCreateResponderWithKey(privateKey: ByteArray): Long
    }
    
    private var closed = false
    
    /**
     * Get the local public key
     */
    val publicKey: ByteArray
        get() {
            checkNotClosed()
            return nativeGetPublicKey(nativeHandle)
                ?: throw NoiseException.InvalidState()
        }
    
    /**
     * Check if handshake is complete
     */
    val isHandshakeComplete: Boolean
        get() {
            checkNotClosed()
            return nativeIsHandshakeComplete(nativeHandle)
        }
    
    /**
     * Write a handshake or transport message
     */
    fun writeMessage(payload: ByteArray = byteArrayOf()): ByteArray {
        checkNotClosed()
        
        val result = nativeWriteMessage(nativeHandle, payload)
        return result ?: throw getLastError()
    }
    
    /**
     * Read a handshake or transport message
     */
    fun readMessage(message: ByteArray): ByteArray {
        checkNotClosed()
        
        val result = nativeReadMessage(nativeHandle, message)
        return result ?: throw getLastError()
    }
    
    /**
     * Encrypt a message (transport mode only)
     */
    fun encrypt(plaintext: ByteArray): ByteArray {
        checkNotClosed()
        
        val result = nativeEncrypt(nativeHandle, plaintext)
        return result ?: throw getLastError()
    }
    
    /**
     * Decrypt a message (transport mode only)
     */
    fun decrypt(ciphertext: ByteArray): ByteArray {
        checkNotClosed()
        
        val result = nativeDecrypt(nativeHandle, ciphertext)
        return result ?: throw getLastError()
    }
    
    /**
     * Close the session and free resources
     */
    override fun close() {
        if (!closed && nativeHandle != 0L) {
            nativeDestroy(nativeHandle)
            closed = true
        }
    }
    
    private fun checkNotClosed() {
        if (closed) {
            throw IllegalStateException("Session is closed")
        }
    }
    
    private fun getLastError(): NoiseException {
        val errorCode = nativeGetLastError(nativeHandle)
        return when (errorCode) {
            1 -> NoiseException.InvalidParameter()
            2 -> NoiseException.OutOfMemory()
            3 -> NoiseException.HandshakeFailed()
            4 -> NoiseException.EncryptionFailed()
            5 -> NoiseException.DecryptionFailed()
            6 -> NoiseException.BufferTooSmall(0) // TODO: Get actual needed size
            7 -> NoiseException.InvalidState()
            8 -> NoiseException.ProtocolError()
            else -> NoiseException.Unknown(errorCode)
        }
    }
    
    // Native methods
    private external fun nativeDestroy(handle: Long)
    private external fun nativeGetPublicKey(handle: Long): ByteArray?
    private external fun nativeIsHandshakeComplete(handle: Long): Boolean
    private external fun nativeWriteMessage(handle: Long, payload: ByteArray): ByteArray?
    private external fun nativeReadMessage(handle: Long, message: ByteArray): ByteArray?
    private external fun nativeEncrypt(handle: Long, plaintext: ByteArray): ByteArray?
    private external fun nativeDecrypt(handle: Long, ciphertext: ByteArray): ByteArray?
    private external fun nativeGetLastError(handle: Long): Int
}

/**
 * Extension functions for convenience
 */
object NoiseProtocol {
    /**
     * Perform a complete handshake between two sessions
     */
    fun performHandshake(initiator: NoiseSession, responder: NoiseSession) {
        // Message 1: initiator -> responder
        val msg1 = initiator.writeMessage()
        responder.readMessage(msg1)
        
        // Message 2: responder -> initiator
        val msg2 = responder.writeMessage()
        initiator.readMessage(msg2)
        
        // Message 3: initiator -> responder
        val msg3 = initiator.writeMessage()
        responder.readMessage(msg3)
        
        // Verify both are in transport mode
        if (!initiator.isHandshakeComplete || !responder.isHandshakeComplete) {
            throw NoiseException.HandshakeFailed()
        }
    }
}

/**
 * Extension functions for ByteArray
 */
fun ByteArray.toHexString(): String {
    return joinToString("") { "%02x".format(it) }
}

fun String.hexToByteArray(): ByteArray {
    check(length % 2 == 0) { "Hex string must have even length" }
    
    return chunked(2)
        .map { it.toInt(16).toByte() }
        .toByteArray()
}