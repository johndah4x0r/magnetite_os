/*!
    Generic platform-agnostic structures

    The structures defined in the submodules are either fully
    or partially binary-compatible with the contained types
    (if they contain anything at all).
*/

// Volatile wrapper type
pub mod volatile;

// Array-like fat pointer type
pub mod array_like;

// Spin lock wrapper type
pub mod spin_lock;
