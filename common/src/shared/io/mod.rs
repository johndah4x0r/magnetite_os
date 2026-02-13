/*!
    Platform-agnostic exposure of I/O operations

    This module defines the contract between
    platform-agnostic users and platform-specific
    providers.

    This module aspires to be a minimal substitute for `std::io`;
    it attempts to offer identical usage and functionality wherever
    it is feasible and appropriate.
*/

/*
    Shared traits that define contracts between
    platform-agnostic users and platform-specific
    providers

    These traits define the contract for specific
    I/O operations, which implementors must comply
    with.

    As per decision 2025-08-30, part 2, platform-specific
    definitions must be separated from shared defintions,
    and must be guarded off whenever possible.

    TODO: finish doc-comments and reword, so that we aren't using
    the exact same wording as the Rust standard library (though
    this is mostly a legal detail, and not a functional one)

    TODO: fix `ReadError` and `ẀriteError` (which are vestigial) into
    something similar to `std::io::Error`, as we shouldn't let every
    R/W type define their own error types.
*/

/**
    Trait to mark type as readable

    Implementors of `Read` are called "readers". Readers are defined by
    one required method [`read`](Read::read)
*/
pub trait Read {
    /*
        Required implementations:
        - Read error type
        - Read function
    */
    /// A fixed read error type for this particular reader
    type ReadError;

    /**
        Pull some bytes from this source into the specified
        buffer, returning how many bytes were read.

        This function does not provide any guarantees about whether it
        blocks waiting for data, but if an object needs to block for a
        ead and cannot, it will typically signal this via an `Err`
        return value.

        If the return value of this method is `Ok(n)`, then implementations
        must guarantee that `0 <= n <= buf.len()`. A non-zero value for `n`
        indicates that the buffer `buf` has been filled in with `n` bytes of
        data from this source. If `n` is 0, then it can indicate one of two
        scenarios:

        - This reader has reached its “end of file” and will likely no longer
        be able to produce bytes. Note that this does not mean that the reader
        will always no longer be able to produce bytes.
        - The buffer specified was 0 bytes in length.

        # Errors
        If this function encounters any form of I/O or other error, an error of
        the type `ReadError` will be returned. If an error is returned then it
        must be guaranteed that no bytes were read.

        It is **not** an error if the returned value is smaller than the buffer size,
        even when the reader is not at the end of the stream yet. This may happen for
        example because fewer bytes are actually available right now (e. g. being close
        to end-of-file) or because `read()` was interrupted.

        # Safety
        As this trait is safe to implement, callers in unsafe code cannot rely on
        `n <= buf.len()` for safety. Extra care needs to be taken when unsafe functions
        are used to access the read bytes. Callers have to ensure that no unchecked
        out-of-bounds accesses are possible even if `n > buf.len()`.
    */
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::ReadError>;

    /* Given implementations */

    /**
        Reads the exact number of bytes required to fill `buf`.

        This function reads as many bytes as necessary to
        completely fill the specified buffer `buf`.

        # Errors
        If this function returns an error, it is unspecified how many
        bytes it has read, but it will never read more than would be
        necessary to completely fill the buffer.
    */
    // FIXME: find a mathematically better and safer alternative
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Self::ReadError> {
        // Get buffer length
        let l: usize = buf.len();
        let mut rem = l;

        // Read `rem` bytes, then subtract how many bytes were successfuly read
        // - we'll assume that the value returned by `read` never exceeds `rem`,
        // and even if it does, it'll simply break the loop
        while rem > 0 {
            // Obtain number of bytes actually read
            // - propagate error syntactically
            let n = self.read(&mut buf[l - rem..l])?;

            // Perform zero-clamped subtraction
            rem -= n.min(rem);
        }

        Ok(())
    }

    // TODO
}

/// Trait to mark type as writeable
pub trait Write {
    /*
        Required implementations:
        - Write error type
        - Write function
    */
    type WriteError;

    /**
        Writes the provided buffer into this writer,
        returning how many bytes were written.

        This function will attempt to write the entire contents of `buf`,
        but the entire write might not succeed, or the write may also
        generate an error. Typically, a call to `write` represents one
        attempt to write to any wrapped object.

        # Errors
        Each call to `write` may generate an I/O error indicating that the
        operation could not be completed. If an error is returned then no
        bytes in the buffer were written to this writer.

        It is **not** considered an error if the entire buffer could not
        be written to this writer.

    */
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::WriteError>;

    /**
        Flushes this output stream, ensuring that all intermediately
        buffered contents reach their destination.

        # Errors
        It is considered an error if not all bytes could be written
        due to I/O errors or EOF being reached.
    */
    fn flush(&mut self) -> Result<(), Self::WriteError>;

    /* Given implementations */

    // Write string literal into the writer
    #[inline(always)]
    fn write_str(&mut self, literal: &str) -> Result<usize, Self::WriteError> {
        self.write(literal.as_bytes())
    }
}
