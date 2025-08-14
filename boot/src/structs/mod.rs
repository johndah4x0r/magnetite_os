/*
    Data structures that can be expected to be
    shared throughout the boot process
*/

pub mod wrappers;
use wrappers::{HalVtableEntry, HalVtableEntryWriter};

use core::marker::Sync;
use core::ops;
use core::sync::atomic::{AtomicIsize, Ordering};

// BIOS parameter block structure
// - if it shoud ever become necessary
#[repr(C, packed)]
pub struct BiosPB {
    oem_label_raw: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fat_count: u8,
    root_dir_entries: u16,
    sectors: u16,
    medium_type: u8,
    sectors_per_fat: u16,
    heads: u8,
    hidden_sectors: u32,
    large_sectors: u32,
    drive_number: u16,
    signature: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    filesystem: [u8; 8],
}

// HAL vector table marker trait
// - not the authoritative table itself
pub trait HalVectorTable {}

// Access-controlled vector table wrapper
// - the authoritative table
// - implement some form of reference
//   counting favoring readers
// - binary-compatibility is somewhat
//   respected
// - should be instantiated using macros
// - client code *must* have the same
//   vector table template as the host
//   for compile-time resolution to work
//   at all
#[repr(C)]
pub struct HalVtableAC<V: HalVectorTable> {
    count: AtomicIsize,
    vt: V,
}

impl<V: HalVectorTable> HalVtableAC<V> {
    // Create new instance of 'HalVtableAC'
    pub const fn new(count: AtomicIsize, vt: V) -> Self {
        HalVtableAC { count, vt }
    }

    // Dispatch routine using selector closure
    // - return a reader guard
    pub fn dispatch<S, F>(&self, selector: S) -> HalVtableEntryLock<'_, F>
    where
        S: FnOnce(&V) -> &HalVtableEntry<F>,
        F: Copy,
    {
        // Step 1 - acquire lock
        loop {
            // Read current lock state
            let current = self.count.load(Ordering::Acquire);

            // Step 1a - wait until the write lock
            // is released
            if current < 0 {
                core::hint::spin_loop();
                continue;
            }

            // Step 1b - attempt to increment read count
            if self
                .count
                .compare_exchange(current, current + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                // Step 2 - perform dispatch
                let e = HalVtableEntryLock {
                    count: &self.count,
                    entry: selector(&self.vt),
                };

                return e;
            }

            core::hint::spin_loop();
        }
    }

    // Modify VT entry using selector closure
    // and provided vector
    // - we'll trust that the entries in
    //   question are internally mutable
    pub fn modify<S, F>(&self, selector: S, vector: F)
    where
        S: FnOnce(&V) -> &HalVtableEntry<F>,
        F: Copy,
    {
        // Step 1 - acquire lock
        loop {
            // - attempt to increment read count
            if self
                .count
                .compare_exchange(0, -1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                // Step 2 - perform modification
                // SAFETY: As both 'HalVtableEntry' and
                // 'HalVtableEntryWriter' contain only
                // one field, they should be binary-
                // compatible with one another.
                unsafe {
                    let v_ptr =
                        selector(&self.vt).get_cell() as *const _ as *const HalVtableEntryWriter<F>;

                    (&*v_ptr).store(vector);
                }

                break;
            }

            core::hint::spin_loop();
        }

        // Step 3 - release lock
        self.count.store(0, Ordering::Release);
    }
}

unsafe impl<V: HalVectorTable> Sync for HalVtableAC<V> {}

pub struct HalVtableEntryLock<'a, F: Copy> {
    count: &'a AtomicIsize,
    entry: &'a HalVtableEntry<F>,
}

// Implement transparent dereferencing
impl<F: Copy> ops::Deref for HalVtableEntryLock<'_, F> {
    type Target = HalVtableEntry<F>;

    fn deref(&self) -> &Self::Target {
        self.entry
    }
}

// Implement lock reduction on destruction
// - why do we need mutability again?
impl<F: Copy> ops::Drop for HalVtableEntryLock<'_, F> {
    fn drop(&mut self) {
        // Decrease reader count
        self.count.fetch_sub(1, Ordering::Release);
    }
}
