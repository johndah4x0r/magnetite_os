#![no_std]
#![no_main]

// Definition uses
use core::hint;
use core::panic::PanicInfo;
use core::slice::from_raw_parts;
use core::sync::atomic::{AtomicUsize, Ordering};

// - internal definitions
extern crate common;
use common::shared::GenericError;
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
static PANIC_FLAG: AtomicUsize = AtomicUsize::new(0);

// Instantiate VGA console with default values
static VGA_CONSOLE: Mutex<VgaConsole> = Mutex::new(unsafe { VgaConsole::defaults() });

// Initial routine
//  - call it '_start' for the sake of brevity
// TODO
#[inline(never)]
#[unsafe(no_mangle)]
pub extern "C" fn _start(
    bios_pb: &'static BiosPB,
    bootdev: u64,
    e820_map_desc: &'static ArrayLike<'static, LongE820>,
) -> ! {
    // Attempt to validate the E820 map descriptor
    // TODO: validate it *properly*
    if let Ok(e820_map) = e820_map_desc.try_into() {
        main(bios_pb, bootdev, e820_map).unwrap();
    } else {
        panic!("received an invalid E820 map descriptor");
    }

    // Halt the system
    freeze();
}

// Inner main routine
// - error types are non-exhaustive, but most of
//   them are of the type `io::Error`, which
//   implements `Into<GenericError>`
// - in general, the error types must implement
//   `Into<GenericError>`
fn main(_bios_bp: &BiosPB, _bootdev: u64, _e820_map: &[LongE820]) -> Result<(), GenericError> {
    // Obtain lock handle
    let mut handle = VGA_CONSOLE.lock();

    // Clear screen
    handle.clear()?;

    // Initialize the shadow buffer
    handle.init();

    // Write to screen
    writeln!(&mut handle, "Hello, world!")?;
    writeln!(&mut handle, "This is a test!")?;
    writeln!(&mut handle, "The quick brown fox jumps over the lazy dog")?;

    // Commit changes
    handle.flush()?;

    Ok(())
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
    PANIC_FLAG.fetch_add(1, Ordering::SeqCst)
}

// Routine for first panic invocation
// TODO: decide whether to assume control over the console
#[inline(always)]
fn single_panic(info: &PanicInfo<'_>) -> ! {
    // 1. forcibly unlock the console, if necessary
    unsafe {
        VGA_CONSOLE.unlock();
    }

    // 2. write to the console, first by arbitration, then by force
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

    // - be paranoid, and truncate the message
    let n = msg_b.len().min(buf.len());

    for i in 0..n {
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
