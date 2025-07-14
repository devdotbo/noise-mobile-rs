use crate::core::error::{NoiseError, Result};
use snow::{Builder, HandshakeState, TransportState};
use zeroize::Zeroize;

/// Represents a Noise Protocol session that can be either in handshake or transport mode
pub struct NoiseSession {
    state: NoiseState,
    buffer: Vec<u8>,
    remote_static: Option<Vec<u8>>,
}

/// The current state of a Noise session
pub enum NoiseState {
    /// Session is in handshake phase
    Handshake(Box<HandshakeState>),
    /// Session is in transport phase (handshake complete)
    Transport(Box<TransportState>),
    /// Temporary state during transition
    Transitioning,
}

impl Drop for NoiseSession {
    fn drop(&mut self) {
        self.buffer.zeroize();
        if let Some(ref mut key) = self.remote_static {
            key.zeroize();
        }
    }
}

impl NoiseSession {
    /// Maximum message length supported by Noise
    pub const MAX_MESSAGE_LEN: usize = 65535;
    
    /// Noise protocol pattern (XX provides mutual authentication)
    pub const NOISE_PARAMS: &'static str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
    
    /// Create a new Noise session as initiator
    pub fn new_initiator() -> Result<Self> {
        let params = Self::NOISE_PARAMS.parse()?;
        let builder = Builder::new(params);
        let keypair = builder.generate_keypair()?;
        
        let handshake = builder
            .local_private_key(&keypair.private)?
            .build_initiator()?;
            
        Ok(NoiseSession {
            state: NoiseState::Handshake(Box::new(handshake)),
            buffer: vec![0u8; Self::MAX_MESSAGE_LEN],
            remote_static: None,
        })
    }
    
    /// Create a new Noise session as responder
    pub fn new_responder() -> Result<Self> {
        let params = Self::NOISE_PARAMS.parse()?;
        let builder = Builder::new(params);
        let keypair = builder.generate_keypair()?;
        
        let handshake = builder
            .local_private_key(&keypair.private)?
            .build_responder()?;
            
        Ok(NoiseSession {
            state: NoiseState::Handshake(Box::new(handshake)),
            buffer: vec![0u8; Self::MAX_MESSAGE_LEN],
            remote_static: None,
        })
    }
    
    /// Create a new Noise session with a specific private key
    pub fn with_private_key(private_key: &[u8], is_initiator: bool) -> Result<Self> {
        let params = Self::NOISE_PARAMS.parse()?;
        let builder = Builder::new(params);
        
        let handshake = if is_initiator {
            builder
                .local_private_key(private_key)?
                .build_initiator()?
        } else {
            builder
                .local_private_key(private_key)?
                .build_responder()?
        };
            
        Ok(NoiseSession {
            state: NoiseState::Handshake(Box::new(handshake)),
            buffer: vec![0u8; Self::MAX_MESSAGE_LEN],
            remote_static: None,
        })
    }
    
    /// Check if the session is still in handshake state
    pub fn is_handshake_state(&self) -> bool {
        matches!(self.state, NoiseState::Handshake(_))
    }
    
    /// Check if the session is in transport state (handshake complete)
    pub fn is_transport_state(&self) -> bool {
        matches!(self.state, NoiseState::Transport(_))
    }
    
    /// Get the remote peer's static public key (only available after handshake)
    pub fn get_remote_static(&self) -> Option<&[u8]> {
        self.remote_static.as_deref()
    }
    
    /// Write a handshake message
    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        if let NoiseState::Handshake(ref mut handshake) = &mut self.state {
            let len = handshake.write_message(payload, &mut self.buffer)?;
            let result = self.buffer[..len].to_vec();
            
            // Check if handshake is complete after writing
            if handshake.is_handshake_finished() {
                // Store remote static key before transitioning
                self.remote_static = handshake.get_remote_static()
                    .map(|k| k.to_vec());
                    
                // Take ownership of the handshake state to transition
                let old_state = std::mem::replace(&mut self.state, NoiseState::Transitioning);
                if let NoiseState::Handshake(handshake) = old_state {
                    let transport = handshake.into_transport_mode()?;
                    self.state = NoiseState::Transport(Box::new(transport));
                }
            }
            
            Ok(result)
        } else {
            Err(NoiseError::InvalidState("Cannot write handshake message in transport mode".to_string()))
        }
    }
    
    /// Read a handshake message
    pub fn read_message(&mut self, message: &[u8]) -> Result<Vec<u8>> {
        if let NoiseState::Handshake(ref mut handshake) = &mut self.state {
            let len = handshake.read_message(message, &mut self.buffer)?;
            let result = self.buffer[..len].to_vec();
            
            // Check if handshake is complete after reading
            if handshake.is_handshake_finished() {
                // Store remote static key before transitioning
                self.remote_static = handshake.get_remote_static()
                    .map(|k| k.to_vec());
                    
                // Take ownership of the handshake state to transition
                let old_state = std::mem::replace(&mut self.state, NoiseState::Transitioning);
                if let NoiseState::Handshake(handshake) = old_state {
                    let transport = handshake.into_transport_mode()?;
                    self.state = NoiseState::Transport(Box::new(transport));
                }
            }
            
            Ok(result)
        } else {
            Err(NoiseError::InvalidState("Cannot read handshake message in transport mode".to_string()))
        }
    }
    
    /// Encrypt a message (only available after handshake completion)
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        match &mut self.state {
            NoiseState::Handshake(_) => {
                Err(NoiseError::InvalidState("Cannot encrypt before handshake completion".to_string()))
            }
            NoiseState::Transport(ref mut transport) => {
                let len = transport.write_message(plaintext, &mut self.buffer)?;
                Ok(self.buffer[..len].to_vec())
            }
            NoiseState::Transitioning => {
                Err(NoiseError::InvalidState("Session is in transition".to_string()))
            }
        }
    }
    
    /// Decrypt a message (only available after handshake completion)
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        match &mut self.state {
            NoiseState::Handshake(_) => {
                Err(NoiseError::InvalidState("Cannot decrypt before handshake completion".to_string()))
            }
            NoiseState::Transport(ref mut transport) => {
                let len = transport.read_message(ciphertext, &mut self.buffer)?;
                Ok(self.buffer[..len].to_vec())
            }
            NoiseState::Transitioning => {
                Err(NoiseError::InvalidState("Session is in transition".to_string()))
            }
        }
    }
    
    /// Process a message - automatically handles handshake or transport mode
    pub fn process_message(&mut self, input: &[u8]) -> Result<Vec<u8>> {
        match &self.state {
            NoiseState::Handshake(_) => self.read_message(input),
            NoiseState::Transport(_) => self.decrypt(input),
            NoiseState::Transitioning => Err(NoiseError::InvalidState("Session is in transition".to_string())),
        }
    }
    
    /// Generate the next message - automatically handles handshake or transport mode
    pub fn generate_message(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        match &self.state {
            NoiseState::Handshake(_) => self.write_message(payload),
            NoiseState::Transport(_) => self.encrypt(payload),
            NoiseState::Transitioning => Err(NoiseError::InvalidState("Session is in transition".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn perform_handshake() -> Result<(NoiseSession, NoiseSession)> {
        let mut initiator = NoiseSession::new_initiator()?;
        let mut responder = NoiseSession::new_responder()?;
        
        // Message 1: initiator -> responder (e)
        let msg1 = initiator.write_message(&[])?;
        responder.read_message(&msg1)?;
        
        // Message 2: responder -> initiator (e, ee, s, es)
        let msg2 = responder.write_message(&[])?;
        initiator.read_message(&msg2)?;
        
        // Message 3: initiator -> responder (s, se)
        let msg3 = initiator.write_message(&[])?;
        responder.read_message(&msg3)?;
        
        Ok((initiator, responder))
    }
    
    #[test]
    fn test_handshake_state_transitions() {
        let (initiator, responder) = perform_handshake().unwrap();
        
        assert!(initiator.is_transport_state());
        assert!(responder.is_transport_state());
        assert!(!initiator.is_handshake_state());
        assert!(!responder.is_handshake_state());
    }
    
    #[test]
    fn test_encryption_decryption() {
        let (mut alice, mut bob) = perform_handshake().unwrap();
        
        let plaintext = b"Hello, Bob!";
        let ciphertext = alice.encrypt(plaintext).unwrap();
        let decrypted = bob.decrypt(&ciphertext).unwrap();
        
        assert_eq!(plaintext, &decrypted[..]);
    }
    
    #[test]
    fn test_bidirectional_communication() {
        let (mut alice, mut bob) = perform_handshake().unwrap();
        
        // Alice -> Bob
        let msg1 = b"Hello from Alice";
        let ct1 = alice.encrypt(msg1).unwrap();
        let pt1 = bob.decrypt(&ct1).unwrap();
        assert_eq!(msg1, &pt1[..]);
        
        // Bob -> Alice
        let msg2 = b"Hello from Bob";
        let ct2 = bob.encrypt(msg2).unwrap();
        let pt2 = alice.decrypt(&ct2).unwrap();
        assert_eq!(msg2, &pt2[..]);
    }
    
    #[test]
    fn test_invalid_state_errors() {
        let mut session = NoiseSession::new_initiator().unwrap();
        
        // Cannot encrypt during handshake
        assert!(matches!(
            session.encrypt(b"test"),
            Err(NoiseError::InvalidState(_))
        ));
        
        // Cannot decrypt during handshake
        assert!(matches!(
            session.decrypt(b"test"),
            Err(NoiseError::InvalidState(_))
        ));
    }
}