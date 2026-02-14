/*!
    Module defining FFI-compatible "fat pointers"
    to an array-like region in memory
*/

// Definition uses
use core::convert::TryFrom;
use core::marker::PhantomData;
use core::mem;

/**
    "Fat pointer" to an immutable array-like structure

    The datatype `ArrayLike` serves as an FFI-safe substitute for the primitive
    [`&[T]`](slice), as the ABI for `&[T]` is not stable, and is therefore not FFI-safe.

    # Example use
    A potential application for `ArrayLike` is bare-metal argument passing:
    ```rust
    #[inline(never)]
    #[unsafe(no_mangle)]
    pub extern "C" fn main(
        e820_map: &'static ArrayLike<'static, LongE820>,
    ) -> ! {
        // -- main routine -- //
    }
    ```
    Here, the caller of `main` initializes a structure identical to `ArrayLike` in
    layout (also known as a "descriptor"), then passes an FFI-safe pointer to the
    "descriptor" via the parameter `e820_map`.

    As `ArrayLike` has a known size at
    compile time, the reference `&ArrayLike<'_, T>` is structurally identical to
    the thin pointer `*const ArrayLike<'_, T>`.

    # Safety
    It is the user's responsibility to ensure that the resulting "pointer" points
    to a contiguous buffer with elements that correspond to `T`.
*/
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

/**
    "Fat pointer" to a mutable array-like structure

    The datatype `ArrayLikeMut` serves as an FFI-safe substitute for the primitive
    [`&mut [T]`](slice), as the ABI for `&mut [T]` is not stable, and is therefore not FFI-safe.

    # Example use
    A potential application for `ArrayLikeMut` is passing pointers to mutable buffers:
    ```rust
    #[inline(never)]
    #[unsafe(no_mangle)]
    pub extern "C" fn main(
        page_table: &'static ArrayLikeMut<'static, PageEntry>,
    ) -> ! {
        // -- main routine -- //
    }
    ```
    Here, the caller of `main` initializes a structure identical to `ArrayLikeMut` in
    layout (also known as a "descriptor"), then passes an FFI-safe pointer to the
    "descriptor" via the parameter `page_table`.

    As `ArrayLikeMut` has a known size at
    compile time, the reference `&ArrayLikeMut<'_, T>` is structurally identical to
    the thin pointer `*const ArrayLikeMut<'_, T>`.

    # Safety
    It is the user's responsibility to ensure that the resulting "pointer" points
    to a contiguous buffer with elements that correspond to `T`.
*/
#[repr(C)]
pub struct ArrayLikeMut<'a, T> {
    pub data: *mut T,
    pub size: usize,
    _marker: PhantomData<&'a mut T>,
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
