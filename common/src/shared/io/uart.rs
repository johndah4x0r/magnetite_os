/*
    Platform-agnostic exposure of serial I/O

    This module defines the contract between
    platform-agnostic users and platform-specific
    providers.
*/

use crate::shared::traits::{CharDevice, LockableDevice};
use core::marker::Sync;

/*
    Expect the `io::uart` submodule from the relevant
    platform submodule, and then expect the wrapper type
    for the polling driver, its read-write guard, and
    initialization error codes

    This effectively defines what the UART driver module
    must expose to `common::shared`. The expectation, is
    that the driver module deals with platform-specific
    behavior internally, then exposes platform-agnostic
    definities to `common::shared`, which disseminates
    them in a controlled manner.

    Based on `arch::x86::io::uart`.

    TODO: generalize x86-specific behavior
*/
use super::__arch_io::uart::PollingUart as __PollingUart;
pub use super::__arch_io::uart::{BAUD_RATE, InitError, PollingUartGuard, RAW_UART_PORTS};

// Wrapper type for `__arch_io::uart::PollingUart`
// - used to enforce function signatures
#[repr(transparent)]
pub struct PollingUart<'a>(__PollingUart<'a>);

impl<'a> PollingUart<'a> {
    // Create uninitialized instance of `PollingUart`
    pub const fn new(port: UartPort) -> Self {
        PollingUart(__PollingUart::new(port))
    }

    // Query if the port is initialized
    pub fn is_initialized(&'a self) -> bool {
        self.0.is_initialized()
    }

    // Initialize port
    pub fn initialize(&'a self, rate: Option<usize>) -> Result<(), InitError> {
        self.0.initialize(rate)
    }
}

unsafe impl Sync for PollingUart<'_> {}
impl !CharDevice<'_> for PollingUart<'_> {}
impl<'a> LockableDevice<'a> for PollingUart<'a> {
    type GuardType = PollingUartGuard<'a>;
    type Error = InitError;

    // Return a temporary read-write guard
    fn lock(&'a self) -> Result<Self::GuardType, Self::Error> {
        self.0.lock()
    }
}

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
        UartPort { addr, id }
    }
}
