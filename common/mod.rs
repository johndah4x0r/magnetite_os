/*
    A collection of shared resources at the source level

    As per decision 2025-08-30, part 2, platform-specific
    definitions must be separated from shared defintions,
    and must be guarded off whenever possible.
*/

// ISA-specific definitions
pub mod arch;

// Platform-specific definitions
pub mod plat;

// Platform-agnostic definitions
pub mod shared;
