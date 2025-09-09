#![no_std]
#![no_main]

// Shared resources module
extern crate common;

// Definition uses
use common::shared::io::uart::*;
use common::shared::structs::ArrayLike;
use common::shared::io::{LockableDevice, Read, Write};
use core::panic::PanicInfo;
use core::arch::asm;
use core::hint::{black_box, spin_loop};

// - x86-specific structures
use common::arch::x86::structs::{BiosPB, LongE820};

// Set up UART ports
pub static UART_PORTS: [PollingUart; 4] = [
    PollingUart::new(RAW_UART_PORTS[0]),
    PollingUart::new(RAW_UART_PORTS[1]),
    PollingUart::new(RAW_UART_PORTS[2]),
    PollingUart::new(RAW_UART_PORTS[3]),
];

// Routine to initialize serial ports
#[inline(never)]
fn init_serial() {
    // Initialize all ports
    // - 'black_box' used to prevent loop unrolling
    for p in black_box(&UART_PORTS[..]) {
        // - catch result
        let _ = p.initialize(Some(BAUD_RATE / 3)).unwrap();
    }
}

// Routine to make a test transmission over COM1
#[inline(never)]
fn test_com1() {
    // Obtain reference to COM1 port
    let com1 = &UART_PORTS[0];

    {
        // Obtain R/W lock
        let mut handle = com1.lock().unwrap();

        // Write to serial output
        handle.write_str("Something rude!\n").unwrap();
    }
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
    unsafe { asm!("xchg bx, bx"); }
    init_serial();

    unsafe { asm!("xchg bx, bx"); }
    test_com1();

    loop {
        spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
