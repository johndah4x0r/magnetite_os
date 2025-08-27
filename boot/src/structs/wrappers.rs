/*
    Wrapper structs that can be expected to be
    fully or partially binary-compatible with
    the wrapped types
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
    pub unsafe fn store(&self, val: T) {
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

/*
    Memory-compatible read-only HAL VT entry

    This type can be used to guarantee inner mutability
    and volatile access for in-situ statics (through
    the write-only handle), though it cannot fully guarantee
    thread safety or mutability for aliased memory.
*/
#[repr(C)]
pub struct HalVtableEntry<T>(UnsafeCell<T>);

impl<T: Copy> HalVtableEntry<T> {
    // Return new instance of 'HalVtableEntry'
    #[inline(always)]
    pub const fn new(val: T) -> Self {
        HalVtableEntry(UnsafeCell::new(val))
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

    // Provide access to inner cell
    // - attempting to modify the inner
    //   cell without acquiring a write
    //   lock is UB
    #[inline(always)]
    pub const fn get_cell(&self) -> *const UnsafeCell<T> {
        &self.0
    }
}

/*
    Memory-compatible read-write HAL VT entry

    This type can be used to guarantee inner mutability
    and volatile access for in-situ statics (through
    the write-only handle), though it cannot fully guarantee
    thread safety or mutability for aliased memory.

    Can only be obtained through `HalVtableAC`.
*/
#[repr(C)]
pub struct HalVtableEntryWriter<T>(UnsafeCell<T>);

impl<T: Copy> HalVtableEntryWriter<T> {
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
    pub unsafe fn store(&self, val: T) {
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
