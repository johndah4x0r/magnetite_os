/*!
    Architecture-specific definitions

    As per decision 2025-08-30, part 2, platform-specific
    definitions must be separated from shared defintions,
    and must be guarded off whenever possible (though they
    must still be present at the source level).
*/

// Define macro for exposing platform-specific submodules
macro_rules! arch_mod {
    ($cfg:meta, $real_mod:ident) => {
        #[$cfg]
        pub mod $real_mod;

        #[$cfg]
        pub(crate) mod __arch {
            pub use super::$real_mod::*;
        }
    };
}

// Definitions specific to the IA-32 and x86-64
// platforms (known collectively as x86)
arch_mod!(cfg(any(target_arch = "x86", target_arch = "x86_64")), x86);
