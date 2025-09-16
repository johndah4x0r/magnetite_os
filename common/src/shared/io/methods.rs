/*
    Shared methods that wrap around platform-specific routines
*/

// Expect the current architecture module to
// expose `__memcpy`
use crate::arch::__arch::io::__memcpy;

// Performs a copy between two non-overlapping buffers
// - length is explicitly provided, as per
//   the C contract for `memcpy`
// - using `unsafe(no_mangle)` in Rust 2024 edition
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    // - use platform-specific intrinsic,
    //   propagating the returned pointer
    unsafe { __memcpy(dest, src, len) }
}

// Performs a copy between two non-overlapping slices
// - length is provided by the smallest slice
#[inline(always)]
pub fn __copy_nonoverlapping<'a>(dest: &'a mut [u8], src: &'a [u8]) -> &'a [u8] {
    // Calculate smallest length
    let len = dest.len().min(src.len());

    // - no-op if smallest length is equal to zero
    if len > 0 {
        let _ = unsafe { __memcpy(&mut dest[0], &src[0], len) };
    }

    // By convention, we should return an
    // immutable reference to the destination
    dest as &[u8]
}