#![no_std]
#![no_main]

mod macros;
mod structs;

use core::panic::PanicInfo;
use core::ptr::write_volatile;
use structs::BiosPB;

// Define HAL section
relocate! {
    fn default_inb(port: u16) -> u8 {
        (port as u8) + 3
    } => ".hal";

    fn default_outb(port: u16, arg: u8) {

    } => ".hal";

    fn default_inw(port: u16) -> u16 {
        2
    } => ".hal";

    fn default_outw(port: u16, arg: u16) {

    } => ".hal";
}

// Define HAL vector table section
// Call template struct 'HalDispatches'
hal_vt_instance! {
    pub static HAL_VT: HalDispatches = {
        inb: extern "C" fn(u16) -> u8 = default_inb,
        outb: extern "C" fn(u16, u8) = default_outb,
        inw: extern "C" fn(u16) -> u16 = default_inw,
        outw: extern "C" fn(u16, u16) = default_outw,
    }; => ".vt_hal";
}

// Initial routine
//  - call it 'main' for the sake of brevity
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn main(bpb: &BiosPB, bootdev: u64, e820_ptr: *const u8) -> ! {
    let x = HAL_VT.dispatch(|x| &x.inb);

    let mut a: u8 = 0;

    loop {
        unsafe {
            write_volatile(&mut a as *mut u8, a + 2);
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
