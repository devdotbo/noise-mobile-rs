use crate::core::error::Result;
use crate::core::session::NoiseSession;

pub struct BatchedCrypto {
    session: NoiseSession,
    pending_encrypts: Vec<Vec<u8>>,
    pending_decrypts: Vec<Vec<u8>>,
}

impl BatchedCrypto {
    pub fn new(session: NoiseSession) -> Self {
        Self {
            session,
            pending_encrypts: Vec::new(),
            pending_decrypts: Vec::new(),
        }
    }
    
    pub fn queue_encrypt(&mut self, plaintext: Vec<u8>) {
        self.pending_encrypts.push(plaintext);
    }
    
    pub fn pending_count(&self) -> usize {
        self.pending_encrypts.len() + self.pending_decrypts.len()
    }
}