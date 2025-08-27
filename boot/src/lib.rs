#![no_std]
#![no_main]

mod macros;
mod structs;

use core::panic::PanicInfo;
use core::ptr::write_volatile;
use structs::{ArrayLike, BiosPB, LongE820};

// Define HAL section
// TODO
relocate! {
    fn default_inb(port: u16) -> u8 {
        (port as u8) + 3
    } => ".hal";

    fn default_outb(port: u16, _arg: u8) {
        let _ = port;
    } => ".hal";

    fn default_inw(port: u16) -> u16 {
        port + 2
    } => ".hal";

    fn default_outw(port: u16, _arg: u16) {
        let _ = port;
    } => ".hal";

    fn default_ind(port: u16) -> u32 {
        (port as u32) + 2
    } => ".hal";

    fn default_outd(port: u16, _arg: u32) {
        let _ = port;
    } => ".hal";
}

// Define HAL vector table section
// Call template struct 'HalDispatches'
// TODO
hal_vt_instance! {
    pub static HAL_VT: HalDispatches = {
        inb: extern "C" fn(u16) -> u8 = default_inb,
        outb: extern "C" fn(u16, u8) = default_outb,
        inw: extern "C" fn(u16) -> u16 = default_inw,
        outw: extern "C" fn(u16, u16) = default_outw,
        ind: extern "C" fn(u16) -> u32 = default_ind,
        outd: extern "C" fn(u16, u32) = default_outd,
    }; => ".vt_hal";
}

// Initial routine
//  - call it 'main' for the sake of brevity
// TODO
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn main(
    bios_pb: &BiosPB,
    bootdev: u64,
    e820_map: &'static ArrayLike<'static, LongE820>,
) -> ! {
    let mut a: u8 = HAL_VT.dispatch(|x| &x.inb, |&f| f(0x3f8));

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
