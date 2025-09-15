/*
    Console driver module for x86

    This module only supports VGA text output and primitive
    text input, as the module assumes polling I/O instead
    of interrupt-based I/O.

    TODO: implement detailed error types
*/

use crate::shared::io::{CharDevice, LockableDevice};
use crate::shared::structs::VolatileCell;

use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::ops::Drop;
use core::slice::from_raw_parts_mut;
use core::sync::atomic::{AtomicBool, Ordering};
use core::marker::{Sync, PhantomData};

// Default text buffer address
pub const DEF_ADDR_TXT: usize = 0xb8000;

// Default dimensions
pub const DEF_NUM_COLS: usize = 80;
pub const DEF_NUM_ROWS: usize = 25;

// Abstract console output instance
// - does not expose direct memory access
// - does not guarantee cross-platform
// compatibility, apart from the use of
// platform-agnostic contracts
// - embeds a 128-byte return buffer
pub struct ConsoleOutput<'a> {
    addr: usize,
    max_x: usize,
    max_y: usize,
    cur_x: UnsafeCell<usize>,
    cur_y: UnsafeCell<usize>,
    ret_buf: UnsafeCell<[u8; 128]>,
    _lock: AtomicBool,
    _marker: PhantomData<&'a AtomicBool>,
}

impl ConsoleOutput<'_> {
    // Create console output instance
    pub const fn new(addr: usize, max_cols: usize, max_rows: usize) -> Self {
        ConsoleOutput {
            addr,
            max_x: max_cols - 1,
            max_y: max_rows - 1,
            cur_x: UnsafeCell::new(0),
            cur_y: UnsafeCell::new(0),
            ret_buf: UnsafeCell::new([0; 128]),
            _lock: AtomicBool::new(false),
            _marker: PhantomData,
        }
    }
}

impl<'a> LockableDevice<'a> for ConsoleOutput<'a> {
    type GuardType = ConsoleOutputGuard<'a>;
    type Error = ();

    // Locks console output and returns guard type
    // with I/O traits
    // - this operation should never fail, so we should
    // be able to return an `Ok(...)` - though rigorous
    // checks might be implemented in the future
    fn lock(&'a self) -> Result<Self::GuardType, Self::Error> {
        // Instantiate buffer pointer
        let buf_ptr = self.addr as *mut VolatileCell<u16>;

        // Calculate buffer size
        let buf_size = (self.max_x + 1) * (self.max_y + 1);

        // Instantiate return buffer reference
        let ret_buf_ref = unsafe { &mut *self.ret_buf.get() };

        Ok(ConsoleOutputGuard {
            buf: unsafe { from_raw_parts_mut(buf_ptr, buf_size) },
            max_x: self.max_x,
            max_y: self.max_y,
            cur_x: unsafe { &mut *self.cur_x.get() },
            cur_y: unsafe { &mut *self.cur_y.get() },
            ret_buf: &mut ret_buf_ref[..],
            _lock: &self._lock,
        })
    }
}

unsafe impl Sync for ConsoleOutput<'_> {}
impl !CharDevice<'_> for ConsoleOutput<'_> {}

// Guard type for `ConsoleOutput`
// - the only type allowed to implement
// read and write access, as it is owned
// entirely by the instantiating scope
// - internally uses a linear buffer
pub struct ConsoleOutputGuard<'a> {
    buf: &'a mut [VolatileCell<u16>],
    max_x: usize,
    max_y: usize,
    cur_x: &'a mut usize,
    cur_y: &'a mut usize,
    ret_buf: &'a mut [u8],
    _lock: &'a AtomicBool,
}

impl<'a> CharDevice<'a> for ConsoleOutputGuard<'a> {
    type ReadError = ();
    type WriteError = ();

    // Read bytes from the return buffer
    // - just copy from the return buffer
    // to the provided buffer
    fn char_read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        // Calculate 
    }
}

impl Drop for ConsoleOutputGuard<'_> {
    fn drop(&mut self) {
        self._lock.store(false, Ordering::Release);
    }
}