/*
    Shared traits that define contracts between
    platform-agnostic users and platform-specific
    providers

    These traits define the contract for specific
    I/O operations, which implementors must comply
    with.

    TODO: expand where appropriate
*/

// Trait to mark type as readable
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

// Trait to mark type as writeable
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

// Trait to mark type as a character device
pub trait CharDevice<'a> {
    type ReadError;
    type WriteError;
    fn char_read(&mut self, buf: &mut [u8]) -> Result<usize, Self::ReadError>;
    fn char_write(&mut self, buf: &[u8]) -> Result<usize, Self::WriteError>;
}

// - implement read operation for all character devices
impl<'a, T: CharDevice<'a>> Read<'a> for T {
    type ReadError = <Self as CharDevice<'a>>::ReadError;

    #[inline(always)]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::ReadError> {
        self.char_read(buf)
    }
}

// - implement write operation for all character devices
impl<'a, T: CharDevice<'a>> Write<'a> for T {
    type WriteError = <Self as CharDevice<'a>>::WriteError;

    #[inline(always)]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::WriteError> {
        self.char_write(buf)
    }
}

// Trait to mark device as lockable
pub trait LockableDevice<'a> {
    // Guard must be droppable
    type GuardType: Drop;
    type Error;

    fn lock(&'a self) -> Result<Self::GuardType, Self::Error>;
}
