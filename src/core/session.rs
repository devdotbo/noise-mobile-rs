use crate::core::error::{NoiseError, Result};
use snow::{Builder, HandshakeState, TransportState};
use zeroize::Zeroize;

pub struct NoiseSession {
    state: NoiseState,
    buffer: Vec<u8>,
}

pub enum NoiseState {
    Handshake(Box<HandshakeState>),
    Transport(Box<TransportState>),
}

impl Drop for NoiseSession {
    fn drop(&mut self) {
        self.buffer.zeroize();
    }
}

impl NoiseSession {
    const MAX_MESSAGE_LEN: usize = 65535;
    const NOISE_PARAMS: &'static str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
    
    pub fn is_handshake_state(&self) -> bool {
        matches!(self.state, NoiseState::Handshake(_))
    }
    
    pub fn is_transport_state(&self) -> bool {
        matches!(self.state, NoiseState::Transport(_))
    }
}