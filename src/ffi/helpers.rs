//! Helper functions for safe FFI operations

use libc::{c_uchar, size_t};
use std::ptr;
use std::slice;

/// Safely convert a C pointer and length to a Rust slice
/// Returns None if the pointer is null or length is 0
pub unsafe fn c_to_slice<'a>(ptr: *const c_uchar, len: size_t) -> Option<&'a [u8]> {
    if ptr.is_null() || len == 0 {
        None
    } else {
        Some(slice::from_raw_parts(ptr, len))
    }
}

/// Safely convert a mutable C pointer and length to a Rust mutable slice
/// Returns None if the pointer is null or length is 0
pub unsafe fn c_to_slice_mut<'a>(ptr: *mut c_uchar, len: size_t) -> Option<&'a mut [u8]> {
    if ptr.is_null() || len == 0 {
        None
    } else {
        Some(slice::from_raw_parts_mut(ptr, len))
    }
}

/// Safely copy data from a Rust slice to a C buffer
/// Returns true if successful, false if buffer too small
pub unsafe fn copy_to_c_buffer(
    src: &[u8],
    dst: *mut c_uchar,
    dst_len: *mut size_t,
) -> bool {
    if dst_len.is_null() {
        return false;
    }
    
    let required_len = src.len();
    let available_len = *dst_len;
    
    // Always update the length to indicate required size
    *dst_len = required_len;
    
    if dst.is_null() || available_len < required_len {
        return false;
    }
    
    ptr::copy_nonoverlapping(src.as_ptr(), dst, required_len);
    true
}

/// Validate that a session pointer is not null and properly aligned
pub fn validate_session_ptr(ptr: *mut crate::ffi::types::NoiseSessionFFI) -> bool {
    !ptr.is_null() && (ptr as usize) % std::mem::align_of::<crate::core::session::NoiseSession>() == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_c_to_slice_null() {
        unsafe {
            assert!(c_to_slice(ptr::null(), 10).is_none());
            assert!(c_to_slice(ptr::null(), 0).is_none());
        }
    }
    
    #[test]
    fn test_c_to_slice_valid() {
        let data = vec![1u8, 2, 3, 4, 5];
        unsafe {
            let slice = c_to_slice(data.as_ptr(), data.len()).unwrap();
            assert_eq!(slice, &data[..]);
        }
    }
    
    #[test]
    fn test_copy_to_c_buffer() {
        let src = vec![1u8, 2, 3, 4, 5];
        let mut dst = vec![0u8; 10];
        let mut dst_len = dst.len();
        
        unsafe {
            assert!(copy_to_c_buffer(&src, dst.as_mut_ptr(), &mut dst_len));
            assert_eq!(dst_len, src.len());
            assert_eq!(&dst[..src.len()], &src[..]);
        }
    }
    
    #[test]
    fn test_copy_to_c_buffer_too_small() {
        let src = vec![1u8, 2, 3, 4, 5];
        let mut dst = vec![0u8; 3];
        let mut dst_len = dst.len();
        
        unsafe {
            assert!(!copy_to_c_buffer(&src, dst.as_mut_ptr(), &mut dst_len));
            assert_eq!(dst_len, src.len()); // Should be updated to required size
        }
    }
}