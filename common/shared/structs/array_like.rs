/*
    An include file defining the volatile cell wrapper type
*/

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