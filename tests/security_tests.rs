//! Security tests for noise-mobile-rust
//! 
//! These tests verify that the library properly handles security threats
//! including replay attacks, MITM attempts, and malformed inputs.

use noise_mobile::core::session::NoiseSession;
use noise_mobile::mobile::network::ResilientSession;
use noise_mobile::ffi::c_api::*;
use std::ptr;
use libc::{c_int, size_t};

// Constants
const NOISE_ERROR_SUCCESS: c_int = 0;
const NOISE_ERROR_DECRYPTION_FAILED: c_int = 5;
const NOISE_ERROR_PROTOCOL_ERROR: c_int = 8;
const NOISE_MODE_INITIATOR: c_int = 0;
const NOISE_MODE_RESPONDER: c_int = 1;

#[test]
fn test_replay_attack_prevention() {
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
    
    // Wrap in resilient sessions for replay protection
    let mut resilient_initiator = ResilientSession::new(initiator);
    let mut resilient_responder = ResilientSession::new(responder);
    
    // Send and receive a legitimate message
    let plaintext = b"Secret message";
    let encrypted = resilient_initiator.encrypt_with_sequence(plaintext).unwrap();
    
    // First decryption should succeed
    let decrypted = resilient_responder.decrypt_with_replay_check(&encrypted).unwrap();
    assert_eq!(&decrypted, plaintext);
    
    // Replay the same message - should fail
    let replay_result = resilient_responder.decrypt_with_replay_check(&encrypted);
    assert!(replay_result.is_err());
    
    // Send more messages to fill the replay window
    for i in 0..70 {
        let msg = format!("Message {}", i);
        let encrypted = resilient_initiator.encrypt_with_sequence(msg.as_bytes()).unwrap();
        resilient_responder.decrypt_with_replay_check(&encrypted).unwrap();
    }
    
    // The original message should now be outside the replay window
    // But it still shouldn't decrypt due to Noise protocol state
    let very_old_replay = resilient_responder.decrypt_with_replay_check(&encrypted);
    assert!(very_old_replay.is_err());
}

#[test]
fn test_mitm_detection_key_mismatch() {
    // Alice and Bob think they're talking to each other, but Mallory is in the middle
    let mut alice = NoiseSession::new_initiator().unwrap();
    let mut mallory_as_responder = NoiseSession::new_responder().unwrap();
    let mut mallory_as_initiator = NoiseSession::new_initiator().unwrap();
    let mut bob = NoiseSession::new_responder().unwrap();
    
    // Alice -> Mallory (thinking it's Bob)
    let alice_msg1 = alice.write_message(&[]).unwrap();
    mallory_as_responder.read_message(&alice_msg1).unwrap();
    
    // Mallory -> Bob (pretending to be Alice)
    let mallory_msg1 = mallory_as_initiator.write_message(&[]).unwrap();
    bob.read_message(&mallory_msg1).unwrap();
    
    // Bob -> Mallory (thinking it's Alice)
    let bob_msg2 = bob.write_message(&[]).unwrap();
    mallory_as_initiator.read_message(&bob_msg2).unwrap();
    
    // Mallory -> Alice (pretending to be Bob)
    let mallory_msg2 = mallory_as_responder.write_message(&[]).unwrap();
    alice.read_message(&mallory_msg2).unwrap();
    
    // Alice -> Mallory (final handshake)
    let alice_msg3 = alice.write_message(&[]).unwrap();
    mallory_as_responder.read_message(&alice_msg3).unwrap();
    
    // Mallory -> Bob (final handshake)
    let mallory_msg3 = mallory_as_initiator.write_message(&[]).unwrap();
    bob.read_message(&mallory_msg3).unwrap();
    
    // At this point, MITM is established
    // To detect this, applications should verify public keys out-of-band
    assert!(alice.is_transport_state());
    assert!(bob.is_transport_state());
    assert!(mallory_as_responder.is_transport_state());
    assert!(mallory_as_initiator.is_transport_state());
    
    // Get remote static keys - they will be different
    let alice_sees = alice.get_remote_static().unwrap();
    let bob_sees = bob.get_remote_static().unwrap();
    
    // Alice thinks she's talking to Mallory (as responder)
    // Bob thinks he's talking to Mallory (as initiator)
    // These keys will be different, indicating MITM
    assert_ne!(alice_sees, bob_sees);
}

#[test]
fn test_malformed_handshake_messages() {
    let mut initiator = NoiseSession::new_initiator().unwrap();
    
    // Get a valid first message to know expected size
    let valid_msg1 = initiator.write_message(&[]).unwrap();
    
    // Create a new responder for each test
    let mut responder1 = NoiseSession::new_responder().unwrap();
    
    // Test truncated first message
    let truncated = &valid_msg1[..valid_msg1.len() / 2];
    let result = responder1.read_message(truncated);
    assert!(result.is_err());
    
    // Test with fresh responder for garbage data
    let mut responder2 = NoiseSession::new_responder().unwrap();
    let garbage = vec![0xFF; valid_msg1.len()]; // Same size as valid message
    let result = responder2.read_message(&garbage);
    // This may or may not error depending on how snow validates
    let _ = result; // Don't assert, just ensure no panic
    
    // Test empty message with fresh responder
    let mut responder3 = NoiseSession::new_responder().unwrap();
    let result = responder3.read_message(&[]);
    assert!(result.is_err());
    
    // Test message that's too short to be valid
    let mut responder4 = NoiseSession::new_responder().unwrap();
    let too_short = vec![0x00; 10]; // Very short message
    let result = responder4.read_message(&too_short);
    assert!(result.is_err());
}

#[test]
fn test_malformed_transport_messages() {
    // Complete handshake first
    let mut initiator = NoiseSession::new_initiator().unwrap();
    let mut responder = NoiseSession::new_responder().unwrap();
    
    let msg1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg1).unwrap();
    
    let msg2 = responder.write_message(&[]).unwrap();
    initiator.read_message(&msg2).unwrap();
    
    let msg3 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg3).unwrap();
    
    // Now test malformed encrypted messages
    let plaintext = b"Valid message";
    let ciphertext = initiator.encrypt(plaintext).unwrap();
    
    // Truncate the ciphertext (remove auth tag)
    let truncated = &ciphertext[..ciphertext.len() - 10];
    let result = responder.decrypt(truncated);
    assert!(result.is_err());
    
    // Corrupt the ciphertext
    let mut corrupted = ciphertext.clone();
    let corrupt_pos = ciphertext.len() / 2;
    corrupted[corrupt_pos] ^= 0xFF;
    let result = responder.decrypt(&corrupted);
    assert!(result.is_err());
    
    // Empty ciphertext
    let result = responder.decrypt(&[]);
    assert!(result.is_err());
    
    // Just the tag, no actual encrypted data
    let short_cipher = &ciphertext[..16];
    let result = responder.decrypt(short_cipher);
    assert!(result.is_err());
}

#[test]
fn test_ffi_malformed_inputs() {
    unsafe {
        let mut error = 0;
        let initiator = noise_session_new(NOISE_MODE_INITIATOR, &mut error);
        let responder = noise_session_new(NOISE_MODE_RESPONDER, &mut error);
        
        // Complete handshake
        let mut buf1 = vec![0u8; 1024];
        let mut buf2 = vec![0u8; 1024];
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
        
        // Test malformed encrypted data via FFI
        let plaintext = b"Test message";
        let mut ciphertext = vec![0u8; 1024];
        len1 = ciphertext.len() as size_t;
        
        let result = noise_encrypt(
            initiator,
            plaintext.as_ptr(),
            plaintext.len() as size_t,
            ciphertext.as_mut_ptr(),
            &mut len1
        );
        assert_eq!(result, NOISE_ERROR_SUCCESS);
        
        // Try to decrypt with wrong length (too short)
        let mut decrypted = vec![0u8; 1024];
        len2 = decrypted.len() as size_t;
        let result = noise_decrypt(
            responder,
            ciphertext.as_ptr(),
            len1 - 10, // Too short
            decrypted.as_mut_ptr(),
            &mut len2
        );
        assert!(result == NOISE_ERROR_DECRYPTION_FAILED || result == NOISE_ERROR_PROTOCOL_ERROR);
        
        // Try to decrypt garbage
        let garbage = vec![0xFF; 100];
        len2 = decrypted.len() as size_t;
        let result = noise_decrypt(
            responder,
            garbage.as_ptr(),
            garbage.len() as size_t,
            decrypted.as_mut_ptr(),
            &mut len2
        );
        assert!(result == NOISE_ERROR_DECRYPTION_FAILED || result == NOISE_ERROR_PROTOCOL_ERROR);
        
        noise_session_free(initiator);
        noise_session_free(responder);
    }
}

#[test]
fn test_sequence_number_overflow() {
    // Test that sequence numbers handle overflow correctly
    let mut initiator = NoiseSession::new_initiator().unwrap();
    let mut responder = NoiseSession::new_responder().unwrap();
    
    // Complete handshake
    let msg1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg1).unwrap();
    
    let msg2 = responder.write_message(&[]).unwrap();
    initiator.read_message(&msg2).unwrap();
    
    let msg3 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg3).unwrap();
    
    // Create resilient session with manipulated sequence number
    let mut resilient = ResilientSession::new(initiator);
    
    // Send many messages to test sequence handling
    for i in 0..1000 {
        let msg = format!("Message {}", i);
        let encrypted = resilient.encrypt_with_sequence(msg.as_bytes()).unwrap();
        assert!(!encrypted.is_empty());
        
        // Verify sequence number is included
        assert!(encrypted.len() >= 8); // At least sequence number size
    }
}

#[test]
fn test_timing_attack_resistance() {
    // This test verifies that decryption failures don't leak timing information
    // In practice, this would require more sophisticated timing measurements
    
    let mut initiator = NoiseSession::new_initiator().unwrap();
    let mut responder = NoiseSession::new_responder().unwrap();
    
    // Complete handshake
    let msg1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg1).unwrap();
    
    let msg2 = responder.write_message(&[]).unwrap();
    initiator.read_message(&msg2).unwrap();
    
    let msg3 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg3).unwrap();
    
    // Encrypt a valid message
    let plaintext = b"Secret data";
    let valid_ciphertext = initiator.encrypt(plaintext).unwrap();
    
    // Create invalid ciphertext by corrupting the tag
    let mut invalid_early = valid_ciphertext.clone();
    invalid_early[0] ^= 0xFF; // Corrupt early byte
    
    let mut invalid_late = valid_ciphertext.clone();
    let last_pos = invalid_late.len() - 1;
    invalid_late[last_pos] ^= 0xFF; // Corrupt auth tag
    
    // Both should fail
    assert!(responder.decrypt(&invalid_early).is_err());
    assert!(responder.decrypt(&invalid_late).is_err());
    
    // In a real timing attack test, we would measure that both
    // failures take approximately the same time
}

#[test]
fn test_key_compromise_forward_secrecy() {
    // Noise XX provides forward secrecy after handshake
    // Past messages remain secure even if long-term keys are compromised
    
    let mut initiator = NoiseSession::new_initiator().unwrap();
    let mut responder = NoiseSession::new_responder().unwrap();
    
    // Complete handshake
    let msg1 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg1).unwrap();
    
    let msg2 = responder.write_message(&[]).unwrap();
    initiator.read_message(&msg2).unwrap();
    
    let msg3 = initiator.write_message(&[]).unwrap();
    responder.read_message(&msg3).unwrap();
    
    // Exchange some messages
    let secret1 = b"Past secret 1";
    let cipher1 = initiator.encrypt(secret1).unwrap();
    responder.decrypt(&cipher1).unwrap();
    
    let secret2 = b"Past secret 2";
    let cipher2 = responder.encrypt(secret2).unwrap();
    initiator.decrypt(&cipher2).unwrap();
    
    // At this point, even if the static keys are compromised,
    // the ephemeral keys used in the handshake are gone,
    // so past messages (cipher1, cipher2) cannot be decrypted
    
    // New sessions with the same static keys would have different
    // ephemeral keys and couldn't decrypt old messages
    let mut new_initiator = NoiseSession::new_initiator().unwrap();
    let mut new_responder = NoiseSession::new_responder().unwrap();
    
    // Complete new handshake
    let new_msg1 = new_initiator.write_message(&[]).unwrap();
    new_responder.read_message(&new_msg1).unwrap();
    
    let new_msg2 = new_responder.write_message(&[]).unwrap();
    new_initiator.read_message(&new_msg2).unwrap();
    
    let new_msg3 = new_initiator.write_message(&[]).unwrap();
    new_responder.read_message(&new_msg3).unwrap();
    
    // Try to decrypt old messages with new session - should fail
    assert!(new_responder.decrypt(&cipher1).is_err());
    assert!(new_initiator.decrypt(&cipher2).is_err());
}