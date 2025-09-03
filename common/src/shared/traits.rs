/*
    Shared traits that define contracts between
    platform-agnostic users and platform-specific
    providers

    TODO: expand where appropriate
*/

// Trait to mark type as readable
pub trait Read {
    type ReadError;
    fn read(&self, buf: &mut [u8]) -> Result<usize, Self::ReadError>;
}

// Trait to mark type as writeable
pub trait Write {
    type WriteError;
    fn write(&self, buf: &[u8]) -> Result<usize, Self::WriteError>;
}

// Trait to mark type as a character device
pub trait CharDevice<'a>: Read + Write {
    fn char_read(&self, buf: &mut [u8]) -> Result<usize, Self::ReadError> {
        self.read(buf)
    }

    fn char_write(&self, buf: &[u8]) -> Result<usize, Self::WriteError> {
        self.write(buf)
    }
}

// Trait to mark device as lockable
pub trait LockableDevice<'a> {
    // Guard must be droppable
    type GuardType: Drop;
    type Error;

    fn lock(&'a self) -> Result<Self::GuardType, Self::Error>;
}
