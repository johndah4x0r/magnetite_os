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

// Performs a non-overlapping copy between two
// memory regions
// - length is explicitly provided
// - caller must ensure that both arguments point
// to valid memory (writing to read-only memory is UB,
// reading from uninitialized memory is UB)
// TODO: remove or simplify redundant operations
#[inline(always)]
pub unsafe fn __memcpy_checked(dest: *mut u8, src: *const u8, len: usize) -> *const u8 {
    // Guard against redundancy
    if len == 0 {
        return dest as *const u8;
    }

    // Set up remaining bytes count
    let mut remaining = len;

    // Find where the destination and source
    // buffers are located in memory
    let dest_start = dest as usize;
    let src_start = src as usize;

    let mut dest_ptr = dest;
    let mut src_ptr = src;

    // Find where the buffers end
    let dest_end = dest_start + len;
    let src_end =  src_start + len;

    // Make sure the regions don't trivially overlap
    assert!(
        dest_start != src_start && dest_end != src_end,
        "Destination and source regions trivially overlap"
    );

    // Find the leftmost and the rightmost regions
    let right_start = dest_start.max(src_start);
    let left_end = dest_end.min(src_end);

    // Make sure the regions don't overlap at all
    // - make sure the condition
    // `left_start <= right_start <= left_end <= right_end`
    // is never satisfied (we only need to account
    // for the middle condition)
    assert!(
        right_start > left_end,
        "Destination and source regions partly overlap"
    );

    // Obtain alignment and aligned addresses for `usize`
    let align = mem::align_of::<usize>();
    let dest_start_offset = align - dest_start % align;
    let src_start_offset = align - src_start % align;
    let dest_end_offset = dest_end % align;
    let src_end_offset = src_end % align;

    let aligned_dest_start = dest_start + dest_start_offset;
    let aligned_src_start = src_start + src_start_offset;
    let aligned_dest_end = dest_end - dest_end_offset;
    let aligned_src_start = src_end - src_end_offset;

    // Do not proceed if the operands don't
    // have the same alignment offset
    assert!(
        dest_start_offset == src_start_offset && dest_end_offset == src_end_offset,
        "Destination and source alignment offsets do not match"
    );

    // Perform byte-wise copy of unaligned bytes
    let n = remaining.min(align - dest_start_offset);

    for i in 0..n {
        unsafe { *dest_ptr.add(i) = *src_ptr.add(i); }
    }

    remaining -= n;

    if remaining == 0 {
        return dest as *const u8;
    }

    // Calculate number of blocks to copy

    // By convention, we should return an
    // immutable pointer to the destination
    dest as *const u8
}