use thiserror::Error;

#[derive(Debug, Error)]
pub enum NoiseError {
    #[error("Invalid parameter provided")]
    InvalidParameter,
    
    #[error("Handshake failed")]
    HandshakeFailed,
    
    #[error("Encryption failed")]
    EncryptionFailed,
    
    #[error("Decryption failed")]
    DecryptionFailed,
    
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    #[error("Buffer too small: needed {needed}, got {got}")]
    BufferTooSmall { needed: usize, got: usize },
    
    #[error("Out of memory")]
    OutOfMemory,
    
    #[error("Replay detected")]
    ReplayDetected,
    
    #[error("Invalid message")]
    InvalidMessage,
    
    #[error("Snow error: {0}")]
    Snow(#[from] snow::Error),
}

pub type Result<T> = std::result::Result<T, NoiseError>;