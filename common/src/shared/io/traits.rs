/*
    Shared traits that define contracts between
    platform-agnostic users and platform-specific
    providers

    These traits define the contract for specific
    I/O operations, which implementors must comply
    with.
*/

/// Trait to mark type as readable
pub trait Read<'a> {
    /*
        Required implementations:
        - Read error type
        - Read function
    */
    type ReadError;
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::ReadError>;

    /* Given implementations */

    // TODO
}

/// Trait to mark type as writeable
pub trait Write<'a> {
    /*
        Required implementations:
        - Write error type
        - Write function
    */
    type WriteError;
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::WriteError>;

    /* Given implementations */

    // Write string literal to serial
    #[inline(always)]
    fn write_str(&mut self, literal: &str) -> Result<usize, Self::WriteError> {
        self.write(literal.as_bytes())
    }
}
