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

    This lock should only be used *if and when* there are no
    other options, as it has the potential to be unfair, and
    the lock-acquiring process burns processor cycles.
*/
pub struct Mutex<T> {
    data: UnsafeCell<T>,
    locked: AtomicBool,
}

impl<T> Mutex<T> {
    /// Create new instance of `Mutex`
    pub const fn new(val: T) -> Self {
        Mutex {
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

    /**
        Obtains lock handle, waiting if necessary

        # Safety
        If a thread calls `lock` twice, then that thread **will deadlock**,
        and it will retain exclusive access to the protected resource
        **indefinitely** unless
        - the lock is forcibly released, or
        - the thread is properly terminated
    */
    pub fn lock(&self) -> MutexGuard<'_, T> {
        // Acquire lock by brute force
        while self.cas_weak().is_err() {
            spin_loop();
        }

        // Return lock handle
        MutexGuard {
            data_ptr: self.data.get(),
            locked_ref: &self.locked,
        }
    }

    /**
        Attempts to obtain lock handle, returning
        immediately if the lock is currently held
    */
    pub fn try_lock(&self) -> Result<MutexGuard<'_, T>, ()> {
        // Kindly acquire the lock
        if self.cas_strong().is_ok() {
            Ok(MutexGuard {
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
unsafe impl<T: Send> Sync for Mutex<T> {}

/**
    Transparent lock handle with automatic lock release

    The lock that issued the handle will be released as
    soon as the handle is dropped.
*/
pub struct MutexGuard<'a, T> {
    data_ptr: *mut T,
    locked_ref: &'a AtomicBool,
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We know where `self.data` points to...
        unsafe { &*self.data_ptr }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: We know where `self.data` points to...
        unsafe { &mut *self.data_ptr }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.locked_ref.store(false, Ordering::Release);
    }
}
