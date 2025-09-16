/*
    Shared methods that wrap around platform-specific routines
*/

// Expect the current architecture module to
// expose `__memcpy`
use crate::arch::__arch::io::__memcpy_checked;

// Perform a non-overlapping copy between two slices
// - length is provided by the smallest slice
#[inline(always)]
pub fn __copy_nonoverlapping<'a>(dest: &'a mut [u8], src: &'a [u8]) -> &'a [u8] {
    // Calculate smallest length
    let len = dest.len().min(src.len());

    // - no-op if smallest length is equal to zero
    if len > 0 {
        let _ = unsafe { __memcpy_checked(&mut dest[0], &src[0], len) };
    }

    // By convention, we should return an
    // immutable reference to the destination
    dest as &[u8]
}