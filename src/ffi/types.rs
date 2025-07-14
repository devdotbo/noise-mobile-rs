use libc::c_int;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseErrorCode {
    Success = 0,
    InvalidParameter = 1,
    OutOfMemory = 2,
    HandshakeFailed = 3,
    EncryptionFailed = 4,
    DecryptionFailed = 5,
    BufferTooSmall = 6,
    InvalidState = 7,
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
            NoiseError::Snow(_) => NoiseErrorCode::HandshakeFailed,
            _ => NoiseErrorCode::HandshakeFailed,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseMode {
    Initiator = 0,
    Responder = 1,
}

#[repr(C)]
pub struct NoiseSessionFFI {
    _private: [u8; 0],
}