/*
    Definitions that are shared by bootloader and
    kernel code, and are platform-agnostic by design.

    As per decision 2025-08-30, part 2, platform-specific
    definitions must be separated from shared defintions,
    and must be guarded off whenever possible.
*/

// Platform-agnostic macros
pub mod macros;

// General structures
pub mod structs;
