#![no_std]
#![no_main]

mod macros;
mod structs;

use core::panic::PanicInfo;
use structs::BiosPB;

// Initial routine
//  - call it 'main' for the sake of brevity
// TODO
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn main(
    _bpb: &BiosPB, 
    _bootdev: u64, 
    _e820_ptr: *const u8) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
