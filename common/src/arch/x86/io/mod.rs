/*
    x86-specific I/O defintions
*/

// Export submodules
pub mod uart;

// Definition uses
use core::arch::asm;

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
