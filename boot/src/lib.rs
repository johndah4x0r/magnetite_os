#![no_std]
#![no_main]

mod structs;
mod macros;

use core::mem::drop;
use core::arch::asm;
use core::ptr::write_volatile;
use core::panic::PanicInfo;
use core::ptr::read_volatile;
use structs::{BiosPB, HalVectorTable};

// Define HAL
relocate! {
    fn inb(port: u16) -> u8 {
        (port as u8) + 3
    } => ".hal";

    fn outb(port: u16, arg: u8) {

    } => ".hal";

    fn inw(port: u16) -> u16 {
        2
    } => ".hal";

    fn outw(port: u16, arg: u16) {

    } => ".hal";

    pub static HAL_VT: HalVectorTable = HalVectorTable {
        inb,
        outb,
        inw,
        outw,
    }; => ".vt_hal";
}

// Initial routine
//  - call it 'main' for the sake of brevity
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn main(bpb: &BiosPB, bootdev: u64, e820_ptr: *const u8) -> ! {
    // Just for the sake of laughs and giggles!
    let vt: *const HalVectorTable = &HAL_VT as *const HalVectorTable as usize as *const HalVectorTable;
    unsafe { asm!("xchg bx, bx", "nop", options(nomem)); }

    // This dereference might get inlined, so
    // volatile access is very much preferred.
    let mut f = unsafe { read_volatile(vt) };

    let mut a: u8 = 0;

    loop {
        unsafe { write_volatile(&mut a as *mut u8, a + 2); }
    }

    drop(f);
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}