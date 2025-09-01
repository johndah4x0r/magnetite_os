/*
    A collection of shared resources at the source level

    As per decision 2025-08-30, part 2, platform-specific
    definitions must be separated from shared defintions,
    and must be guarded off whenever possible.
*/

// Platform-agnostic definitions
mod shared;

// Platform-specific definitions
mod arch;
