/*
    Data structures that can be expected to be
    shared throughout the boot process
*/

pub mod wrappers;
use wrappers::{HalVtableEntry, HalVtableEntryWriter};

use core::convert::TryFrom;
use core::marker::{PhantomData, Sync};
use core::mem;
use core::ops;
use core::sync::atomic::{AtomicIsize, Ordering};

// Fat "pointer" to an immutable
// array-like structure
// - MAY correspond 1:1 to &[T] in terms
//   of behavior - and if possible, layout
// - MUST be used to safely pass array-like
//   structures in FFI when convenience
//   is desired
#[repr(C)]
pub struct ArrayLike<'a, T> {
    pub data: *const T,
    pub size: usize,
    _marker: PhantomData<&'a T>,
}

// - conversion must be made explicit
impl<'a, T> TryFrom<&'a ArrayLike<'a, T>> for &'a [T] {
    type Error = ();

    fn try_from(value: &'a ArrayLike<'a, T>) -> Result<&'a [T], ()> {
        // Enforce pointer validity
        let align = mem::align_of::<T>();
        let offset = (value.data as usize) % align;

        if value.data.is_null() || offset != 0 {
            Err(())
        } else {
            // SAFETY:
            // - We are quite literally checking whether
            //   the data pointer is convertable
            // - We trust the instantiator to provide
            //   the structure with valid parameters
            //   (reasonable expectation in
            //   low-level contracts)
            // - For low-level applications, a lifetime
            //   of 'static is a reasonable expectation
            unsafe { Ok(core::slice::from_raw_parts(value.data, value.size)) }
        }
    }
}

// Fat "pointer" to a mutable
// array-like structure
// - MAY correspond 1:1 to &mut [T] in terms
//   of behavior - and if possible, layout
// - MUST be used to safely pass array-like
//   structures in FFI when convenience
//   is desired
#[repr(C)]
pub struct ArrayLikeMut<'a, T> {
    pub data: *mut T,
    pub size: usize,
    _marker: PhantomData<&'a T>,
}

// - conversion must be made explicit
impl<'a, T> TryFrom<&'a ArrayLikeMut<'a, T>> for &'a [T] {
    type Error = ();

    fn try_from(value: &'a ArrayLikeMut<'a, T>) -> Result<&'a [T], ()> {
        // Enforce pointer validity
        let align = mem::align_of::<T>();
        let offset = (value.data as usize) % align;

        if value.data.is_null() || offset != 0 {
            Err(())
        } else {
            // SAFETY:
            // - We are quite literally checking whether
            //   the data pointer is convertable
            // - We trust the instantiator to provide
            //   the structure with valid parameters
            //   (reasonable expectation in
            //   low-level contracts)
            // - For low-level applications, a lifetime
            //   of 'static is a reasonable expectation
            unsafe { Ok(core::slice::from_raw_parts(value.data, value.size)) }
        }
    }
}

// - conversion must be made explicit, and
//   exclusive mutability must be enforced
impl<'a, T> TryFrom<&'a mut ArrayLikeMut<'a, T>> for &'a mut [T] {
    type Error = ();

    fn try_from(value: &'a mut ArrayLikeMut<'a, T>) -> Result<&'a mut [T], ()> {
        // Enforce pointer validity
        let align = mem::align_of::<T>();
        let offset = (value.data as usize) % align;

        if value.data.is_null() || offset != 0 {
            Err(())
        } else {
            // SAFETY:
            // - We are quite literally checking whether
            //   the data pointer is convertable
            // - We trust the instantiator to provide
            //   the structure with valid parameters
            //   (reasonable expectation in
            //   low-level contracts)
            // - For low-level applications, a lifetime
            //   of 'static is a reasonable expectation
            unsafe { Ok(core::slice::from_raw_parts_mut(value.data, value.size)) }
        }
    }
}

// Short (20 B) E820 entry
// - expect little-endian encoding (x86-exclusive)
#[repr(C, packed)]
pub struct ShortE820 {
    pub base: u64,
    pub size: u64,
    pub area_type: u32,
}

// Long (24 B) E820 entry
// - expect little-endian encoding (x86-exclusive)
#[repr(C, packed)]
pub struct LongE820 {
    pub base: u64,
    pub size: u64,
    pub area_type: u32,
    pub acpi_attr: u32,
}

// BIOS parameter block structure
// - if it should ever become necessary
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

    // Dispatch routine using provided selector closure,
    // and execute it using provided action closure
    // FIXME: What even is this cursed contract?
    pub fn dispatch<S, A, F, R>(&self, selector: S, action: A) -> R
    where
        S: FnOnce(&V) -> &HalVtableEntry<F>,
        A: FnOnce(&F) -> R,
        F: Copy,
    {
        // Maybe-cells for later use
        let mut maybe_f: Option<F> = None;
        let mut maybe_r: Option<R> = None;

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
                // Step 2 - perform dispatch and break loop
                maybe_f = Some(selector(&self.vt).load());
                break;
            }

            core::hint::spin_loop();
        }

        // Step 3 - perform provided action
        if let Some(f) = maybe_f {
            maybe_r = Some(action(&f));
        }

        // Step 4 - release lock
        self.count.fetch_sub(1, Ordering::Release);

        // Step 5 - return
        maybe_r.unwrap()
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
                    let v_ptr = selector(&self.vt).get_cell() as *const HalVtableEntryWriter<F>;

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

// HAL VT entry lock (deprecated?)
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
