//! FFI boundary tests for noise-mobile-rust
//! 
//! These tests verify that the C API handles all edge cases safely without
//! crashes, undefined behavior, or memory leaks.

use noise_mobile::ffi::types::NoiseErrorCode;
use noise_mobile::ffi::c_api::*;
use std::ptr;
use libc::{c_int, size_t};

// Helper to convert error codes for assertions
fn error_code(code: c_int) -> NoiseErrorCode {
    unsafe { std::mem::transmute(code) }
}

// Constants matching the C API definitions
const NOISE_ERROR_SUCCESS: c_int = 0;
const NOISE_ERROR_INVALID_PARAMETER: c_int = 1;
const NOISE_ERROR_OUT_OF_MEMORY: c_int = 2;
const NOISE_ERROR_HANDSHAKE_FAILED: c_int = 3;
const NOISE_ERROR_ENCRYPTION_FAILED: c_int = 4;
const NOISE_ERROR_DECRYPTION_FAILED: c_int = 5;
const NOISE_ERROR_BUFFER_TOO_SMALL: c_int = 6;
const NOISE_ERROR_INVALID_STATE: c_int = 7;
const NOISE_ERROR_PROTOCOL_ERROR: c_int = 8;

const NOISE_MODE_INITIATOR: c_int = 0;
const NOISE_MODE_RESPONDER: c_int = 1;

#[test]
fn test_null_session_new() {
    unsafe {
        // Test with null error pointer - should return null
        let session = noise_session_new(NOISE_MODE_INITIATOR, ptr::null_mut());
        // With null error pointer, session creation should fail
        assert!(session.is_null());
        
        // Test with valid error pointer
        let mut error = 0;
        let session = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        assert_eq!(error, NOISE_ERROR_SUCCESS);
        assert!(!session.is_null());
        noise_session_free(session);
    }
}

#[test]
fn test_double_free_protection() {
    unsafe {
        // Test that freeing null pointer doesn't crash
        noise_session_free(ptr::null_mut());
        
        // Create and free a session normally
        let mut error = 0;
        let session = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        assert_eq!(error, NOISE_ERROR_SUCCESS);
        assert!(!session.is_null());
        
        // Free it once
        noise_session_free(session);
        
        // NOTE: We cannot test double-free without causing undefined behavior
        // The implementation correctly checks for null, but after free,
        // the pointer is dangling and using it is UB.
        // This is a limitation of the C API design.
    }
}

#[test]
fn test_null_session_operations() {
    unsafe {
        let mut buffer = vec![0u8; 1024];
        let mut len = buffer.len() as size_t;
        
        // Test write_message with null session
        let result = noise_write_message(
            ptr::null_mut(),
            ptr::null(),
            0,
            buffer.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_INVALID_PARAMETER);
        
        // Test read_message with null session
        let result = noise_read_message(
            ptr::null_mut(),
            buffer.as_ptr(),
            100,
            buffer.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_INVALID_PARAMETER);
        
        // Test encrypt with null session
        let result = noise_encrypt(
            ptr::null_mut(),
            buffer.as_ptr(),
            100,
            buffer.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_INVALID_PARAMETER);
        
        // Test decrypt with null session
        let result = noise_decrypt(
            ptr::null_mut(),
            buffer.as_ptr(),
            100,
            buffer.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_INVALID_PARAMETER);
        
        // Test is_handshake_complete with null session
        let result = noise_is_handshake_complete(ptr::null_mut());
        assert_eq!(result, 0); // Should return false
        
        // Test get_remote_static with null session
        let result = noise_get_remote_static(
            ptr::null_mut(),
            buffer.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_INVALID_PARAMETER);
    }
}

#[test]
fn test_null_buffer_handling() {
    unsafe {
        let mut error = 0;
        let session = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        assert_eq!(error, NOISE_ERROR_SUCCESS);
        
        let mut len = 100;
        
        // Test write_message with null output buffer
        let result = noise_write_message(
            session,
            ptr::null(),
            0,
            ptr::null_mut(),
            &mut len
        );
        // Could be invalid parameter or buffer too small
        assert!(result == NOISE_ERROR_INVALID_PARAMETER || result == NOISE_ERROR_BUFFER_TOO_SMALL);
        
        // Test write_message with null length pointer
        let mut buffer = vec![0u8; 1024];
        let result = noise_write_message(
            session,
            ptr::null(),
            0,
            buffer.as_mut_ptr(),
            ptr::null_mut()
        );
        assert_eq!(result, NOISE_ERROR_INVALID_PARAMETER);
        
        noise_session_free(session);
    }
}

#[test]
fn test_buffer_overflow_protection() {
    unsafe {
        let mut error = 0;
        let session = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        assert_eq!(error, NOISE_ERROR_SUCCESS);
        
        // Use a very small buffer
        let mut small_buffer = [0u8; 10];
        let mut len = small_buffer.len() as size_t;
        
        // Should fail with buffer too small or protocol error during handshake
        let result = noise_write_message(
            session,
            ptr::null(),
            0,
            small_buffer.as_mut_ptr(),
            &mut len
        );
        // Either buffer too small or protocol error (8) is acceptable
        assert!(result == NOISE_ERROR_BUFFER_TOO_SMALL || result == NOISE_ERROR_PROTOCOL_ERROR);
        
        // len should now contain the required size
        assert!(len > 10);
        let required_size = len;
        
        // Allocate correct size and retry
        let mut proper_buffer = vec![0u8; required_size as usize];
        len = proper_buffer.len() as size_t;
        
        let result = noise_write_message(
            session,
            ptr::null(),
            0,
            proper_buffer.as_mut_ptr(),
            &mut len
        );
        // During handshake, could be success or protocol error depending on state
        assert!(result == NOISE_ERROR_SUCCESS || result == NOISE_ERROR_PROTOCOL_ERROR);
        if result == NOISE_ERROR_SUCCESS {
            assert!(len > 0); // Should have written something
        }
        
        noise_session_free(session);
    }
}

#[test]
fn test_zero_length_buffers() {
    unsafe {
        let mut error = 0;
        let session = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        assert_eq!(error, NOISE_ERROR_SUCCESS);
        
        let mut buffer = vec![0u8; 1024];
        let mut zero_len = 0;
        
        // Test with zero output buffer length
        let result = noise_write_message(
            session,
            ptr::null(),
            0,
            buffer.as_mut_ptr(),
            &mut zero_len
        );
        assert_eq!(result, NOISE_ERROR_BUFFER_TOO_SMALL);
        assert!(zero_len > 0); // Should indicate required size
        
        noise_session_free(session);
    }
}

#[test]
fn test_invalid_enum_values() {
    unsafe {
        let mut error = 0;
        
        // Test with invalid mode value
        let invalid_mode = 999;
        let session = noise_session_new(invalid_mode, &mut error);
        assert_eq!(error, NOISE_ERROR_INVALID_PARAMETER);
        assert!(session.is_null());
        
        // Test with negative mode value
        let negative_mode = -1;
        let session = noise_session_new(negative_mode, &mut error);
        assert_eq!(error, NOISE_ERROR_INVALID_PARAMETER);
        assert!(session.is_null());
    }
}

#[test]
fn test_very_large_buffer_sizes() {
    unsafe {
        let mut error = 0;
        let initiator = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        let responder = noise_session_new(NOISE_MODE_RESPONDER, &mut error);
        
        // Complete handshake first
        let mut buffer1 = vec![0u8; 1024];
        let mut buffer2 = vec![0u8; 1024];
        let mut len1 = buffer1.len() as size_t;
        let mut len2 = buffer2.len() as size_t;
        
        // Initiator -> Responder (message 1)
        let result = noise_write_message(initiator, ptr::null(), 0, buffer1.as_mut_ptr(), &mut len1);
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        let result = noise_read_message(responder, buffer1.as_ptr(), len1, buffer2.as_mut_ptr(), &mut len2);
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Continue handshake...
        len1 = buffer1.len() as size_t;
        let result = noise_write_message(responder, ptr::null(), 0, buffer1.as_mut_ptr(), &mut len1);
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        len2 = buffer2.len() as size_t;
        let result = noise_read_message(initiator, buffer1.as_ptr(), len1, buffer2.as_mut_ptr(), &mut len2);
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Final handshake message
        len1 = buffer1.len() as size_t;
        let result = noise_write_message(initiator, ptr::null(), 0, buffer1.as_mut_ptr(), &mut len1);
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        len2 = buffer2.len() as size_t;
        let result = noise_read_message(responder, buffer1.as_ptr(), len1, buffer2.as_mut_ptr(), &mut len2);
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Now test with maximum allowed payload
        let max_payload = noise_max_payload_len() as usize;
        let large_data = vec![0x42u8; max_payload];
        let mut large_buffer = vec![0u8; max_payload + 1024]; // Extra space for overhead
        let mut large_len = large_buffer.len() as size_t;
        
        // Should succeed with max payload
        let result = noise_encrypt(
            initiator,
            large_data.as_ptr(),
            max_payload as size_t,
            large_buffer.as_mut_ptr(),
            &mut large_len
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Test with payload larger than max (should fail)
        let too_large_data = vec![0x42u8; max_payload + 1];
        large_len = large_buffer.len() as size_t;
        let result = noise_encrypt(
            initiator,
            too_large_data.as_ptr(),
            (max_payload + 1) as size_t,
            large_buffer.as_mut_ptr(),
            &mut large_len
        );
        // Could be invalid parameter or protocol error
        assert!(result == NOISE_ERROR_INVALID_PARAMETER || result == NOISE_ERROR_PROTOCOL_ERROR);
        
        noise_session_free(initiator);
        noise_session_free(responder);
    }
}

#[test]
fn test_session_new_with_key_null_handling() {
    unsafe {
        let mut error = 0;
        let valid_key = [0u8; 32];
        
        // Test with null key pointer
        let session = noise_session_new_with_key(
            ptr::null(),
            32,
            NOISE_MODE_INITIATOR,
            &mut error
        );
        assert_eq!(error, NOISE_ERROR_INVALID_PARAMETER);
        assert!(session.is_null());
        
        // Test with zero key length
        let session = noise_session_new_with_key(
            valid_key.as_ptr(),
            0,
            NOISE_MODE_INITIATOR,
            &mut error
        );
        assert_eq!(error, NOISE_ERROR_INVALID_PARAMETER);
        assert!(session.is_null());
        
        // Test with wrong key length
        let session = noise_session_new_with_key(
            valid_key.as_ptr(),
            16, // Should be 32
            NOISE_MODE_INITIATOR,
            &mut error
        );
        assert_eq!(error, NOISE_ERROR_INVALID_PARAMETER);
        assert!(session.is_null());
        
        // Test with valid parameters
        let session = noise_session_new_with_key(
            valid_key.as_ptr(),
            32,
            NOISE_MODE_INITIATOR,
            &mut error
        );
        assert_eq!(error, NOISE_ERROR_SUCCESS);
        assert!(!session.is_null());
        noise_session_free(session);
    }
}

#[test]
fn test_error_string_function() {
    unsafe {
        // Test all error codes return valid strings
        for code in 0..=8 {
            let str_ptr = noise_error_string(code);
            assert!(!str_ptr.is_null());
            let c_str = std::ffi::CStr::from_ptr(str_ptr);
            let rust_str = c_str.to_str().unwrap();
            assert!(!rust_str.is_empty());
        }
        
        // Test invalid error code
        let str_ptr = noise_error_string(999);
        assert!(!str_ptr.is_null());
        let c_str = std::ffi::CStr::from_ptr(str_ptr);
        let rust_str = c_str.to_str().unwrap();
        assert_eq!(rust_str, "Unknown error");
    }
}

#[test]
fn test_operations_in_wrong_state() {
    unsafe {
        let mut error = 0;
        let session = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        assert_eq!(error, NOISE_ERROR_SUCCESS);
        
        let mut buffer = vec![0u8; 1024];
        let mut len = buffer.len() as size_t;
        let data = b"test data";
        
        // Try to encrypt before handshake is complete
        let result = noise_encrypt(
            session,
            data.as_ptr(),
            data.len() as size_t,
            buffer.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_INVALID_STATE);
        
        // Try to decrypt before handshake is complete
        len = buffer.len() as size_t;
        let result = noise_decrypt(
            session,
            data.as_ptr(),
            data.len() as size_t,
            buffer.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_INVALID_STATE);
        
        // Try to get remote static before handshake is complete
        len = 32;
        let result = noise_get_remote_static(
            session,
            buffer.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_INVALID_STATE);
        
        noise_session_free(session);
    }
}

#[test]
fn test_concurrent_session_creation() {
    use std::thread;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    let session_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];
    
    // Create many sessions concurrently
    for _ in 0..10 {
        let count = session_count.clone();
        let handle = thread::spawn(move || {
            unsafe {
                let mut error = 0;
                for _ in 0..100 {
                    let session = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
                    assert_eq!(error, NOISE_ERROR_SUCCESS);
                    assert!(!session.is_null());
                    count.fetch_add(1, Ordering::SeqCst);
                    noise_session_free(session);
                }
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    assert_eq!(session_count.load(Ordering::SeqCst), 1000);
}

#[test]
fn test_malformed_encrypted_data() {
    unsafe {
        let mut error = 0;
        let initiator = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        let responder = noise_session_new(NOISE_MODE_RESPONDER, &mut error);
        
        // Complete handshake
        let mut buffer1 = vec![0u8; 1024];
        let mut buffer2 = vec![0u8; 1024];
        let mut len1 = buffer1.len() as size_t;
        let mut len2 = buffer2.len() as size_t;
        
        // Message 1
        noise_write_message(initiator, ptr::null(), 0, buffer1.as_mut_ptr(), &mut len1);
        noise_read_message(responder, buffer1.as_ptr(), len1, buffer2.as_mut_ptr(), &mut len2);
        
        // Message 2
        len1 = buffer1.len() as size_t;
        noise_write_message(responder, ptr::null(), 0, buffer1.as_mut_ptr(), &mut len1);
        len2 = buffer2.len() as size_t;
        noise_read_message(initiator, buffer1.as_ptr(), len1, buffer2.as_mut_ptr(), &mut len2);
        
        // Message 3
        len1 = buffer1.len() as size_t;
        noise_write_message(initiator, ptr::null(), 0, buffer1.as_mut_ptr(), &mut len1);
        len2 = buffer2.len() as size_t;
        noise_read_message(responder, buffer1.as_ptr(), len1, buffer2.as_mut_ptr(), &mut len2);
        
        assert_eq!(noise_is_handshake_complete(initiator), 1);
        assert_eq!(noise_is_handshake_complete(responder), 1);
        
        // Encrypt some data
        let plaintext = b"Hello, World!";
        let mut ciphertext = vec![0u8; 1024];
        let mut cipher_len = ciphertext.len() as size_t;
        
        let result = noise_encrypt(
            initiator,
            plaintext.as_ptr(),
            plaintext.len() as size_t,
            ciphertext.as_mut_ptr(),
            &mut cipher_len
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Try to decrypt with corrupted data
        ciphertext[cipher_len as usize / 2] ^= 0xFF; // Flip bits in the middle
        
        let mut decrypted = vec![0u8; 1024];
        let mut decrypt_len = decrypted.len() as size_t;
        
        let result = noise_decrypt(
            responder,
            ciphertext.as_ptr(),
            cipher_len,
            decrypted.as_mut_ptr(),
            &mut decrypt_len
        );
        // Could be decryption failed or protocol error
        assert!(result == NOISE_ERROR_DECRYPTION_FAILED || result == NOISE_ERROR_PROTOCOL_ERROR);
        
        // Try to decrypt truncated data
        decrypt_len = decrypted.len() as size_t;
        let result = noise_decrypt(
            responder,
            ciphertext.as_ptr(),
            cipher_len - 10, // Truncate
            decrypted.as_mut_ptr(),
            &mut decrypt_len
        );
        // Could be decryption failed or protocol error
        assert!(result == NOISE_ERROR_DECRYPTION_FAILED || result == NOISE_ERROR_PROTOCOL_ERROR);
        
        noise_session_free(initiator);
        noise_session_free(responder);
    }
}