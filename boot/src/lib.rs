#![no_std]
#![no_main]

// Definition uses
extern crate common;
use common::shared::structs::array_like::ArrayLike;
use core::panic::PanicInfo;

// - BIOS-specific structures
use common::plat::pc_bios::structs::{BiosPB, LongE820};
use common::plat::pc_bios::vga::console::VgaConsole;

// Initial routine
//  - call it 'main' for the sake of brevity
// TODO
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn main(
    _bios_pb: &'static BiosPB,
    _bootdev: u64,
    _e820_map: &'static ArrayLike<'static, LongE820>,
) -> ! {
    // Instantiate VGA console
    let mut console = VgaConsole::defaults();

    // Clear screen
    console.clear();

    // Write to screen
    console.write_str("Hello world!\n");
    console.write_str("This is a test!\n");
    console.write_str("The quick brown fox jumps over the lazy dog");

    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
