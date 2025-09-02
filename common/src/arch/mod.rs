/*
    Architecture-specific definitions

    As per decision 2025-08-30, part 2, platform-specific
    definitions must be separated from shared defintions,
    and must be guarded off whenever possible.

    Definitions must be guarded off, so that incompatible
    routines aren't included at compile time - though they
    must still be present at the source level.
*/

// Definitions specific to the IA-32 and x86-64
// platforms (known collectively as x86)
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86;
