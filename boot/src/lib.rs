#![no_std]
#![no_main]

// Shared resources module
extern crate common;

// Definition uses
use common::shared::structs::ArrayLike;
use core::panic::PanicInfo;

// - x86-specific structures
use common::arch::x86::structs::{BiosPB, LongE820};

// Initial routine
//  - call it 'main' for the sake of brevity
// TODO
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn main(
    _bios_pb: &BiosPB,
    _bootdev: u64,
    _e820_map: &'static ArrayLike<'static, LongE820>,
) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
