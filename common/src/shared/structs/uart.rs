/*
    An include file defining the UART port wrapper type
*/

// Fat UART port
// - this is platform-agnostic, as port I/O
// is addressed either by port numbers or
// MMIO addresses
#[derive(Debug, Copy, Clone)]
pub struct UartPort {
    pub addr: usize,
    pub id: usize,
}

impl UartPort {
    // Create new instance of `UartPort`
    pub const fn new(addr: usize, id: usize) -> Self {
        UartPort {
            addr,
            id,
        }
    }
}