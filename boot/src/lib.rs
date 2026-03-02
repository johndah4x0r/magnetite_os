#![no_std]
#![no_main]

// Definition uses
use core::hint;
use core::panic::PanicInfo;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use core::sync::atomic::{AtomicUsize, Ordering};

#[macro_use]
extern crate alloc;
use alloc::vec::Vec;

// - internal definitions
extern crate common;
use common::shared::GenericError;
use common::shared::io::Write;
use common::shared::structs::array_like::ArrayLike;
use common::shared::structs::spin_lock::Mutex;
use common::shared::structs::volatile::VolatileCell;

mod allocator;
use allocator::BumpAllocator;

// - BIOS-specific structures
use common::plat::pc_bios::structs::{BiosPB, LongE820};
use common::plat::pc_bios::vga::console;
use common::plat::pc_bios::vesa::ScreenInfo;
use console::VgaConsole;

// Double-panic message
static MSG_DOUBLE_PANIC: &'static str =
    "(2/2) **bootloader panicked** (info corrupted or too risky to acquire)";

// Keep track of panic invocations to prevent re-entry
static PANIC_FLAG: AtomicUsize = AtomicUsize::new(0);

// Instatiate allocator
#[global_allocator]
#[unsafe(link_section = ".bss.allocator")]
static ALLOCATOR: BumpAllocator<LongE820> = BumpAllocator::new();

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
    screen_info: &'static ScreenInfo,
) -> ! {
    // Attempt to validate the E820 map descriptor
    // TODO: validate it *properly*
    if let Ok(e820_map) = e820_map_desc.try_into() {
        main(bios_pb, bootdev, e820_map, screen_info).unwrap();
    } else {
        panic!("received an invalid E820 map descriptor");
    }

    let mut handle = VGA_CONSOLE.lock();
    handle.unset_shadowed();

    writeln!(
        &mut handle,
        "\n W: We somehow escaped `main()`...\n\tIn future revisions, this will be considered an error."
    )
    .unwrap();

    // Halt the system
    freeze();
}

// Inner main routine
// - error types are non-exhaustive, but most of
//   them are of the type `io::Error`, which
//   implements `Into<GenericError>`
// - in general, the error types must implement
//   `Into<GenericError>`
fn main(
    _bios_bp: &BiosPB,
    bootdev: u64,
    e820_map: &'static [LongE820],
    screen_info: &'static ScreenInfo,
) -> Result<(), GenericError> {
    // Initialize allocator
    ALLOCATOR.init(e820_map, 0)?;

    // Obtain lock handle
    let mut handle = VGA_CONSOLE.lock();

    // Clear screen
    handle.clear()?;

    // Initialize text buffer
    let (cells_x, cells_y) = (screen_info.cells_x(), screen_info.cells_y());
    let num_cells = cells_x * cells_y;

    // - allocate a 125%-sized buffer
    let mut buf: Vec<u16> = vec![0; num_cells + num_cells / 4];
    let text_buf: &'static mut [u16] = buf.leak();

    // Initialize the console's geometry and shadow buffer
    unsafe {
        handle.set_dims(cells_x, cells_y);
    }
    handle.init(text_buf);

    // Write to screen
    writeln!(
        &mut handle,
        "*** Welcome to magnetite_os, revision 2026-03-02 ***\n"
    )?;

    // Print screen info
    writeln!(&mut handle, " --- (Screen information) --- ")?;
    writeln!(
        &mut handle,
        " >  Mode:\t\t\t\t {:0>3x}h",
        screen_info.mode()
    )?;
    writeln!(
        &mut handle,
        " >  Bytes per pixel:\t {}",
        screen_info.bytes_per_pixel()
    )?;
    writeln!(
        &mut handle,
        " >  Pixels:\t\t\t\t {} x {}",
        screen_info.width(),
        screen_info.height()
    )?;
    writeln!(&mut handle, " >  Pitch:\t\t\t\t {}", screen_info.pitch())?;
    writeln!(
        &mut handle,
        " >  Cells:\t\t\t\t {} x {}",
        screen_info.cells_x(),
        screen_info.cells_y()
    )?;

    // Print boot device number
    writeln!(
        &mut handle,
        "\nBoot device identifier: 0x{:0>2x}\n",
        bootdev
    )?;

    // Iterate over E820 map entries, then show them
    // - we trust that `e820_map` points to real entries
    writeln!(
        &mut handle,
        "I: E820 entries (base, size, type, ACPI attributes):"
    )?;

    for entry in e820_map {
        // Print debug representation of each entry
        writeln!(
            &mut handle,
            " >  0x{:0>16x}\t0x{:0>16x}\t0x{:0>8x}\t0x{:0>8x}",
            entry.base(),
            entry.size(),
            entry.area_type(),
            entry.acpi_attr()
        )?;
    }

    writeln!(
        &mut handle,
        "E820 map descriptor:\t{:?},\t{}\n",
        e820_map as *const _ as *const (),
        e820_map.len(),
    )?;

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
        if let Some(loc) = info.location() {
            writeln!(
                c,
                "(1/2) **bootloader panicked** ({})\n E: {}",
                loc,
                info.message()
            )
        } else {
            writeln!(
                c,
                "(1/2) **bootloader panicked** (source location unknown)\n E: {}",
                info.message()
            )
        }
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
                .write(b"\n(dev: possible error in formatting or console I/O)\n")
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
