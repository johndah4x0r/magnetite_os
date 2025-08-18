/*
    Data structures that can be expected to be
    shared throughout the boot process
*/

pub mod wrappers;

use core::convert::TryFrom;
use core::marker::PhantomData;
use core::mem;

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
