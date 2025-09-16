/*
    x86-specific I/O defintions
*/

// Export submodules
pub mod uart;
pub mod console;

// Definition uses
use core::arch::asm;
use core::mem;

// Read byte from provided port
#[inline(always)]
pub unsafe fn in_b(port: u16) -> u8 {
    let val: u8;
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") val,
        );
    }

    val
}

// Read word from provided port
#[inline(always)]
pub unsafe fn in_w(port: u16) -> u16 {
    let val: u16;
    unsafe {
        asm!(
            "in ax, dx",
            in("dx") port,
            out("ax") val,
        );
    }

    val
}

// Read double word from provided port
#[inline(always)]
pub unsafe fn in_d(port: u16) -> u32 {
    let val: u32;
    unsafe {
        asm!(
            "in eax, dx",
            in("dx") port,
            out("eax") val,
        );
    }

    val
}

// Write provided byte to provided port
#[inline(always)]
pub unsafe fn out_b(port: u16, val: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") val,
        );
    }
}

// Write provided word to provided port
#[inline(always)]
pub unsafe fn out_w(port: u16, val: u16) {
    unsafe {
        asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") val,
        );
    }
}

// Write provided double word to provided port
#[inline(always)]
pub unsafe fn out_d(port: u16, val: u32) {
    unsafe {
        asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") val,
        );
    }
}

// Performs a non-overlapping copy between two memory regions
// - length is explicitly provided
// - caller must ensure that both arguments point
//   to valid memory (writing to read-only memory is UB,
//   reading from uninitialized memory is UB)
// TODO: remove or simplify redundant operations
pub unsafe extern "C" fn __memcpy_checked(dest: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    // Guard against redundancy
    if len == 0 {
        return dest;
    }

    // Set up remaining bytes count
    let mut remaining = len;

    let mut dest_ptr = dest;
    let mut src_ptr = src;

    // Make sure the regions don't trivially overlap
    assert!(
        dest.addr() != src.addr(), 
        "Destination and source regions trivially overlap"
    );

    // Find the leftmost and the rightmost regions
    let right_start = dest.addr().max(src.addr());
    let left_end = unsafe { dest.add(len).addr().min(src.add(len).addr()) };

    // Make sure the regions don't overlap at all
    // - make sure the condition
    // `left_start <= right_start <= left_end <= right_end`
    // is never satisfied (we only need to account
    // for the middle condition)
    assert!(
        right_start > left_end,
        "Destination and source regions partly overlap"
    );

    // Obtain alignment for `usize`
    let align = mem::align_of::<usize>();

    // - perform bytewise copy if the provided
    // regions are sufficiently small
    if len <= 2 * align {
        return unsafe { __memcpy_bytewise(dest, src, len) };
    }

    // Calculate aligned addresses
    let dest_start_offset = align - dest.align_offset(align);
    let src_start_offset = align - src.align_offset(align);
    let dest_end_offset = unsafe { dest.add(len).align_offset(align) };
    let src_end_offset = unsafe { src.add(len).align_offset(align) };

    // Do not proceed if the operands don't have
    // the same alignment offset
    assert!(
        dest_start_offset == src_start_offset && dest_end_offset == src_end_offset,
        "Destination and source alignment offsets do not match"
    );

    // Perform byte-wise copy of unaligned bytes
    let n = remaining.min(dest_start_offset);
    let _ = unsafe { __memcpy_bytewise(dest, src, n) };
    remaining -= n;

    // - stop if no bytes are remaining
    if remaining == 0 {
        return dest;
    }

    unsafe {
        // Increment pointers
        dest_ptr = dest_ptr.add(n);
        src_ptr = src_ptr.add(n);

        // At this point, `dest_ptr` and `src_ptr` should
        // be aligned, allowing us to use `__memcpy`
        let _ = __memcpy(dest_ptr, src_ptr, remaining);
    }

    // By convention, we should return an
    // immutable pointer to the destination
    dest
}

// Perform bytewise non-overlapping copy
#[inline(always)]
pub unsafe extern "C" fn __memcpy_bytewise(dest: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    for i in 0..len {
        unsafe { *dest.add(i) = *src.add(i); }
    }

    dest
}

// Performs a copy between non-overlapping memory regions
// - performs word-sized copies whenever possible,
//   completing the rest of the copy using
//   byte-wise copying
// - assumes aligned pointers at the input
// - inlined whenever possible to reduce call
//   overhead (may not apply to external
//   callers)
// - length is explicitly provided
// - caller must ensure that both arguments point
//   to valid memory (writing to read-only memory is UB,
//   reading from uninitialized memory is UB)
// TODO: implement vectorized copying
#[inline(always)]
pub unsafe extern "C" fn __memcpy(dest: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    // Guard against redundancy
    if len == 0 {
        return dest;
    }

    // Copy destination and source pointers
    let mut dest_ptr_word = dest as *mut usize;
    let mut src_ptr_word = src as *const usize;

    // Obtain alignment for `usize`
    // - serves as a pessimistic proxy for its
    // size in memory
    let align = mem::align_of::<usize>();

    // Calculate word-sized blocks and remainder
    let num_word = len / align;
    let num_byte = len % align;

    // Copy word-sized blocks
    for _ in 0..num_word {
        unsafe {
            *dest_ptr_word = *src_ptr_word;

            // - shift destination and source
            // pointers by one word size
            dest_ptr_word = dest_ptr_word.add(1);
            src_ptr_word = src_ptr_word.add(1);
        }
    }

    // Re-cast the pointers and copy remaining bytes
    let _ = unsafe { 
        __memcpy_bytewise(
            dest_ptr_word as *mut u8,
            src_ptr_word as *const u8,
            num_byte
        )
    };

    // By convention, `memcpy` either returns a
    // pointer to a "suitable created object",
    // or a pointer to the destination buffer
    dest
}