#![no_std]
#![no_main]

mod structs;

use core::panic::PanicInfo;
use structs::BiosPB;

// Initial routine
//  - call it 'main' for the sake of brevity
#[unsafe(no_mangle)]
pub extern "C" fn main(bpb: &BiosPB) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}