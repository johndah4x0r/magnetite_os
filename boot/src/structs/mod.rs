/*
    Data structures that can be expected to be
    shared throughout the boot process
*/

pub mod wrappers;
use core::ptr::Thin;
use core::ptr::write_volatile;
use core::sync::atomic::AtomicUsize;
use wrappers::HalVtableEntry;

use core::arch::asm;
use core::convert::TryFrom;
use core::marker::{PhantomData, Sync};
use core::mem;
use core::ptr::read_volatile;
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
    base: AtomicUsize,
    count: AtomicIsize,
    vt: V,
}

impl<V: HalVectorTable> HalVtableAC<V> {
    // Create new instance of 'HalVtableAC'
    pub const fn new(count: AtomicIsize, vt: V) -> Self {
        HalVtableAC {
            base: AtomicUsize::new(0),
            count,
            vt,
        }
    }

    // Set internal base
    #[inline(never)]
    pub fn initialize(&self) -> Result<usize, usize> {
        let base = &self.base as *const _ as usize;
        // Output breakpoint and result
        unsafe {
            asm!(
                "xchg bx, bx",
                "xchg r8, r8",
                in("r8") base,
            );
        }

        // Set the base, but only if
        // it is currently unset
        let old_base = self.base.load(Ordering::Acquire);
        if old_base == 0 {
            self.base.store(base, Ordering::Release);
            Ok(base)
        } else {
            // Return old base
            Err(old_base)
        }
    }

    // Translate provided pointer-like object,
    // so that relative offsets match
    #[inline(never)]
    fn translate<P: Thin + Copy>(&self, old_ptr: P) -> Result<P, ()> {
        // Obtain old and new base
        let old_base = self.base.load(Ordering::Acquire);
        let new_base = &self.base as *const _ as usize;

        // Do not perform translation if the bases match
        if new_base == old_base {
            return Ok(old_ptr);
        }

        // Nor should we perform translation
        // if the old base is unset
        if old_base == 0 {
            return Err(());
        }

        // Perform pointer-level aliasing
        // SAFETY: As `P` is supposedly thin, it
        // should be relatively safe to transmute
        // it to `*const ()`
        let p = &old_ptr as *const _ as *const *const ();

        // Output breakpoint and result
        unsafe {
            asm!(
                "xchg bx, bx",
                "xchg r8, r8",
                in("r8") p as usize,
            );
        }

        // Obtain linear address
        // SAFETY: (see above)
        let old_addr = unsafe { read_volatile(p) as usize };

        // Output breakpoint and result
        unsafe {
            asm!(
                "xchg bx, bx",
                "xchg r9, r9",
                in("r9") old_addr,
            );
        }

        // Calculate absolute offset
        let abs_offset: usize = old_addr.abs_diff(old_base);

        // - do not proceed if we can't even
        // perform signed arithmetic
        if abs_offset > (isize::MAX as usize) {
            return Err(());
        }

        // Calculate signed offset
        // - start by assuming a positive offset
        let mut offset = abs_offset as isize;

        // - determine offset sign
        if old_addr < old_base {
            offset *= -1;
        }

        // Calculate new pointer
        if let Some(new_addr) = new_base.checked_add_signed(offset) {
            // - perform pointer-level aliasing
            // SAFETY: As `P` is supposedly thin,
            // it should be relatively safe to
            // derive it from `*const ()`
            let p = &new_addr as *const _ as *const P;

            // - obtain new pointer
            // SAFETY: (see above)
            Ok(unsafe { read_volatile(p) })
        } else {
            Err(())
        }
    }

    // Dispatch routine using provided selector closure,
    // and execute it using provided action closure
    // FIXME: What even is this cursed contract?
    #[inline(never)]
    pub fn dispatch<S, A, F, R>(&self, selector: S, action: A) -> Result<R, ()>
    where
        S: FnOnce(&V) -> &HalVtableEntry<F>,
        A: FnOnce(F) -> R,
        F: Thin + Copy,
    {
        // Maybe-cells for later use
        let maybe_f: Option<F>;
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
            let new_f = self.translate(f)?;
            maybe_r = Some(action(new_f));
        }

        // Step 4 - release lock
        self.count.fetch_sub(1, Ordering::Release);

        // Step 5 - return
        if let Some(r) = maybe_r {
            Ok(r)
        } else {
            Err(())
        }
    }

    // Modify VT entry using selector closure
    // and provided vector
    // - we'll trust that the entries in
    //   question are internally mutable
    pub fn modify<S, F>(&self, selector: S, vector: F) -> Result<(), ()>
    where
        S: FnOnce(&V) -> &HalVtableEntry<F>,
        F: Thin + Copy,
    {
        // Step 1 - acquire lock
        loop {
            // - attempt to acqure write lock
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
                    let v_ptr = self.translate(selector(&self.vt).get())?;
                    write_volatile(v_ptr, vector);
                }

                break;
            }

            core::hint::spin_loop();
        }

        // Step 3 - release lock
        self.count.store(0, Ordering::Release);

        Ok(())
    }
}

unsafe impl<V: HalVectorTable> Sync for HalVtableAC<V> {}
