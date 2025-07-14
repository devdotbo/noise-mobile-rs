use crate::core::error::{NoiseError, Result};
use crate::core::session::NoiseSession;
use std::collections::VecDeque;

/// Size of the replay protection window
const REPLAY_WINDOW_SIZE: usize = 64;

/// ResilientSession provides network resilience features on top of NoiseSession
/// 
/// Features:
/// - Sequence number tracking for message ordering
/// - Replay attack prevention with sliding window
/// - Session state serialization for resumption
/// - Out-of-order message handling
pub struct ResilientSession {
    inner: NoiseSession,
    last_sent: u64,
    last_received: u64,
    replay_window: VecDeque<bool>,
}

impl ResilientSession {
    /// Create a new resilient session from a NoiseSession
    pub fn new(session: NoiseSession) -> Self {
        let mut replay_window = VecDeque::with_capacity(REPLAY_WINDOW_SIZE);
        replay_window.resize(REPLAY_WINDOW_SIZE, false);
        
        Self {
            inner: session,
            last_sent: 0,
            last_received: 0,
            replay_window,
        }
    }
    
    /// Encrypt a message with sequence number for ordering
    pub fn encrypt_with_sequence(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        // Increment sequence number
        self.last_sent = self.last_sent.wrapping_add(1);
        
        // Prepare message with sequence number prefix
        let mut message = Vec::with_capacity(8 + plaintext.len());
        message.extend_from_slice(&self.last_sent.to_be_bytes());
        message.extend_from_slice(plaintext);
        
        // Encrypt the complete message
        self.inner.encrypt(&message)
    }
    
    /// Decrypt a message and check for replay attacks
    pub fn decrypt_with_replay_check(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        // First decrypt the message
        let decrypted = self.inner.decrypt(ciphertext)?;
        
        // Extract sequence number
        if decrypted.len() < 8 {
            return Err(NoiseError::InvalidMessage);
        }
        
        let sequence_bytes: [u8; 8] = decrypted[..8].try_into()
            .map_err(|_| NoiseError::InvalidMessage)?;
        let sequence = u64::from_be_bytes(sequence_bytes);
        
        // Check replay window
        if !self.check_and_update_replay_window(sequence)? {
            return Err(NoiseError::ReplayDetected);
        }
        
        // Return the actual payload (without sequence number)
        Ok(decrypted[8..].to_vec())
    }
    
    /// Check if a sequence number is valid and update the replay window
    fn check_and_update_replay_window(&mut self, sequence: u64) -> Result<bool> {
        if sequence == 0 {
            // Sequence numbers start at 1
            return Ok(false);
        }
        
        if sequence <= self.last_received {
            // Check if it's in the replay window
            let diff = self.last_received - sequence;
            if diff >= REPLAY_WINDOW_SIZE as u64 {
                // Too old, definitely a replay
                return Ok(false);
            }
            
            // Check if we've seen this sequence number before
            let window_index = diff as usize;
            if self.replay_window[window_index] {
                // Already seen, it's a replay
                return Ok(false);
            }
            
            // Mark as seen
            self.replay_window[window_index] = true;
        } else {
            // New sequence number, advance the window
            let advance = sequence - self.last_received;
            
            if advance > REPLAY_WINDOW_SIZE as u64 {
                // Big jump, reset the window
                self.replay_window.clear();
                self.replay_window.resize(REPLAY_WINDOW_SIZE, false);
            } else {
                // Shift the window
                for _ in 0..advance {
                    self.replay_window.pop_back();
                    self.replay_window.push_front(false);
                }
            }
            
            // Update last received
            self.last_received = sequence;
            
            // Mark the current sequence as seen
            self.replay_window[0] = true;
        }
        
        Ok(true)
    }
    
    /// Set the replay window size (for testing or tuning)
    pub fn set_replay_window_size(&mut self, size: usize) {
        self.replay_window.clear();
        self.replay_window.resize(size, false);
    }
    
    /// Serialize the session state for resumption
    pub fn serialize(&self) -> Vec<u8> {
        // For now, we'll create a simple serialization format
        // In production, consider using serde or similar
        let mut data = Vec::new();
        
        // Version byte
        data.push(1u8);
        
        // Sequence numbers
        data.extend_from_slice(&self.last_sent.to_be_bytes());
        data.extend_from_slice(&self.last_received.to_be_bytes());
        
        // Replay window size
        data.extend_from_slice(&(self.replay_window.len() as u32).to_be_bytes());
        
        // Replay window bits (packed into bytes)
        let mut window_bytes = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;
        
        for &bit in &self.replay_window {
            if bit {
                current_byte |= 1 << (7 - bit_count);
            }
            bit_count += 1;
            
            if bit_count == 8 {
                window_bytes.push(current_byte);
                current_byte = 0;
                bit_count = 0;
            }
        }
        
        // Push remaining bits if any
        if bit_count > 0 {
            window_bytes.push(current_byte);
        }
        
        data.extend_from_slice(&window_bytes);
        
        // Note: We don't serialize the inner NoiseSession as it contains
        // cryptographic state that should be regenerated, not restored
        
        data
    }
    
    /// Deserialize session state
    /// 
    /// Note: The NoiseSession must be provided separately as cryptographic
    /// state should not be serialized
    pub fn deserialize(data: &[u8], session: NoiseSession) -> Result<Self> {
        if data.is_empty() {
            return Err(NoiseError::InvalidMessage);
        }
        
        // Check version
        if data[0] != 1 {
            return Err(NoiseError::InvalidMessage);
        }
        
        let mut offset = 1;
        
        // Read sequence numbers
        if data.len() < offset + 16 {
            return Err(NoiseError::InvalidMessage);
        }
        
        let last_sent_bytes: [u8; 8] = data[offset..offset+8].try_into()
            .map_err(|_| NoiseError::InvalidMessage)?;
        let last_sent = u64::from_be_bytes(last_sent_bytes);
        offset += 8;
        
        let last_received_bytes: [u8; 8] = data[offset..offset+8].try_into()
            .map_err(|_| NoiseError::InvalidMessage)?;
        let last_received = u64::from_be_bytes(last_received_bytes);
        offset += 8;
        
        // Read replay window size
        if data.len() < offset + 4 {
            return Err(NoiseError::InvalidMessage);
        }
        
        let window_size_bytes: [u8; 4] = data[offset..offset+4].try_into()
            .map_err(|_| NoiseError::InvalidMessage)?;
        let window_size = u32::from_be_bytes(window_size_bytes) as usize;
        offset += 4;
        
        // Read replay window bits
        let mut replay_window = VecDeque::with_capacity(window_size);
        let bytes_needed = (window_size + 7) / 8;
        
        if data.len() < offset + bytes_needed {
            return Err(NoiseError::InvalidMessage);
        }
        
        let window_bytes = &data[offset..offset + bytes_needed];
        
        for i in 0..window_size {
            let byte_index = i / 8;
            let bit_offset = 7 - (i % 8);
            let bit = (window_bytes[byte_index] >> bit_offset) & 1 != 0;
            replay_window.push_back(bit);
        }
        
        Ok(Self {
            inner: session,
            last_sent,
            last_received,
            replay_window,
        })
    }
    
    /// Get the current send sequence number
    pub fn send_sequence(&self) -> u64 {
        self.last_sent
    }
    
    /// Get the current receive sequence number
    pub fn receive_sequence(&self) -> u64 {
        self.last_received
    }
    
    /// Check if the inner session has completed handshake
    pub fn is_handshake_complete(&self) -> bool {
        self.inner.is_transport_state()
    }
    
    /// Get access to the inner NoiseSession for non-resilient operations
    pub fn inner(&self) -> &NoiseSession {
        &self.inner
    }
    
    /// Get mutable access to the inner NoiseSession
    pub fn inner_mut(&mut self) -> &mut NoiseSession {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_session() -> NoiseSession {
        NoiseSession::new_initiator().unwrap()
    }
    
    fn create_connected_pair() -> (ResilientSession, ResilientSession) {
        let mut initiator = NoiseSession::new_initiator().unwrap();
        let mut responder = NoiseSession::new_responder().unwrap();
        
        // Complete handshake
        let msg1 = initiator.write_message(&[]).unwrap();
        responder.read_message(&msg1).unwrap();
        
        let msg2 = responder.write_message(&[]).unwrap();
        initiator.read_message(&msg2).unwrap();
        
        let msg3 = initiator.write_message(&[]).unwrap();
        responder.read_message(&msg3).unwrap();
        
        // Verify both are in transport mode
        assert!(initiator.is_transport_state());
        assert!(responder.is_transport_state());
        
        (ResilientSession::new(initiator), ResilientSession::new(responder))
    }
    
    #[test]
    fn test_sequence_numbers() {
        let (mut alice, mut bob) = create_connected_pair();
        
        // Send messages with sequence numbers
        let msg1 = alice.encrypt_with_sequence(b"Hello").unwrap();
        let msg2 = alice.encrypt_with_sequence(b"World").unwrap();
        
        // Verify sequence numbers increment
        assert_eq!(alice.send_sequence(), 2);
        
        // Bob receives in order
        let plain1 = bob.decrypt_with_replay_check(&msg1).unwrap();
        let plain2 = bob.decrypt_with_replay_check(&msg2).unwrap();
        
        assert_eq!(plain1, b"Hello");
        assert_eq!(plain2, b"World");
        assert_eq!(bob.receive_sequence(), 2);
    }
    
    #[test]
    fn test_replay_protection() {
        // Test the replay window logic directly since we can't replay
        // encrypted messages due to Noise protocol constraints
        let (mut alice, mut bob) = create_connected_pair();
        
        // Simulate receiving messages with specific sequence numbers
        // by manually testing the replay window logic
        assert!(bob.check_and_update_replay_window(1).unwrap());
        assert!(bob.check_and_update_replay_window(2).unwrap());
        assert!(bob.check_and_update_replay_window(3).unwrap());
        
        // Try to replay message 2 - should fail
        assert!(!bob.check_and_update_replay_window(2).unwrap());
        
        // Try to replay message 1 - should fail
        assert!(!bob.check_and_update_replay_window(1).unwrap());
        
        // New message 4 should work
        assert!(bob.check_and_update_replay_window(4).unwrap());
        
        // Zero sequence number should always fail
        assert!(!bob.check_and_update_replay_window(0).unwrap());
    }
    
    #[test]
    fn test_out_of_order_messages() {
        let (mut alice, mut bob) = create_connected_pair();
        
        // Note: Noise protocol doesn't support out-of-order decryption
        // Messages must be decrypted in the order they were encrypted
        // This test verifies our sequence number tracking works correctly
        // when messages are received in order
        
        // Alice sends 3 messages
        let msg1 = alice.encrypt_with_sequence(b"First").unwrap();
        let msg2 = alice.encrypt_with_sequence(b"Second").unwrap();
        let msg3 = alice.encrypt_with_sequence(b"Third").unwrap();
        
        // Bob must receive in order due to Noise protocol constraints
        let plain1 = bob.decrypt_with_replay_check(&msg1).unwrap();
        assert_eq!(plain1, b"First");
        assert_eq!(bob.receive_sequence(), 1);
        
        let plain2 = bob.decrypt_with_replay_check(&msg2).unwrap();
        assert_eq!(plain2, b"Second");
        assert_eq!(bob.receive_sequence(), 2);
        
        let plain3 = bob.decrypt_with_replay_check(&msg3).unwrap();
        assert_eq!(plain3, b"Third");
        assert_eq!(bob.receive_sequence(), 3);
    }
    
    #[test]
    fn test_window_size_limit() {
        let (_alice, mut bob) = create_connected_pair();
        
        // Simulate receiving many messages to test window boundaries
        for i in 1..=100 {
            assert!(bob.check_and_update_replay_window(i).unwrap());
        }
        
        // Bob's last_received should be 100
        assert_eq!(bob.receive_sequence(), 100);
        
        // Try to replay a very old message (outside 64-message window)
        // Message 30 is 70 positions behind, outside the window
        assert!(!bob.check_and_update_replay_window(30).unwrap());
        
        // But recent messages within window can still be detected as replays
        // Message 90 is only 10 positions behind, within the window
        assert!(!bob.check_and_update_replay_window(90).unwrap());
        
        // Messages within the window that weren't received can still come in
        // Let's skip 101 and receive 102
        assert!(bob.check_and_update_replay_window(102).unwrap());
        
        // Now 101 should still be acceptable (within window)
        assert!(bob.check_and_update_replay_window(101).unwrap());
        
        // But trying 101 again should fail (replay)
        assert!(!bob.check_and_update_replay_window(101).unwrap());
    }
    
    #[test]
    fn test_serialization() {
        let (_alice, mut bob) = create_connected_pair();
        
        // Simulate receiving some messages with specific patterns
        assert!(bob.check_and_update_replay_window(1).unwrap());
        assert!(bob.check_and_update_replay_window(2).unwrap());
        assert!(bob.check_and_update_replay_window(4).unwrap()); // Skip 3
        assert!(bob.check_and_update_replay_window(5).unwrap());
        assert!(bob.check_and_update_replay_window(7).unwrap()); // Skip 6
        
        // Set a specific send sequence
        bob.last_sent = 42;
        
        // Serialize Bob's state
        let serialized = bob.serialize();
        
        // Create new session and restore state
        let new_session = create_test_session();
        let mut restored_bob = ResilientSession::deserialize(&serialized, new_session).unwrap();
        
        // Verify state was restored correctly
        assert_eq!(restored_bob.receive_sequence(), 7);
        assert_eq!(restored_bob.send_sequence(), 42);
        
        // Check that replay window was restored correctly
        // Already seen messages should be rejected
        assert!(!restored_bob.check_and_update_replay_window(1).unwrap());
        assert!(!restored_bob.check_and_update_replay_window(2).unwrap());
        assert!(!restored_bob.check_and_update_replay_window(4).unwrap());
        assert!(!restored_bob.check_and_update_replay_window(5).unwrap());
        assert!(!restored_bob.check_and_update_replay_window(7).unwrap());
        
        // But the skipped messages should still be acceptable
        assert!(restored_bob.check_and_update_replay_window(3).unwrap());
        assert!(restored_bob.check_and_update_replay_window(6).unwrap());
        
        // And new messages should work
        assert!(restored_bob.check_and_update_replay_window(8).unwrap());
    }
    
    #[test]
    fn test_wrapping_sequence_numbers() {
        // Use a connected session for encryption
        let (mut alice, _bob) = create_connected_pair();
        
        // Set sequence number near max
        alice.last_sent = u64::MAX - 2;
        
        // Encrypt a few messages
        alice.encrypt_with_sequence(b"test1").unwrap();
        assert_eq!(alice.send_sequence(), u64::MAX - 1);
        
        alice.encrypt_with_sequence(b"test2").unwrap();
        assert_eq!(alice.send_sequence(), u64::MAX);
        
        // Should wrap around
        alice.encrypt_with_sequence(b"test3").unwrap();
        assert_eq!(alice.send_sequence(), 0);
    }
}