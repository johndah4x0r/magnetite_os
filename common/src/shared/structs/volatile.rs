/*
    An include file defining the volatile cell wrapper type
*/

use core::cell::UnsafeCell;
use core::ptr;
use core::sync::atomic;
use core::sync::atomic::Ordering;

/* Memory-compatible volatile cell

    This type can be used to guarantee inner mutability
    and volatile access for in-situ statics (through
    the write-only handle), though it cannot fully guarantee
    thread safety or mutability for aliased memory.
*/
#[repr(C)]
pub struct VolatileCell<T>(UnsafeCell<T>);

impl<T: Copy> VolatileCell<T> {
    // Create new instance of `VolatileCell`
    pub const fn new(val: T) -> Self {
        VolatileCell(
            UnsafeCell::new(val)
        )
    }
    // Perform volatile read
    // This should be safe, as we own the
    // internal cell - safety hinges on 'T'
    // implementing 'Copy' or 'Clone'
    #[inline(always)]
    pub fn load(&self) -> T {
        atomic::fence(Ordering::Acquire);
        let val = unsafe {
            // - obtain pointer from inner cell, then read
            ptr::read_volatile(self.0.get())
        };

        atomic::fence(Ordering::Acquire);
        val
    }

    // Perform volatile write
    // This should be safe, as we own the
    // internal cell - safety hinges on
    // the cell being physically writable,
    // as well as 'T' implementing 'Copy'
    // or 'Clone'
    // - immutable 'self' for statics
    #[inline(always)]
    pub fn store(&self, val: T) {
        atomic::fence(Ordering::Release);
        unsafe {
            // - obtain pointer from inner cell, then write
            ptr::write_volatile(self.0.get(), val);
        }

        atomic::fence(Ordering::Release);
    }

    // Get mutable reference to inner cell
    // - marked as unsafe, as compliance from
    // the compiler cannot be guaranteed
    #[inline(always)]
    pub const unsafe fn get_mut(&self) -> &mut T {
        // - since we're playing unsafe anyways, why
        // not perform pointer trickery
        unsafe { &mut *self.0.get() }
    }
}
