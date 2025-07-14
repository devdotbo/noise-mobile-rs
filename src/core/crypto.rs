use zeroize::Zeroize;

#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SecureBuffer {
    data: Vec<u8>,
}

impl SecureBuffer {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0u8; size],
        }
    }
    
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
    
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

pub const NOISE_MAX_MESSAGE_LEN: usize = 65535;
pub const NOISE_MAX_PAYLOAD_LEN: usize = 65535 - 16; // Subtract AEAD tag
pub const NOISE_TAG_LEN: usize = 16;