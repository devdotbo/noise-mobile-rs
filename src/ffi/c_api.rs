//! C-compatible API for the noise-mobile-rust library

use crate::core::session::NoiseSession;
use crate::ffi::types::{NoiseErrorCode, NoiseSessionFFI};
use libc::{c_char, c_int, c_uchar, size_t};
use std::ptr;
use std::slice;

// Constants for C API
pub const NOISE_MODE_INITIATOR: c_int = 0;
pub const NOISE_MODE_RESPONDER: c_int = 1;

pub const NOISE_ERROR_SUCCESS: c_int = 0;
pub const NOISE_ERROR_INVALID_PARAMETER: c_int = 1;
pub const NOISE_ERROR_OUT_OF_MEMORY: c_int = 2;
pub const NOISE_ERROR_HANDSHAKE_FAILED: c_int = 3;
pub const NOISE_ERROR_ENCRYPTION_FAILED: c_int = 4;
pub const NOISE_ERROR_DECRYPTION_FAILED: c_int = 5;
pub const NOISE_ERROR_BUFFER_TOO_SMALL: c_int = 6;
pub const NOISE_ERROR_INVALID_STATE: c_int = 7;
pub const NOISE_ERROR_PROTOCOL_ERROR: c_int = 8;

/// Create a new Noise session
#[no_mangle]
pub extern "C" fn noise_session_new(
    mode: c_int,
    error: *mut c_int,
) -> *mut NoiseSessionFFI {
    if error.is_null() {
        return ptr::null_mut();
    }
    
    let session = match mode {
        0 => NoiseSession::new_initiator(),
        1 => NoiseSession::new_responder(),
        _ => {
            unsafe { *error = NoiseErrorCode::InvalidParameter as c_int; }
            return ptr::null_mut();
        }
    };
    
    match session {
        Ok(s) => {
            unsafe { *error = NoiseErrorCode::Success as c_int; }
            Box::into_raw(Box::new(s)) as *mut NoiseSessionFFI
        }
        Err(e) => {
            unsafe { *error = NoiseErrorCode::from(e) as c_int; }
            ptr::null_mut()
        }
    }
}

/// Create a new Noise session with a specific private key
#[no_mangle]
pub extern "C" fn noise_session_new_with_key(
    private_key: *const c_uchar,
    private_key_len: size_t,
    mode: c_int,
    error: *mut c_int,
) -> *mut NoiseSessionFFI {
    if error.is_null() || private_key.is_null() || private_key_len != 32 {
        if !error.is_null() {
            unsafe { *error = NoiseErrorCode::InvalidParameter as c_int; }
        }
        return ptr::null_mut();
    }
    
    let private_key_slice = unsafe { slice::from_raw_parts(private_key, private_key_len) };
    
    let is_initiator = match mode {
        0 => true,
        1 => false,
        _ => {
            unsafe { *error = NoiseErrorCode::InvalidParameter as c_int; }
            return ptr::null_mut();
        }
    };
    
    match NoiseSession::with_private_key(private_key_slice, is_initiator) {
        Ok(s) => {
            unsafe { *error = NoiseErrorCode::Success as c_int; }
            Box::into_raw(Box::new(s)) as *mut NoiseSessionFFI
        }
        Err(e) => {
            unsafe { *error = NoiseErrorCode::from(e) as c_int; }
            ptr::null_mut()
        }
    }
}

/// Free a Noise session
#[no_mangle]
pub extern "C" fn noise_session_free(session: *mut NoiseSessionFFI) {
    if !session.is_null() {
        unsafe {
            let _ = Box::from_raw(session as *mut NoiseSession);
        }
    }
}

/// Write a handshake message
#[no_mangle]
pub extern "C" fn noise_write_message(
    session: *mut NoiseSessionFFI,
    payload: *const c_uchar,
    payload_len: size_t,
    output: *mut c_uchar,
    output_len: *mut size_t,
) -> c_int {
    if !crate::ffi::helpers::validate_session_ptr(session) || output_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &mut *(session as *mut NoiseSession) };
    let payload_slice = unsafe { 
        crate::ffi::helpers::c_to_slice(payload, payload_len).unwrap_or(&[])
    };
    
    match session.write_message(payload_slice) {
        Ok(msg) => {
            if unsafe { crate::ffi::helpers::copy_to_c_buffer(&msg, output, output_len) } {
                NoiseErrorCode::Success as c_int
            } else {
                NoiseErrorCode::BufferTooSmall as c_int
            }
        }
        Err(e) => NoiseErrorCode::from(e) as c_int,
    }
}

/// Read a handshake message
#[no_mangle]
pub extern "C" fn noise_read_message(
    session: *mut NoiseSessionFFI,
    input: *const c_uchar,
    input_len: size_t,
    payload: *mut c_uchar,
    payload_len: *mut size_t,
) -> c_int {
    if !crate::ffi::helpers::validate_session_ptr(session) || input.is_null() || payload_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &mut *(session as *mut NoiseSession) };
    let input_slice = match unsafe { crate::ffi::helpers::c_to_slice(input, input_len) } {
        Some(slice) => slice,
        None => return NoiseErrorCode::InvalidParameter as c_int,
    };
    
    match session.read_message(input_slice) {
        Ok(msg) => {
            if msg.is_empty() {
                unsafe { *payload_len = 0; }
                NoiseErrorCode::Success as c_int
            } else if unsafe { crate::ffi::helpers::copy_to_c_buffer(&msg, payload, payload_len) } {
                NoiseErrorCode::Success as c_int
            } else {
                NoiseErrorCode::BufferTooSmall as c_int
            }
        }
        Err(e) => NoiseErrorCode::from(e) as c_int,
    }
}

/// Check if handshake is complete
#[no_mangle]
pub extern "C" fn noise_is_handshake_complete(session: *mut NoiseSessionFFI) -> c_int {
    if !crate::ffi::helpers::validate_session_ptr(session) {
        return 0;
    }
    
    let session = unsafe { &*(session as *mut NoiseSession) };
    if session.is_transport_state() { 1 } else { 0 }
}

/// Encrypt a message
#[no_mangle]
pub extern "C" fn noise_encrypt(
    session: *mut NoiseSessionFFI,
    plaintext: *const c_uchar,
    plaintext_len: size_t,
    ciphertext: *mut c_uchar,
    ciphertext_len: *mut size_t,
) -> c_int {
    if !crate::ffi::helpers::validate_session_ptr(session) || ciphertext_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &mut *(session as *mut NoiseSession) };
    let plaintext_slice = match unsafe { crate::ffi::helpers::c_to_slice(plaintext, plaintext_len) } {
        Some(slice) => slice,
        None => return NoiseErrorCode::InvalidParameter as c_int,
    };
    
    match session.encrypt(plaintext_slice) {
        Ok(ct) => {
            if unsafe { crate::ffi::helpers::copy_to_c_buffer(&ct, ciphertext, ciphertext_len) } {
                NoiseErrorCode::Success as c_int
            } else {
                NoiseErrorCode::BufferTooSmall as c_int
            }
        }
        Err(e) => NoiseErrorCode::from(e) as c_int,
    }
}

/// Decrypt a message
#[no_mangle]
pub extern "C" fn noise_decrypt(
    session: *mut NoiseSessionFFI,
    ciphertext: *const c_uchar,
    ciphertext_len: size_t,
    plaintext: *mut c_uchar,
    plaintext_len: *mut size_t,
) -> c_int {
    if !crate::ffi::helpers::validate_session_ptr(session) || plaintext_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &mut *(session as *mut NoiseSession) };
    let ciphertext_slice = match unsafe { crate::ffi::helpers::c_to_slice(ciphertext, ciphertext_len) } {
        Some(slice) => slice,
        None => return NoiseErrorCode::InvalidParameter as c_int,
    };
    
    match session.decrypt(ciphertext_slice) {
        Ok(pt) => {
            if unsafe { crate::ffi::helpers::copy_to_c_buffer(&pt, plaintext, plaintext_len) } {
                NoiseErrorCode::Success as c_int
            } else {
                NoiseErrorCode::BufferTooSmall as c_int
            }
        }
        Err(e) => NoiseErrorCode::from(e) as c_int,
    }
}

/// Get the remote peer's static public key
#[no_mangle]
pub extern "C" fn noise_get_remote_static(
    session: *mut NoiseSessionFFI,
    output: *mut c_uchar,
    output_len: *mut size_t,
) -> c_int {
    if !crate::ffi::helpers::validate_session_ptr(session) || output_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &*(session as *mut NoiseSession) };
    
    match session.get_remote_static() {
        Some(key) => {
            if unsafe { crate::ffi::helpers::copy_to_c_buffer(key, output, output_len) } {
                NoiseErrorCode::Success as c_int
            } else {
                NoiseErrorCode::BufferTooSmall as c_int
            }
        }
        None => {
            unsafe { *output_len = 0; }
            NoiseErrorCode::InvalidState as c_int
        }
    }
}

/// Get the maximum message length
#[no_mangle]
pub extern "C" fn noise_max_message_len() -> size_t {
    crate::core::crypto::NOISE_MAX_MESSAGE_LEN
}

/// Get the maximum payload length
#[no_mangle]
pub extern "C" fn noise_max_payload_len() -> size_t {
    crate::core::crypto::NOISE_MAX_PAYLOAD_LEN
}

/// Get error string for an error code
#[no_mangle]
pub extern "C" fn noise_error_string(error: c_int) -> *const c_char {
    match error {
        0 => b"Success\0".as_ptr() as *const c_char,
        1 => b"Invalid parameter\0".as_ptr() as *const c_char,
        2 => b"Out of memory\0".as_ptr() as *const c_char,
        3 => b"Handshake failed\0".as_ptr() as *const c_char,
        4 => b"Encryption failed\0".as_ptr() as *const c_char,
        5 => b"Decryption failed\0".as_ptr() as *const c_char,
        6 => b"Buffer too small\0".as_ptr() as *const c_char,
        7 => b"Invalid state\0".as_ptr() as *const c_char,
        8 => b"Protocol error\0".as_ptr() as *const c_char,
        _ => b"Unknown error\0".as_ptr() as *const c_char,
    }
}