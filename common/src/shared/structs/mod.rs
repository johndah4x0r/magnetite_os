/*!
    Generic platform-agnostic structures

    The structures defined in the submodules are either fully
    or partially binary-compatible with the contained types
    (if they contain anything at all).
*/

use core::ops::{Index, IndexMut, Deref, DerefMut};

// Volatile wrapper type
pub mod volatile;

// Array-like fat pointer type
pub mod array_like;

// Spin lock wrapper type
pub mod spin_lock;

/**
    Textbook ring buffer with slice-like semantics

    This buffer **cannot** be used in `const` or `static`, as
    its contents cannot be initialized at compile-time.
*/
pub struct RingBuf<T: Default + Copy, const N: usize> {
    inner: Array<T, N>,
    head: usize,
}

impl<T: Default + Copy, const N: usize> RingBuf<T, N> {
    // - constant assertion (Rust 1.57+)
    const _ASSERT_NON_ZERO: () = assert!(N > 0);

    /// Create new instance of `RingBuf`
    pub fn new() -> Self {
        RingBuf {
            inner: Array::<T, N>::default(),
            head: 0,
        }
    }

    /// Rotate the buffer left by a specified amount
    /// (equivalent to shifting the buffer head to the right)
    pub fn rol(&mut self, n: usize) {
        self.head = (self.head + n) % N;
    }

    /// Rotate the buffer right by a specified amount
    /// (equivalent to shifting the buffer head to the left)
    pub fn ror(&mut self, n: usize) {
        // - calculate new head position using N's complement
        // (we trust that `N != 0`)
        let m = n.div_ceil(N);
        self.head = (self.head + m * N - n) % N;
    }
}

impl<T: Default + Copy, const N: usize> Index<usize> for RingBuf<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        let n = (self.head + index) % N;
        &self.inner[n]
    }
}

impl<T: Default + Copy, const N: usize> IndexMut<usize> for RingBuf<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let n = (self.head + index) % N;
        &mut self.inner[n]
    }
}

/// Wrapper type for an array of type `T`
// - at this point, we aren't coding
// in Rust - we are speaking legalese
#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct Array<T: Default + Copy, const N: usize>([T; N]);

impl<T: Default + Copy, const N: usize> Default for Array<T, N> {
    fn default() -> Self {
        Array([T::default(); N])
    }
}

impl<T: Default + Copy, const N: usize> Deref for Array<T, N> {
    type Target = [T; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Default + Copy, const N: usize> DerefMut for Array<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
