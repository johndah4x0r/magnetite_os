/*
    A collection of shared resources at the source level

    As per decision 2025-08-30, part 2, platform-specific
    definitions must be separated from shared defintions,
    and must be guarded off whenever possible.
*/

#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(associated_type_defaults)]

// Platform-agnostic definitions
pub mod shared;

// Platform-specific definitions
// - will expose auto-selected `_this`
pub mod arch;
