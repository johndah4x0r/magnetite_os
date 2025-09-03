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
