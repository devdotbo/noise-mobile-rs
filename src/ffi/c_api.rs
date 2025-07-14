//! C-compatible API for the noise-mobile-rust library

use crate::core::session::NoiseSession;
use crate::ffi::types::{NoiseErrorCode, NoiseSessionFFI};
use libc::{c_char, c_int, c_uchar, size_t};
use std::ptr;
use std::slice;

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
    if session.is_null() || output.is_null() || output_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &mut *(session as *mut NoiseSession) };
    let payload_slice = if payload.is_null() || payload_len == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(payload, payload_len) }
    };
    
    match session.write_message(payload_slice) {
        Ok(msg) => {
            let msg_len = msg.len();
            let available_len = unsafe { *output_len };
            
            if available_len < msg_len {
                unsafe { *output_len = msg_len; }
                return NoiseErrorCode::BufferTooSmall as c_int;
            }
            
            unsafe {
                ptr::copy_nonoverlapping(msg.as_ptr(), output, msg_len);
                *output_len = msg_len;
            }
            NoiseErrorCode::Success as c_int
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
    if session.is_null() || input.is_null() || payload_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &mut *(session as *mut NoiseSession) };
    let input_slice = unsafe { slice::from_raw_parts(input, input_len) };
    
    match session.read_message(input_slice) {
        Ok(msg) => {
            let msg_len = msg.len();
            
            if msg_len > 0 && !payload.is_null() {
                let available_len = unsafe { *payload_len };
                
                if available_len < msg_len {
                    unsafe { *payload_len = msg_len; }
                    return NoiseErrorCode::BufferTooSmall as c_int;
                }
                
                unsafe {
                    ptr::copy_nonoverlapping(msg.as_ptr(), payload, msg_len);
                }
            }
            
            unsafe { *payload_len = msg_len; }
            NoiseErrorCode::Success as c_int
        }
        Err(e) => NoiseErrorCode::from(e) as c_int,
    }
}

/// Check if handshake is complete
#[no_mangle]
pub extern "C" fn noise_is_handshake_complete(session: *mut NoiseSessionFFI) -> c_int {
    if session.is_null() {
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
    if session.is_null() || plaintext.is_null() || ciphertext.is_null() || ciphertext_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &mut *(session as *mut NoiseSession) };
    let plaintext_slice = unsafe { slice::from_raw_parts(plaintext, plaintext_len) };
    
    match session.encrypt(plaintext_slice) {
        Ok(ct) => {
            let ct_len = ct.len();
            let available_len = unsafe { *ciphertext_len };
            
            if available_len < ct_len {
                unsafe { *ciphertext_len = ct_len; }
                return NoiseErrorCode::BufferTooSmall as c_int;
            }
            
            unsafe {
                ptr::copy_nonoverlapping(ct.as_ptr(), ciphertext, ct_len);
                *ciphertext_len = ct_len;
            }
            NoiseErrorCode::Success as c_int
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
    if session.is_null() || ciphertext.is_null() || plaintext.is_null() || plaintext_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &mut *(session as *mut NoiseSession) };
    let ciphertext_slice = unsafe { slice::from_raw_parts(ciphertext, ciphertext_len) };
    
    match session.decrypt(ciphertext_slice) {
        Ok(pt) => {
            let pt_len = pt.len();
            let available_len = unsafe { *plaintext_len };
            
            if available_len < pt_len {
                unsafe { *plaintext_len = pt_len; }
                return NoiseErrorCode::BufferTooSmall as c_int;
            }
            
            unsafe {
                ptr::copy_nonoverlapping(pt.as_ptr(), plaintext, pt_len);
                *plaintext_len = pt_len;
            }
            NoiseErrorCode::Success as c_int
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
    if session.is_null() || output.is_null() || output_len.is_null() {
        return NoiseErrorCode::InvalidParameter as c_int;
    }
    
    let session = unsafe { &*(session as *mut NoiseSession) };
    
    match session.get_remote_static() {
        Some(key) => {
            let key_len = key.len();
            let available_len = unsafe { *output_len };
            
            if available_len < key_len {
                unsafe { *output_len = key_len; }
                return NoiseErrorCode::BufferTooSmall as c_int;
            }
            
            unsafe {
                ptr::copy_nonoverlapping(key.as_ptr(), output, key_len);
                *output_len = key_len;
            }
            NoiseErrorCode::Success as c_int
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