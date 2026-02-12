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

// I/O operations
pub mod io;
