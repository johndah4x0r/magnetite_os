/*!
    Generic platform-agnostic structures

    The structures defined in the submodules are either fully
    or partially binary-compatible with the contained types
    (if they contain anything at all).
*/

use core::ops::{Index, IndexMut};

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

// - use linear buffer internally
pub struct RingBuf<'a, T: Default + Copy> {
    inner: &'a mut [T],
    cols: usize,
    rows: usize,
    head: usize,
}

impl<'a, T: Default + Copy> RingBuf<'a, T> {
    /**
        Create new instance of `RingBuf`

        # Usage
        Both `rows` and `cols` must be non-zero. The provided
        buffer `buf` must be able to accomodate at least
        `rows * cols` elements of type `T`.
    */
    pub fn new(buf: &'a mut [T], cols: usize, rows: usize) -> Option<Self> {
        // - calculate capcity
        let capacity = rows * cols;

        // - this should cop out if either
        // `rows` or `cols` is equal to zero
        if buf.len() < capacity || rows == 0 || cols == 0 {
            return None;
        }

        // - obtain buffer as large as the calculated capacity
        let inner = &mut buf[0..capacity];

        // - initialize contents
        for e in inner {
            *e = T::default();
        }

        Some(RingBuf {
            inner: &mut buf[0..capacity],
            rows,
            cols,
            head: 0,
        })
    }

    /// Rotate the buffer left by a specified amount
    /// (equivalent to shifting the buffer head to the right)
    pub fn rol(&mut self, n: usize) {
        self.head = (self.head + n) % self.rows;
    }

    /// Rotate the buffer right by a specified amount
    /// (equivalent to shifting the buffer head to the left)
    pub fn ror(&mut self, n: usize) {
        // - calculate new head position using N's complement
        // (we trust that `N != 0`)
        let m = n.div_ceil(self.rows);
        self.head = (self.head + m * self.rows - n) % self.rows;
    }
}

impl<'a, T: Default + Copy> Index<usize> for RingBuf<'a, T> {
    type Output = [T];

    fn index(&self, index: usize) -> &Self::Output {
        let n = (self.head + index) % self.rows;
        &self.inner[n * self.cols..(n + 1) * self.cols]
    }
}

impl<'a, T: Default + Copy> IndexMut<usize> for RingBuf<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let n = (self.head + index) % self.rows;
        &mut self.inner[n * self.cols..(n + 1) * self.cols]
    }
}
