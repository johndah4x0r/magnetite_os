/*
    Platform-agnostic exposure of I/O operations

    This module defines the contract between
    platform-agnostic users and platform-specific
    providers.
*/

// Expect the `io` submodule
mod __arch_io {
    pub use crate::arch::__arch::io::*;
}

// Expose serial I/O
// - DO NOT forward `io::uart` as-is; define
// the contract at the module level
pub mod uart;
