//! FFI-safe type definitions for the noise-mobile-rust library

/// FFI-safe error codes returned by C API functions
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseErrorCode {
    /// Operation completed successfully
    Success = 0,
    /// Invalid parameter provided
    InvalidParameter = 1,
    /// Out of memory
    OutOfMemory = 2,
    /// Handshake failed
    HandshakeFailed = 3,
    /// Encryption operation failed
    EncryptionFailed = 4,
    /// Decryption operation failed
    DecryptionFailed = 5,
    /// Provided buffer is too small
    BufferTooSmall = 6,
    /// Operation invalid in current state
    InvalidState = 7,
    /// General protocol error
    ProtocolError = 8,
}

impl From<crate::core::error::NoiseError> for NoiseErrorCode {
    fn from(err: crate::core::error::NoiseError) -> Self {
        use crate::core::error::NoiseError;
        match err {
            NoiseError::InvalidParameter => NoiseErrorCode::InvalidParameter,
            NoiseError::OutOfMemory => NoiseErrorCode::OutOfMemory,
            NoiseError::HandshakeFailed => NoiseErrorCode::HandshakeFailed,
            NoiseError::EncryptionFailed => NoiseErrorCode::EncryptionFailed,
            NoiseError::DecryptionFailed => NoiseErrorCode::DecryptionFailed,
            NoiseError::BufferTooSmall { .. } => NoiseErrorCode::BufferTooSmall,
            NoiseError::InvalidState(_) => NoiseErrorCode::InvalidState,
            NoiseError::Snow(_) => NoiseErrorCode::ProtocolError,
            NoiseError::ReplayDetected => NoiseErrorCode::DecryptionFailed,
            NoiseError::InvalidMessage => NoiseErrorCode::ProtocolError,
        }
    }
}

/// FFI-safe session mode
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseMode {
    /// Session acts as initiator (client)
    Initiator = 0,
    /// Session acts as responder (server)
    Responder = 1,
}

/// Opaque pointer type for Noise sessions
#[repr(C)]
pub struct NoiseSessionFFI {
    _private: [u8; 0],
}

/// FFI-safe buffer structure for data exchange
#[repr(C)]
pub struct NoiseBuffer {
    /// Pointer to data
    pub data: *mut u8,
    /// Length of data
    pub len: usize,
    /// Capacity of buffer
    pub capacity: usize,
}

impl NoiseBuffer {
    /// Create a new empty buffer
    pub const fn new() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }
    
    /// Check if buffer is null
    pub fn is_null(&self) -> bool {
        self.data.is_null()
    }
}