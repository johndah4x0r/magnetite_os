#![no_std]
#![no_main]

// Definition uses
use core::hint;
use core::panic::PanicInfo;
use core::slice::from_raw_parts;

// - internal definitions
extern crate common;
use common::shared::io::Write;
use common::shared::structs::array_like::ArrayLike;
use common::shared::structs::spin_lock::Mutex;
use common::shared::structs::volatile::VolatileCell;

// - BIOS-specific structures
use common::plat::pc_bios::structs::{BiosPB, LongE820};
use common::plat::pc_bios::vga::console;
use console::VgaConsole;

// Double-panic message
static MSG_DOUBLE_PANIC: &'static str =
    "(2/2) bootloader panicked: (info corrupted or too risky to acquire)\n";

// Keep track of panic invocations to prevent re-entry
static mut PANIC_FLAG: usize = 0;

// Instantiate VGA console with default values
static VGA_CONSOLE: Mutex<VgaConsole> = Mutex::new(unsafe { VgaConsole::defaults() });

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
    // Obtain lock handle
    let mut handle = VGA_CONSOLE.lock();

    // Clear screen
    handle.clear().unwrap();

    // Initialize the shadow buffer
    handle.init();

    // Write to screen
    writeln!(&mut handle, "Hello, world!").unwrap();
    writeln!(&mut handle, "This is a test!").unwrap();
    writeln!(&mut handle, "The quick brown fox jumps over the lazy dog").unwrap();

    // Commit changes
    handle.flush().unwrap();

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    // Increment panic flag, then process it
    // - the increment operation may panic
    match panic_fetch_add() {
        0 => single_panic(info),
        1 => double_panic(info),
        _ => triple_panic(info),
    };
}

// Routine for fetching the value of
// `PANIC_FLAG`, then incremenitng it
#[inline(always)]
fn panic_fetch_add() -> usize {
    unsafe {
        // - increment flag
        PANIC_FLAG += 1;

        // return pre-increment flag
        PANIC_FLAG - 1
    }
}

// Routine for first panic invocation
// TODO: decide whether to assume control over the console
#[inline(always)]
fn single_panic(info: &PanicInfo<'_>) -> ! {
    // 1. forcibly unlock the console
    unsafe {
        VGA_CONSOLE.unlock();
    }

    // 2. write to the console, first by arbitrage, then by force
    let f = |c: &mut VgaConsole| {
        c.unset_shadowed();
        writeln!(c, "(1/2) bootloader panicked: {:?}", info)
    };

    // - absorb errors, rather than unwrapping them
    // and knowingly triggering a panic
    let e = match VGA_CONSOLE.try_lock_repeat(255) {
        Ok(mut g) => f(&mut *g),
        Err(()) => {
            let c = unsafe { VGA_CONSOLE.get_mut() };
            f(c)
        }
    };

    // 3. if an I/O error is encountered,
    // report it (for debugging reasons)
    // - ignore generated error
    // FIXME: this might be dangerous, depending
    // on how `VgaConsole` is implemented
    if e.is_err() {
        let _ = unsafe {
            VGA_CONSOLE
                .get_mut()
                .write(b"(dev: possible error in formatting or console I/O)\n")
        };
    }

    // 4. freeze the system
    freeze();
}

// Routine for second panic invocation
// TODO: do something useful
// TODO: make this less MacGyver-like, now
// that `VgaConsole` is relatively stable
#[inline(always)]
fn double_panic(_info: &PanicInfo<'_>) -> ! {
    // 1. create a window into the default VGA text buffer
    // - don't try to be smart here
    let buf: &[VolatileCell<u16>] = unsafe {
        from_raw_parts(
            console::DEF_BUF_ADDR as *const _,
            console::DEF_NUM_COLS * console::DEF_NUM_ROWS,
        )
    };

    // 2. write the double-panic message
    // from the start of the text buffer
    let msg_b = MSG_DOUBLE_PANIC.as_bytes();
    for i in 0..msg_b.len() {
        let c = console::DEF_ATTR | (msg_b[i] as u16);
        buf[i].store(c)
    }

    // 3. freeze the system
    freeze();
}

// Routine for third panic invocation
// TODO: consider resetting the system
#[inline(always)]
fn triple_panic(_info: &PanicInfo<'_>) -> ! {
    freeze();
}

// Freeze the system
// TODO: use native instructions
#[inline(always)]
fn freeze() -> ! {
    loop {
        hint::spin_loop();
    }
}
