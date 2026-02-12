/*!
    Module defining spin-lock wrapper types
*/

// Definition uses
use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::marker::{Send, Sync};
use core::ops::{Deref, DerefMut, Drop};
use core::sync::atomic::{AtomicBool, Ordering};

/**
    Simple spin lock with a single lock holder

    This lock type implicitly admits interior mutability,
    which is safe to expose due to mutual exclusion.

    It is also FFI-compatible, provided that the enclosed
    type `T` is also FFI-compatible. If used across FFI,
    it is the responsibility of the outside user to guarantee
    atomicity and discipline access to the protected data.

    This lock should only be used *if and when* there are no
    other options, as it has the potential to be unfair, and
    the lock-acquiring process burns processor cycles.
*/
#[repr(C)]
pub struct SpinLock<T> {
    data: UnsafeCell<T>,
    locked: AtomicBool,
}

impl<T> SpinLock<T> {
    /// Create new instance of `SpinLock`
    pub const fn new(val: T) -> Self {
        SpinLock {
            data: UnsafeCell::new(val),
            locked: AtomicBool::new(false),
        }
    }

    // Internal: acquire lock strongly
    // - it's a short-hand, so that I won't have
    // to repeatedly type out 80-something characters
    #[inline(always)]
    fn cas_strong(&self) -> Result<bool, bool> {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
    }

    // Internal: acquire lock weakly
    // - it's a short-hand, so that I won't have
    // to repeatedly type out 80-something characters
    #[inline(always)]
    fn cas_weak(&self) -> Result<bool, bool> {
        self.locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
    }

    /// Obtain lock handle, wait if necessary
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        // Acquire lock by brute force
        while self.cas_weak().is_err() {
            spin_loop();
        }

        // Return lock handle
        SpinLockGuard {
            data_ptr: self.data.get(),
            locked_ref: &self.locked,
        }
    }

    /// Attempt to obtain lock handle, returning
    /// immediately if the lock is currently held
    pub fn try_lock(&self) -> Result<SpinLockGuard<'_, T>, ()> {
        // Kindly acquire the lock
        if self.cas_strong().is_ok() {
            Ok(SpinLockGuard {
                data_ptr: self.data.get(),
                locked_ref: &self.locked,
            })
        } else {
            Err(())
        }
    }

    /// Forcibly release the lock
    pub unsafe fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

// - allow reference sharing
unsafe impl<T: Send> Sync for SpinLock<T> {}

/**
    Transparent lock handle with automatic lock release

    The lock that issued the handle will be released as
    soon as the handle is dropped.
*/
pub struct SpinLockGuard<'a, T> {
    data_ptr: *mut T,
    locked_ref: &'a AtomicBool,
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We know where `self.data` points to...
        unsafe { &*self.data_ptr }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: We know where `self.data` points to...
        unsafe { &mut *self.data_ptr }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.locked_ref.store(false, Ordering::Release);
    }
}
