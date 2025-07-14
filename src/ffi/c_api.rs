use crate::core::session::NoiseSession;
use crate::ffi::types::{NoiseErrorCode, NoiseMode, NoiseSessionFFI};
use libc::{c_int, c_uchar, size_t};
use std::ptr;

#[no_mangle]
pub extern "C" fn noise_session_new(
    mode: c_int,
    error: *mut c_int,
) -> *mut NoiseSessionFFI {
    if error.is_null() {
        return ptr::null_mut();
    }
    
    // Placeholder implementation
    unsafe { *error = NoiseErrorCode::Success as c_int; }
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn noise_session_free(session: *mut NoiseSessionFFI) {
    if !session.is_null() {
        unsafe {
            let _ = Box::from_raw(session as *mut NoiseSession);
        }
    }
}

#[no_mangle]
pub extern "C" fn noise_max_message_len() -> size_t {
    crate::core::crypto::NOISE_MAX_MESSAGE_LEN
}

#[no_mangle]
pub extern "C" fn noise_max_payload_len() -> size_t {
    crate::core::crypto::NOISE_MAX_PAYLOAD_LEN
}