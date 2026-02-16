/*!
    Module defining wrapper types for volatile memory operations
*/

use core::cell::UnsafeCell;
use core::ptr;
use core::sync::atomic;
use core::sync::atomic::Ordering;

/**
    Memory layout-compatible volatile cell

    This type can be used to guarantee volatile access
    patterns to memory, though it cannot guarentee
    interior mutability and thread safety.

    As this type is intended primarily for use with memory
    mapped I/O, a constructor will not be exposed, and one
    has to create references to `VolatileCell<T>` instead:
    ```rust
    let mmio_ref: &VolatileCell<u8> = unsafe { &*(MMIO_ADDR as *const _) };

    let read_val: u8 = mmio_ref.load();
    mmio_ref.store(write_val);
    ```

    # Safety
    This type assumes interior mutability internally, so that writes
    can be performed behind otherwise-immutable representations like
    `*const VolatileCell<T>` and `&VolatileCell<T>`.

    One must therefore validate that `*const VolatileCell<T>` and
    `&VolatileCell<T>`
    - point to valid read-write memory, and
    - are properly aligned for `T`

    See [`pointer`] for more details on how to work safely with
    pointers and references.

    [`pointer`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html
*/
#[repr(transparent)]
pub struct VolatileCell<T>(UnsafeCell<T>);

impl<T: Copy> VolatileCell<T> {
    /// Performs volatile read
    #[inline(always)]
    pub fn load(&self) -> T {
        let val = unsafe {
            // SAFETY: This should be safe, as we own the
            // internal cell. Safety hinges on 'T'
            // implementing 'Copy' or 'Clone'

            // - obtain pointer from inner cell, then read
            ptr::read_volatile(self.0.get())
        };

        val
    }

    /// Performs volatile write
    #[inline(always)]
    pub fn store(&self, val: T) {
        unsafe {
            // SAFETY: This should be safe, as we own the internal cell.
            // Safety hinges on the cell being physically writeable
            // and 'T' implementing 'Copy' or 'Clone'

            // - obtain pointer from inner cell, then write
            ptr::write_volatile(self.0.get(), val);
        }
    }

    /// Get mutable reference to inner cell
    // - marked as unsafe, as compliance from
    // the compiler cannot be guaranteed
    #[inline(always)]
    pub const unsafe fn get_mut(&self) -> &mut T {
        // - since we're playing unsafe anyways, why
        // not perform pointer trickery
        unsafe { &mut *self.0.get() }
    }

    /**
        Adds an unsigned offset to this instance's
        pointer, then returns a reference

        The pointer is advanced by `count * size_of::<T>()` bytes.

        # Safety
        See [`pointer::add`] for more information.

        [`pointer::add`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.add
    */
    #[inline(always)]
    pub const unsafe fn add(&self, count: usize) -> &Self {
        let p = self as *const Self;
        unsafe { &*p.add(count) }
    }

    /**
        Adds a signed offset to this instance's
        pointer, then returns a reference

        The pointer is moved by `count * size_of::<T>()` bytes.

        # Safety
        See [`pointer::offset`] for more information.

        [`pointer::offset`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.offset
    */
    #[inline(always)]
    pub const unsafe fn offset(&self, count: isize) -> &Self {
        let p = self as *const Self;
        unsafe { &*p.offset(count) }
    }
}

// - YOLO!
unsafe impl<T> Sync for VolatileCell<T> {}
unsafe impl<T> Send for VolatileCell<T> {}

/**
    Memory layout-compatible volatile cell with memory fencing

    This type can be used to promote ordered access patterns
    to memory, in addition to guaranteeing their presense,
    though it cannot guarentee interior mutability and
    thread safety.

    The placement of memory fences are based on the assumption
    that the platform uses a total-store-order memory model. If
    a different memory model is assumed, then it is recommended
    to use [`VolatileCell<T>`] with custom fencing primitives.

    As this type is intended primarily for use with memory
    mapped I/O, a constructor will not be exposed, and one
    has to create references to `FencedVolatileCell<T>` instead:
    ```rust
    let mmio_ref: &FencedVolatileCell<u8> = unsafe { &*(MMIO_ADDR as *const _) };

    let read_val: u8 = mmio_ref.load();
    mmio_ref.store(write_val);
    ```

    # Safety
    This type assumes interior mutability internally, so that writes
    can be performed behind otherwise-immutable representations like
    `*const FencedVolatileCell<T>` and `&FencedVolatileCell<T>`.

    One must therefore validate that `*const FencedVolatileCell<T>` and
    `&FencedVolatileCell<T>`
    - point to valid read-write memory, and
    - are properly aligned for `T`

    See [`pointer`] for more details on how to work safely with
    pointers and references.

    [`pointer`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html
    [`VolatileCell<T>`]: VolatileCell
*/
#[repr(transparent)]
pub struct FencedVolatileCell<T>(UnsafeCell<T>);

impl<T: Copy> FencedVolatileCell<T> {
    /**
        Performs weakly fenced volatile read

        This operation uses [`fence()`] and [`Ordering::Acquire`] internally.

        [`fence()`]: core::sync::atomic::fence
        [`Ordering::Acquire`]: core::sync::atomic::Ordering::Acquire
    */
    #[inline(always)]
    pub fn load(&self) -> T {
        let val = unsafe {
            // SAFETY: This should be safe, as we own the
            // internal cell. Safety hinges on 'T'
            // implementing 'Copy' or 'Clone'

            // - obtain pointer from inner cell, then read
            ptr::read_volatile(self.0.get())
        };

        atomic::fence(Ordering::Acquire);
        val
    }

    /**
        Performs weakly fenced volatile write

        This operation uses [`fence()`] and [`Ordering::Release`] internally.

        [`fence()`]: core::sync::atomic::fence
        [`Ordering::Release`]: core::sync::atomic::Ordering::Release
    */
    #[inline(always)]
    pub fn store(&self, val: T) {
        atomic::fence(Ordering::Release);
        unsafe {
            // SAFETY: This should be safe, as we own the internal cell.
            // Safety hinges on the cell being physically writeable
            // and 'T' implementing 'Copy' or 'Clone'

            // - obtain pointer from inner cell, then write
            ptr::write_volatile(self.0.get(), val);
        }
    }

    /**
        Performs fenced volatile read

        This operation uses [`fence()`] and a [`Ordering::SeqCst`] sandwich internally.

        [`fence()`]: core::sync::atomic::fence
        [`Ordering::Acquire`]: core::sync::atomic::Ordering::Acquire
    */
    #[inline(always)]
    pub fn load_strong(&self) -> T {
        atomic::fence(Ordering::SeqCst);
        let val = unsafe {
            // SAFETY: This should be safe, as we own the
            // internal cell. Safety hinges on 'T'
            // implementing 'Copy' or 'Clone'

            // - obtain pointer from inner cell, then read
            ptr::read_volatile(self.0.get())
        };

        atomic::fence(Ordering::SeqCst);
        val
    }

    /**
        Performs fenced volatile write

        This operation uses [`fence()`] and an [`Ordering::SeqCst`] sandwich internally.

        [`fence()`]: core::sync::atomic::fence
        [`Ordering::Release`]: core::sync::atomic::Ordering::Release
    */
    #[inline(always)]
    pub fn store_strong(&self, val: T) {
        atomic::fence(Ordering::SeqCst);
        unsafe {
            // SAFETY: This should be safe, as we own the internal cell.
            // Safety hinges on the cell being physically writeable
            // and 'T' implementing 'Copy' or 'Clone'

            // - obtain pointer from inner cell, then write
            ptr::write_volatile(self.0.get(), val);
        }

        atomic::fence(Ordering::SeqCst);
    }

    /// Gets mutable reference to inner cell
    // - marked as unsafe, as compliance from
    // the compiler cannot be guaranteed
    #[inline(always)]
    pub const unsafe fn get_mut(&self) -> &mut T {
        // - since we're playing unsafe anyways, why
        // not perform pointer trickery
        unsafe { &mut *self.0.get() }
    }

    /**
        Adds an unsigned offset to this instance's
        pointer, then returns a reference

        The pointer is advanced by `count * size_of::<T>()` bytes.

        # Safety
        See [`pointer::add`] for more information.

        [`pointer::add`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.add
    */
    #[inline(always)]
    pub const unsafe fn add(&self, count: usize) -> &Self {
        let p = self as *const Self;
        unsafe { &*p.add(count) }
    }

    /**
        Adds a signed offset to this instance's
        pointer, then returns a reference

        The pointer is moved by `count * size_of::<T>()` bytes.

        # Safety
        See [`pointer::offset`] for more information.

        [`pointer::offset`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.offset
    */
    #[inline(always)]
    pub const unsafe fn offset(&self, count: isize) -> &Self {
        let p = self as *const Self;
        unsafe { &*p.offset(count) }
    }
}

// - YOLO!
unsafe impl<T> Sync for FencedVolatileCell<T> {}
unsafe impl<T> Send for FencedVolatileCell<T> {}
