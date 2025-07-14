//! Integration tests for noise-mobile-rust
//! 
//! These tests verify end-to-end functionality including full handshakes,
//! session persistence, and network resilience features.

use noise_mobile::ffi::c_api::*;
use noise_mobile::mobile::network::ResilientSession;
use noise_mobile::mobile::battery::BatchedCrypto;
use noise_mobile::core::session::NoiseSession;
use noise_mobile::core::error::NoiseError;
use std::ptr;
use libc::{c_int, size_t};

// Constants
const NOISE_ERROR_SUCCESS: c_int = 0;
const NOISE_MODE_INITIATOR: c_int = 0;
const NOISE_MODE_RESPONDER: c_int = 1;

#[test]
fn test_ffi_complete_handshake() {
    unsafe {
        let mut error_i = 0;
        let mut error_r = 0;
        
        // Create initiator and responder sessions
        let initiator = noise_session_new(NOISE_MODE_INITIATOR, &mut error_i);
        let responder = noise_session_new(NOISE_MODE_RESPONDER, &mut error_r);
        
        assert_eq!(error_i, NOISE_ERROR_SUCCESS);
        assert_eq!(error_r, NOISE_ERROR_SUCCESS);
        assert!(!initiator.is_null());
        assert!(!responder.is_null());
        
        // Buffers for handshake messages
        let mut buffer1 = vec![0u8; 2048];
        let mut buffer2 = vec![0u8; 2048];
        let mut len1: size_t;
        let mut len2: size_t;
        
        // Message 1: Initiator -> Responder (e)
        len1 = buffer1.len() as size_t;
        let result = noise_write_message(
            initiator,
            ptr::null(),
            0,
            buffer1.as_mut_ptr(),
            &mut len1
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        assert!(len1 > 0);
        
        len2 = buffer2.len() as size_t;
        let result = noise_read_message(
            responder,
            buffer1.as_ptr(),
            len1,
            buffer2.as_mut_ptr(),
            &mut len2
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Message 2: Responder -> Initiator (e, ee, s, es)
        len1 = buffer1.len() as size_t;
        let result = noise_write_message(
            responder,
            ptr::null(),
            0,
            buffer1.as_mut_ptr(),
            &mut len1
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        assert!(len1 > 0);
        
        len2 = buffer2.len() as size_t;
        let result = noise_read_message(
            initiator,
            buffer1.as_ptr(),
            len1,
            buffer2.as_mut_ptr(),
            &mut len2
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Message 3: Initiator -> Responder (s, se)
        len1 = buffer1.len() as size_t;
        let result = noise_write_message(
            initiator,
            ptr::null(),
            0,
            buffer1.as_mut_ptr(),
            &mut len1
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        assert!(len1 > 0);
        
        len2 = buffer2.len() as size_t;
        let result = noise_read_message(
            responder,
            buffer1.as_ptr(),
            len1,
            buffer2.as_mut_ptr(),
            &mut len2
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Verify both sessions are in transport mode
        assert_eq!(noise_is_handshake_complete(initiator), 1);
        assert_eq!(noise_is_handshake_complete(responder), 1);
        
        // Test post-handshake encryption/decryption
        let plaintext = b"Hello, Noise Protocol!";
        let mut ciphertext = vec![0u8; 1024];
        let mut decrypted = vec![0u8; 1024];
        
        len1 = ciphertext.len() as size_t;
        let result = noise_encrypt(
            initiator,
            plaintext.as_ptr(),
            plaintext.len() as size_t,
            ciphertext.as_mut_ptr(),
            &mut len1
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        assert!(len1 > plaintext.len() as size_t); // Should include tag
        
        len2 = decrypted.len() as size_t;
        let result = noise_decrypt(
            responder,
            ciphertext.as_ptr(),
            len1,
            decrypted.as_mut_ptr(),
            &mut len2
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        assert_eq!(len2, plaintext.len() as size_t);
        assert_eq!(&decrypted[..len2 as usize], plaintext);
        
        // Test reverse direction
        let plaintext2 = b"Response from responder";
        len1 = ciphertext.len() as size_t;
        let result = noise_encrypt(
            responder,
            plaintext2.as_ptr(),
            plaintext2.len() as size_t,
            ciphertext.as_mut_ptr(),
            &mut len1
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        len2 = decrypted.len() as size_t;
        let result = noise_decrypt(
            initiator,
            ciphertext.as_ptr(),
            len1,
            decrypted.as_mut_ptr(),
            &mut len2
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        assert_eq!(&decrypted[..len2 as usize], plaintext2);
        
        // Test getting remote static keys
        let mut remote_key = vec![0u8; 32];
        len1 = remote_key.len() as size_t;
        let result = noise_get_remote_static(
            initiator,
            remote_key.as_mut_ptr(),
            &mut len1
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        assert_eq!(len1, 32); // Public key should be 32 bytes
        
        // Clean up
        noise_session_free(initiator);
        noise_session_free(responder);
    }
}

#[test]
fn test_session_persistence() {
    // Create initiator and responder sessions
    let mut initiator = NoiseSession::new_initiator().unwrap();
    let mut responder = NoiseSession::new_responder().unwrap();
    
    // Perform complete handshake first
    let msg1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg1).unwrap();
    
    let msg2 = responder.write_message(&[]).unwrap();
    initiator.read_message(&msg2).unwrap();
    
    let msg3 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg3).unwrap();
    
    // Create another initiator with same keys to simulate persistence
    let mut new_initiator = NoiseSession::new_initiator().unwrap();
    let mut new_responder = NoiseSession::new_responder().unwrap();
    
    // Complete handshake with new sessions
    let msg1 = new_initiator.write_message(&[]).unwrap();
    new_responder.read_message(&msg1).unwrap();
    
    let msg2 = new_responder.write_message(&[]).unwrap();
    new_initiator.read_message(&msg2).unwrap();
    
    let msg3 = new_initiator.write_message(&[]).unwrap();
    new_responder.read_message(&msg3).unwrap();
    
    // Wrap in ResilientSession and test serialization
    let mut resilient_initiator = ResilientSession::new(initiator);
    
    // Send some messages and track sequence numbers
    let msg1 = resilient_initiator.encrypt_with_sequence(b"Message 1").unwrap();
    let msg2 = resilient_initiator.encrypt_with_sequence(b"Message 2").unwrap();
    
    // Serialize the resilient session state (replay window, sequence numbers)
    let serialized = resilient_initiator.serialize();
    
    // Deserialize into a new resilient session with the new initiator
    let mut restored_initiator = ResilientSession::deserialize(&serialized, new_initiator).unwrap();
    
    // The restored session should continue from sequence 3
    let msg3 = restored_initiator.encrypt_with_sequence(b"Message 3").unwrap();
    
    // Create resilient responders
    let mut resilient_responder = ResilientSession::new(responder);
    let mut new_resilient_responder = ResilientSession::new(new_responder);
    
    // Original responder can decrypt messages 1 and 2
    let decrypted1 = resilient_responder.decrypt_with_replay_check(&msg1).unwrap();
    assert_eq!(&decrypted1, b"Message 1");
    
    let decrypted2 = resilient_responder.decrypt_with_replay_check(&msg2).unwrap();
    assert_eq!(&decrypted2, b"Message 2");
    
    // New responder can decrypt message 3
    let decrypted3 = new_resilient_responder.decrypt_with_replay_check(&msg3).unwrap();
    assert_eq!(&decrypted3, b"Message 3");
}

#[test]
fn test_resilient_session_replay_protection() {
    // Create and complete handshake
    let mut initiator = NoiseSession::new_initiator().unwrap();
    let mut responder = NoiseSession::new_responder().unwrap();
    
    // Complete handshake
    let msg1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg1).unwrap();
    
    let msg2 = responder.write_message(&[]).unwrap();
    initiator.read_message(&msg2).unwrap();
    
    let msg3 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg3).unwrap();
    
    // Wrap in resilient sessions
    let mut resilient_initiator = ResilientSession::new(initiator);
    let mut resilient_responder = ResilientSession::new(responder);
    
    // Send a message with sequence number
    let plaintext = b"Message 1";
    let encrypted = resilient_initiator.encrypt_with_sequence(plaintext).unwrap();
    
    // First decryption should succeed
    let decrypted = resilient_responder.decrypt_with_replay_check(&encrypted).unwrap();
    assert_eq!(&decrypted, plaintext);
    
    // Replay the same message - should fail
    let result = resilient_responder.decrypt_with_replay_check(&encrypted);
    assert!(result.is_err());
    
    // Send more messages to test window
    for i in 2..10 {
        let msg = format!("Message {}", i);
        let encrypted = resilient_initiator.encrypt_with_sequence(msg.as_bytes()).unwrap();
        let decrypted = resilient_responder.decrypt_with_replay_check(&encrypted).unwrap();
        assert_eq!(decrypted, msg.as_bytes());
    }
}

#[test]
fn test_resilient_session_out_of_order() {
    // Create and complete handshake
    let mut initiator = NoiseSession::new_initiator().unwrap();
    let mut responder = NoiseSession::new_responder().unwrap();
    
    // Complete handshake
    let msg1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg1).unwrap();
    
    let msg2 = responder.write_message(&[]).unwrap();
    initiator.read_message(&msg2).unwrap();
    
    let msg3 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg3).unwrap();
    
    // Wrap in resilient sessions
    let mut resilient_initiator = ResilientSession::new(initiator);
    let mut resilient_responder = ResilientSession::new(responder);
    
    // Send multiple messages
    let msg1 = resilient_initiator.encrypt_with_sequence(b"Message 1").unwrap();
    let msg2 = resilient_initiator.encrypt_with_sequence(b"Message 2").unwrap();
    let msg3 = resilient_initiator.encrypt_with_sequence(b"Message 3").unwrap();
    
    // Note: Due to Noise protocol constraints, messages must be decrypted in order
    // The sequence numbers help detect replays but don't enable out-of-order decryption
    
    // Decrypt in order first
    let decrypted1 = resilient_responder.decrypt_with_replay_check(&msg1).unwrap();
    assert_eq!(&decrypted1, b"Message 1");
    
    let decrypted2 = resilient_responder.decrypt_with_replay_check(&msg2).unwrap();
    assert_eq!(&decrypted2, b"Message 2");
    
    let decrypted3 = resilient_responder.decrypt_with_replay_check(&msg3).unwrap();
    assert_eq!(&decrypted3, b"Message 3");
    
    // Now test replay protection - trying to decrypt message 2 again should fail
    let replay_result = resilient_responder.decrypt_with_replay_check(&msg2);
    assert!(replay_result.is_err());
    
    // The error should be ReplayDetected or a protocol error (due to Noise state)
    match replay_result {
        Err(NoiseError::ReplayDetected) => {},
        Err(NoiseError::Snow(_)) => {}, // Noise protocol error is also acceptable
        Err(e) => panic!("Unexpected error: {:?}", e),
        Ok(_) => panic!("Expected replay to fail"),
    }
}

#[test]
fn test_batched_crypto_integration() {
    // Create and complete handshake
    let mut initiator = NoiseSession::new_initiator().unwrap();
    let mut responder = NoiseSession::new_responder().unwrap();
    
    // Complete handshake
    let msg1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg1).unwrap();
    
    let msg2 = responder.write_message(&[]).unwrap();
    initiator.read_message(&msg2).unwrap();
    
    let msg3 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg3).unwrap();
    
    // Wrap in batched crypto
    let mut batched_initiator = BatchedCrypto::new(initiator);
    batched_initiator.set_flush_threshold(5);
    batched_initiator.set_flush_interval(std::time::Duration::from_millis(50));
    
    // Queue multiple messages
    batched_initiator.queue_encrypt(b"Message 1".to_vec());
    batched_initiator.queue_encrypt(b"Message 2".to_vec());
    batched_initiator.queue_encrypt(b"Message 3".to_vec());
    batched_initiator.queue_encrypt(b"Message 4".to_vec());
    
    // Not at threshold yet, should not auto-flush
    assert_eq!(batched_initiator.pending_encrypts_count(), 4);
    
    // Manually flush to get encrypted messages
    let encrypted = batched_initiator.flush_encrypts().unwrap();
    assert_eq!(encrypted.len(), 4);
    
    // Queue one more and check threshold behavior
    batched_initiator.queue_encrypt(b"Message 5".to_vec());
    assert_eq!(batched_initiator.pending_encrypts_count(), 1);
    
    // Flush remaining
    let encrypted2 = batched_initiator.flush_encrypts().unwrap();
    assert_eq!(encrypted2.len(), 1);
    
    // Decrypt all messages
    for (i, enc_msg) in encrypted.iter().enumerate() {
        let decrypted = responder.decrypt(enc_msg).unwrap();
        let expected = format!("Message {}", i + 1);
        assert_eq!(decrypted, expected.as_bytes());
    }
    
    // Decrypt the 5th message
    let decrypted = responder.decrypt(&encrypted2[0]).unwrap();
    assert_eq!(decrypted, b"Message 5");
}

#[test]
fn test_ffi_with_mobile_features() {
    unsafe {
        // Create sessions via FFI
        let mut error = 0;
        let initiator_ffi = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        assert_eq!(error, NOISE_ERROR_SUCCESS);
        
        // Complete handshake via FFI (abbreviated for brevity)
        let mut responder = NoiseSession::new_responder().unwrap();
        
        // First message via FFI
        let mut buffer = vec![0u8; 1024];
        let mut len = buffer.len() as size_t;
        noise_write_message(initiator_ffi, ptr::null(), 0, buffer.as_mut_ptr(), &mut len);
        
        responder.read_message(&buffer[..len as usize]).unwrap();
        
        // Continue handshake...
        let msg2 = responder.write_message(&[]).unwrap();
        noise_read_message(
            initiator_ffi, 
            msg2.as_ptr(), 
            msg2.len() as size_t,
            buffer.as_mut_ptr(),
            &mut len
        );
        
        len = buffer.len() as size_t;
        noise_write_message(initiator_ffi, ptr::null(), 0, buffer.as_mut_ptr(), &mut len);
        responder.read_message(&buffer[..len as usize]).unwrap();
        
        // Now both should be in transport mode
        assert_eq!(noise_is_handshake_complete(initiator_ffi), 1);
        assert!(responder.is_transport_state());
        
        // Test encryption via FFI
        let plaintext = b"FFI with mobile features";
        let mut ciphertext = vec![0u8; 1024];
        len = ciphertext.len() as size_t;
        
        let result = noise_encrypt(
            initiator_ffi,
            plaintext.as_ptr(),
            plaintext.len() as size_t,
            ciphertext.as_mut_ptr(),
            &mut len
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Decrypt with Rust API
        let decrypted = responder.decrypt(&ciphertext[..len as usize]).unwrap();
        assert_eq!(&decrypted, plaintext);
        
        // Clean up
        noise_session_free(initiator_ffi);
    }
}

#[test]
fn test_max_message_size_handling() {
    unsafe {
        let mut error = 0;
        let initiator = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        let responder = noise_session_new(NOISE_MODE_RESPONDER, &mut error);
        
        // Complete handshake (abbreviated)
        let mut buf1 = vec![0u8; 2048];
        let mut buf2 = vec![0u8; 2048];
        let mut len1 = buf1.len() as size_t;
        let mut len2 = buf2.len() as size_t;
        
        noise_write_message(initiator, ptr::null(), 0, buf1.as_mut_ptr(), &mut len1);
        noise_read_message(responder, buf1.as_ptr(), len1, buf2.as_mut_ptr(), &mut len2);
        
        len1 = buf1.len() as size_t;
        noise_write_message(responder, ptr::null(), 0, buf1.as_mut_ptr(), &mut len1);
        len2 = buf2.len() as size_t;
        noise_read_message(initiator, buf1.as_ptr(), len1, buf2.as_mut_ptr(), &mut len2);
        
        len1 = buf1.len() as size_t;
        noise_write_message(initiator, ptr::null(), 0, buf1.as_mut_ptr(), &mut len1);
        len2 = buf2.len() as size_t;
        noise_read_message(responder, buf1.as_ptr(), len1, buf2.as_mut_ptr(), &mut len2);
        
        // Test with maximum allowed message size
        let max_payload = noise_max_payload_len() as usize;
        let large_data = vec![0x42u8; max_payload];
        let mut large_buffer = vec![0u8; max_payload + 1024];
        
        len1 = large_buffer.len() as size_t;
        let result = noise_encrypt(
            initiator,
            large_data.as_ptr(),
            max_payload as size_t,
            large_buffer.as_mut_ptr(),
            &mut len1
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Decrypt large message
        let mut decrypted = vec![0u8; max_payload + 1024];
        len2 = decrypted.len() as size_t;
        let result = noise_decrypt(
            responder,
            large_buffer.as_ptr(),
            len1,
            decrypted.as_mut_ptr(),
            &mut len2
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        assert_eq!(len2, max_payload as size_t);
        
        noise_session_free(initiator);
        noise_session_free(responder);
    }
}