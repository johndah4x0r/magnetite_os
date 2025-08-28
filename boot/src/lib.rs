#![no_std]
#![no_main]
#![feature(ptr_metadata)]

mod macros;
mod structs;

use core::arch::asm;
use core::panic::PanicInfo;
use core::ptr::write_volatile;
use structs::{ArrayLike, BiosPB, LongE820};

// Define HAL section
// TODO
relocate! {
    unsafe fn default_inb(port: u16) -> u8 {
        let mut val: u8;
        unsafe { asm!(
            "in al, dx",
            in("dx") port,
            out("al") val,
        ) };
        val
    } => ".hal.text";

    unsafe fn default_outb(port: u16, arg: u8) {
        unsafe { asm!(
            "out dx, al",
            in("dx") port,
            in("al") arg,
        ) };
    } => ".hal.text";

    unsafe fn default_inw(port: u16) -> u16 {
        let mut val: u16;
        unsafe { asm!(
            "in ax, dx",
            in("dx") port,
            out("ax") val,
        ) };
        val
    } => ".hal.text";

    unsafe fn default_outw(port: u16, arg: u16) {
        unsafe { asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") arg,
        ) };
    } => ".hal.text";

    unsafe fn default_ind(port: u16) -> u32 {
        let mut val: u32;
        unsafe { asm!(
            "in eax, dx",
            in("dx") port,
            out("eax") val,
        ) };
        val
    } => ".hal.text";

    unsafe fn default_outd(port: u16, arg: u32) {
        unsafe { asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") arg,
        ) };
    } => ".hal.text";
}

// Define HAL vector table section
// Call template struct 'HalDispatches'
// TODO
hal_vt_instance! {
    pub static HAL_VT: HalDispatches = {
        inb: unsafe extern "C" fn(u16) -> u8 = default_inb,
        outb: unsafe extern "C" fn(u16, u8) = default_outb,
        inw: unsafe extern "C" fn(u16) -> u16 = default_inw,
        outw: unsafe extern "C" fn(u16, u16) = default_outw,
        ind: unsafe extern "C" fn(u16) -> u32 = default_ind,
        outd: unsafe extern "C" fn(u16, u32) = default_outd,
    }; => ".hal.vt";
}

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
    let _ = HAL_VT.dispatch(|x| &x.inw, |f| unsafe { f(0x3f8) });

    let mut a = 0u16;
    loop {
        unsafe {
            write_volatile(&mut a as *mut u16, a + 2);
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
