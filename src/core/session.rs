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
            .local_private_key(&keypair.private)
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
            .local_private_key(&keypair.private)
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
                .local_private_key(private_key)
                .build_initiator()?
        } else {
            builder
                .local_private_key(private_key)
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
}