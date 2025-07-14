import Foundation

/// Noise session mode
public enum NoiseMode: Int32 {
    case initiator = 0
    case responder = 1
}

/// Noise protocol errors
public enum NoiseError: Error {
    case invalidParameter
    case outOfMemory
    case handshakeFailed
    case encryptionFailed
    case decryptionFailed
    case bufferTooSmall(needed: Int)
    case invalidState
    case protocolError
    case unknown(Int32)
    
    init(code: Int32) {
        switch code {
        case 1: self = .invalidParameter
        case 2: self = .outOfMemory
        case 3: self = .handshakeFailed
        case 4: self = .encryptionFailed
        case 5: self = .decryptionFailed
        case 6: self = .bufferTooSmall(needed: 0)
        case 7: self = .invalidState
        case 8: self = .protocolError
        default: self = .unknown(code)
        }
    }
}

/// Swift wrapper for Noise protocol sessions
public class NoiseSession {
    private var session: OpaquePointer?
    
    /// Create a new Noise session
    public init(mode: NoiseMode) throws {
        var error: Int32 = 0
        session = noise_session_new(mode.rawValue, &error)
        
        if error != 0 {
            throw NoiseError(code: error)
        }
        
        if session == nil {
            throw NoiseError.outOfMemory
        }
    }
    
    /// Create a new Noise session with a specific private key
    public init(mode: NoiseMode, privateKey: Data) throws {
        guard privateKey.count == 32 else {
            throw NoiseError.invalidParameter
        }
        
        var error: Int32 = 0
        session = privateKey.withUnsafeBytes { keyBytes in
            noise_session_new_with_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress,
                privateKey.count,
                mode.rawValue,
                &error
            )
        }
        
        if error != 0 {
            throw NoiseError(code: error)
        }
        
        if session == nil {
            throw NoiseError.outOfMemory
        }
    }
    
    deinit {
        if let session = session {
            noise_session_free(session)
        }
    }
    
    /// Get the local public key
    public var publicKey: Data {
        get throws {
            guard let session = session else {
                throw NoiseError.invalidState
            }
            
            var keyBuffer = Data(count: 32)
            var keyLen = keyBuffer.count
            
            let result = keyBuffer.withUnsafeMutableBytes { buffer in
                noise_get_public_key(
                    session,
                    buffer.bindMemory(to: UInt8.self).baseAddress,
                    &keyLen
                )
            }
            
            if result != 0 {
                throw NoiseError(code: result)
            }
            
            return keyBuffer
        }
    }
    
    /// Check if handshake is complete
    public var isHandshakeComplete: Bool {
        guard let session = session else { return false }
        return noise_is_handshake_complete(session) != 0
    }
    
    /// Write a handshake or transport message
    public func writeMessage(_ payload: Data = Data()) throws -> Data {
        guard let session = session else {
            throw NoiseError.invalidState
        }
        
        // Allocate buffer for output (max message size + overhead)
        var outputBuffer = Data(count: 65535)
        var outputLen = outputBuffer.count
        
        let result = outputBuffer.withUnsafeMutableBytes { output in
            if payload.isEmpty {
                return noise_write_message(
                    session,
                    nil,
                    0,
                    output.bindMemory(to: UInt8.self).baseAddress,
                    &outputLen
                )
            } else {
                return payload.withUnsafeBytes { input in
                    noise_write_message(
                        session,
                        input.bindMemory(to: UInt8.self).baseAddress,
                        payload.count,
                        output.bindMemory(to: UInt8.self).baseAddress,
                        &outputLen
                    )
                }
            }
        }
        
        if result == 6 { // BUFFER_TOO_SMALL
            throw NoiseError.bufferTooSmall(needed: outputLen)
        } else if result != 0 {
            throw NoiseError(code: result)
        }
        
        return outputBuffer.prefix(outputLen)
    }
    
    /// Read a handshake or transport message
    public func readMessage(_ message: Data) throws -> Data {
        guard let session = session else {
            throw NoiseError.invalidState
        }
        
        var outputBuffer = Data(count: 65535)
        var outputLen = outputBuffer.count
        
        let result = outputBuffer.withUnsafeMutableBytes { output in
            message.withUnsafeBytes { input in
                noise_read_message(
                    session,
                    input.bindMemory(to: UInt8.self).baseAddress,
                    message.count,
                    output.bindMemory(to: UInt8.self).baseAddress,
                    &outputLen
                )
            }
        }
        
        if result == 6 { // BUFFER_TOO_SMALL
            throw NoiseError.bufferTooSmall(needed: outputLen)
        } else if result != 0 {
            throw NoiseError(code: result)
        }
        
        return outputBuffer.prefix(outputLen)
    }
    
    /// Encrypt a message (transport mode only)
    public func encrypt(_ plaintext: Data) throws -> Data {
        guard let session = session else {
            throw NoiseError.invalidState
        }
        
        var ciphertext = Data(count: plaintext.count + 16) // Add AEAD tag
        var cipherLen = ciphertext.count
        
        let result = ciphertext.withUnsafeMutableBytes { cipher in
            plaintext.withUnsafeBytes { plain in
                noise_encrypt(
                    session,
                    plain.bindMemory(to: UInt8.self).baseAddress,
                    plaintext.count,
                    cipher.bindMemory(to: UInt8.self).baseAddress,
                    &cipherLen
                )
            }
        }
        
        if result != 0 {
            throw NoiseError(code: result)
        }
        
        return ciphertext.prefix(cipherLen)
    }
    
    /// Decrypt a message (transport mode only)
    public func decrypt(_ ciphertext: Data) throws -> Data {
        guard let session = session else {
            throw NoiseError.invalidState
        }
        
        var plaintext = Data(count: ciphertext.count)
        var plainLen = plaintext.count
        
        let result = plaintext.withUnsafeMutableBytes { plain in
            ciphertext.withUnsafeBytes { cipher in
                noise_decrypt(
                    session,
                    cipher.bindMemory(to: UInt8.self).baseAddress,
                    ciphertext.count,
                    plain.bindMemory(to: UInt8.self).baseAddress,
                    &plainLen
                )
            }
        }
        
        if result != 0 {
            throw NoiseError(code: result)
        }
        
        return plaintext.prefix(plainLen)
    }
}

// MARK: - Convenience Extensions

extension NoiseSession {
    /// Perform a complete handshake as initiator
    public static func performHandshake(
        initiator: NoiseSession,
        responder: NoiseSession
    ) throws {
        // Message 1: initiator -> responder
        let msg1 = try initiator.writeMessage()
        _ = try responder.readMessage(msg1)
        
        // Message 2: responder -> initiator
        let msg2 = try responder.writeMessage()
        _ = try initiator.readMessage(msg2)
        
        // Message 3: initiator -> responder
        let msg3 = try initiator.writeMessage()
        _ = try responder.readMessage(msg3)
        
        // Verify both are in transport mode
        guard initiator.isHandshakeComplete && responder.isHandshakeComplete else {
            throw NoiseError.handshakeFailed
        }
    }
}

// MARK: - Hex String Support

extension Data {
    /// Convert data to hex string
    var hexString: String {
        map { String(format: "%02hhx", $0) }.joined()
    }
    
    /// Create data from hex string
    init?(hexString: String) {
        let len = hexString.count / 2
        var data = Data(capacity: len)
        var index = hexString.startIndex
        
        for _ in 0..<len {
            let nextIndex = hexString.index(index, offsetBy: 2)
            guard let byte = UInt8(hexString[index..<nextIndex], radix: 16) else {
                return nil
            }
            data.append(byte)
            index = nextIndex
        }
        
        self = data
    }
}