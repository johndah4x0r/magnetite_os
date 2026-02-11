/*
    Architecture-specific definitions

    As per decision 2025-08-30, part 2, platform-specific
    definitions must be separated from shared defintions,
    and must be guarded off whenever possible.
*/

// Definitions specific to the PC platform
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod pc_bios;
