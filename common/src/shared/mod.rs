/*!
    Platform-agnostic definitions

    The definitions here are shared by bootloader and
    kernel code, and should be platform-agnostic by design.

    As per decision 2025-08-30, part 2, platform-specific
    definitions must be separated from shared defintions,
    and must be guarded off whenever possible.
*/

use core::include;

// Platform-agnostic macros (include it)
include!("macros.rs");

// Generic structures
pub mod structs;

// Contract traits
pub mod traits;

// Sanitized I/O definitions
pub mod io;

/**
    A finite set of error types

    This type is functionally equivalent to [`io::ErrorPayload`], except
    that it can be used outside of I/O contexts. The variants that this
    type offers reflect the most common error types found in early-stage
    `no_std` environments.
*/
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum GenericError {
    /// Error code
    ErrorCode(usize),

    /// Error message
    ErrorMessage(&'static str),

    /// Other error payload
    Other,

    /// No payload
    Empty,
}
