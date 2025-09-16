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

// Determines whether two equally sized buffers overlap
#[inline(always)]
fn is_overlapping(dest: *const u8, src: *const u8, len: usize) -> bool {
    // Obtain destination and source addresses
    let dest_addr = dest.addr();
    let src_addr = src.addr();

    let dest_end = dest_addr + len;
    let src_end = src_addr + len;

    // Find the leftmost and the rightmost regions
    let right_start = dest_addr.max(src_addr);
    let left_end = dest_end.min(src_end);

    dest_addr == src_addr || right_start <= left_end
}

// Determines whether two equally sized buffers have
// the same alignment offset
#[inline(always)]
fn is_align_offset_eq(dest: *const u8, src: *const u8, len: usize) -> bool {
    // Obtain alignment for `usize`
    let align = mem::align_of::<usize>();

    // Calculate aligned addresses
    let dest_start_offset = align - dest.align_offset(align);
    let src_start_offset = align - src.align_offset(align);
    let dest_end_offset = unsafe { dest.add(len).align_offset(align) };
    let src_end_offset = unsafe { src.add(len).align_offset(align) };

    dest_start_offset == src_start_offset && dest_end_offset == src_end_offset
}

// Performs a non-overlapping copy between two memory regions
// - length is explicitly provided
// - caller must ensure that both arguments point to
//   valid memory (writing to read-only memory is UB,
//   reading from uninitialized memory is UB)
// - alignment and non-overlap are strongly recommended,
//   though the method won't fail (instead leading to UB)
// TODO: remove or simplify redundant operations
pub unsafe fn __memcpy(dest: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    // Guard against redundancy
    if len == 0 {
        return dest;
    }

    // Set up remaining bytes count
    let mut remaining = len;

    let mut dest_ptr = dest;
    let mut src_ptr = src;

    // Obtain alignment for `usize`
    let align = mem::align_of::<usize>();

    // Perform bytewise copy if
    // 1. the provided regions are sufficiently small, or
    // 2. either regions overlap, or
    // 3. alignment offsets don't match
    if is_overlapping(dest, src, len) || len <= 2 * align || !is_align_offset_eq(dest, src, len) {
        return unsafe { copy_bytes(dest, src, len) }
    }

    // Calculate prefix size
    let size_prefix = align - dest.align_offset(align);

    // Perform byte-wise copy of unaligned bytes
    let n = remaining.min(size_prefix);
    let _ = unsafe { copy_bytes(dest, src, n) };
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
        let _ = __memcpy_unchecked(dest_ptr, src_ptr, remaining);
    }

    // By convention, we should return an
    // immutable pointer to the destination
    dest
}

// Performs byte-wise non-overlapping copy
// - uses R-registers
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn copy_bytes(dest: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    // - DF must be cleared for `movs`
    // to work in left-to-right mode
    unsafe {
        asm!(
            "cld",
            "rep movsb",
            in("rcx") len,
            in("rsi") src,
            in("rdi") dest,
        );
    }

    dest
}

// Performs byte-wise non-overlapping copy
// - uses E-registers
#[cfg(target_arch = "x86")]
#[inline(always)]
unsafe fn copy_bytes(dest: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    // - DF must be cleared for `movs`
    // to work in left-to-right mode
    unsafe {
        asm!(
            "cld",
            "rep movsb",
            in("ecx") len,
            in("esi") src,
            in("edi") dest,
        );
    }

    dest
}

// Performs word-wise non-overlapping copy
// - uses R-registers
// - aligned access recommended
#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn copy_words(dest: *mut usize, src: *const usize, len: usize) -> *mut usize {
    // - DF must be cleared for `movs`
    // to work in left-to-right mode
    unsafe {
        asm!(
            "cld",
            "rep movsq",
            in("rcx") len,
            in("rsi") src,
            in("rdi") dest,
        );
    }

    dest
}

// Performs word-wise non-overlapping copy
// - uses E-registers
// - aligned access recommended
#[cfg(target_arch = "x86")]
#[inline(always)]
unsafe fn copy_words(dest: *mut usize, src: *const usize, len: usize) -> *mut usize {
    // - DF must be cleared for `movs`
    // to work in left-to-right mode
    unsafe {
        asm!(
            "cld",
            "rep movsd",
            in("ecx") len,
            in("esi") src,
            in("edi") dest,
        );
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
// - must be invisible to the compiler, as `memcpy` is
//   intended to be exposed elsewhere
// TODO: implement vectorized copying
#[inline(always)]
pub unsafe fn __memcpy_unchecked(dest: *mut u8, src: *const u8, len: usize) -> *mut u8 {
    // Guard against redundancy
    if len == 0 {
        return dest;
    }

    // Copy destination and source pointers
    // - endianness might play a role here, though
    // we are simply copying without performing
    // intermediate operations, so byte order
    // shouldn't be affected at all

    // Obtain size of `usize`
    let size_word = mem::size_of::<usize>();

    // Calculate word-sized blocks and remainder
    let num_word = len / size_word;
    let num_byte = len % size_word;

    // Copy word-sized blocks
    if num_word != 0 {
        let _ = unsafe { 
            copy_words(dest as *mut usize, src as *const usize, num_word)
        };
    }
    

    // Re-cast the pointers and copy remaining bytes
    if num_byte != 0 {
        let _ = unsafe { 
            copy_bytes(
                dest.add(num_word * size_word),
                src.add(num_word * size_word),
                num_byte,
            )
        };
    }
    
    // By convention, `memcpy` either returns a
    // pointer to a "suitable created object",
    // or a pointer to the destination buffer
    dest
}
