/*
    Data structures that can be expected to be
    shared throughout the boot process
*/

pub mod wrappers;
use wrappers::{HalVtableEntry, HalVtableEntryWriter};

use core::marker::Sync;
use core::mem;
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
    pub fn dispatch<S, F>(&self, selector: S) -> F
    where
        S: FnOnce(&V) -> &HalVtableEntry<F>,
        F: Copy,
    {
        selector(&self.vt).load()
    }

    // Modify VT entry using selector closure
    pub unsafe fn modify<S, F>(&self, selector: S, vector: F)
    where
        S: FnOnce(&V) -> &HalVtableEntry<F>,
        F: Copy,
    {
        // Perform transmutation
        // SAFETY: As both 'HalVtableEntry' and
        // 'HalVtableEntryWriter' contain only
        // one field, they should be binary-
        // compatible with one another.
        let v: &HalVtableEntryWriter<F> = unsafe { mem::transmute(selector(&self.vt)) };

        v.store(vector);
    }
}

unsafe impl<V: HalVectorTable> Sync for HalVtableAC<V> {}
